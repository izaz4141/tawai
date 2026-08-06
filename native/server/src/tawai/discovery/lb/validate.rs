use axum::{Json, extract::State, response::IntoResponse};
use serde::Deserialize;
use tawai_core::discovery::listenbrainz;
use tawai_core::signals::discovery::ValidateLBTokenResponse;
use utoipa::ToSchema;

use crate::server::SharedState;

#[derive(Deserialize, ToSchema)]
pub struct ValidateTokenPayload {
    pub token: String,
}

#[utoipa::path(
    post,
    path = "/api/tawai/discovery/lb/validate",
    tags = ["tawai.discovery"],
    security(("ApiKeyAuth" = [])),
    request_body = ValidateTokenPayload,
    responses(
        (status = 200, description = "Token validation result", body = ValidateLBTokenResponse),
    )
)]
pub async fn handle_validate_lb_token(
    State(state): State<SharedState>,
    Json(payload): Json<ValidateTokenPayload>,
) -> impl IntoResponse {
    match listenbrainz::validate_token(state.context.client(), &payload.token).await {
        Ok(v) => (axum::http::StatusCode::OK, Json(v)).into_response(),
        Err(e) => (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}
