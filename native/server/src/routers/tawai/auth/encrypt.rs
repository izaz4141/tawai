use crate::utils::server::SharedState;
use axum::{Json, extract::State, response::IntoResponse};
use tawai_core::signals::crypt::{EncryptRequest, EncryptResponse};

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
    let master_key = match state.master_key() {
        Ok(key) => key,
        Err(status) => return status.into_response(),
    };
    let encrypted_text = match master_key.encrypt(&req.plain_key) {
        Ok(encrypted) => encrypted,
        Err(e) => {
            tawai_core::utils::logger::error(&format!("Unable to encrypt text: {:#}", &e));
            return (axum::http::StatusCode::INTERNAL_SERVER_ERROR,).into_response();
        }
    };
    Json(EncryptResponse {
        id: req.id,
        encrypted_key: encrypted_text,
    })
    .into_response()
}
