use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use tawai_core::dclient::DownloadClient;
use tawai_core::signals::download::{DownloadResumeRequest, DownloadResumeResponse};

use crate::server::SharedState;

#[utoipa::path(
    post,
    path = "/api/tawai/download/resume",
    tags = ["tawai.download"],
    security(("ApiKeyAuth" = [])),
    request_body = DownloadResumeRequest,
    responses(
        (status = 200, description = "Download resumed", body = DownloadResumeResponse),
        (status = 400, description = "Resume failed"),
    )
)]
pub async fn handle_resume(
    State(state): State<SharedState>,
    Json(req): Json<DownloadResumeRequest>,
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

    match client.resume(&req.download_id).await {
        Ok(()) => Json(DownloadResumeResponse {
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
