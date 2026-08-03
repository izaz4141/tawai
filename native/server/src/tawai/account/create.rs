use crate::server::SharedState;
use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::http::header::{HeaderMap, HeaderName};
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};
use tawai_core::db::account;
use tawai_core::utils::{encryption, helper, logger, security};
use utoipa::ToSchema;

const X_PASSWORD: HeaderName = HeaderName::from_static("x-password");

fn sanitize_role(role: Option<&str>) -> &'static str {
    match role {
        Some("admin") => "admin",
        _ => "user",
    }
}

#[derive(Deserialize, ToSchema)]
pub struct CreateAccountRequest {
    pub admin_username: String,
    pub username: String,
    pub password: String,
    pub display_name: Option<String>,
    pub role: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct CreateAccountResponse {
    pub success: bool,
    pub user_id: String,
    pub username: String,
    pub display_name: String,
    pub role: String,
    pub api_key: String,
}

#[utoipa::path(
    post,
    path = "/api/tawai/account/create",
    tags = ["tawai.account"],
    security(("ApiKeyAuth" = [])),
    params(
        ("X-Password" = String, Header, description = "Admin password"),
    ),
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
    headers: HeaderMap,
    Json(payload): Json<CreateAccountRequest>,
) -> impl IntoResponse {
    let admin_password = headers
        .get(X_PASSWORD)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();

    let db = state.context.db().await;
    let mk = state.context.master_key.read().await.clone();

    let admin = account::get_user_by_username(db.pool(), &payload.admin_username, &mk)
        .await
        .unwrap_or(None);

    let is_admin = match &admin {
        Some(u) => {
            u.role == "admin"
                && security::validate_password(&u.password_hash, admin_password).unwrap_or(false)
        }
        None => false,
    };

    if !is_admin {
        return (
            StatusCode::UNAUTHORIZED,
            Json(CreateAccountResponse {
                success: false,
                user_id: String::new(),
                username: payload.username,
                display_name: String::new(),
                role: String::new(),
                api_key: String::new(),
            }),
        )
            .into_response();
    }

    let taken = account::get_user_by_username(db.pool(), &payload.username, &mk)
        .await
        .unwrap_or(None)
        .is_some();
    if taken {
        return (
            StatusCode::CONFLICT,
            Json(CreateAccountResponse {
                success: false,
                user_id: String::new(),
                username: payload.username,
                display_name: String::new(),
                role: String::new(),
                api_key: String::new(),
            }),
        )
            .into_response();
    }

    let salt = security::generate_salt();
    let hash = match security::hash_password(&payload.password, &salt) {
        Ok(h) => h,
        Err(e) => {
            logger::error(&format!("Failed to hash password: {}", e));
            return (StatusCode::INTERNAL_SERVER_ERROR,).into_response();
        }
    };

    let api_key = uuid::Uuid::new_v4().to_string();
    let encrypted = encryption::encrypt(&api_key, &mk).unwrap_or_else(|_| api_key.clone());
    let api_key_hash = helper::sha256_hex(&api_key);
    let display_name = payload.display_name.clone().unwrap_or_default();
    let role = sanitize_role(payload.role.as_deref());

    match account::create_user(
        db.pool(),
        &payload.username,
        &display_name,
        &hash,
        &encrypted,
        &api_key_hash,
        role,
    )
    .await
    {
        Ok(user_id) => (
            StatusCode::OK,
            Json(CreateAccountResponse {
                success: true,
                user_id,
                username: payload.username,
                display_name,
                role: role.to_string(),
                api_key,
            }),
        )
            .into_response(),
        Err(e) => {
            logger::error(&format!("Failed to create user: {}", e));
            (StatusCode::INTERNAL_SERVER_ERROR,).into_response()
        }
    }
}
