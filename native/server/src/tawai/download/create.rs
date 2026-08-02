use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use tawai_core::dclient::DownloadClient;
use tawai_core::signals::download::{DownloadCreateRequest, DownloadCreateResponse};

use crate::server::SharedState;

#[utoipa::path(
    post,
    path = "/api/tawai/download/create",
    tags = ["tawai.download"],
    security(("ApiKeyAuth" = [])),
    request_body = DownloadCreateRequest,
    responses(
        (status = 200, description = "Download created", body = DownloadCreateResponse),
        (status = 400, description = "Create failed"),
    )
)]
pub async fn handle_create(
    State(state): State<SharedState>,
    Json(req): Json<DownloadCreateRequest>,
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

    let extra = req
        .extra
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok());
    match client.create(&req.url, &req.dest, extra).await {
        Ok(download_id) => Json(DownloadCreateResponse {
            id: req.id,
            download_id,
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
