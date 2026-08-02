use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use tawai_core::dclient::DownloadClient;
use tawai_core::signals::download::{DownloadGetInfoRequest, DownloadGetInfoResponse};

use crate::server::SharedState;

#[utoipa::path(
    post,
    path = "/api/tawai/download/get-info",
    tags = ["tawai.download"],
    security(("ApiKeyAuth" = [])),
    request_body = DownloadGetInfoRequest,
    responses(
        (status = 200, description = "Info retrieved", body = DownloadGetInfoResponse),
        (status = 400, description = "Failed"),
    )
)]
pub async fn handle_get_info(
    State(state): State<SharedState>,
    Json(req): Json<DownloadGetInfoRequest>,
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

    match client.get_info(&req.url).await {
        Ok(info) => Json(DownloadGetInfoResponse {
            id: req.id,
            info: info.to_string(),
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
