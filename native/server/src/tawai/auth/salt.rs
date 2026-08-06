use axum::response::IntoResponse;
use axum::{Json, extract::Query};
use tawai_core::signals::crypt::{GenerateSalt, SaltOutput};
use tawai_core::utils::security;

#[utoipa::path(
    get,
    path = "/api/tawai/auth/generate-salt",
    tags = ["tawai.auth"],
    security(("ApiKeyAuth" = [])),
    responses(
        (status = 200, description = "Salt generated successfully", body = SaltOutput)
    )
)]
pub async fn handle_generate_salt(Query(query): Query<GenerateSalt>) -> impl IntoResponse {
    Json(SaltOutput {
        id: query.id,
        salt: security::generate_salt(),
    })
}
