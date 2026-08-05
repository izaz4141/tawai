use axum::{
    Extension, Json,
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::Deserialize;
use tawai_core::db;
use tawai_core::signals::ListDownloadsResponse;

use crate::server::SharedState;

#[derive(Debug, Deserialize)]
pub struct ListDownloadParams {
    pub source: Option<String>,
}

#[utoipa::path(
    get,
    path = "/api/tawai/download/list",
    tags = ["tawai.download"],
    security(("ApiKeyAuth" = [])),
    params(
        ("source" = Option<String>, Query, description = "Filter by source (slskd, nadekodon, etc.)"),
    ),
    responses(
        (status = 200, description = "Download list", body = ListDownloadsResponse),
        (status = 400, description = "Failed to fetch downloads"),
    )
)]
pub async fn handle_list_downloads(
    State(state): State<SharedState>,
    Extension(user_id): Extension<String>,
    Query(params): Query<ListDownloadParams>,
) -> impl IntoResponse {
    let db = state.context.db().await;

    match db::download::list_downloads(db.pool(), &user_id, params.source.as_deref()).await {
        Ok(downloads) => Json(ListDownloadsResponse { downloads }).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}
