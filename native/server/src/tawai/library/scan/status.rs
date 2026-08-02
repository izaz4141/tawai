use axum::{Json, extract::State, response::IntoResponse};
use serde_json::json;
use std::sync::atomic::Ordering;

use crate::server::SharedState;

pub async fn handle_scan_status(State(state): State<SharedState>) -> impl IntoResponse {
    let running = state.context.scan_running.load(Ordering::SeqCst);
    let progress = state.context.scan_progress.read().await.clone();
    Json(json!({ "running": running, "progress": progress }))
}
