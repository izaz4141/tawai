use crate::db::{database::DatabasePool, library, library_source};
use crate::dclient::nadekodon;
use crate::libsources::get_parser;
use crate::signals::library::TrackInfo;
use crate::tools::duplicates;
use crate::utils::config::AppConfig;
use crate::utils::logger;

pub struct PlayTrackResult {
    pub resolved_track_id: Option<String>,
    pub file_path: String,
    pub headers: Option<Vec<(String, String)>>,
    pub error: Option<String>,
}

pub async fn resolve_track_source(
    file_path: &str,
    source_type: &str,
    source_url: &str,
    client: &reqwest::Client,
) -> (String, Option<Vec<(String, String)>>) {
    if file_path.starts_with("recommendation://") {
        return (String::new(), None);
    }
    if !file_path.starts_with("jellyfin://") {
        return (file_path.to_string(), None);
    }
    let parser = match get_parser(source_type, client.clone()) {
        Some(p) => p,
        None => return (file_path.to_string(), None),
    };
    match parser.resolve_stream_url(file_path, source_url).await {
        Ok((url, headers)) => (url, Some(headers)),
        Err(e) => {
            logger::error(&format!("resolve stream URL failed: {}", e));
            (file_path.to_string(), None)
        }
    }
}

async fn resolve_and_fallback(
    pool: &DatabasePool,
    client: &reqwest::Client,
    track: &TrackInfo,
    cfg: Option<&AppConfig>,
) -> PlayTrackResult {
    let (source_type, source_url) = library_source::get_source_by_track_id(pool, &track.id)
        .await
        .ok()
        .flatten()
        .unwrap_or_default();
    let (path, headers) =
        resolve_track_source(&track.file_path, &source_type, &source_url, client).await;

    if !path.is_empty() {
        return PlayTrackResult {
            resolved_track_id: Some(track.id.clone()),
            file_path: path,
            headers,
            error: None,
        };
    }

    match cfg {
        Some(cfg) => {
            match nadekodon::resolve_audio_url(cfg, client, &track.artists_string, &track.title)
                .await
            {
                Ok(Some(url)) => PlayTrackResult {
                    resolved_track_id: Some(track.id.clone()),
                    file_path: url,
                    headers: None,
                    error: None,
                },
                _ => PlayTrackResult {
                    resolved_track_id: Some(track.id.clone()),
                    file_path: String::new(),
                    headers: None,
                    error: Some("No audio source found".to_string()),
                },
            }
        }
        None => PlayTrackResult {
            resolved_track_id: Some(track.id.clone()),
            file_path: String::new(),
            headers: None,
            error: Some("No audio source found".to_string()),
        },
    }
}

pub async fn resolve_playable_track(
    pool: &DatabasePool,
    client: &reqwest::Client,
    track_id: Option<&str>,
    title: Option<&str>,
    artists_string: Option<&str>,
    album_title: Option<&str>,
    mbid_recording: Option<&str>,
    cfg: Option<&AppConfig>,
) -> PlayTrackResult {
    if let Some(tid) = track_id {
        match library::lookup_track(pool, tid).await {
            Ok(Some(t)) => return resolve_and_fallback(pool, client, &t, cfg).await,
            Ok(None) => {}
            Err(e) => {
                return PlayTrackResult {
                    resolved_track_id: None,
                    file_path: String::new(),
                    headers: None,
                    error: Some(e.to_string()),
                };
            }
        }
    }

    let (Some(t), Some(a), Some(al)) = (title, artists_string, album_title) else {
        return PlayTrackResult {
            resolved_track_id: None,
            file_path: String::new(),
            headers: None,
            error: Some("Track not found".to_string()),
        };
    };

    match duplicates::find_matching_track(pool, t, a, Some(al), mbid_recording, 0.9).await {
        Ok(Some((matched_track, _))) => {
            return resolve_and_fallback(pool, client, &matched_track, cfg).await;
        }
        Ok(None) => {}
        Err(e) => {
            logger::error(&format!("find matching track failed: {}", e));
        }
    }

    match cfg {
        Some(cfg) => match nadekodon::resolve_audio_url(cfg, client, a, t).await {
            Ok(Some(url)) => PlayTrackResult {
                resolved_track_id: None,
                file_path: url,
                headers: None,
                error: None,
            },
            _ => PlayTrackResult {
                resolved_track_id: None,
                file_path: String::new(),
                headers: None,
                error: Some("No audio source found".to_string()),
            },
        },
        None => PlayTrackResult {
            resolved_track_id: None,
            file_path: String::new(),
            headers: None,
            error: Some("No audio source found".to_string()),
        },
    }
}
