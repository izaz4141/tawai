use std::sync::Arc;

use axum::{
    body::Body,
    extract::{Extension, Path, State},
    http::{HeaderMap, HeaderValue, Method, Request, StatusCode, header},
    response::{IntoResponse, Response},
};
use mime::Mime;
use tawai_core::audio::ffmpeg;
use tawai_core::db::library;
use tower_http::services::ServeFile;

use crate::security::QueryToken;
use crate::server::SharedState;

fn dash_content_type(name: &str) -> &'static str {
    if name.ends_with(".mpd") {
        "application/dash+xml"
    } else if name.starts_with("init-") {
        "audio/mp4"
    } else {
        "video/iso.segment"
    }
}

/// Validate a DASH cache file name. Allows `manifest.mpd`, `init-<n>.m4s`
/// and `seg-<n>-<n>.m4s`; rejects anything that could escape the cache dir.
fn is_valid_dash_file(name: &str) -> bool {
    if name.is_empty() || name.contains('/') || name.contains('\\') || name.contains("..") {
        return false;
    }
    if name == "manifest.mpd" {
        return true;
    }
    let digits = |s: &str| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit());
    if let Some(rest) = name.strip_prefix("init-") {
        return rest.ends_with(".m4s") && digits(&rest[..rest.len() - 4]);
    }
    if let Some(rest) = name.strip_prefix("seg-") {
        if let Some(idx) = rest.rfind('-') {
            let left = &rest[..idx];
            let right = &rest[idx + 1..];
            return right.ends_with(".m4s") && digits(left) && digits(&right[..right.len() - 4]);
        }
    }
    false
}

/// Ensure DASH segments for `track_id` exist in `dir` and are up to date with
/// the source file, generating them via ffmpeg when missing or stale.
/// Concurrent requests for the same track are serialized by a per-track lock.
async fn ensure_dash_generated(
    state: &SharedState,
    track_id: &str,
    source_path: &str,
    dir: &str,
) -> anyhow::Result<()> {
    if ffmpeg::dash_cache_fresh(source_path, dir).await? {
        return Ok(());
    }
    let lock = {
        let mut map = state.dash_generation_locks.lock().unwrap();
        map.entry(track_id.to_string())
            .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
            .clone()
    };
    let _guard = lock.lock().await;
    if ffmpeg::dash_cache_fresh(source_path, dir).await? {
        return Ok(());
    }
    ffmpeg::generate_dash_segments(source_path, dir).await
}

/// The ffmpeg DASH template attribute values emitted by
/// `tawai_core::audio::ffmpeg::generate_dash_segments`. Relative segment URLs
/// resolve against the MPD base URI without the `?token=` query, so the token
/// must be embedded here for segment requests to authenticate.
const INIT_TEMPLATE: &str = "init-$Bandwidth$.m4s";
const MEDIA_TEMPLATE: &str = "seg-$Bandwidth$-$Number$.m4s";

/// Serve `manifest.mpd` with the validated `token` embedded into the segment
/// template URLs so DASH clients authenticate every init/segment request.
async fn serve_manifest(dir: &str, token: &str) -> Result<Response, ()> {
    let manifest = format!("{}/manifest.mpd", dir);
    let mut body = match tokio::fs::read(&manifest).await {
        Ok(b) => String::from_utf8(b).map_err(|_| ())?,
        Err(_) => return Err(()),
    };
    let init = format!("{}?token={}", INIT_TEMPLATE, token);
    let media = format!("{}?token={}", MEDIA_TEMPLATE, token);
    body = body
        .replace(INIT_TEMPLATE, &init)
        .replace(MEDIA_TEMPLATE, &media);

    let mut response = (
        [(header::CONTENT_TYPE, "application/dash+xml")],
        Body::from(body),
    )
        .into_response();
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    Ok(response)
}

#[utoipa::path(
    get,
    path = "/api/tawai/playback/dash/{id}/{file}",
    tags = ["tawai.playback"],
    security(("TokenQueryAuth" = [])),
    params(
        ("id" = String, Path, description = "Track ID"),
        ("file" = String, Path, description = "DASH artifact (manifest.mpd, init-<bandwidth>.m4s or seg-<bandwidth>-<number>.m4s)"),
    ),
    responses(
        (status = 200, description = "DASH manifest or segment (supports HTTP range requests)"),
        (status = 206, description = "Partial content for byte-range requests"),
        (status = 404, description = "Track, manifest or segment not found"),
        (status = 416, description = "Requested byte range not satisfiable")
    )
)]
pub async fn handle_dash_file(
    State(state): State<SharedState>,
    Extension(QueryToken(token)): Extension<QueryToken>,
    Path((id, file)): Path<(String, String)>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let db = state.context.db().await;

    let track = match library::lookup_track(db.pool(), &id).await {
        Ok(Some(t)) => t,
        Ok(None) => return (StatusCode::NOT_FOUND, Body::empty()).into_response(),
        Err(e) => {
            tawai_core::utils::logger::error(&format!("dash track lookup failed: {}", e));
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

    if !is_valid_dash_file(&file) {
        return (StatusCode::NOT_FOUND, Body::empty()).into_response();
    }

    let cfg = state.context.cfg().await;
    let dir = crate::server::dash_cache_dir_for_track(&cfg, &track.id);

    if let Err(e) = ensure_dash_generated(&state, &track.id, &track.file_path, &dir).await {
        tawai_core::utils::logger::error(&format!("dash generation failed: {}", e));
        return (StatusCode::INTERNAL_SERVER_ERROR, Body::empty()).into_response();
    }

    if file == "manifest.mpd" {
        return match serve_manifest(&dir, &token).await {
            Ok(response) => response,
            Err(_) => (StatusCode::NOT_FOUND, Body::empty()).into_response(),
        };
    }

    let path = format!("{}/{}", dir, file);
    let mime: Mime = dash_content_type(&file)
        .parse()
        .unwrap_or(mime::APPLICATION_OCTET_STREAM);
    let mut request = Request::new(Body::empty());
    *request.method_mut() = Method::GET;
    *request.headers_mut() = headers;
    let mut serve = ServeFile::new_with_mime(&path, &mime);
    match serve.try_call(request).await {
        Ok(response) => {
            let (mut parts, body) = response.into_parts();
            parts.headers.insert(
                header::CACHE_CONTROL,
                HeaderValue::from_static("public, max-age=86400"),
            );
            Response::from_parts(parts, Body::new(body))
        }
        Err(e) => {
            tawai_core::utils::logger::error(&format!("dash file serve failed: {}", e));
            (StatusCode::INTERNAL_SERVER_ERROR, Body::empty()).into_response()
        }
    }
}
