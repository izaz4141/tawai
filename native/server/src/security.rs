use axum::{
    body::Body,
    extract::{FromRequestParts, Query, State},
    http::{HeaderMap, Request, StatusCode},
    middleware::Next,
    response::IntoResponse,
};
use axum_extra::extract::CookieJar;
use jsonwebtoken::{
    Algorithm, DecodingKey, EncodingKey, Header, TokenData, Validation, decode, encode,
};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::server::{SharedState, secure_compare};

const JWT_ALGORITHM: Algorithm = Algorithm::HS256;
const JWT_EXPIRY_HOURS: u64 = 3;

#[derive(Serialize, Deserialize, Clone)]
pub struct JwtClaims {
    pub sub: String,
    pub exp: u64,
    pub csrf: String,
    pub iat: u64,
}

#[derive(Serialize, Deserialize)]
pub struct JwtResponse {
    pub access_token: String,
    pub csrf_token: String,
    pub expires_in: u64,
}

async fn get_jwt_secret(state: &SharedState) -> Vec<u8> {
    state.context.master_key.read().await.clone().into_bytes()
}

fn get_current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

fn get_expiry_timestamp() -> u64 {
    get_current_timestamp() + (JWT_EXPIRY_HOURS * 3600)
}

fn create_csrf_token() -> String {
    use uuid::Uuid;
    Uuid::new_v4().to_string()
}

async fn create_jwt_token(
    state: &SharedState,
    user_id: &str,
    csrf_token: &str,
) -> Result<String, jsonwebtoken::errors::Error> {
    let now = get_current_timestamp();
    let claims = JwtClaims {
        sub: user_id.to_string(),
        exp: get_expiry_timestamp(),
        csrf: csrf_token.to_string(),
        iat: now,
    };

    let header = Header::new(JWT_ALGORITHM);
    encode(
        &header,
        &claims,
        &EncodingKey::from_secret(&get_jwt_secret(state).await),
    )
}

pub async fn validate_jwt_token(
    state: &SharedState,
    token: &str,
) -> Result<TokenData<JwtClaims>, jsonwebtoken::errors::Error> {
    let mut validation = Validation::new(JWT_ALGORITHM);
    validation.validate_exp = true;

    decode::<JwtClaims>(
        token,
        &DecodingKey::from_secret(&get_jwt_secret(state).await),
        &validation,
    )
}

pub async fn create_jwt_response(
    state: &SharedState,
    user_id: &str,
) -> Result<JwtResponse, jsonwebtoken::errors::Error> {
    let csrf = create_csrf_token();
    let access_token = create_jwt_token(state, user_id, &csrf).await?;
    let expires_in = JWT_EXPIRY_HOURS * 3600;

    Ok(JwtResponse {
        access_token,
        csrf_token: csrf,
        expires_in,
    })
}

pub async fn validate_jwt_request(
    state: &SharedState,
    jar: &CookieJar,
    headers: &HeaderMap,
) -> Result<JwtClaims, String> {
    let jwt = jar.get("tawai_jwt");
    let csrf_header = headers
        .get("x-csrf-token")
        .and_then(|v| v.to_str().ok())
        .map(String::from);

    let token = jwt.ok_or("No JWT cookie")?;
    let csrf = csrf_header.ok_or("No CSRF header")?;

    let token_data = validate_jwt_token(state, token.value())
        .await
        .map_err(|e| e.to_string())?;
    let claims = token_data.claims;

    if !secure_compare(&claims.csrf, &csrf) {
        return Err("CSRF token mismatch".to_string());
    }

    Ok(claims)
}

async fn resolve_api_key_user(state: &SharedState, api_key: &str) -> Result<String, StatusCode> {
    let db = state.context.db().await;
    tawai_core::db::account::get_user_id_by_api_key(db.pool(), api_key)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::UNAUTHORIZED)
}

async fn authenticate_via_jwt(
    state: &SharedState,
    req: Request<Body>,
) -> Result<(Request<Body>, String), StatusCode> {
    let (mut parts, body) = req.into_parts();
    let jar = CookieJar::from_request_parts(&mut parts, &state)
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    let claims = validate_jwt_request(state, &jar, &parts.headers)
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    let req = Request::from_parts(parts, body);
    Ok((req, claims.sub))
}

pub async fn check_api_key(
    State(state): State<SharedState>,
    req: Request<Body>,
    next: Next,
) -> Result<impl IntoResponse, StatusCode> {
    // 1. Try X-API-Key header -> per-user DB lookup
    if let Some(key) = req.headers().get("X-API-Key")
        && let Ok(k) = key.to_str()
        && let Ok(user_id) = resolve_api_key_user(&state, k).await
    {
        let mut req = req;
        req.extensions_mut().insert(user_id);
        return Ok(next.run(req).await);
    }

    // 2. Fall back to JWT cookie + CSRF header
    let (req, user_id) = authenticate_via_jwt(&state, req).await?;
    let mut req = req;
    req.extensions_mut().insert(user_id);
    Ok(next.run(req).await)
}

pub async fn require_admin(
    State(state): State<SharedState>,
    req: Request<Body>,
    next: Next,
) -> Result<impl IntoResponse, StatusCode> {
    // 1. Try X-API-Key header -> per-user DB lookup
    if let Some(key) = req.headers().get("X-API-Key")
        && let Ok(k) = key.to_str()
        && let Ok(user_id) = resolve_api_key_user(&state, k).await
    {
        let db = state.context.db().await;
        let role = tawai_core::db::account::get_user_role(db.pool(), &user_id)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        if role.as_deref() == Some("admin") {
            let mut req = req;
            req.extensions_mut().insert(user_id);
            return Ok(next.run(req).await);
        }
        return Err(StatusCode::FORBIDDEN);
    }

    // 2. Fall back to JWT cookie + CSRF header
    let (req, user_id) = authenticate_via_jwt(&state, req).await?;
    let db = state.context.db().await;
    let role = tawai_core::db::account::get_user_role(db.pool(), &user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    match role.as_deref() {
        Some("admin") => {
            let mut req = req;
            req.extensions_mut().insert(user_id);
            Ok(next.run(req).await)
        }
        _ => Err(StatusCode::FORBIDDEN),
    }
}

#[derive(Deserialize)]
pub struct AuthQuery {
    pub token: String,
}
pub async fn auth_query(
    State(state): State<SharedState>,
    req: Request<Body>,
    next: Next,
) -> Result<impl IntoResponse, StatusCode> {
    let Query(params) =
        Query::<AuthQuery>::try_from_uri(req.uri()).map_err(|_| StatusCode::UNAUTHORIZED)?;

    let token_data = validate_jwt_token(&state, &params.token)
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    let mut req = req;
    req.extensions_mut().insert(token_data.claims.sub);

    Ok(next.run(req).await)
}
