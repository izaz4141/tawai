use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use tawai_core::dclient::DownloadClient;
use tawai_core::signals::download::{DownloadCancelRequest, DownloadCancelResponse};

use crate::server::SharedState;

#[utoipa::path(
    post,
    path = "/api/tawai/download/cancel",
    tags = ["tawai.download"],
    security(("ApiKeyAuth" = [])),
    request_body = DownloadCancelRequest,
    responses(
        (status = 200, description = "Download cancelled", body = DownloadCancelResponse),
        (status = 400, description = "Cancel failed"),
    )
)]
pub async fn handle_cancel(
    State(state): State<SharedState>,
    Json(req): Json<DownloadCancelRequest>,
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
    match client.cancel(&req.download_id, None, Some(db.pool())).await {
        Ok(()) => Json(DownloadCancelResponse {
            id: req.id,
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
