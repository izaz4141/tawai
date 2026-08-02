use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use tawai_core::dclient::DownloadClient;
use tawai_core::signals::download::{DownloadPauseRequest, DownloadPauseResponse};

use crate::server::SharedState;

#[utoipa::path(
    post,
    path = "/api/tawai/download/pause",
    tags = ["tawai.download"],
    security(("ApiKeyAuth" = [])),
    request_body = DownloadPauseRequest,
    responses(
        (status = 200, description = "Download paused", body = DownloadPauseResponse),
        (status = 400, description = "Pause failed"),
    )
)]
pub async fn handle_pause(
    State(state): State<SharedState>,
    Json(req): Json<DownloadPauseRequest>,
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

    match client.pause(&req.download_id).await {
        Ok(()) => Json(DownloadPauseResponse {
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
