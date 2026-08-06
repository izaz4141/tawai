use axum::{
    Json,
    body::Body,
    extract::{Extension, Path, Query, State},
    http::{StatusCode, header},
    response::IntoResponse,
};
use serde::Deserialize;
use tawai_core::audio::ffmpeg;
use tawai_core::db::{library, library_source};
use tawai_core::utils::playback::resolve_track_source;
use tokio::fs::File;
use tokio_util::io::ReaderStream;

use crate::server::SharedState;

#[derive(Deserialize)]
pub struct StreamQuery {
    pub bitrate: Option<String>,
}

fn content_type(path: &str) -> &'static str {
    if path.ends_with(".mp3") {
        "audio/mpeg"
    } else if path.ends_with(".flac") {
        "audio/flac"
    } else if path.ends_with(".ogg") {
        "audio/ogg"
    } else if path.ends_with(".wav") {
        "audio/wav"
    } else if path.ends_with(".m4a") || path.ends_with(".aac") {
        "audio/mp4"
    } else if path.ends_with(".opus") {
        "audio/opus"
    } else if path.ends_with(".webm") {
        "audio/webm"
    } else {
        "application/octet-stream"
    }
}

#[utoipa::path(
    get,
    path = "/api/tawai/playback/stream/{id}",
    tags = ["tawai.playback"],
    security(("TokenQueryAuth" = [])),
    params(
        ("id" = String, Path, description = "Track ID"),
        ("bitrate" = Option<String>, Query, description = "Preferred bitrate (lossless, 128, 192, 256, 320)"),
    ),
    responses(
        (status = 200, description = "Audio stream (original or transcoded)"),
        (status = 404, description = "Track not found")
    )
)]
pub async fn handle_stream_track(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    Query(query): Query<StreamQuery>,
) -> impl IntoResponse {
    let db = state.context.db().await;

    let track = match library::lookup_track(db.pool(), &id).await {
        Ok(Some(t)) => t,
        Ok(None) => return (StatusCode::NOT_FOUND, Body::empty()).into_response(),
        Err(e) => {
            tawai_core::utils::logger::error(&format!("stream track lookup failed: {}", e));
            return (StatusCode::INTERNAL_SERVER_ERROR, Body::empty()).into_response();
        }
    };

    if track.file_path.starts_with("recommendation://") {
        return (StatusCode::NOT_FOUND, Body::empty()).into_response();
    }

    if track.file_path.starts_with("jellyfin://") {
        let (source_type, source_url) =
            library_source::get_source_by_track_id(db.pool(), &track.id)
                .await
                .ok()
                .flatten()
                .unwrap_or_default();
        let (url, headers) = resolve_track_source(
            &track.file_path,
            &source_type,
            &source_url,
            state.context.client(),
        )
        .await;
        return Json(tawai_core::signals::playback::PlayTrackResponse {
            id: track.id.clone(),
            file_path: url,
            error: None,
            headers,
        })
        .into_response();
    }

    let path = &track.file_path;

    if let Some(bitrate) = &query.bitrate {
        if bitrate != "lossless" {
            match ffmpeg::transcode_stream(path, bitrate) {
                Ok(transcode) => {
                    let body = Body::from_stream(transcode);
                    let ct = content_type(path);
                    let headers = [(header::CONTENT_TYPE, ct)];
                    return (headers, body).into_response();
                }
                Err(e) => {
                    tawai_core::utils::logger::error(&format!("ffmpeg transcoding failed: {}", e));
                    return (StatusCode::INTERNAL_SERVER_ERROR, Body::empty()).into_response();
                }
            }
        }
    }

    let file = match File::open(path).await {
        Ok(f) => f,
        Err(e) => {
            tawai_core::utils::logger::error(&format!("stream track file open failed: {}", e));
            return (StatusCode::NOT_FOUND, Body::empty()).into_response();
        }
    };

    let stream = ReaderStream::new(file);
    let body = Body::from_stream(stream);
    let ct = content_type(path);
    let headers = [(header::CONTENT_TYPE, ct)];
    (headers, body).into_response()
}
