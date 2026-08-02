use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use base64::Engine as _;
use serde::Deserialize;
use tawai_core::{
    audio,
    db::{account, library, user_settings},
    signals::library::TrackInfo,
    tools,
};
use utoipa::ToSchema;

use crate::server::SharedState;

#[derive(Deserialize, ToSchema)]
pub struct UnidentifiedQuery {
    pub source_id: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct ApplyIdentifyBody {
    pub track_id: String,
    pub title: String,
    pub artist: String,
    pub artist_mbid: Option<String>,
    pub album: String,
    pub album_mbid: Option<String>,
    pub album_disambiguation: Option<String>,
    pub release_date: Option<String>,
    pub track_num: Option<i32>,
    pub disc_num: Option<i32>,
    pub mbid_recording: Option<String>,
    pub lyrics: Option<String>,
    pub cover_bytes: Option<String>,
    pub total_discs: Option<i32>,
}

#[utoipa::path(
    get,
    path = "/api/tawai/library/identify/unidentified",
    tags = ["tawai.library"],
    security(("ApiKeyAuth" = [])),
    params(
        ("source_id" = Option<String>, Query, description = "Filter by library source ID"),
    ),
    responses(
        (status = 200, description = "List of tracks without MBID or album MBID", body = Vec<TrackInfo>),
    )
)]
pub async fn handle_list_unidentified(
    State(state): State<SharedState>,
    Query(query): Query<UnidentifiedQuery>,
) -> impl IntoResponse {
    let db = state.context.db().await;
    match library::list_unidentified_tracks(db.pool(), query.source_id.as_deref()).await {
        Ok(tracks) => Json(tracks).into_response(),
        Err(e) => {
            tawai_core::utils::logger::error(&format!("list_unidentified failed: {e}"));
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(Vec::<TrackInfo>::new()),
            )
                .into_response()
        }
    }
}

#[utoipa::path(
    post,
    path = "/api/tawai/library/identify/apply",
    tags = ["tawai.library"],
    security(("ApiKeyAuth" = [])),
    responses(
        (status = 200, description = "Identification applied"),
        (status = 400, description = "Error applying identification"),
    )
)]
pub async fn handle_apply_identification(
    State(state): State<SharedState>,
    Json(body): Json<ApplyIdentifyBody>,
) -> impl IntoResponse {
    let db = state.context.db().await;

    let track = match library::lookup_track(db.pool(), &body.track_id).await {
        Ok(Some(t)) => t,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({
                    "error": "Track not found"
                })),
            )
                .into_response();
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": e.to_string()
                })),
            )
                .into_response();
        }
    };

    let file_path = std::path::PathBuf::from(&track.file_path);

    let cover_bytes = match &body.cover_bytes {
        Some(s) if !s.is_empty() => match base64::engine::general_purpose::STANDARD.decode(s) {
            Ok(bytes) => Some(bytes),
            Err(e) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({
                        "error": format!("Invalid cover_bytes base64: {e}")
                    })),
                )
                    .into_response();
            }
        },
        _ => None,
    };

    let write_tag = audio::tags::AudioTag {
        title: body.title.clone(),
        artist: body.artist.clone(),
        album: body.album.clone(),
        album_artist: body.artist.clone(),
        album_disambiguation: body.album_disambiguation.clone(),
        release_date: body.release_date.clone(),
        track_number: body.track_num.unwrap_or(track.track_num.unwrap_or(0)),
        disc_number: body.disc_num.unwrap_or(track.disc_num.unwrap_or(1)),
        mbid_recording: body.mbid_recording.clone(),
        mbid_artist: body.artist_mbid.clone(),
        mbid_release: body.album_mbid.clone(),
        mbid_release_artist: body.artist_mbid.clone(),
        lyrics: body.lyrics.clone(),
        cover: cover_bytes.clone(),
        ..Default::default()
    };

    if let Err(e) = audio::tags::write_audio_tags(&file_path, &write_tag) {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": format!("write_audio_tags failed: {e}")
            })),
        )
            .into_response();
    }

    if let Some(cover) = &cover_bytes {
        let _ = library::update_album_cover(db.pool(), &track.album_id, cover).await;
    }

    let new_file_path = match {
        let db2 = state.context.db().await;
        let pattern = user_settings::get_setting(
            db2.pool(),
            &account::DEFAULT_USERNAME,
            "identify_naming_pattern",
        )
        .await
        .filter(|s| !s.is_empty());

        if let Some(pattern) = pattern {
            tools::rename::rename_audio_file(&file_path, &pattern, &write_tag)
                .map(|p| Some(p.to_string_lossy().to_string()))
        } else {
            Ok(None)
        }
    } {
        Ok(path) => path,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": format!("rename_audio_file failed: {e}")
                })),
            )
                .into_response();
        }
    };

    let artist_pairs = [(body.artist.clone(), body.artist_mbid.clone())];
    let album_artist_pairs = [(body.artist.clone(), body.artist_mbid.clone())];

    if let Err(e) = library::update_track(
        db.pool(),
        &body.track_id,
        &body.title,
        &artist_pairs,
        &body.album,
        &album_artist_pairs,
        body.album_mbid.as_deref(),
        body.release_date,
        body.track_num,
        body.disc_num,
        body.mbid_recording.as_deref(),
        body.lyrics.as_deref(),
        cover_bytes.as_deref(),
        new_file_path.as_deref(),
        body.album_disambiguation.clone(),
        body.total_discs.unwrap_or(0),
    )
    .await
    {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": format!("update_track failed: {e}")
            })),
        )
            .into_response();
    }

    Json(serde_json::json!({ "success": true })).into_response()
}
