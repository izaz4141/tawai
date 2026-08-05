use std::collections::{HashMap, HashSet};
use std::io::{Seek, Write};
use std::sync::Arc;

use anyhow::{Context, Result};
use futures::StreamExt;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use tokio::sync::RwLock;

use crate::audio::tags::{self, AudioTag};
use crate::libsources::ParsedTrack;
use crate::signals::discovery::JellyfinLibraryInfo;
use crate::utils::logger;

const TICK_RATE: f64 = 10_000_000.0;
const DEVICE_INFO: &str =
    "MediaBrowser Client=\"Tawai\", Device=\"Tawai\", DeviceId=\"Tawai\", Version=\"1.0.0\"";

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct AuthResponse {
    access_token: String,
    user: AuthUser,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct AuthUser {
    id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ItemsResponse {
    items: Vec<ItemDto>,
    total_record_count: Option<i32>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ViewsResponse {
    items: Vec<ViewDto>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ViewDto {
    id: String,
    name: String,
    collection_type: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ItemDto {
    id: String,
    name: String,
    artists: Option<Vec<String>>,
    album_artist: Option<String>,
    album: Option<String>,
    index_number: Option<i32>,
    parent_index_number: Option<i32>,
    run_time_ticks: Option<i64>,
    media_sources: Option<Vec<MediaSourceDto>>,
    genres: Option<Vec<String>>,
    production_year: Option<i32>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct MediaSourceDto {
    id: String,
    bitrate: Option<i32>,
    container: Option<String>,
    size: Option<i64>,
}

pub struct JellyfinParser {
    client: reqwest::Client,
    auth_cache: RwLock<HashMap<String, (String, String)>>,
}

impl JellyfinParser {
    pub fn new(client: reqwest::Client) -> Self {
        Self {
            client,
            auth_cache: RwLock::new(HashMap::new()),
        }
    }

    /// Returns `(base_url, token, user_id, library_id)` for the given source URL,
    /// using a cached token if available.
    async fn get_or_authenticate(
        &self,
        url: &str,
    ) -> Result<(String, String, String, Option<String>)> {
        let (base_url, username, password, library_id) = parse_jellyfin_url(url)?;

        {
            let cache = self.auth_cache.read().await;
            if let Some((token, user_id)) = cache.get(&base_url) {
                return Ok((base_url, token.clone(), user_id.clone(), library_id));
            }
        }

        let (token, user_id) = self.authenticate(&base_url, &username, &password).await?;

        {
            let mut cache = self.auth_cache.write().await;
            cache.insert(base_url.clone(), (token.clone(), user_id.clone()));
        }

        Ok((base_url, token, user_id, library_id))
    }

    /// Cheap: return `jellyfin://{id}` paths for all audio items.
    /// No file downloads, no tag parsing.
    pub async fn enumerate_paths(&self, url: &str) -> Result<Vec<String>> {
        let (base_url, token, user_id, library_id) = self.get_or_authenticate(url).await?;
        let items = self
            .list_audio_items(&base_url, &user_id, &token, library_id.as_deref())
            .await?;
        Ok(items
            .into_iter()
            .map(|i| format!("jellyfin://{}", i.id))
            .collect())
    }

    /// Expensive: download + parse only the audio items whose
    /// `jellyfin://{id}` path is in `paths`.
    pub async fn scan_paths(&self, url: &str, paths: &[String]) -> Result<Vec<ParsedTrack>> {
        let (base_url, token, user_id, library_id) = self.get_or_authenticate(url).await?;
        let items = self
            .list_audio_items(&base_url, &user_id, &token, library_id.as_deref())
            .await?;

        let wanted: HashSet<&str> = paths.iter().map(|s| s.as_str()).collect();
        let items: Vec<ItemDto> = items
            .into_iter()
            .filter(|i| wanted.contains(format!("jellyfin://{}", i.id).as_str()))
            .collect();

        let tracks = futures::stream::iter(items)
            .map(|item| async {
                let result = self.download_and_extract(&item.id, &base_url, &token).await;
                (item, result)
            })
            .buffer_unordered(5)
            .filter_map(|(item, result)| {
                let base = base_url.clone();
                async move {
                    match result {
                        Ok((hash, tag, dur, sr, br, fsize)) => {
                            Some(build_track(item, tag, hash, dur, sr, br, fsize))
                        }
                        Err(e) => {
                            logger::warn(&format!(
                                "Failed to download/extract Jellyfin item {} ({}), fallback to API metadata: {}",
                                item.id, item.name, e
                            ));
                            Some(map_item_to_track(item, &base))
                        }
                    }
                }
            })
            .collect::<Vec<_>>()
            .await;

        Ok(tracks)
    }

    pub async fn scan_file(&self, url: &str, file_path: &str) -> Result<ParsedTrack> {
        let (base_url, token, _, _) = self.get_or_authenticate(url).await?;
        let item_id = file_path
            .strip_prefix("jellyfin://")
            .context("Invalid jellyfin file path")?
            .to_string();
        let result = self.download_and_extract(&item_id, &base_url, &token).await;
        match result {
            Ok((hash, tag, dur, sr, br, fsize)) => Ok(ParsedTrack {
                tag,
                file_path: file_path.to_string(),
                file_hash: Some(hash),
                duration_secs: dur,
                sample_rate: sr,
                bitrate: br,
                file_size: fsize,
            }),
            Err(e) => {
                logger::warn(&format!(
                    "Failed to download/extract Jellyfin item {}, fallback to API metadata: {}",
                    item_id, e
                ));
                let item = self
                    .fetch_item(&base_url, &token, &item_id)
                    .await?
                    .ok_or_else(|| anyhow::anyhow!("Item {} not found", item_id))?;
                Ok(map_item_to_track(item, &base_url))
            }
        }
    }

    pub async fn fetch_libraries(&self, url: &str) -> Result<Vec<JellyfinLibraryInfo>> {
        let (base_url, token, user_id, _) = self.get_or_authenticate(url).await?;

        let views_url = format!("{}/Users/{}/Views", base_url, user_id);
        let auth_header = format!("{}, Token=\"{}\"", DEVICE_INFO, token);
        let resp = self
            .client
            .get(&views_url)
            .header("Authorization", &auth_header)
            .send()
            .await?;

        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("Jellyfin views request failed: {}", text);
        }

        let data: ViewsResponse = resp.json().await?;

        let libraries: Vec<JellyfinLibraryInfo> = data
            .items
            .into_iter()
            .filter(|v| v.collection_type.as_deref() == Some("music"))
            .map(|v| JellyfinLibraryInfo {
                id: v.id,
                name: v.name,
            })
            .collect();

        Ok(libraries)
    }

    pub async fn resolve_stream_url(
        &self,
        file_path: &str,
        source_url: &str,
    ) -> Result<(String, Vec<(String, String)>)> {
        let item_id = file_path
            .strip_prefix("jellyfin://")
            .context("Invalid Jellyfin file path: missing jellyfin:// prefix")?;

        let (base_url, token, _, _) = self.get_or_authenticate(source_url).await?;

        let stream_url = format!("{}/Audio/{}/stream?static=true", base_url, item_id);
        let auth_header = format!("{}, Token=\"{}\"", DEVICE_INFO, token);

        Ok((stream_url, vec![("Authorization".to_string(), auth_header)]))
    }

    async fn authenticate(
        &self,
        base_url: &str,
        username: &str,
        password: &str,
    ) -> Result<(String, String)> {
        let url = format!("{}/Users/AuthenticateByName", base_url);
        let body = serde_json::json!({
            "Username": username,
            "Pw": password,
        });

        let resp = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("X-Emby-Authorization", DEVICE_INFO)
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("Jellyfin authentication failed: {}", text);
        }

        let data: AuthResponse = resp.json().await?;

        Ok((data.access_token, data.user.id))
    }

    async fn list_audio_items(
        &self,
        base_url: &str,
        user_id: &str,
        token: &str,
        library_id: Option<&str>,
    ) -> Result<Vec<ItemDto>> {
        let mut url = format!(
            "{}/Users/{}/Items?IncludeItemTypes=Audio&Recursive=true&Fields=Genres,MediaSources&Limit=500",
            base_url, user_id
        );

        if let Some(lid) = library_id {
            url = format!("{}&ParentId={}", url, lid);
        }

        let mut all_items = Vec::new();
        let mut start_index = 0;

        loop {
            let page_url = format!("{}&StartIndex={}", url, start_index);
            let auth_header = format!("{}, Token=\"{}\"", DEVICE_INFO, token);
            let resp = self
                .client
                .get(&page_url)
                .header("Authorization", &auth_header)
                .send()
                .await?;

            if !resp.status().is_success() {
                let text = resp.text().await.unwrap_or_default();
                anyhow::bail!("Jellyfin list items failed: {}", text);
            }

            let data: ItemsResponse = resp.json().await?;

            let count = data.items.len();
            all_items.extend(data.items);

            if count < 500 {
                break;
            }
            start_index += 500;
        }

        Ok(all_items)
    }

    async fn download_and_extract(
        &self,
        item_id: &str,
        base_url: &str,
        token: &str,
    ) -> Result<(String, AudioTag, f64, Option<u32>, Option<u32>, u64)> {
        let stream_url = format!("{}/Audio/{}/stream?static=true", base_url, item_id);
        let auth_header = format!("{}, Token=\"{}\"", DEVICE_INFO, token);

        let response = self
            .client
            .get(&stream_url)
            .header("Authorization", &auth_header)
            .send()
            .await?;

        if !response.status().is_success() {
            anyhow::bail!(
                "Jellyfin audio stream failed with status {}",
                response.status()
            );
        }

        let mut tmp = tempfile::NamedTempFile::new()?;
        let mut hasher = Sha256::new();
        let mut byte_stream = response.bytes_stream();

        while let Some(chunk) = byte_stream.next().await {
            let chunk = chunk?;
            hasher.update(&chunk);
            tmp.write_all(&chunk)?;
        }

        let file_hash = hex::encode(hasher.finalize());
        let file_size = std::fs::metadata(tmp.path()).map(|m| m.len()).unwrap_or(0);

        tmp.seek(std::io::SeekFrom::Start(0))?;

        let (tag, duration_secs, sample_rate, bitrate) = tags::read_audio_tags(tmp.path())?;

        Ok((
            file_hash,
            tag,
            duration_secs,
            sample_rate,
            bitrate,
            file_size,
        ))
    }

    async fn fetch_item(
        &self,
        base_url: &str,
        token: &str,
        item_id: &str,
    ) -> Result<Option<ItemDto>> {
        let (_, user_id) = {
            let cache = self.auth_cache.read().await;
            cache
                .get(base_url)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("No cached auth for {}", base_url))?
        };
        let url = format!("{}/Users/{}/Items/{}", base_url, user_id, item_id);
        let auth_header = format!("{}, Token=\"{}\"", DEVICE_INFO, token);
        let resp = self
            .client
            .get(&url)
            .header("Authorization", &auth_header)
            .send()
            .await?;
        if resp.status().is_success() {
            Ok(Some(resp.json().await?))
        } else {
            Ok(None)
        }
    }
}

fn build_track(
    item: ItemDto,
    tag: AudioTag,
    file_hash: String,
    duration_secs: f64,
    sample_rate: Option<u32>,
    bitrate: Option<u32>,
    file_size: u64,
) -> ParsedTrack {
    ParsedTrack {
        tag,
        file_path: format!("jellyfin://{}", item.id),
        file_hash: Some(file_hash),
        duration_secs,
        sample_rate,
        bitrate,
        file_size,
    }
}

fn map_item_to_track(item: ItemDto, _base_url: &str) -> ParsedTrack {
    let artist = item
        .artists
        .as_ref()
        .and_then(|a| a.first().cloned())
        .unwrap_or_default();
    let album_artist = item.album_artist.clone().unwrap_or_else(|| artist.clone());

    ParsedTrack {
        tag: AudioTag {
            title: item.name,
            artist,
            artist_sort: String::new(),
            album: item.album.unwrap_or_default(),
            album_artist,
            album_artist_sort: String::new(),
            genres: item.genres.unwrap_or_default(),
            release_date: item.production_year.map(|y| format!("{:04}", y)),
            track_number: item.index_number.unwrap_or(0),
            disc_number: item.parent_index_number.unwrap_or(1),
            mbid_recording: None,
            mbid_artist: None,
            mbid_release_artist: None,
            mbid_release: None,
            acoust_id: None,
            acoust_id_fingerprint: None,
            lyrics: None,
            cover: None,
            album_disambiguation: None,
            total_discs: 0,
            artists: Vec::new(),
            album_artists: Vec::new(),
            track_gain: None,
            track_peak: None,
        },
        file_path: format!("jellyfin://{}", item.id),
        file_hash: None,
        duration_secs: item
            .run_time_ticks
            .map(|t| t as f64 / TICK_RATE)
            .unwrap_or(0.0),
        sample_rate: None,
        bitrate: item
            .media_sources
            .as_ref()
            .and_then(|s| s.first())
            .and_then(|s| s.bitrate)
            .map(|b| b as u32),
        file_size: item
            .media_sources
            .as_ref()
            .and_then(|s| s.first())
            .and_then(|s| s.size)
            .unwrap_or(0) as u64,
    }
}

fn parse_jellyfin_url(input: &str) -> Result<(String, String, String, Option<String>)> {
    // Format: http://username:password@host:port[?libraryId=xxx]
    let without_protocol = input
        .strip_prefix("http://")
        .or_else(|| input.strip_prefix("https://"))
        .unwrap_or(input);

    let (credentials, host_part) = match without_protocol.split_once('@') {
        Some((creds, rest)) => (creds, rest),
        None => anyhow::bail!("Jellyfin URL must include user:password@host"),
    };

    let (username, password) = match credentials.split_once(':') {
        Some((u, p)) => (u.to_string(), p.to_string()),
        None => anyhow::bail!("Jellyfin URL must include both username and password"),
    };

    let scheme = if input.starts_with("https") {
        "https"
    } else {
        "http"
    };

    let (host_and_port, library_id) = if let Some(idx) = host_part.find('?') {
        let query = &host_part[idx + 1..];
        let base = &host_part[..idx];
        let lid = query.split('&').find_map(|pair| {
            let mut parts = pair.splitn(2, '=');
            match (parts.next(), parts.next()) {
                (Some("libraryId"), Some(val)) if !val.is_empty() => Some(val.to_string()),
                _ => None,
            }
        });
        (base, lid)
    } else {
        (host_part, None)
    };

    let base_url = format!("{}://{}", scheme, host_and_port);

    Ok((base_url, username, password, library_id))
}
