use crate::server::SharedState;
use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use tawai_core::signals::account::{DeleteAccountRequest, DeleteAccountResponse};
use tawai_core::utils::account::{AccountError, delete_user};

#[utoipa::path(
    post,
    path = "/api/tawai/account/delete",
    tags = ["tawai.account"],
    security(("ApiKeyAuth" = [])),
    request_body = DeleteAccountRequest,
    responses(
        (status = 200, description = "Account deleted"),
        (status = 400, description = "Cannot delete the last admin"),
        (status = 401, description = "Invalid admin credentials"),
        (status = 404, description = "Target account not found"),
        (status = 500, description = "Server error")
    ),
)]
pub async fn handle_delete_account(
    State(state): State<SharedState>,
    Json(payload): Json<DeleteAccountRequest>,
) -> impl IntoResponse {
    let db = state.context.db().await;
    let mk = state.context.master_key.read().await.clone();

    match delete_user(&db, &mk, &payload).await {
        Ok(response) => (StatusCode::OK, Json(response)).into_response(),
        Err(e) => match e.downcast_ref::<AccountError>() {
            Some(AccountError::Unauthorized) => (
                StatusCode::UNAUTHORIZED,
                Json(DeleteAccountResponse {
                    id: payload.id,
                    success: false,
                    username: payload.target_username,
                }),
            )
                .into_response(),
            Some(AccountError::NotFound) => (
                StatusCode::NOT_FOUND,
                Json(DeleteAccountResponse {
                    id: payload.id,
                    success: false,
                    username: payload.target_username,
                }),
            )
                .into_response(),
            Some(AccountError::BadRequest) => (
                StatusCode::BAD_REQUEST,
                Json(DeleteAccountResponse {
                    id: payload.id,
                    success: false,
                    username: payload.target_username,
                }),
            )
                .into_response(),
            _ => {
                tawai_core::utils::logger::error(&format!("Failed to delete user: {}", e));
                (StatusCode::INTERNAL_SERVER_ERROR,).into_response()
            }
        },
    }
}
