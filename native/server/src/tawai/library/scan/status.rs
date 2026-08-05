use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use std::sync::atomic::Ordering;
use tawai_core::signals::library::{ScanStatusRequest, ScanStatusResponse};

use crate::server::SharedState;

#[utoipa::path(
    post,
    path = "/api/tawai/library/scan/status",
    tags = ["tawai.library.scan"],
    security(("ApiKeyAuth" = [])),
    request_body = ScanStatusRequest,
    responses(
        (status = 200, description = "Current scan status", body = ScanStatusResponse),
        (status = 403, description = "Forbidden"),
    )
)]
pub async fn handle_scan_status(
    State(state): State<SharedState>,
    Json(req): Json<ScanStatusRequest>,
) -> impl IntoResponse {
    let running = state.context.scan_running.load(Ordering::SeqCst);
    let progress = state.context.scan_progress.read().await.clone();
    (
        StatusCode::OK,
        Json(ScanStatusResponse {
            id: req.id,
            running,
            progress,
        }),
    )
        .into_response()
}
