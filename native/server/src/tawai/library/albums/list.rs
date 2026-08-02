use axum::{
    Json,
    extract::{Query, State},
    response::IntoResponse,
};
use serde::Deserialize;
use serde::Serialize;
use tawai_core::db::library;
use tawai_core::signals::library::AlbumInfo;
use utoipa::ToSchema;

use crate::server::SharedState;

#[derive(Deserialize)]
pub struct AlbumsQuery {
    pub artist_id: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct AlbumsResponse {
    pub albums: Vec<AlbumInfo>,
}

pub async fn handle_list_albums(
    State(state): State<SharedState>,
    Query(query): Query<AlbumsQuery>,
) -> impl IntoResponse {
    let db = state.context.db().await;
    let albums = library::list_albums(db.pool(), query.artist_id.as_deref())
        .await
        .unwrap_or_default();
    Json(AlbumsResponse { albums }).into_response()
}
