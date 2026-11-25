use crate::utils::server::SharedState;
use axum::extract::{Extension, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use tawai_core::db::account;
use tawai_core::utils::security;

#[utoipa::path(
    post,
    path = "/api/tawai/auth/verify-password",
    tags = ["tawai.auth"],
    security(("ApiKeyAuth" = [])),
    params(
        ("X-Password" = String, Header, description = "Current password to verify")
    ),
    responses(
        (status = 200, description = "Password verified successfully"),
        (status = 401, description = "Invalid password")
    )
)]
pub async fn handle_verify_password(
    State(state): State<SharedState>,
    Extension(user_id): Extension<String>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let db = state.context.db().await;
    let mk = match state.master_key() {
        Ok(key) => key,
        Err(status) => return status.into_response(),
    };

    let password = match headers.get("X-Password").and_then(|v| v.to_str().ok()) {
        Some(p) => p,
        None => return StatusCode::UNAUTHORIZED.into_response(),
    };

    let valid = match account::get_user_by_id(db.pool(), &user_id, &mk).await {
        Ok(user) => security::validate_password(&user.password_hash, password).unwrap_or(false),
        Err(_) => false,
    };

    if valid {
        StatusCode::OK.into_response()
    } else {
        StatusCode::UNAUTHORIZED.into_response()
    }
}
