use std::collections::HashSet;
use std::path::Path;
use std::time::Duration;

use anyhow::Result;
use futures::StreamExt;
use tokio::io::AsyncWriteExt;
use tokio::time::sleep;

use crate::audio::tags::{derive_sort_name, parse_artists, AudioTag};
use crate::db::account::DEFAULT_USERNAME;
use crate::db::user_settings;
use crate::db::{database::DatabasePool, library, library_source};
use crate::dclient::nadekodon::resolve_audio_format;
use crate::discovery::listenbrainz;
use crate::metadata::musicbrainz;
use crate::signals::library::TrackInfo;
use crate::signals::metadata::RecordingInfo;
use crate::tools::duplicates;
use crate::tools::rename::{dest_from_root, DEFAULT_PATTERN};
use crate::utils::config::AppConfig;
use crate::utils::logger;

const MAX_TRACKS: usize = 25;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ApiType {
    Recommendations,
    CreatedFor,
}

#[derive(Debug, Clone, Copy)]
pub struct RecommendationSource {
    pub rec_src: &'static str,
    pub rec_type: &'static str,
    pub display_name: &'static str,
    pub source_type: &'static str,
    pub api_type: ApiType,
    pub api_rec_type: &'static str,
    pub refresh_interval: Duration,
}

pub static ALL_RECOMMENDATION_SOURCES: &[RecommendationSource] = &[
    RecommendationSource {
        rec_src: "listenbrainz",
        rec_type: "weekly-explo",
        display_name: "Weekly Exploration",
        source_type: "recommendation:listenbrainz_weekly-explo",
        api_type: ApiType::CreatedFor,
        api_rec_type: "weekly-exploration",
        refresh_interval: Duration::from_secs(7 * 24 * 60 * 60),
    },
    RecommendationSource {
        rec_src: "listenbrainz",
        rec_type: "year",
        display_name: "Year in Music",
        source_type: "recommendation:listenbrainz_year",
        api_type: ApiType::CreatedFor,
        api_rec_type: "year",
        refresh_interval: Duration::from_secs(30 * 24 * 60 * 60),
    },
    RecommendationSource {
        rec_src: "listenbrainz",
        rec_type: "weekly",
        display_name: "Weekly",
        source_type: "recommendation:listenbrainz_weekly",
        api_type: ApiType::CreatedFor,
        api_rec_type: "weekly",
        refresh_interval: Duration::from_secs(7 * 24 * 60 * 60),
    },
    RecommendationSource {
        rec_src: "listenbrainz",
        rec_type: "daily",
        display_name: "Daily",
        source_type: "recommendation:listenbrainz_daily",
        api_type: ApiType::CreatedFor,
        api_rec_type: "daily",
        refresh_interval: Duration::from_secs(24 * 60 * 60),
    },
    RecommendationSource {
        rec_src: "listenbrainz",
        rec_type: "top",
        display_name: "Top Recommendations",
        source_type: "recommendation:listenbrainz_top",
        api_type: ApiType::Recommendations,
        api_rec_type: "top",
        refresh_interval: Duration::from_secs(7 * 24 * 60 * 60),
    },
    RecommendationSource {
        rec_src: "listenbrainz",
        rec_type: "raw",
        display_name: "Raw Recommendations",
        source_type: "recommendation:listenbrainz_raw",
        api_type: ApiType::Recommendations,
        api_rec_type: "raw",
        refresh_interval: Duration::from_secs(7 * 24 * 60 * 60),
    },
    RecommendationSource {
        rec_src: "listenbrainz",
        rec_type: "similar",
        display_name: "Similar Artists",
        source_type: "recommendation:listenbrainz_similar",
        api_type: ApiType::Recommendations,
        api_rec_type: "similar",
        refresh_interval: Duration::from_secs(7 * 24 * 60 * 60),
    },
];

impl RecommendationSource {
    pub fn from_key(key: &str) -> Option<&'static Self> {
        let rec_type = key.strip_prefix("recommendation:")?;
        ALL_RECOMMENDATION_SOURCES.iter().find(|s| {
            let full = format!("{}_{}", s.rec_src, s.rec_type);
            rec_type == full
        })
    }

    pub fn from_api_rec_type(api_rec_type: &str) -> Option<&'static Self> {
        ALL_RECOMMENDATION_SOURCES
            .iter()
            .find(|s| s.api_rec_type == api_rec_type)
    }

    pub fn display_name_with_user(&self, username: &str) -> String {
        format!("{} ({})", self.display_name, username)
    }

    pub fn source_url(&self) -> String {
        format!("{}://{}", self.rec_src, self.api_rec_type)
    }

    /// For CreatedFor sources: the filter type to pass to fetch_createdfor.
    /// Year maps to "discoveries", others use api_rec_type directly.
    pub fn created_for_filter(&self) -> &'static str {
        debug_assert_eq!(self.api_type, ApiType::CreatedFor);
        if self.rec_type == "year" {
            "discoveries"
        } else {
            self.api_rec_type
        }
    }
}

// ── SourceParser integration ──────────────────────────────────────────────

/// Enumerate existing recommendation track paths from the database.
pub async fn enumerate_paths(
    pool: &DatabasePool,
    source: &RecommendationSource,
    master_key: &str,
) -> Result<Vec<String>> {
    let all_sources = library_source::list_all_sources(pool, master_key).await?;
    let source_id = all_sources
        .iter()
        .find(|s| s.source_type == source.source_type)
        .map(|s| s.id.as_str())
        .ok_or_else(|| anyhow::anyhow!("source not found for {}", source.source_type))?;

    let tracks = library::list_tracks_by_source(pool, source_id).await?;
    Ok(tracks.into_iter().map(|t| t.file_path).collect())
}

/// Resolve a playable stream URL for a recommendation track via nadekodon (yt-dlp).
pub async fn resolve_stream_url(
    pool: &DatabasePool,
    file_path: &str,
    client: &reqwest::Client,
    cfg: Option<&AppConfig>,
) -> Result<(String, Vec<(String, String)>)> {
    let track = library::lookup_track_by_file_path(pool, file_path)
        .await?
        .ok_or_else(|| anyhow::anyhow!("track not found for {}", file_path))?;

    let cfg = cfg.ok_or_else(|| anyhow::anyhow!("config not available"))?;
    match crate::dclient::nadekodon::resolve_audio_url(cfg, client, &track.artists_string, &track.title).await {
        Ok(Some(url)) => Ok((url, vec![])),
        Ok(None) => Err(anyhow::anyhow!("no audio source found")),
        Err(e) => Err(e),
    }
}

/// Delete a recommendation track from the library.
pub async fn delete(pool: &DatabasePool, file_path: &str) -> Result<()> {
    library::delete_track_by_file_path(pool, file_path).await
}

/// Download a recommendation track: resolve the single direct audio URL via
/// nadekodon's yt-dlp query (no nadekodon download manager — the stream is one
/// audio URL, no muxing needed), fetch it directly, write it to the final path
/// built from the configured naming pattern plus the real extension, embed the
/// track's tags, and register a completed download row. Returns the download ID.
/// When the track has no library row (discovery previews), `extra` supplies
/// `{"title", "artist", "mbid", "album"}` as the metadata fallback.
pub async fn download(
    pool: &DatabasePool,
    file_path: &str,
    dest_path: &str,
    client: &reqwest::Client,
    cfg: &AppConfig,
    user_id: &str,
    extra: Option<&str>,
    master_key: &str,
) -> Result<String> {
    let track = match library::lookup_track_by_file_path(pool, file_path).await? {
        Some(track) => track,
        None => parse_extra_track(file_path, extra)
            .ok_or_else(|| anyhow::anyhow!("track not found for {}", file_path))?,
    };

    // The destination must be an editable library source root — we cannot place
    // downloaded files onto jellyfin or recommendation sources.
    let dest = Path::new(dest_path);
    let editable_sources = library_source::list_all_sources(pool, master_key).await?;
    let local_source = editable_sources.iter().find(|s| {
        crate::libsources::is_editable(&s.source_type)
            && s.urls.iter().any(|u| {
                if s.source_type == "tawai" && u.starts_with("tawai://") {
                    return false;
                }
                let root = Path::new(u);
                dest == root || dest.starts_with(root)
            })
    });
    let Some(local_source) = local_source else {
        anyhow::bail!("destination must be an editable library source: {dest_path}");
    };

    let format = resolve_audio_format(cfg, client, &track.artists_string, &track.title)
        .await?
        .ok_or_else(|| anyhow::anyhow!("no audio stream found for {}", file_path))?;

    let tag = audio_tag_for_track(&track);
    let pattern = user_settings::get_setting(pool, DEFAULT_USERNAME, "identify_naming_pattern")
        .await
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| DEFAULT_PATTERN.to_string());
    let fallback_stem = format!("{} - {}", track.artists_string, track.title);
    let ext = if format.ext.is_empty() { "mp3" } else { &format.ext };
    let final_path = dest_from_root(dest_path, &pattern, &tag, ext, &fallback_stem);

    if let Some(parent) = final_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut resp = client.get(&format.url).send().await?;
    if !resp.status().is_success() {
        anyhow::bail!(
            "audio stream fetch failed: {} for {}",
            resp.status(),
            format.url
        );
    }

    let mut file = tokio::fs::File::create(&final_path).await?;
    let mut stream = resp.bytes_stream();
    let mut total: u64 = 0;
    let stream_result: Result<()> = async {
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            total += chunk.len() as u64;
            file.write_all(&chunk).await?;
        }
        file.flush().await?;
        Ok(())
    }
    .await;
    if let Err(e) = stream_result {
        let _ = std::fs::remove_file(&final_path);
        return Err(e);
    }
    if total == 0 {
        let _ = std::fs::remove_file(&final_path);
        anyhow::bail!("downloaded file is empty: {}", final_path.display());
    }

    if let Err(e) = crate::audio::tags::write_audio_tags(&final_path, &tag) {
        logger::warn(&format!(
            "failed to write tags to {}: {}",
            final_path.display(),
            e
        ));
    }

    let final_str = final_path.to_string_lossy().to_string();
    let fname = final_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(&final_str)
        .to_string();

    // For tawai destinations the file must land in the actual remote library
    // dir too — push the finished local backup up to the remote server. A
    // backup-only download is not acceptable, so remove the local file if the
    // remote import fails.
    if local_source.source_type == "tawai" {
        let conn = crate::libsources::tawai::pick_remote(&local_source.urls, client)
            .await
            .ok_or_else(|| anyhow::anyhow!("no reachable tawai server for destination"))?;
        let bytes = tokio::fs::read(&final_path).await?;
        if let Err(e) =
            crate::libsources::tawai::upload_to_remote(client, &conn, bytes, &fname).await
        {
            let _ = std::fs::remove_file(&final_path);
            anyhow::bail!("failed to add download to remote tawai library: {e}");
        }
    }

    let download_id = crate::db::download::insert_download(
        pool,
        user_id,
        "nadekodon",
        "",
        &format.url,
        &final_str,
        &fname,
    )
    .await?;
    crate::db::download::update_download_state(
        pool,
        &download_id,
        "completed",
        "",
        total as i64,
        total as i64,
    )
    .await?;

    // Refresh the local library source so the new file shows up immediately.
    let scan_result = crate::audio::scan::run_scan(
        pool,
        client.clone(),
        std::slice::from_ref(local_source),
        false,
        None,
        master_key,
        None,
    )
    .await;
    if let Some(err) = scan_result.error {
        logger::warn(&format!(
            "failed to scan library source '{}' after download: {}",
            local_source.name, err
        ));
    }

    Ok(download_id)
}

/// Build a synthetic `TrackInfo` for tracks without a library row (e.g.
/// discovery previews), from the `extra` JSON payload sent by the client:
/// `{"title", "artist", "mbid", "album"}`.
fn parse_extra_track(file_path: &str, extra: Option<&str>) -> Option<TrackInfo> {
    let extra = extra?;
    let v: serde_json::Value = serde_json::from_str(extra).ok()?;
    let title = v["title"].as_str()?.to_string();
    let artist = v["artist"].as_str().unwrap_or("Unknown Artist").to_string();
    let mbid = v["mbid"].as_str().map(String::from);
    let album = v["album"].as_str().map(String::from).unwrap_or_default();
    Some(TrackInfo {
        id: mbid.clone().unwrap_or_else(|| file_path.to_string()),
        title,
        album_id: String::new(),
        album_title: album,
        artists: vec![],
        artists_string: artist,
        track_num: None,
        disc_num: None,
        duration_secs: None,
        file_path: file_path.to_string(),
        file_size: None,
        bitrate: None,
        mbid_recording: mbid.clone(),
        artist_mbid: None,
        album_mbid: None,
        lyrics: None,
        release_date: None,
        track_gain: None,
        track_peak: None,
        source: String::new(),
        source_type: String::new(),
        genres: vec![],
        file_hash: None,
        sample_rate: None,
        acoust_id_fingerprint: None,
        acoust_id: None,
    })
}

/// Build an audio tag from the recommendation track's metadata so the
/// downloaded file carries the same tags as the source track.
fn audio_tag_for_track(t: &TrackInfo) -> AudioTag {
    let mut tag = AudioTag {
        title: t.title.clone(),
        artist: t.artists_string.clone(),
        artists: t.artists.iter().map(|a| a.name.clone()).collect(),
        album: t.album_title.clone(),
        album_artist: t.artists_string.clone(),
        album_artists: t.artists.iter().map(|a| a.name.clone()).collect(),
        genres: t.genres.clone(),
        release_date: t.release_date.clone(),
        track_number: t.track_num.unwrap_or(0),
        disc_number: t.disc_num.unwrap_or(0),
        mbid_recording: t.mbid_recording.clone(),
        mbid_artist: t.artist_mbid.clone(),
        mbid_release: t.album_mbid.clone(),
        lyrics: t.lyrics.clone(),
        track_gain: t.track_gain,
        track_peak: t.track_peak,
        ..Default::default()
    };
    if tag.artists.is_empty() {
        tag.artists = parse_artists(&tag.artist);
    }
    if tag.album_artists.is_empty() {
        tag.album_artists = parse_artists(&tag.album_artist);
    }
    if tag.artist_sort.is_empty() {
        tag.artist_sort = derive_sort_name(&tag.artist);
    }
    if tag.album_artist_sort.is_empty() {
        tag.album_artist_sort = derive_sort_name(&tag.album_artist);
    }
    tag
}

/// Sync a single recommendation source: fetch fresh MBIDs from ListenBrainz,
/// diff against existing tracks, and insert new ones.
pub async fn sync(
    pool: &DatabasePool,
    source: &RecommendationSource,
    source_id: &str,
    client: &reqwest::Client,
    token: &str,
    user_name: &str,
) -> Result<(u32, u32)> {
    let recs = fetch_recording_mbids(client, token, user_name, source)
        .await
        .map_err(|e| anyhow::anyhow!(e))?;

    let fresh_mbids: HashSet<&str> = recs.iter().map(|r| r.id.as_str()).collect();

    let existing_tracks = library::list_tracks_by_source(pool, source_id).await?;
    let existing_mbids: HashSet<&str> = existing_tracks
        .iter()
        .filter_map(|t| t.mbid_recording.as_deref())
        .collect();

    if existing_mbids == fresh_mbids {
        library_source::touch_source_sync_at(pool, source_id)
            .await
            .ok();
        return Ok((0, 0));
    }

    let removed_count = existing_mbids.len() as u32;

    if let Err(e) = library::delete_tracks_by_source_id(pool, source_id).await {
        logger::error(&format!("failed to delete old rec tracks: {}", e));
    }

    let mut to_insert: Vec<&RecordingInfo> = Vec::new();
    let mut existing_ids: Vec<String> = Vec::new();
    for rec in &recs {
        let artist_name = if rec.artist.is_empty() {
            "Unknown Artist"
        } else {
            &rec.artist
        };
        match duplicates::find_track_by_recording(pool, Some(&rec.id), &rec.title, artist_name)
            .await
        {
            Ok(Some(track_id)) => existing_ids.push(track_id),
            _ => to_insert.push(rec),
        }
    }

    let mut added = 0u32;
    for rec in &to_insert {
        let Some(enriched) = fetch_recording_mb_data(client, &rec.id).await else {
            continue;
        };
        if let Some(track_id) = dedup_or_insert_track(pool, rec, &enriched, source_id).await {
            added += 1;
        }
        sleep(Duration::from_millis(1100)).await;
    }

    added += existing_ids.len() as u32;

    library_source::touch_source_sync_at(pool, source_id)
        .await
        .ok();

    Ok((added, removed_count))
}

// ── Internal helpers ──────────────────────────────────────────────────────

async fn fetch_recording_mbids(
    client: &reqwest::Client,
    token: &str,
    user_name: &str,
    source: &RecommendationSource,
) -> Result<Vec<RecordingInfo>, String> {
    match source.api_type {
        ApiType::Recommendations => listenbrainz::fetch_recommendations(
            client,
            token,
            user_name,
            source.api_rec_type,
            Some(MAX_TRACKS as i32),
            None,
        )
        .await
        .map_err(|e| e.to_string()),
        ApiType::CreatedFor => {
            listenbrainz::fetch_createdfor(client, token, user_name, source.created_for_filter(), 0)
                .await
                .map(|cr| cr.recordings)
                .map_err(|e| e.to_string())
        }
    }
}

async fn fetch_recording_mb_data(client: &reqwest::Client, mbid: &str) -> Option<RecordingInfo> {
    for attempt in 0..3 {
        if attempt > 0 {
            sleep(Duration::from_millis(500)).await;
        }
        match musicbrainz::fetch_recording(client, mbid).await {
            Ok(e) => return Some(e),
            Err(e) => {
                logger::debug(&format!(
                    "attempt {} failed for {}: {}",
                    attempt + 1,
                    mbid,
                    e
                ));
            }
        }
    }
    None
}

async fn dedup_or_insert_track(
    pool: &DatabasePool,
    rec: &RecordingInfo,
    enriched: &RecordingInfo,
    source_id: &str,
) -> Option<String> {
    let mbid = &rec.id;
    let file_path = format!("recommendation://{}/{}", source_id, mbid);

    let artist_mbid = enriched.artist_id.clone();
    let artist_name = if enriched.artist.is_empty() {
        "Unknown Artist"
    } else {
        &enriched.artist
    };
    let artist_id = match library::insert_artist(pool, artist_name, artist_name, artist_mbid).await
    {
        Ok(id) => id,
        Err(e) => {
            logger::error(&format!("failed to insert artist: {}", e));
            return None;
        }
    };

    let release = enriched.releases.first();
    let album_title = release.map(|r| r.title.as_str()).unwrap_or("Unknown Album");
    let album_mbid = release.map(|r| r.id.clone());
    let release_date = release.and_then(|r| r.date.clone());
    let disambiguation = release.and_then(|r| r.disambiguation.clone());
    let total_discs = release.and_then(|r| r.total_discs).unwrap_or(0);

    let album_id = match library::insert_album(
        pool,
        album_title,
        &artist_id,
        release_date,
        album_mbid,
        None,
        disambiguation,
        total_discs,
    )
    .await
    {
        Ok(id) => id,
        Err(e) => {
            logger::error(&format!("failed to insert album: {}", e));
            return None;
        }
    };

    library::insert_album_artists(pool, &album_id, &[artist_id.clone()])
        .await
        .ok();

    let cover: Option<Vec<u8>> = match &enriched.cover {
        Some(url) => match reqwest::get(url).await {
            Ok(resp) if resp.status().is_success() => resp.bytes().await.ok().map(|b| b.to_vec()),
            _ => None,
        },
        None => None,
    };
    let track_title = &enriched.title;
    let duration = enriched.duration_secs.unwrap_or(0.0);

    let track_id = match library::insert_track(
        pool,
        track_title,
        &album_id,
        &artist_id,
        1,
        1,
        duration,
        &file_path,
        None,
        source_id,
        None,
        None,
        None,
        Some(mbid.clone()),
        cover.as_deref(),
        None,
        None,
        None,
    )
    .await
    {
        Ok(id) => id,
        Err(e) => {
            logger::error(&format!("failed to insert track: {}", e));
            return None;
        }
    };

    library::insert_track_artists(pool, &track_id, &[artist_id])
        .await
        .ok();

    Some(track_id)
}
