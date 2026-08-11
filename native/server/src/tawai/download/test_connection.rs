use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use tawai_core::dclient::DownloadClient;
use tawai_core::signals::download::{
    DownloadTestConnectionRequest, DownloadTestConnectionResponse,
};

use crate::server::SharedState;

#[utoipa::path(
    post,
    path = "/api/tawai/download/test-connection",
    tags = ["tawai.download"],
    security(("ApiKeyAuth" = [])),
    request_body = DownloadTestConnectionRequest,
    responses(
        (status = 200, description = "Connection OK", body = DownloadTestConnectionResponse),
        (status = 400, description = "Connection failed"),
    )
)]
pub async fn handle_test_connection(
    State(state): State<SharedState>,
    Json(req): Json<DownloadTestConnectionRequest>,
) -> impl IntoResponse {
    let cfg = state.context.cfg().await;
    match DownloadClient::test_connection(&req, &cfg, state.context.client()).await {
        Ok(version) => Json(DownloadTestConnectionResponse {
            id: req.id,
            success: true,
            version: Some(version),
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
