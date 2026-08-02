use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use tawai_core::db::library;
use tawai_core::signals::library::TrackInfo;
use utoipa::ToSchema;

use crate::server::SharedState;

#[derive(Serialize, ToSchema)]
pub struct PlaylistTracksResponse {
    pub tracks: Vec<TrackInfo>,
}

#[derive(Deserialize, ToSchema)]
pub struct AddTrackBody {
    pub track_id: String,
}

#[derive(Deserialize, ToSchema)]
pub struct ReorderTracksBody {
    pub track_ids: Vec<String>,
}

pub async fn handle_get_playlist_tracks(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let db = state.context.db().await;
    match library::get_playlist_tracks(db.pool(), &id).await {
        Ok(tracks) => Json(PlaylistTracksResponse { tracks }).into_response(),
        Err(e) => {
            tawai_core::utils::logger::error(&format!("get playlist tracks failed: {}", e));
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response()
        }
    }
}

pub async fn handle_add_track_to_playlist(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    Json(body): Json<AddTrackBody>,
) -> impl IntoResponse {
    let db = state.context.db().await;
    match library::add_track_to_playlist(db.pool(), &id, &body.track_id).await {
        Ok(()) => StatusCode::OK.into_response(),
        Err(e) => {
            tawai_core::utils::logger::error(&format!("add track to playlist failed: {}", e));
            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response()
        }
    }
}

pub async fn handle_remove_track_from_playlist(
    State(state): State<SharedState>,
    Path((id, track_id)): Path<(String, String)>,
) -> impl IntoResponse {
    let db = state.context.db().await;
    match library::remove_track_from_playlist(db.pool(), &id, &track_id).await {
        Ok(()) => StatusCode::OK.into_response(),
        Err(e) => {
            tawai_core::utils::logger::error(&format!("remove track from playlist failed: {}", e));
            (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response()
        }
    }
}

pub async fn handle_reorder_playlist_tracks(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    Json(body): Json<ReorderTracksBody>,
) -> impl IntoResponse {
    let db = state.context.db().await;
    match library::reorder_playlist_tracks(db.pool(), &id, &body.track_ids).await {
        Ok(()) => StatusCode::OK.into_response(),
        Err(e) => {
            tawai_core::utils::logger::error(&format!("reorder playlist tracks failed: {}", e));
            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response()
        }
    }
}
