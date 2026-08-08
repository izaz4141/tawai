use crate::server::SharedState;
use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use tawai_core::signals::account::{CreateAccountRequest, CreateAccountResponse};
use tawai_core::utils::account::{AccountError, create_user};

#[utoipa::path(
    post,
    path = "/api/tawai/account/create",
    tags = ["tawai.account"],
    security(("ApiKeyAuth" = [])),
    request_body = CreateAccountRequest,
    responses(
        (status = 200, description = "Account created"),
        (status = 401, description = "Invalid admin credentials"),
        (status = 409, description = "Username already taken"),
        (status = 500, description = "Server error")
    ),
)]
pub async fn handle_create_account(
    State(state): State<SharedState>,
    Json(payload): Json<CreateAccountRequest>,
) -> impl IntoResponse {
    let db = state.context.db().await;
    let mk = state.context.master_key.read().await.clone();

    match create_user(&db, &mk, &payload).await {
        Ok(response) => (StatusCode::OK, Json(response)).into_response(),
        Err(e) => match e.downcast_ref::<AccountError>() {
            Some(AccountError::Unauthorized) => (
                StatusCode::UNAUTHORIZED,
                Json(CreateAccountResponse {
                    id: payload.id,
                    success: false,
                    user_id: String::new(),
                    username: payload.username,
                    display_name: String::new(),
                    role: String::new(),
                    api_key: String::new(),
                }),
            )
                .into_response(),
            Some(AccountError::Conflict) => (
                StatusCode::CONFLICT,
                Json(CreateAccountResponse {
                    id: payload.id,
                    success: false,
                    user_id: String::new(),
                    username: payload.username,
                    display_name: String::new(),
                    role: String::new(),
                    api_key: String::new(),
                }),
            )
                .into_response(),
            _ => {
                tawai_core::utils::logger::error(&format!("Failed to create user: {}", e));
                (StatusCode::INTERNAL_SERVER_ERROR,).into_response()
            }
        },
    }
}
