use axum::{
    Json,
    extract::{Query, State},
    response::IntoResponse,
};
use serde::Deserialize;
use serde::Serialize;
use tawai_core::db::library;
use tawai_core::signals::library::TrackInfo;
use utoipa::ToSchema;

use crate::server::SharedState;

#[derive(Deserialize)]
pub struct TracksQuery {
    pub album_id: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct TracksResponse {
    pub tracks: Vec<TrackInfo>,
}

#[utoipa::path(
    get,
    path = "/api/tawai/library/tracks",
    tags = ["tawai.library"],
    security(("ApiKeyAuth" = [])),
    params(
        ("album_id" = Option<String>, Query, description = "Filter by album ID"),
    ),
    responses(
        (status = 200, description = "List of tracks", body = TracksResponse),
    )
)]
pub async fn handle_list_tracks(
    State(state): State<SharedState>,
    Query(query): Query<TracksQuery>,
) -> impl IntoResponse {
    let db = state.context.db().await;
    let tracks = library::list_tracks(db.pool(), query.album_id.as_deref())
        .await
        .unwrap_or_default();
    Json(TracksResponse { tracks }).into_response()
}
