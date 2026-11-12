use std::path::Path;
use std::sync::Arc;

use axum::{
    Json,
    extract::{Extension, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};

use tawai_core::audio::ffmpeg::DASH_SEGMENT_DURATION_SECS;
use tawai_core::audio::probe::{AudioProbe, probe_dash_artifacts, probe_file, probe_url};
use tawai_core::dclient::iprev;
use tawai_core::signals::library::TrackInfo;
use tawai_core::signals::playback::{ProbeStreamRequest, ProbeStreamResponse};
use tawai_core::utils::playback::resolve_playable_track;

use crate::server::{SharedState, dash_cache_dir_for_track};

use super::play::transcode_bitrate;
use super::stream::{ensure_dash_init_generated, ensure_dash_segment_generated};
use super::{TrackAccess, user_can_read_track};

fn is_url(value: &str) -> bool {
    value.starts_with("http://") || value.starts_with("https://")
}

async fn probe_local_file(path: String) -> anyhow::Result<AudioProbe> {
    tokio::task::spawn_blocking(move || probe_file(Path::new(&path)))
        .await
        .map_err(|error| anyhow::anyhow!("probe task failed: {error}"))?
}

#[utoipa::path(
    post,
    path = "/api/tawai/playback/probe",
    tags = ["tawai.playback"],
    security(("ApiKeyAuth" = [])),
    responses(
        (status = 200, description = "Detected stream metadata"),
        (status = 403, description = "Forbidden: user cannot access the track's source"),
        (status = 404, description = "Track not found")
    )
)]
pub async fn handle_probe_stream(
    State(state): State<SharedState>,
    Extension(user_id): Extension<String>,
    headers: HeaderMap,
    Json(req): Json<ProbeStreamRequest>,
) -> impl IntoResponse {
    if let Some(track) = &req.track {
        if track.source == "preview" || track.source_type == "preview" {
            return Json(probe_preview(&state, req.id, track).await).into_response();
        }
        if track.source == "external" || track.source_type == "external" {
            return Json(ProbeStreamResponse {
                id: req.id,
                mode: Some("external".to_string()),
                info: None,
                error: Some("External files are probed on the local device".to_string()),
            })
            .into_response();
        }
    }

    let db = state.context.db().await;
    let cfg = state.context.cfg().await;
    let master_key = state.context.master_key.read().await.clone();

    let result = resolve_playable_track(
        db.pool(),
        state.context.client(),
        req.track_id.as_deref(),
        req.track.as_ref().map(|track| track.title.as_str()),
        req.track
            .as_ref()
            .map(|track| track.artists_string.as_str()),
        req.track.as_ref().map(|track| track.album_title.as_str()),
        req.track
            .as_ref()
            .and_then(|track| track.mbid_recording.as_deref()),
        Some(&cfg),
        &master_key,
        Arc::downgrade(&state.context),
    )
    .await;

    if let Some(track_id) = &result.resolved_track_id {
        match user_can_read_track(db.pool(), &user_id, track_id, &master_key).await {
            TrackAccess::Allowed => {}
            TrackAccess::NoSource | TrackAccess::NoUser => {
                return (
                    StatusCode::NOT_FOUND,
                    Json(serde_json::json!({"error": "Track not found"})),
                )
                    .into_response();
            }
            TrackAccess::Denied => {
                return (
                    StatusCode::FORBIDDEN,
                    Json(serde_json::json!({
                        "error": "User does not have access to this track"
                    })),
                )
                    .into_response();
            }
        }
    }

    if let Some(error) = result.error {
        let status = if error == "Track not found" || error == "No audio source found" {
            StatusCode::NOT_FOUND
        } else {
            StatusCode::INTERNAL_SERVER_ERROR
        };
        return (
            status,
            Json(ProbeStreamResponse {
                id: req.id,
                mode: None,
                info: None,
                error: Some(error),
            }),
        )
            .into_response();
    }

    let bitrate = transcode_bitrate(&headers, req.bitrate.as_deref());
    // Mirror `/play`: only a resolved, non-URL file is turned into DASH.
    let transcode =
        bitrate.is_some() && result.resolved_track_id.is_some() && !is_url(&result.file_path);

    let (mode, probe) = if transcode {
        let Some(track_id) = result.resolved_track_id.clone() else {
            return Json(ProbeStreamResponse {
                id: req.id,
                mode: Some("transcode".to_string()),
                info: None,
                error: Some("Track not found".to_string()),
            })
            .into_response();
        };
        let bitrate_kbps = bitrate.unwrap_or_default();
        let directory = dash_cache_dir_for_track(&cfg, &track_id);
        let init_path = format!("{directory}/init-{}.m4s", bitrate_kbps * 1000);
        let segment_path = format!("{directory}/seg-{}-1.m4s", bitrate_kbps * 1000);

        if let Err(error) = ensure_dash_init_generated(
            &state,
            &track_id,
            &result.file_path,
            &directory,
            bitrate_kbps,
        )
        .await
        {
            tawai_core::utils::logger::error(&format!(
                "dash probe init generation failed: {error}"
            ));
            return Json(ProbeStreamResponse {
                id: req.id,
                mode: Some("transcode".to_string()),
                info: None,
                error: Some(error.to_string()),
            })
            .into_response();
        }

        if let Err(error) = ensure_dash_segment_generated(
            &state,
            &track_id,
            &result.file_path,
            &directory,
            bitrate_kbps,
            1,
        )
        .await
        {
            tawai_core::utils::logger::error(&format!(
                "dash probe segment generation failed: {error}"
            ));
            return Json(ProbeStreamResponse {
                id: req.id,
                mode: Some("transcode".to_string()),
                info: None,
                error: Some(error.to_string()),
            })
            .into_response();
        }

        let segment_duration = req
            .track
            .as_ref()
            .and_then(|track| track.duration_secs)
            .map(|duration| duration.min(DASH_SEGMENT_DURATION_SECS as f64));
        (
            "transcode".to_string(),
            probe_dash_artifacts(
                Path::new(&init_path),
                Path::new(&segment_path),
                segment_duration,
            )
            .await,
        )
    } else if is_url(&result.file_path) {
        let remote_headers = result.headers.clone().unwrap_or_default();
        (
            "direct".to_string(),
            probe_url(&result.file_path, &remote_headers).await,
        )
    } else if !result.file_path.is_empty() {
        (
            "direct".to_string(),
            probe_local_file(result.file_path.clone()).await,
        )
    } else {
        return Json(ProbeStreamResponse {
            id: req.id,
            mode: None,
            info: None,
            error: Some("No audio source found".to_string()),
        })
        .into_response();
    };

    match probe {
        Ok(info) => Json(ProbeStreamResponse {
            id: req.id,
            mode: Some(mode),
            info: Some(info),
            error: None,
        })
        .into_response(),
        Err(error) => {
            tawai_core::utils::logger::error(&format!("stream probe failed: {error}"));
            Json(ProbeStreamResponse {
                id: req.id,
                mode: Some(mode),
                info: None,
                error: Some(error.to_string()),
            })
            .into_response()
        }
    }
}

async fn probe_preview(state: &SharedState, id: String, track: &TrackInfo) -> ProbeStreamResponse {
    match iprev::fetch_preview(state.context.client(), &track.artists_string, &track.title).await {
        Ok(Some(url)) => match probe_url(&url, &[]).await {
            Ok(mut info) => {
                info.transport = Some("preview".to_string());
                ProbeStreamResponse {
                    id,
                    mode: Some("preview".to_string()),
                    info: Some(info),
                    error: None,
                }
            }
            Err(error) => ProbeStreamResponse {
                id,
                mode: Some("preview".to_string()),
                info: None,
                error: Some(error.to_string()),
            },
        },
        Ok(None) => ProbeStreamResponse {
            id,
            mode: Some("preview".to_string()),
            info: None,
            error: Some("No preview available".to_string()),
        },
        Err(error) => ProbeStreamResponse {
            id,
            mode: Some("preview".to_string()),
            info: None,
            error: Some(error.to_string()),
        },
    }
}
