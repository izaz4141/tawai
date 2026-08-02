use axum::{
    Json,
    extract::{Path, State},
    response::IntoResponse,
};
use tawai_core::db::library;

use super::list::TracksResponse;
use crate::server::SharedState;

#[utoipa::path(
    get,
    path = "/api/tawai/library/tracks/by-source/{source_id}",
    tags = ["tawai.library"],
    security(("ApiKeyAuth" = [])),
    params(
        ("source_id" = String, Path, description = "Library source ID"),
    ),
    responses(
        (status = 200, description = "List of tracks from source", body = TracksResponse),
    )
)]
pub async fn handle_list_tracks_by_source(
    State(state): State<SharedState>,
    Path(source_id): Path<String>,
) -> impl IntoResponse {
    let db = state.context.db().await;
    match library::list_tracks_by_source(db.pool(), &source_id).await {
        Ok(tracks) => Json(TracksResponse { tracks }).into_response(),
        Err(e) => {
            tawai_core::utils::logger::error(&format!("list_tracks_by_source failed: {e}"));
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(TracksResponse { tracks: vec![] }),
            )
                .into_response()
        }
    }
}
