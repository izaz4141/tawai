use crate::server::SharedState;
use axum::{Json, extract::State, response::IntoResponse};
use tawai_core::signals::crypt::{DecryptRequest, DecryptResponse};
use tawai_core::utils::encryption;

#[utoipa::path(
    post,
    path = "/api/tawai/auth/decrypt",
    tags = ["tawai.auth"],
    security(("ApiKeyAuth" = [])),
    request_body = DecryptRequest,
    responses(
        (status = 200, description = "Key decrypted successfully", body = DecryptResponse),
        (status = 500, description = "Decryption failed"),
    )
)]
pub async fn handle_decrypt(
    State(state): State<SharedState>,
    Json(req): Json<DecryptRequest>,
) -> impl IntoResponse {
    let master_key = state.context.master_key.read().await.clone();
    let decrypted_key = match encryption::decrypt(&req.encrypted_key, &master_key) {
        Ok(key) => key,
        Err(e) => {
            tawai_core::utils::logger::error(&format!("Unable to decrypt key: {:#}", &e));
            return (axum::http::StatusCode::INTERNAL_SERVER_ERROR,).into_response();
        }
    };
    Json(DecryptResponse {
        id: req.id,
        decrypted_key,
    })
    .into_response()
}
