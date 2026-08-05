use std::path::Path;

use axum::{
    Json,
    extract::{Extension, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use base64::Engine as _;
use serde::Deserialize;
use tawai_core::{db::{account, library}, signals::library::TrackInfo};
use utoipa::ToSchema;

use crate::server::SharedState;

#[derive(Deserialize, ToSchema)]
pub struct UnidentifiedQuery {
    pub source_id: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct ApplyIdentifyBody {
    pub track_id: String,
    pub file_path: Option<String>,
    pub source_id: Option<String>,
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
    path = "/api/tawai/library/identify/download-folder",
    tags = ["tawai.library"],
    security(("ApiKeyAuth" = [])),
    responses(
        (status = 200, description = "Tracks read from the download folder", body = Vec<TrackInfo>),
    )
)]
pub async fn handle_list_download_folder_tracks(
    State(state): State<SharedState>,
) -> impl IntoResponse {
    let cfg = state.context.cfg().await;
    let folder = cfg
        .value
        .get("download_folder")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let tracks =
        tawai_core::utils::identify::list_download_folder_tracks(Path::new(&folder))
            .unwrap_or_default();
    Json(tracks).into_response()
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
    Extension(user_id): Extension<String>,
    Json(body): Json<ApplyIdentifyBody>,
) -> impl IntoResponse {
    let db = state.context.db().await;
    let mk = state.context.master_key.read().await.clone();

    let user = match account::get_user_by_id(db.pool(), &user_id, &mk).await {
        Ok(Some(u)) => u,
        _ => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({
                    "error": "User not found"
                })),
            )
                .into_response();
        }
    };

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

    let params = tawai_core::utils::identify::ApplyIdentificationParams {
        track_id: body.track_id,
        file_path: body.file_path,
        target_source_id: body.source_id,
        title: body.title,
        artist: body.artist,
        artist_mbid: body.artist_mbid,
        album: body.album,
        album_mbid: body.album_mbid,
        album_disambiguation: body.album_disambiguation,
        release_date: body.release_date,
        track_num: body.track_num,
        disc_num: body.disc_num,
        mbid_recording: body.mbid_recording,
        lyrics: body.lyrics,
        cover_bytes,
        total_discs: body.total_discs.unwrap_or(0),
    };

    match tawai_core::utils::identify::apply_identification(db.pool(), &user.id, &user.role, &params)
        .await
    {
        Ok(outcome) => Json(serde_json::json!({
            "success": true,
            "new_file_path": outcome.new_file_path,
        }))
        .into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": e.to_string()
            })),
        )
            .into_response(),
    }
}
