use crate::utils::server::SharedState;
use axum::extract::State;
use axum::response::IntoResponse;

#[utoipa::path(
    post,
    path = "/api/tawai/system/restart",
    tags = ["tawai.system"],
    security(("ApiKeyAuth" = [])),
    responses(
        (status = 200, description = "Server restart initiated")
    )
)]
pub async fn handle_restart(State(state): State<SharedState>) -> impl IntoResponse {
    state.context.operation_signals.request_restart();
    axum::http::StatusCode::OK
}
