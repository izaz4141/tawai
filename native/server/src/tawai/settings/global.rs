use axum::{
    Json,
    extract::{Extension, State},
    response::IntoResponse,
};
use serde_json::Value;
use tawai_core::signals::settings::GlobalSettingsResponse;
use tawai_core::utils::config::{PRIVATE_CONFIG_KEYS, strip_keys};

use crate::server::SharedState;

/// Settings that are device-local and must never be exposed or persisted via
/// the global settings API (they belong to each client's own config.json).
const DEVICE_ONLY_KEYS: &[&str] = &[
    "playback_volume",
    "current_user",
    "require_login",
    "check_nightly",
    "accounts",
];

#[utoipa::path(
    get,
    path = "/api/tawai/settings/global",
    tags = ["tawai.settings"],
    security(("ApiKeyAuth" = [])),
    responses(
        (status = 200, description = "Global server settings", body = GlobalSettingsResponse)
    )
)]
pub async fn handle_get_global_settings(
    State(state): State<SharedState>,
    Extension(_username): Extension<String>,
) -> impl IntoResponse {
    let app_cfg = state.context.cfg().await;
    let mut settings = strip_keys(&app_cfg.value, PRIVATE_CONFIG_KEYS);
    if let Value::Object(map) = &mut settings {
        for key in DEVICE_ONLY_KEYS {
            map.remove(*key);
        }
    }
    Json(GlobalSettingsResponse { settings })
}

#[utoipa::path(
    put,
    path = "/api/tawai/settings/global",
    tags = ["tawai.settings"],
    security(("ApiKeyAuth" = [])),
    request_body = Value,
    responses(
        (status = 200, description = "Settings updated")
    )
)]
pub async fn handle_update_global_settings(
    State(state): State<SharedState>,
    Extension(_username): Extension<String>,
    Json(new_config): Json<Value>,
) -> impl IntoResponse {
    let mut filtered = Value::Object(Default::default());
    if let Some(obj) = new_config.as_object() {
        for (key, value) in obj {
            if DEVICE_ONLY_KEYS.contains(&key.as_str()) {
                continue;
            }
            if value.is_null() {
                continue;
            }
            if PRIVATE_CONFIG_KEYS.contains(&key.as_str())
                && value.as_str().map(|s| s.trim().is_empty()).unwrap_or(false)
            {
                continue;
            }
            filtered[key] = value.clone();
        }
    }

    if let Err(e) = state.context.update_config(&filtered).await {
        use axum::http::StatusCode;
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response();
    }

    axum::http::StatusCode::OK.into_response()
}
