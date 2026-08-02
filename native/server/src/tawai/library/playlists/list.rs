use axum::{Json, extract::State, response::IntoResponse};
use serde::Serialize;
use tawai_core::db::library;
use tawai_core::signals::library::PlaylistInfo;
use utoipa::ToSchema;

use crate::server::SharedState;

#[derive(Serialize, ToSchema)]
pub struct PlaylistsResponse {
    pub playlists: Vec<PlaylistInfo>,
}

pub async fn handle_list_playlists(State(state): State<SharedState>) -> impl IntoResponse {
    let db = state.context.db().await;
    let playlists = library::list_playlists(db.pool()).await.unwrap_or_default();
    Json(PlaylistsResponse { playlists }).into_response()
}
