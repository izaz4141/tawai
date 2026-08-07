use crate::server::SharedState;
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
    let mk = state.context.master_key.read().await.clone();

    let password = match headers.get("X-Password").and_then(|v| v.to_str().ok()) {
        Some(p) => p,
        None => return StatusCode::UNAUTHORIZED,
    };

    let valid = match account::get_user_by_id(db.pool(), &user_id, &mk).await {
        Ok(Some(user)) => security::validate_password(&user.password_hash, password).unwrap_or(false),
        _ => false,
    };

    if valid {
        StatusCode::OK
    } else {
        StatusCode::UNAUTHORIZED
    }
}
