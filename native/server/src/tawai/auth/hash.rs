use axum::http::StatusCode;
use axum::{Json, response::IntoResponse};
use tawai_core::signals::crypt::{HashPassword, HashingOutput};
use tawai_core::utils::security;

#[utoipa::path(
    post,
    path = "/api/tawai/auth/hash",
    tags = ["tawai.auth"],
    security(("ApiKeyAuth" = [])),
    request_body = HashPassword,
    responses(
        (status = 200, description = "Hash generated successfully", body = HashingOutput),
        (status = 500, description = "Hashing failed")
    )
)]
pub async fn handle_hashing_password(Json(payload): Json<HashPassword>) -> impl IntoResponse {
    match security::hash_password(&payload.plain_text, &payload.salt) {
        Ok(encrypted) => (
            StatusCode::OK,
            Json(HashingOutput {
                id: payload.id,
                hashed_text: Some(encrypted),
            }),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(HashingOutput {
                id: payload.id,
                hashed_text: None,
            }),
        )
            .into_response(),
    }
}
