use crate::security::{create_jwt_response, validate_jwt_request};
use crate::server::{SharedState, build_jwt_cookie};
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use axum_extra::{TypedHeader, extract::CookieJar};
use headers::{Authorization, authorization::Basic};
use serde::Serialize;
use tawai_core::signals::User;
use tawai_core::utils::{logger, security};
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
pub struct LoginResponse {
    pub user: User,
    pub access_token: String,
    pub csrf_token: String,
    pub expires_in: u64,
}

#[utoipa::path(
    post,
    path = "/api/tawai/auth/login",
    tags = ["tawai.auth"],
    security(("BasicAuth" = [])),
    responses(
        (status = 200, description = "Login successful", body = LoginResponse),
        (status = 401, description = "Invalid credentials")
    )
)]
pub async fn handle_login(
    State(state): State<SharedState>,
    jar: CookieJar,
    headers: HeaderMap,
    TypedHeader(Authorization(auth)): TypedHeader<Authorization<Basic>>,
) -> impl IntoResponse {
    let mut authorized = false;
    let mut logged_in_user: Option<User> = None;

    let supplied_username = auth.username();
    let supplied_password = auth.password();

    // 1. Try existing JWT
    if let Ok(claims) = validate_jwt_request(&state, &jar, &headers).await {
        let db = state.context.db().await;
        let mk = state.context.master_key.read().await.clone();
        match tawai_core::db::account::get_user_by_id(db.pool(), &claims.sub, &mk).await {
            Ok(user) => {
                authorized = true;
                logged_in_user = Some(user);
            }
            Err(e) => {
                logger::error(&format!(
                    "[Login] Failed resolving JWT subject '{}': {}",
                    claims.sub, e
                ));
                return (StatusCode::INTERNAL_SERVER_ERROR,).into_response();
            }
        }
    }

    // 2. Fall back to Basic auth against DB users table
    if !authorized {
        let db = state.context.db().await;
        let mk = state.context.master_key.read().await.clone();
        match tawai_core::db::account::get_user_by_username(db.pool(), supplied_username, &mk).await
        {
            Ok(user) => match security::validate_password(&user.password_hash, supplied_password) {
                Ok(true) => {
                    authorized = true;
                    logged_in_user = Some(user);
                }
                Ok(false) => {}
                Err(e) => {
                    logger::error(&format!(
                        "[Login] Password check error for '{}': {}",
                        supplied_username, e
                    ));
                    return (StatusCode::INTERNAL_SERVER_ERROR,).into_response();
                }
            },
            Err(e) => {
                logger::debug(&format!(
                    "[Login] Failed looking up '{}': {}",
                    supplied_username, e
                ));
            }
        }
    }

    if !authorized {
        return (StatusCode::UNAUTHORIZED,).into_response();
    }

    let user = match logged_in_user {
        Some(user) => user,
        None => {
            logger::error("[Login] Authenticated but user state is missing");
            return (StatusCode::INTERNAL_SERVER_ERROR,).into_response();
        }
    };

    let jwt_response = match create_jwt_response(&state, &user.id).await {
        Ok(res) => res,
        Err(e) => {
            logger::error(&format!(
                "[Login] Failed to issue JWT for '{}': {}",
                user.username, e
            ));
            return (StatusCode::INTERNAL_SERVER_ERROR,).into_response();
        }
    };
    let jar = build_jwt_cookie(jar, &jwt_response);

    (
        jar,
        axum::Json(LoginResponse {
            user,
            access_token: jwt_response.access_token,
            csrf_token: jwt_response.csrf_token,
            expires_in: jwt_response.expires_in,
        }),
    )
        .into_response()
}
