use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use tawai_core::dclient::DownloadClient;
use tawai_core::signals::download::{
    DlGlance, DownloadClientListRequest, DownloadClientListResponse,
};

use crate::server::SharedState;

#[utoipa::path(
    post,
    path = "/api/tawai/download/client-list",
    tags = ["tawai.download"],
    security(("ApiKeyAuth" = [])),
    request_body = DownloadClientListRequest,
    responses(
        (status = 200, description = "Client download list", body = DownloadClientListResponse),
        (status = 400, description = "List failed"),
    )
)]
pub async fn handle_client_list(
    State(state): State<SharedState>,
    Json(req): Json<DownloadClientListRequest>,
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

    match client
        .list(
            req.offset.unwrap_or(0),
            req.limit.unwrap_or(50),
            req.statuses.unwrap_or_default(),
        )
        .await
    {
        Ok(response) => Json(DownloadClientListResponse {
            id: req.id,
            downloads: response.downloads,
            total_count: response.total_count,
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
