use std::collections::HashMap;

use axum::{
    Json,
    extract::{Extension, Path, Query, State},
    response::IntoResponse,
};
use tawai_core::db::user_settings;
use tawai_core::signals::settings::{
    GetAllUserSettingsRequest, GetAllUserSettingsResponse, GetUserSettingRequest,
    GetUserSettingResponse, SetUserSettingRequest, SetUserSettingResponse,
};

use crate::server::SharedState;

#[utoipa::path(
    get,
    path = "/api/tawai/settings/user",
    tags = ["tawai.settings"],
    security(("ApiKeyAuth" = [])),
    responses(
        (status = 200, description = "All user settings", body = GetAllUserSettingsResponse),
    )
)]
pub async fn handle_get_all_user_settings(
    State(state): State<SharedState>,
    Extension(user_id): Extension<String>,
    Query(query): Query<GetAllUserSettingsRequest>,
) -> impl IntoResponse {
    let db = state.context.db().await;
    let pairs = user_settings::get_all_settings(db.pool(), &user_id).await;
    let settings: HashMap<_, _> = pairs.into_iter().collect();
    Json(GetAllUserSettingsResponse {
        id: query.id,
        settings,
    })
    .into_response()
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
        (status = 200, description = "User setting value", body = GetUserSettingResponse),
        (status = 404, description = "Setting not found")
    )
)]
pub async fn handle_get_user_setting(
    State(state): State<SharedState>,
    Extension(user_id): Extension<String>,
    Path(key): Path<String>,
) -> impl IntoResponse {
    let db = state.context.db().await;
    match user_settings::get_setting(db.pool(), &user_id, &key).await {
        Some(value) => Json(GetUserSettingResponse {
            id: String::new(),
            key: key.clone(),
            value,
        })
        .into_response(),
        None => (
            axum::http::StatusCode::NOT_FOUND,
            Json(GetUserSettingResponse {
                id: String::new(),
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
    request_body = SetUserSettingRequest,
    responses(
        (status = 200, description = "Setting saved", body = SetUserSettingResponse),
        (status = 500, description = "Server error")
    )
)]
pub async fn handle_set_user_setting(
    State(state): State<SharedState>,
    Extension(user_id): Extension<String>,
    Path(key): Path<String>,
    Json(payload): Json<SetUserSettingRequest>,
) -> impl IntoResponse {
    let db = state.context.db().await;
    let set_key = if payload.key.is_empty() {
        key
    } else {
        payload.key
    };
    match user_settings::set_setting(db.pool(), &user_id, &set_key, &payload.value).await {
        Ok(()) => Json(SetUserSettingResponse {
            id: payload.id,
            success: true,
        })
        .into_response(),
        Err(e) => {
            tawai_core::utils::logger::error(&format!("set user setting failed: {}", e));
            Json(SetUserSettingResponse {
                id: payload.id,
                success: false,
            })
            .into_response()
        }
    }
}
