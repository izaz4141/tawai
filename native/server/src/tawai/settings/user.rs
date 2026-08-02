use std::collections::HashMap;

use axum::{
    Json,
    extract::{Extension, Path, State},
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use tawai_core::db::{account, user_settings};
use utoipa::ToSchema;

use crate::server::SharedState;

#[derive(Serialize, ToSchema)]
pub struct UserSettingResponse {
    pub key: String,
    pub value: String,
}

#[derive(Serialize, ToSchema)]
pub struct AllUserSettingsResponse {
    pub settings: HashMap<String, String>,
}

#[derive(Deserialize, ToSchema)]
pub struct SetUserSettingPayload {
    pub value: String,
}

#[utoipa::path(
    get,
    path = "/api/tawai/settings/user/{key}",
    tags = ["tawai.settings"],
    security(("ApiKeyAuth" = [])),
    params(
        ("key" = String, Path, description = "Setting key"),
    ),
    responses(
        (status = 200, description = "User setting value", body = UserSettingResponse),
        (status = 404, description = "Setting not found")
    )
)]
pub async fn handle_get_all_user_settings(
    State(state): State<SharedState>,
    Extension(username): Extension<String>,
) -> impl IntoResponse {
    let db = state.context.db().await;
    let mk = state.context.master_key.read().await.clone();
    let user_id = match account::get_user_by_username(db.pool(), &username, &mk).await {
        Ok(Some(u)) => u.id,
        _ => return axum::http::StatusCode::UNAUTHORIZED.into_response(),
    };
    let pairs = user_settings::get_all_settings(db.pool(), &user_id).await;
    let settings: HashMap<_, _> = pairs.into_iter().collect();
    Json(AllUserSettingsResponse { settings }).into_response()
}

#[utoipa::path(
    get,
    path = "/api/tawai/settings/user/{key}",
    tags = ["tawai.settings"],
    security(("ApiKeyAuth" = [])),
    params(
        ("key" = String, Path, description = "Setting key"),
    ),
    responses(
        (status = 200, description = "User setting value", body = UserSettingResponse),
        (status = 404, description = "Setting not found")
    )
)]
pub async fn handle_get_user_setting(
    State(state): State<SharedState>,
    Extension(username): Extension<String>,
    Path(key): Path<String>,
) -> impl IntoResponse {
    let db = state.context.db().await;
    let mk = state.context.master_key.read().await.clone();
    let user_id = match account::get_user_by_username(db.pool(), &username, &mk).await {
        Ok(Some(u)) => u.id,
        _ => return axum::http::StatusCode::UNAUTHORIZED.into_response(),
    };
    match user_settings::get_setting(db.pool(), &user_id, &key).await {
        Some(value) => Json(UserSettingResponse {
            key: key.clone(),
            value,
        })
        .into_response(),
        None => (
            axum::http::StatusCode::NOT_FOUND,
            Json(UserSettingResponse {
                key,
                value: String::new(),
            }),
        )
            .into_response(),
    }
}

#[utoipa::path(
    put,
    path = "/api/tawai/settings/user/{key}",
    tags = ["tawai.settings"],
    security(("ApiKeyAuth" = [])),
    params(
        ("key" = String, Path, description = "Setting key"),
    ),
    request_body = SetUserSettingPayload,
    responses(
        (status = 200, description = "Setting saved"),
        (status = 500, description = "Server error")
    )
)]
pub async fn handle_set_user_setting(
    State(state): State<SharedState>,
    Extension(username): Extension<String>,
    Path(key): Path<String>,
    Json(payload): Json<SetUserSettingPayload>,
) -> impl IntoResponse {
    let db = state.context.db().await;
    let mk = state.context.master_key.read().await.clone();
    let user_id = match account::get_user_by_username(db.pool(), &username, &mk).await {
        Ok(Some(u)) => u.id,
        _ => return axum::http::StatusCode::UNAUTHORIZED.into_response(),
    };
    match user_settings::set_setting(db.pool(), &user_id, &key, &payload.value).await {
        Ok(()) => axum::http::StatusCode::OK.into_response(),
        Err(e) => {
            tawai_core::utils::logger::error(&format!("set user setting failed: {}", e));
            axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
