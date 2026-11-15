use crate::server::SharedState;
use axum::{Json, extract::State, response::IntoResponse};
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
pub struct StatusResponse {
    pub status: String,
    pub version: String,
}

#[utoipa::path(
    get,
    path = "/api/tawai/system/status",
    tags = ["tawai.system"],
    responses(
        (status = 200, description = "Server status", body = StatusResponse)
    )
)]
pub async fn handle_status(State(state): State<SharedState>) -> impl IntoResponse {
    let res = StatusResponse {
        status: "Online".to_string(),
        version: state.context.version().await.to_string(),
    };

    Json(res)
}
