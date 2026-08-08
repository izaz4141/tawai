use crate::security::create_jwt_response;
use crate::server::{SharedState, build_jwt_cookie};
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{Extension, Json};
use axum_extra::extract::CookieJar;
use tawai_core::signals::account::UpdateAccountRequest;
use tawai_core::utils::account::{AccountError, update_user};

#[utoipa::path(
    post,
    path = "/api/tawai/account/update",
    tags = ["tawai.account"],
    security(("ApiKeyAuth" = [])),
    request_body = UpdateAccountRequest,
    responses(
        (status = 200, description = "Credentials updated successfully"),
        (status = 401, description = "Invalid current password"),
        (status = 403, description = "Not authorized to edit this account"),
        (status = 404, description = "Target account not found"),
        (status = 500, description = "Server error")
    ),
)]
pub async fn handle_update_account(
    State(state): State<SharedState>,
    Extension(_user_id): Extension<String>,
    jar: CookieJar,
    Json(payload): Json<UpdateAccountRequest>,
) -> impl IntoResponse {
    let db = state.context.db().await;
    let mk = state.context.master_key.read().await.clone();

    match update_user(&db, &mk, &payload).await {
        Ok(response) => {
            let jwt_response = match create_jwt_response(&state, &payload.operator_user_id).await {
                Ok(res) => res,
                Err(e) => {
                    tawai_core::utils::logger::error(&format!("Failed to refresh JWT: {}", e));
                    return (StatusCode::INTERNAL_SERVER_ERROR,).into_response();
                }
            };
            let jar = build_jwt_cookie(jar, &jwt_response);
            (StatusCode::OK, jar, axum::Json(response)).into_response()
        }
        Err(e) => {
            let status = match e.downcast_ref::<AccountError>() {
                Some(AccountError::Unauthorized) => StatusCode::UNAUTHORIZED,
                Some(AccountError::Forbidden) => StatusCode::FORBIDDEN,
                Some(AccountError::NotFound) => StatusCode::NOT_FOUND,
                Some(AccountError::Conflict) => StatusCode::CONFLICT,
                Some(AccountError::BadRequest) => StatusCode::BAD_REQUEST,
                _ => {
                    tawai_core::utils::logger::error(&format!("update account failed: {}", e));
                    StatusCode::INTERNAL_SERVER_ERROR
                }
            };
            (status,).into_response()
        }
    }
}
