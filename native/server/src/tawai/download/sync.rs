use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use tawai_core::dclient::DownloadClient;
use tawai_core::signals::download::{DownloadSyncRequest, DownloadSyncResponse};

use crate::server::SharedState;

#[utoipa::path(
    post,
    path = "/api/tawai/download/sync",
    tags = ["tawai.download"],
    security(("ApiKeyAuth" = [])),
    request_body = DownloadSyncRequest,
    responses(
        (status = 200, description = "Downloads synced", body = DownloadSyncResponse),
        (status = 400, description = "Sync failed"),
    )
)]
pub async fn handle_sync(
    State(state): State<SharedState>,
    Json(req): Json<DownloadSyncRequest>,
) -> impl IntoResponse {
    let cfg = state.context.cfg().await;
    let client = match DownloadClient::from_config(&req.source_type, &cfg, state.context.client()) {
        Ok(c) => c,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
                .into_response();
        }
    };

    let db = state.context.db().await;
    match client.sync(db.pool()).await {
        Ok(synced) => Json(DownloadSyncResponse {
            id: req.id,
            synced,
            success: true,
            error: None,
        })
        .into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}
