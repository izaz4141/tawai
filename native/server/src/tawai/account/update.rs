use crate::security::create_jwt_response;
use crate::server::{SharedState, build_jwt_cookie};
use axum::extract::State;
use axum::http::StatusCode;
use axum::http::header::{HeaderMap, HeaderName};
use axum::response::IntoResponse;
use axum::{Extension, Json};
use axum_extra::extract::CookieJar;
use serde::{Deserialize, Serialize};
use tawai_core::db::account;
use tawai_core::utils::{logger, security};
use utoipa::ToSchema;

const X_PASSWORD: HeaderName = HeaderName::from_static("x-password");

fn sanitize_role(role: Option<&str>) -> &'static str {
    match role {
        Some("admin") => "admin",
        _ => "user",
    }
}

#[derive(Deserialize, ToSchema)]
pub struct UpdateAccountRequest {
    pub target_username: String,
    pub new_username: Option<String>,
    pub new_password: Option<String>,
    pub display_name: Option<String>,
    pub role: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct UpdateAccountResponse {
    pub success: bool,
    pub user_id: String,
    pub username: String,
    pub display_name: String,
    pub role: String,
    pub api_key: String,
    pub access_token: String,
    pub csrf_token: String,
    pub expires_in: u64,
}

#[utoipa::path(
    post,
    path = "/api/tawai/account/update",
    tags = ["tawai.account"],
    security(("ApiKeyAuth" = [])),
    params(
        ("X-Password" = String, Header, description = "Current password"),
    ),
    request_body = UpdateAccountRequest,
    responses(
        (status = 200, description = "Credentials updated successfully"),
        (status = 401, description = "Invalid current password"),
        (status = 403, description = "Not authorized to edit this account"),
        (status = 500, description = "Server error")
    ),
)]
pub async fn handle_update_account(
    State(state): State<SharedState>,
    Extension(user_id): Extension<String>,
    jar: CookieJar,
    headers: HeaderMap,
    Json(payload): Json<UpdateAccountRequest>,
) -> impl IntoResponse {
    let current_password = headers
        .get(X_PASSWORD)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();

    let db = state.context.db().await;
    let mk = state.context.master_key.read().await.clone();

    let authorizer = account::get_user_by_id(db.pool(), &user_id, &mk)
        .await
        .unwrap_or(None);

    let authorizer = match authorizer {
        Some(u) => u,
        None => return (StatusCode::UNAUTHORIZED,).into_response(),
    };

    let is_valid =
        security::validate_password(&authorizer.password_hash, current_password).unwrap_or(false);
    if !is_valid {
        return (StatusCode::UNAUTHORIZED,).into_response();
    }

    let is_admin = authorizer.role == "admin";
    if payload.target_username != authorizer.username && !is_admin {
        return (StatusCode::FORBIDDEN,).into_response();
    }

    let target = account::get_user_by_username(db.pool(), &payload.target_username, &mk)
        .await
        .unwrap_or(None);

    let target = match target {
        Some(u) => u,
        None => return (StatusCode::NOT_FOUND,).into_response(),
    };

    let mut result_username = target.username.clone();
    if let Some(new_username) = &payload.new_username {
        if !new_username.is_empty() {
            if let Err(e) = account::change_username(db.pool(), &target.id, new_username).await {
                logger::error(&format!("Failed to change username: {}", e));
                return (StatusCode::INTERNAL_SERVER_ERROR,).into_response();
            }
            result_username = new_username.clone();
        }
    }

    if let Some(new_password) = &payload.new_password {
        if !new_password.is_empty() {
            match security::hash_password(new_password, &target.password_hash) {
                Ok(hashed) => {
                    if let Err(e) = account::change_password(db.pool(), &target.id, &hashed).await {
                        logger::error(&format!("Failed to change password: {}", e));
                        return (StatusCode::INTERNAL_SERVER_ERROR,).into_response();
                    }
                }
                Err(e) => {
                    logger::error(&format!("Failed to hash password: {}", e));
                    return (StatusCode::INTERNAL_SERVER_ERROR,).into_response();
                }
            }
        }
    }

    if is_admin {
        let role = sanitize_role(payload.role.as_deref());
        if role != target.role {
            if let Err(e) = account::change_role(db.pool(), &target.id, role).await {
                logger::error(&format!("Failed to change role: {}", e));
                return (StatusCode::INTERNAL_SERVER_ERROR,).into_response();
            }
        }
    }

    if let Some(display_name) = &payload.display_name {
        if !display_name.is_empty() {
            if let Err(e) = account::change_display_name(db.pool(), &target.id, display_name).await
            {
                logger::error(&format!("Failed to change display name: {}", e));
                return (StatusCode::INTERNAL_SERVER_ERROR,).into_response();
            }
        }
    }

    let updated_user = account::get_user_by_username(db.pool(), &result_username, &mk)
        .await
        .unwrap_or(None);

    let (user_id, display_name, role, api_key) = match updated_user {
        Some(u) => (u.id, u.display_name, u.role, u.api_key),
        None => (
            target.id.clone(),
            target.display_name.clone(),
            target.role.clone(),
            String::new(),
        ),
    };

    let jwt_response = create_jwt_response(&state, &authorizer.id).await.unwrap();
    let jar = build_jwt_cookie(jar, &jwt_response);

    (
        StatusCode::OK,
        jar,
        axum::Json(UpdateAccountResponse {
            success: true,
            user_id,
            username: result_username,
            display_name,
            role,
            api_key,
            access_token: jwt_response.access_token,
            csrf_token: jwt_response.csrf_token,
            expires_in: jwt_response.expires_in,
        }),
    )
        .into_response()
}
