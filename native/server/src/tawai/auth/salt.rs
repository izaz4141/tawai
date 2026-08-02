use axum::response::IntoResponse;
use tawai_core::utils::security;

#[utoipa::path(
    get,
    path = "/api/tawai/auth/generate-salt",
    tags = ["tawai.auth"],
    security(("ApiKeyAuth" = [])),
    responses(
        (status = 200, description = "Salt generated successfully", body = String)
    )
)]
pub async fn handle_generate_salt() -> impl IntoResponse {
    security::generate_salt()
}
