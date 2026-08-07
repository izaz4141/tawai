use axum::{
    body::Body,
    extract::{Path, State},
    http::{HeaderMap, Method, Request, StatusCode},
    response::{IntoResponse, Response},
};
use mime::Mime;
use tawai_core::db::library;
use tower_http::services::ServeFile;

use crate::server::SharedState;

fn stream_content_type(path: &str) -> &'static str {
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

/// Progressive stream of a local track's original file, used by web clients
/// (HTML5 audio) that cannot play DASH. Served with full HTTP range support so
/// seeking works.
#[utoipa::path(
    get,
    path = "/api/tawai/playback/stream/{id}",
    tags = ["tawai.playback"],
    security(("TokenQueryAuth" = [])),
    params(
        ("id" = String, Path, description = "Track ID"),
    ),
    responses(
        (status = 200, description = "Audio stream (supports HTTP range requests)"),
        (status = 206, description = "Partial content for byte-range requests"),
        (status = 404, description = "Track not found"),
        (status = 416, description = "Requested byte range not satisfiable")
    )
)]
pub async fn handle_stream_track(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    headers: HeaderMap,
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

    if track.file_path.starts_with("recommendation://")
        || track.file_path.starts_with("jellyfin://")
        || track.file_path.starts_with("http://")
        || track.file_path.starts_with("https://")
    {
        return (StatusCode::NOT_FOUND, Body::empty()).into_response();
    }

    let mime: Mime = stream_content_type(&track.file_path)
        .parse()
        .unwrap_or(mime::APPLICATION_OCTET_STREAM);
    let mut request = Request::new(Body::empty());
    *request.method_mut() = Method::GET;
    *request.headers_mut() = headers;
    let mut serve = ServeFile::new_with_mime(&track.file_path, &mime);
    match serve.try_call(request).await {
        Ok(response) => {
            let (parts, body) = response.into_parts();
            Response::from_parts(parts, Body::new(body))
        }
        Err(e) => {
            tawai_core::utils::logger::error(&format!("stream file serve failed: {}", e));
            (StatusCode::INTERNAL_SERVER_ERROR, Body::empty()).into_response()
        }
    }
}
