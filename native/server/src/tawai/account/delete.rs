use crate::server::SharedState;
use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use tawai_core::db::account;
use tawai_core::signals::account::{DeleteAccountRequest, DeleteAccountResponse};
use tawai_core::utils::{logger, security};

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

    let admin = account::get_user_by_username(db.pool(), &payload.admin_username, &mk)
        .await
        .unwrap_or(None);

    let is_admin = match &admin {
        Some(u) => {
            u.role == "admin"
                && security::validate_password(&u.password_hash, &payload.admin_password)
                    .unwrap_or(false)
        }
        None => false,
    };

    if !is_admin {
        return (
            StatusCode::UNAUTHORIZED,
            Json(DeleteAccountResponse {
                id: payload.id,
                success: false,
                username: payload.target_username,
            }),
        )
            .into_response();
    }

    let target = account::get_user_by_username(db.pool(), &payload.target_username, &mk)
        .await
        .unwrap_or(None);

    let target = match target {
        Some(u) => u,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(DeleteAccountResponse {
                    id: payload.id,
                    success: false,
                    username: payload.target_username,
                }),
            )
                .into_response();
        }
    };

    if target.role == "admin" {
        let users = account::get_all_users(db.pool()).await.unwrap_or_default();
        let admin_count = users.iter().filter(|u| u.role == "admin").count();
        if admin_count <= 1 {
            return (
                StatusCode::BAD_REQUEST,
                Json(DeleteAccountResponse {
                    id: payload.id,
                    success: false,
                    username: payload.target_username,
                }),
            )
                .into_response();
        }
    }

    match account::delete_user(db.pool(), &target.id).await {
        Ok(true) => (
            StatusCode::OK,
            Json(DeleteAccountResponse {
                id: payload.id,
                success: true,
                username: payload.target_username,
            }),
        )
            .into_response(),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(DeleteAccountResponse {
                id: payload.id,
                success: false,
                username: payload.target_username,
            }),
        )
            .into_response(),
        Err(e) => {
            logger::error(&format!("Failed to delete user: {}", e));
            (StatusCode::INTERNAL_SERVER_ERROR,).into_response()
        }
    }
}
