use axum::{
    Json,
    extract::{Query, State},
    response::IntoResponse,
};
use tawai_core::db::library;
use tawai_core::signals::library::{ListAlbumsRequest, ListAlbumsResponse};

use crate::server::SharedState;

pub async fn handle_list_albums(
    State(state): State<SharedState>,
    Query(query): Query<ListAlbumsRequest>,
) -> impl IntoResponse {
    let db = state.context.db().await;
    let albums = library::list_albums(db.pool(), query.artist_id.as_deref())
        .await
        .unwrap_or_default();
    Json(ListAlbumsResponse {
        id: String::new(),
        albums,
    })
    .into_response()
}
