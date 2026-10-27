use axum::{
    Json,
    extract::{Extension, State},
    response::IntoResponse,
};
use serde_json::Value;
use tawai_core::db::account;
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
    "theme_mode",
    "use_dynamic_color",
    "custom_color",
    "retreat_to_tray",
    "server_host",
    "server_port",
];

/// Global settings a non-admin may read from the global settings API.
const NON_ADMIN_GLOBAL_KEYS: &[&str] = &[
    "default_download_source",
    "desired_audio_quality",
    "download_folder",
];

#[utoipa::path(
    get,
    path = "/api/tawai/settings/global",
    tags = ["tawai.settings"],
    security(("ApiKeyAuth" = [])),
    responses(
        (status = 200, description = "Global server settings. Admins receive all non-private, non-device settings; non-admins receive only the allowlisted keys.", body = GlobalSettingsResponse),
        (status = 401, description = "Not authenticated")
    )
)]
pub async fn handle_get_global_settings(
    State(state): State<SharedState>,
    Extension(user_id): Extension<String>,
) -> impl IntoResponse {
    let db = state.context.db().await;
    let is_admin = match account::get_user_role(db.pool(), &user_id).await {
        Ok(role) => role.as_deref() == Some("admin"),
        Err(e) => {
            tawai_core::utils::logger::error(&format!(
                "get global settings role lookup failed: {e}"
            ));
            false
        }
    };

    let app_cfg = state.context.cfg().await;
    let mut settings = strip_keys(&app_cfg.value, PRIVATE_CONFIG_KEYS);
    if let Value::Object(map) = &mut settings {
        if is_admin {
            for key in DEVICE_ONLY_KEYS {
                map.remove(*key);
            }
        } else {
            map.retain(|key, _| NON_ADMIN_GLOBAL_KEYS.contains(&key.as_str()));
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
