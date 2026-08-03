use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::Deserialize;
use tawai_core::{
    audio,
    db::library,
    metadata::musicbrainz,
    signals::library::{MatchCandidate, TrackInfo},
};
use utoipa::ToSchema;

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
        (status = 200, description = "Identify candidates", body = Vec<MatchCandidate>),
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

    Json(candidates).into_response()
}

pub async fn handle_fingerprint_track(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let db = state.context.db().await;

    let (fingerprint, duration) = match library::lookup_fingerprint_by_id(db.pool(), &id).await {
        Ok(Some(v)) => v,
        _ => return Json(None::<tawai_core::signals::metadata::RecordingInfo>).into_response(),
    };

    let info =
        match musicbrainz::lookup_by_fingerprint(state.context.client(), &fingerprint, duration)
            .await
        {
            Ok(info) => info,
            Err(e) => {
                tawai_core::utils::logger::debug(&format!("AcoustID lookup failed: {e}"));
                return Json(None::<tawai_core::signals::metadata::RecordingInfo>).into_response();
            }
        };

    Json(Some(info)).into_response()
}
