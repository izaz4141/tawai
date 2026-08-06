use crate::server::SharedState;
use axum::response::IntoResponse;
use axum::{Json, extract::State};
use tawai_core::db::account;
use tawai_core::signals::account::{VerifyCurrentPasswordRequest, VerifyCurrentPasswordResponse};
use tawai_core::utils::security;

#[utoipa::path(
    post,
    path = "/api/tawai/auth/verify-password",
    tags = ["tawai.auth"],
    security(("ApiKeyAuth" = [])),
    request_body = VerifyCurrentPasswordRequest,
    responses(
        (status = 200, description = "Password verified successfully", body = VerifyCurrentPasswordResponse),
        (status = 401, description = "Invalid password")
    )
)]
pub async fn handle_verify_password(
    State(state): State<SharedState>,
    Json(payload): Json<VerifyCurrentPasswordRequest>,
) -> impl IntoResponse {
    let db = state.context.db().await;
    let mk = state.context.master_key.read().await.clone();

    let valid = match account::get_user_by_username(db.pool(), &payload.username, &mk).await {
        Ok(Some(user)) => {
            security::validate_password(&user.password_hash, &payload.password).unwrap_or(false)
        }
        _ => false,
    };

    Json(VerifyCurrentPasswordResponse {
        id: payload.id,
        valid,
    })
}
