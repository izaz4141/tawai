use axum::{
    Json,
    extract::{Query, State},
    response::IntoResponse,
};
use tawai_core::db::library;
use tawai_core::signals::library::{ListTracksRequest, ListTracksResponse};

use crate::server::SharedState;

#[utoipa::path(
    get,
    path = "/api/tawai/library/tracks",
    tags = ["tawai.library"],
    security(("ApiKeyAuth" = [])),
    params(
        ("album_id" = Option<String>, Query, description = "Filter by album ID"),
    ),
    responses(
        (status = 200, description = "List of tracks", body = ListTracksResponse),
    )
)]
pub async fn handle_list_tracks(
    State(state): State<SharedState>,
    Query(query): Query<ListTracksRequest>,
) -> impl IntoResponse {
    let db = state.context.db().await;
    let tracks = library::list_tracks(db.pool(), query.album_id.as_deref())
        .await
        .unwrap_or_default();
    Json(ListTracksResponse {
        id: String::new(),
        tracks,
    })
    .into_response()
}
