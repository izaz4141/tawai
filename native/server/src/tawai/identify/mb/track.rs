use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use tawai_core::{
    audio,
    db::library,
    metadata::musicbrainz,
    signals::library::{IdentifySingleTrackResponse, MatchCandidate},
    signals::metadata::{FingerprintTrackRequest, FingerprintTrackResponse, RecordingInfo},
};

use crate::server::SharedState;

#[utoipa::path(
    get,
    path = "/api/tawai/identify/mb/track/{id}",
    tags = ["tawai.identify"],
    security(("ApiKeyAuth" = [])),
    params(
        ("id" = String, Path, description = "Track ID"),
    ),
    responses(
        (status = 200, description = "Identify candidates", body = IdentifySingleTrackResponse),
        (status = 404, description = "Track not found"),
    )
)]
pub async fn handle_identify_track(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let db = state.context.db().await;
    let mut candidates: Vec<MatchCandidate> = Vec::new();

    let track = match library::lookup_track(db.pool(), &id).await {
        Ok(Some(t)) => t,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tawai_core::utils::logger::error(&format!("lookup_track failed: {e}"));
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    if let Ok(Some((fingerprint, duration))) =
        library::lookup_fingerprint_by_id(db.pool(), &id).await
    {
        match musicbrainz::lookup_by_fingerprint(state.context.client(), &fingerprint, duration)
            .await
        {
            Ok(info) => {
                if !info.title.is_empty() {
                    let first_release = info.releases.first();
                    let score = if info.acoust_id.is_some() { 0.95 } else { 0.5 };
                    candidates.push(MatchCandidate {
                        score,
                        title: info.title.clone(),
                        artist: info.artist.clone(),
                        artist_id: info.artist_id.clone(),
                        album: first_release.map(|r| r.title.clone()).unwrap_or_default(),
                        album_id: first_release.map(|r| r.id.clone()),
                        recording_id: Some(info.id.clone()),
                        release_date: first_release.and_then(|r| r.date.clone()),
                        acoust_id: info.acoust_id.clone(),
                        duration_secs: info.duration_secs,
                    });
                }
            }
            Err(e) => tawai_core::utils::logger::debug(&format!("AcoustID lookup failed: {e}")),
        }
    }

    let path = std::path::Path::new(&track.file_path);
    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
        let (parsed_artist, parsed_title, _parsed_track) = audio::tags::parse_filename_tags(stem);
        if let Some(title) = parsed_title {
            if title != track.title || candidates.is_empty() {
                let score = if parsed_artist.is_some() { 0.3 } else { 0.2 };
                candidates.push(MatchCandidate {
                    score,
                    title,
                    artist: parsed_artist.unwrap_or_default(),
                    artist_id: None,
                    album: String::new(),
                    album_id: None,
                    recording_id: None,
                    release_date: None,
                    acoust_id: None,
                    duration_secs: None,
                });
            }
        }
    }

    candidates.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    Json(IdentifySingleTrackResponse {
        id: String::new(),
        track_id: id,
        candidates,
    })
    .into_response()
}

#[utoipa::path(
    post,
    path = "/api/tawai/identify/mb/fingerprint",
    tags = ["tawai.identify"],
    security(("ApiKeyAuth" = [])),
    request_body = FingerprintTrackRequest,
    responses(
        (status = 200, description = "Fingerprint lookup result", body = FingerprintTrackResponse),
    )
)]
pub async fn handle_fingerprint_track(
    State(state): State<SharedState>,
    Json(body): Json<FingerprintTrackRequest>,
) -> impl IntoResponse {
    let db = state.context.db().await;

    let (fingerprint, duration) = if let Some(file_path) = body.file_path.as_deref() {
        let path = std::path::Path::new(file_path);
        match audio::fingerprint::compute_fingerprint(path) {
            Ok(fp) => (fp.fingerprint, fp.duration),
            Err(e) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({
                        "error": format!("compute_fingerprint failed: {e}")
                    })),
                )
                    .into_response();
            }
        }
    } else if let Some(track_id) = body.track_id.as_deref() {
        match library::lookup_fingerprint_by_id(db.pool(), track_id).await {
            Ok(Some(v)) => v,
            _ => {
                return Json(FingerprintTrackResponse {
                    id: body.id,
                    track_id: body.track_id,
                    recording: None,
                })
                .into_response();
            }
        }
    } else {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "Either file_path or track_id is required"
            })),
        )
            .into_response();
    };

    let recording =
        match musicbrainz::lookup_by_fingerprint(state.context.client(), &fingerprint, duration)
            .await
        {
            Ok(info) => Some(info),
            Err(e) => {
                tawai_core::utils::logger::debug(&format!("AcoustID lookup failed: {e}"));
                None
            }
        };

    Json(FingerprintTrackResponse {
        id: body.id,
        track_id: body.track_id,
        recording,
    })
    .into_response()
}
