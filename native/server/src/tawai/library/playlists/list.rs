use axum::{
    Json,
    extract::{Query, State},
    response::IntoResponse,
};
use tawai_core::db::library;
use tawai_core::signals::library::{ListPlaylistsRequest, ListPlaylistsResponse};

use crate::server::SharedState;

pub async fn handle_list_playlists(
    State(state): State<SharedState>,
    Query(_query): Query<ListPlaylistsRequest>,
) -> impl IntoResponse {
    let db = state.context.db().await;
    let playlists = library::list_playlists(db.pool()).await.unwrap_or_default();
    Json(ListPlaylistsResponse {
        id: String::new(),
        playlists,
    })
    .into_response()
}
