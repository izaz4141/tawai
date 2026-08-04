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
    match validate_jwt_request(&state, &jar, &headers).await {
        Ok(claims) => {
            logger::debug(&format!("[Login] Valid JWT session for '{}'", claims.sub));
            authorized = true;
            let db = state.context.db().await;
            let mk = state.context.master_key.read().await.clone();
            logged_in_user =
                tawai_core::db::account::get_user_by_username(db.pool(), &claims.sub, &mk)
                    .await
                    .ok()
                    .flatten();
        }
        Err(e) => {
            logger::debug(&format!("[Login] No valid JWT session: {}", e));
        }
    }

    // 2. Fall back to Basic auth against DB users table
    if !authorized {
        let db = state.context.db().await;
        let mk = state.context.master_key.read().await.clone();
        match tawai_core::db::account::get_user_by_username(db.pool(), supplied_username, &mk).await
        {
            Ok(Some(user)) => {
                match security::validate_password(&user.password_hash, supplied_password) {
                    Ok(true) => {
                        logger::debug(&format!(
                            "[Login] Password verified for '{}'",
                            supplied_username
                        ));
                        authorized = true;
                        logged_in_user = Some(user);
                    }
                    Ok(false) => {
                        logger::warn(&format!(
                            "[Login] Password mismatch for '{}'",
                            supplied_username
                        ));
                    }
                    Err(e) => {
                        logger::error(&format!(
                            "[Login] Password check error for '{}': {}",
                            supplied_username, e
                        ));
                    }
                }
            }
            Ok(None) => {
                logger::warn(&format!("[Login] No user '{}' in DB", supplied_username));
            }
            Err(e) => {
                logger::error(&format!(
                    "[Login] DB error looking up '{}': {}",
                    supplied_username, e
                ));
            }
        }
    }

    if !authorized {
        logger::warn(&format!(
            "[Login] FAILED for '{}' -> 401",
            supplied_username
        ));
        return (StatusCode::UNAUTHORIZED,).into_response();
    }

    let user = logged_in_user.unwrap();
    let jwt_response = create_jwt_response(&state, &user.username).await.unwrap();
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
