use crate::server::SharedState;
use axum::http::StatusCode;
use axum::http::header::{HeaderMap, HeaderName};
use axum::response::IntoResponse;
use axum::{Extension, extract::State};
use tawai_core::db::account;
use tawai_core::utils::security;

const X_PASSWORD: HeaderName = HeaderName::from_static("x-password");

#[utoipa::path(
    post,
    path = "/api/tawai/auth/verify-password",
    tags = ["tawai.auth"],
    security(("ApiKeyAuth" = [])),
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
    let password = headers
        .get(X_PASSWORD)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();

    let db = state.context.db().await;
    let mk = state.context.master_key.read().await.clone();
    match account::get_user_by_id(db.pool(), &user_id, &mk).await {
        Ok(Some(user)) => {
            if security::validate_password(&user.password_hash, password).unwrap_or(false) {
                (StatusCode::OK,).into_response()
            } else {
                (StatusCode::UNAUTHORIZED,).into_response()
            }
        }
        _ => (StatusCode::UNAUTHORIZED,).into_response(),
    }
}
