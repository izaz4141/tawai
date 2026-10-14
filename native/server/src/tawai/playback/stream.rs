use std::sync::Arc;

use axum::{
    body::Body,
    extract::{Extension, Path, Query, State},
    http::{HeaderValue, Request, StatusCode, header},
    response::{IntoResponse, Response},
};
use mime::Mime;
use serde::Deserialize;
use tawai_core::audio::dash_manifest::build_dash_manifest;
use tawai_core::audio::ffmpeg;
use tawai_core::audio::ffmpeg::{
    DASH_MAX_BITRATE_KBPS, DASH_MIN_BITRATE_KBPS, DASH_SEGMENT_DURATION_SECS,
    DEFAULT_DASH_BITRATE_KBPS,
};
use tawai_core::db::library;
use tawai_core::signals::library::TrackInfo;
use tower_http::services::ServeFile;

use crate::security::QueryToken;
use crate::server::SharedState;

use super::{TrackAccess, user_can_read_track};

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

fn dash_content_type(name: &str) -> &'static str {
    if name.ends_with(".mpd") {
        "application/dash+xml"
    } else if name.starts_with("init-") {
        "audio/mp4"
    } else {
        "video/iso.segment"
    }
}

/// Whether `bandwidth` (in bits per second) falls inside the accepted
/// transcode range.
fn bitrate_in_range(bandwidth: u32) -> bool {
    (DASH_MIN_BITRATE_KBPS..=DASH_MAX_BITRATE_KBPS).contains(&(bandwidth / 1000))
}

/// Validate a DASH cache file name. Allows `manifest.mpd`, `init-<n>.m4s`
/// and `seg-<n>-<n>.m4s`; rejects anything that could escape the cache dir.
/// The encoded bandwidth must also be inside the accepted transcode range.
fn is_valid_dash_file(name: &str) -> bool {
    if name.is_empty() || name.contains('/') || name.contains('\\') || name.contains("..") {
        return false;
    }
    if name == "manifest.mpd" {
        return true;
    }
    let digits = |s: &str| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit());
    let in_range = |s: &str| s.parse::<u32>().map(bitrate_in_range).unwrap_or(false);
    if let Some(rest) = name.strip_prefix("init-") {
        if let Some(bandwidth) = rest.strip_suffix(".m4s") {
            return digits(bandwidth) && in_range(bandwidth);
        }
        return false;
    }
    if let Some(rest) = name.strip_prefix("seg-") {
        if let Some(idx) = rest.rfind('-') {
            let bandwidth = &rest[..idx];
            let right = &rest[idx + 1..];
            if let Some(number) = right.strip_suffix(".m4s") {
                return digits(bandwidth) && digits(number) && in_range(bandwidth);
            }
        }
    }
    false
}

/// Serialize concurrent generation of artifacts for the same track. Different
/// tracks (and the manifest, which needs no generation) proceed in parallel.
async fn with_track_lock<T>(
    state: &SharedState,
    track_id: &str,
    fut: impl std::future::Future<Output = anyhow::Result<T>>,
) -> anyhow::Result<T> {
    let lock = {
        let mut map = state.dash_generation_locks.lock().unwrap();
        map.entry(track_id.to_string())
            .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
            .clone()
    };
    let _guard = lock.lock().await;
    fut.await
}

/// Ensure `init-<bandwidth>.m4s` for `track` exists in `dir` and is up to
/// date with the source file, transcoding it (plus segment 1) on demand.
async fn ensure_dash_init_generated(
    state: &SharedState,
    track_id: &str,
    source_path: &str,
    dir: &str,
    bitrate_kbps: u32,
) -> anyhow::Result<()> {
    let artifact = format!("{}/init-{}.m4s", dir, bitrate_kbps * 1000);
    if ffmpeg::dash_file_fresh(source_path, &artifact).await? {
        return Ok(());
    }
    with_track_lock(state, track_id, async {
        if ffmpeg::dash_file_fresh(source_path, &artifact).await? {
            return Ok(());
        }
        ffmpeg::generate_dash_segment(source_path, dir, bitrate_kbps, 1, 0.0).await
    })
    .await
}

/// Ensure `seg-<bandwidth>-<number>.m4s` for `track` exists in `dir` and is
/// up to date with the source file, transcoding just that slice on demand.
async fn ensure_dash_segment_generated(
    state: &SharedState,
    track_id: &str,
    source_path: &str,
    dir: &str,
    bitrate_kbps: u32,
    segment_number: u32,
) -> anyhow::Result<()> {
    let artifact = format!("{}/seg-{}-{}.m4s", dir, bitrate_kbps * 1000, segment_number);
    if ffmpeg::dash_file_fresh(source_path, &artifact).await? {
        return Ok(());
    }
    let start_secs = (segment_number - 1) as f64 * DASH_SEGMENT_DURATION_SECS as f64;
    with_track_lock(state, track_id, async {
        if ffmpeg::dash_file_fresh(source_path, &artifact).await? {
            return Ok(());
        }
        ffmpeg::generate_dash_segment(source_path, dir, bitrate_kbps, segment_number, start_secs)
            .await
    })
    .await
}

/// Serve a static VOD manifest with the validated `token` embedded into the
/// segment template URLs so DASH clients authenticate every init/segment
/// request. The manifest describes a single representation at `bitrate_kbps`
/// and carries the track's known duration, so the player knows the full length
/// (and can seek anywhere) from the start.
fn serve_manifest(duration_secs: Option<f64>, bitrate_kbps: u32, token: &str) -> Response {
    let body = build_dash_manifest(duration_secs, bitrate_kbps)
        .replace(
            "init-$Bandwidth$.m4s",
            &format!("init-$Bandwidth$.m4s?token={}", token),
        )
        .replace(
            "seg-$Bandwidth$-$Number$.m4s",
            &format!("seg-$Bandwidth$-$Number$.m4s?token={}", token),
        );

    let mut response = (
        [(header::CONTENT_TYPE, "application/dash+xml")],
        Body::from(body),
    )
        .into_response();
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

/// Parse the bitrate in kbps from an `init-<bw>.m4s` or `seg-<bw>-<n>.m4s`
/// file name.
fn parse_bandwidth(file: &str) -> Option<u32> {
    let digits = file
        .strip_prefix("init-")
        .or_else(|| file.strip_prefix("seg-"))?;
    let digits = digits.split('-').next()?;
    let digits = digits.strip_suffix(".m4s").unwrap_or(digits);
    let bandwidth: u64 = digits.parse().ok()?;
    Some((bandwidth / 1000) as u32)
}

/// Parse the 1-based segment number from a `seg-<bw>-<n>.m4s` file name.
fn parse_segment_number(file: &str) -> Option<u32> {
    let rest = file.strip_prefix("seg-")?;
    let idx = rest.rfind('-')?;
    let num = rest[idx + 1..].strip_suffix(".m4s")?;
    num.parse().ok()
}

/// Look up a local track and check the requesting user may read its source.
/// Remote/streamed sources (which never pass through this module) and any
/// denial are turned into an HTTP response.
async fn authorize_track(
    state: &SharedState,
    user_id: &str,
    id: &str,
) -> Result<TrackInfo, Response> {
    let db = state.context.db().await;
    let mk = state.context.master_key.read().await.clone();

    let track = match library::lookup_track(db.pool(), id).await {
        Ok(Some(t)) => t,
        Ok(None) => return Err((StatusCode::NOT_FOUND, Body::empty()).into_response()),
        Err(e) => {
            tawai_core::utils::logger::error(&format!("media track lookup failed: {}", e));
            return Err((StatusCode::INTERNAL_SERVER_ERROR, Body::empty()).into_response());
        }
    };

    if track.file_path.starts_with("recommendation://")
        || track.file_path.starts_with("jellyfin://")
        || track.file_path.starts_with("http://")
        || track.file_path.starts_with("https://")
    {
        return Err((StatusCode::NOT_FOUND, Body::empty()).into_response());
    }

    match user_can_read_track(db.pool(), user_id, id, &mk).await {
        TrackAccess::Allowed => {}
        TrackAccess::NoSource | TrackAccess::NoUser => {
            return Err((StatusCode::NOT_FOUND, Body::empty()).into_response());
        }
        TrackAccess::Denied => {
            return Err((StatusCode::FORBIDDEN, Body::empty()).into_response());
        }
    }

    Ok(track)
}

/// Progressive stream of a local track's original file, used whenever no
/// transcode is wanted (web players, lossless preference, libsource
/// backup/download). Served with full HTTP range support so seeking works.
#[utoipa::path(
    get,
    path = "/api/tawai/playback/stream/{id}",
    tags = ["tawai.playback"],
    security(("TokenQueryAuth" = []), ("ApiKeyAuth" = [])),
    params(
        ("id" = String, Path, description = "Track ID"),
    ),
    responses(
        (status = 200, description = "Audio stream (supports HTTP range requests)"),
        (status = 206, description = "Partial content for byte-range requests"),
        (status = 403, description = "Forbidden: user cannot access the track's source"),
        (status = 404, description = "Track not found"),
        (status = 416, description = "Requested byte range not satisfiable")
    )
)]
pub async fn handle_media_stream(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    Extension(user_id): Extension<String>,
    request: Request<Body>,
) -> impl IntoResponse {
    let track = match authorize_track(&state, &user_id, &id).await {
        Ok(track) => track,
        Err(response) => return response,
    };

    let mime: Mime = stream_content_type(&track.file_path)
        .parse()
        .unwrap_or(mime::APPLICATION_OCTET_STREAM);
    let mut serve = ServeFile::new_with_mime(&track.file_path, &mime);
    match serve.try_call(request).await {
        Ok(response) => {
            let (parts, body) = response.into_parts();
            Response::from_parts(parts, Body::new(body))
        }
        Err(e) => {
            tawai_core::utils::logger::error(&format!("media stream serve failed: {}", e));
            (StatusCode::INTERNAL_SERVER_ERROR, Body::empty()).into_response()
        }
    }
}

/// Query parameters accepted by the DASH endpoint in addition to the auth
/// middleware's `token`.
#[derive(Deserialize)]
pub struct DashQuery {
    #[serde(default)]
    bitrate: Option<u32>,
}

/// DASH artifacts for a transcoded track: the static manifest plus the
/// per-bitrate init/media segments, generated on demand via ffmpeg.
#[utoipa::path(
    get,
    path = "/api/tawai/playback/stream/{id}/{file}",
    tags = ["tawai.playback"],
    security(("TokenQueryAuth" = [])),
    params(
        ("id" = String, Path, description = "Track ID"),
        ("file" = String, Path, description = "DASH artifact (manifest.mpd, init-<bandwidth>.m4s or seg-<bandwidth>-<number>.m4s)"),
        ("bitrate" = Option<u32>, Query, description = "Transcode bitrate in kbps (manifest only; defaults to 192)"),
    ),
    responses(
        (status = 200, description = "DASH manifest or segment (supports HTTP range requests)"),
        (status = 206, description = "Partial content for byte-range requests"),
        (status = 403, description = "Forbidden: user cannot access the track's source"),
        (status = 404, description = "Track, manifest or segment not found"),
        (status = 416, description = "Requested byte range not satisfiable")
    )
)]
pub async fn handle_media_dash(
    State(state): State<SharedState>,
    Extension(QueryToken(token)): Extension<QueryToken>,
    Extension(user_id): Extension<String>,
    Path((id, file)): Path<(String, String)>,
    Query(query): Query<DashQuery>,
    request: Request<Body>,
) -> impl IntoResponse {
    let track = match authorize_track(&state, &user_id, &id).await {
        Ok(track) => track,
        Err(response) => return response,
    };

    if !is_valid_dash_file(&file) {
        return (StatusCode::NOT_FOUND, Body::empty()).into_response();
    }

    if file == "manifest.mpd" {
        let bitrate_kbps = query.bitrate.unwrap_or(DEFAULT_DASH_BITRATE_KBPS);
        if !(DASH_MIN_BITRATE_KBPS..=DASH_MAX_BITRATE_KBPS).contains(&bitrate_kbps) {
            return (StatusCode::NOT_FOUND, Body::empty()).into_response();
        }
        return serve_manifest(track.duration_secs, bitrate_kbps, &token);
    }

    let cfg = state.context.cfg().await;
    let dir = crate::server::dash_cache_dir_for_track(&cfg, &track.id);

    let bitrate_kbps = match parse_bandwidth(&file) {
        Some(bw) if (DASH_MIN_BITRATE_KBPS..=DASH_MAX_BITRATE_KBPS).contains(&bw) => bw,
        _ => return (StatusCode::NOT_FOUND, Body::empty()).into_response(),
    };

    if file.starts_with("init-") {
        if let Err(e) =
            ensure_dash_init_generated(&state, &track.id, &track.file_path, &dir, bitrate_kbps).await
        {
            tawai_core::utils::logger::error(&format!("dash init generation failed: {}", e));
            return (StatusCode::INTERNAL_SERVER_ERROR, Body::empty()).into_response();
        }
    } else if let Some(number) = parse_segment_number(&file) {
        if number < 1 {
            return (StatusCode::NOT_FOUND, Body::empty()).into_response();
        }
        if let Some(duration) = track.duration_secs {
            let start_secs = (number - 1) as f64 * DASH_SEGMENT_DURATION_SECS as f64;
            if start_secs >= duration {
                return (StatusCode::NOT_FOUND, Body::empty()).into_response();
            }
        }
        if let Err(e) = ensure_dash_segment_generated(
            &state,
            &track.id,
            &track.file_path,
            &dir,
            bitrate_kbps,
            number,
        )
        .await
        {
            tawai_core::utils::logger::error(&format!("dash segment generation failed: {}", e));
            return (StatusCode::INTERNAL_SERVER_ERROR, Body::empty()).into_response();
        }
    } else {
        return (StatusCode::NOT_FOUND, Body::empty()).into_response();
    }

    let path = format!("{}/{}", dir, file);
    let mime: Mime = dash_content_type(&file)
        .parse()
        .unwrap_or(mime::APPLICATION_OCTET_STREAM);
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
