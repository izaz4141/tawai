use axum::{
    Json,
    extract::{Query, State},
    response::IntoResponse,
};
use tawai_core::db::library;
use tawai_core::signals::library::{ListArtistsRequest, ListArtistsResponse};

use crate::server::SharedState;

pub async fn handle_list_artists(
    State(state): State<SharedState>,
    Query(_query): Query<ListArtistsRequest>,
) -> impl IntoResponse {
    let db = state.context.db().await;
    let artists = library::list_artists(db.pool()).await.unwrap_or_default();
    Json(ListArtistsResponse {
        id: String::new(),
        artists,
    })
    .into_response()
}
