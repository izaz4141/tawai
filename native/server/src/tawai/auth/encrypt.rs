use crate::server::SharedState;
use axum::{Json, extract::State, response::IntoResponse};
use tawai_core::signals::crypt::{EncryptRequest, EncryptResponse};
use tawai_core::utils::encryption;

#[utoipa::path(
    post,
    path = "/api/tawai/auth/encrypt",
    tags = ["tawai.auth"],
    security(("ApiKeyAuth" = [])),
    request_body = EncryptRequest,
    responses(
        (status = 200, description = "Text encrypted successfully", body = EncryptResponse),
        (status = 500, description = "Encryption Failed")
    )
)]
pub async fn handle_encrypt(
    State(state): State<SharedState>,
    Json(req): Json<EncryptRequest>,
) -> impl IntoResponse {
    let master_key = state.context.master_key.read().await.clone();
    let encrypted_text = match encryption::encrypt(&req.plain_key, &master_key) {
        Ok(encrypted) => encrypted,
        Err(e) => {
            tawai_core::utils::logger::error(&format!("Unable to encrypt text: {:#}", &e));
            return (axum::http::StatusCode::INTERNAL_SERVER_ERROR,).into_response();
        }
    };
    Json(EncryptResponse {
        id: req.id,
        encrypted_key: encrypted_text,
        master_key,
    })
    .into_response()
}
