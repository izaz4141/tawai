use axum::response::IntoResponse;
use axum::{Json, extract::Query};
use tawai_core::signals::crypt::{GenerateMasterKeyRequest, GenerateMasterKeyResponse};
use tawai_core::utils::encryption;

#[utoipa::path(
    get,
    path = "/api/tawai/auth/generate-master-key",
    tags = ["tawai.auth"],
    security(("ApiKeyAuth" = [])),
    responses(
        (status = 200, description = "Master key generated successfully", body = GenerateMasterKeyResponse)
    )
)]
pub async fn handle_generate_master_key(
    Query(query): Query<GenerateMasterKeyRequest>,
) -> impl IntoResponse {
    Json(GenerateMasterKeyResponse {
        id: query.id,
        master_key: encryption::generate_master_key(),
    })
}
