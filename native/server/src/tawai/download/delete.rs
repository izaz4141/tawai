use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use tawai_core::dclient::DownloadClient;
use tawai_core::signals::download::{DownloadDeleteRequest, DownloadDeleteResponse};

use crate::server::SharedState;

#[utoipa::path(
    post,
    path = "/api/tawai/download/delete",
    tags = ["tawai.download"],
    security(("ApiKeyAuth" = [])),
    request_body = DownloadDeleteRequest,
    responses(
        (status = 200, description = "Download deleted", body = DownloadDeleteResponse),
        (status = 400, description = "Delete failed"),
    )
)]
pub async fn handle_delete(
    State(state): State<SharedState>,
    Json(req): Json<DownloadDeleteRequest>,
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

    match client.delete(&req.download_id, req.delete_file).await {
        Ok(()) => Json(DownloadDeleteResponse {
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
