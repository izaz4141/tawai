use axum::{
    Json,
    extract::{Path, State},
    response::IntoResponse,
};
use serde::Serialize;
use tawai_core::db::library;
use utoipa::ToSchema;

use crate::server::SharedState;

#[derive(Serialize, ToSchema)]
pub struct MbidResponse {
    pub mbid: Option<String>,
}

#[utoipa::path(
    get,
    path = "/api/tawai/library/tracks/album-mbid/{album_id}",
    tags = ["tawai.library"],
    security(("ApiKeyAuth" = [])),
    params(
        ("album_id" = String, Path, description = "Album ID"),
    ),
    responses(
        (status = 200, description = "Album MBID", body = MbidResponse),
    )
)]
pub async fn handle_get_album_mbid(
    State(state): State<SharedState>,
    Path(album_id): Path<String>,
) -> impl IntoResponse {
    let db = state.context.db().await;
    match library::get_album_mbid(db.pool(), &album_id).await {
        Ok(mbid) => Json(MbidResponse { mbid }).into_response(),
        Err(e) => {
            tawai_core::utils::logger::error(&format!("get_album_mbid failed: {e}"));
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(MbidResponse { mbid: None }),
            )
                .into_response()
        }
    }
}
