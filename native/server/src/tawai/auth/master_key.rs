use axum::response::IntoResponse;
use tawai_core::utils::encryption;

#[utoipa::path(
    get,
    path = "/api/tawai/auth/generate-master-key",
    tags = ["tawai.auth"],
    security(("ApiKeyAuth" = [])),
    responses(
        (status = 200, description = "Master key generated successfully", body = String)
    )
)]
pub async fn handle_generate_master_key() -> impl IntoResponse {
    encryption::generate_master_key()
}
