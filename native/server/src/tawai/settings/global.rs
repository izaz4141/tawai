use axum::{
    Json,
    extract::{Extension, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde_json::{Map, Value};
use tawai_core::db::account;
use tawai_core::signals::settings::GlobalSettingsResponse;
use tawai_core::utils::config::{
    current_schema_version, path_is_device, path_is_secret, prune_empty, strip_paths,
};

use crate::server::SharedState;

#[utoipa::path(
    get,
    path = "/api/tawai/settings/global",
    tags = ["tawai.settings"],
    security(("ApiKeyAuth" = [])),
    responses(
        (status = 200, description = "Global server settings. Admins receive every non-secret, non-device setting; non-admins receive only the schema version.", body = GlobalSettingsResponse),
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
    // Secrets never leave the process, and device-local settings belong to
    // each client's own config file.
    let settings = strip_paths(&app_cfg.value, path_is_secret);
    let settings = strip_paths(&settings, path_is_device);

    let settings = if is_admin {
        prune_empty(settings)
    } else {
        // A non-admin needs the version to detect skew, but nothing else.
        let mut out = Map::new();
        if let Some(version) = settings.get("schema_version") {
            out.insert("schema_version".to_string(), version.clone());
        }
        Value::Object(out)
    };

    Json(GlobalSettingsResponse { settings })
}

#[utoipa::path(
    put,
    path = "/api/tawai/settings/global",
    tags = ["tawai.settings"],
    security(("ApiKeyAuth" = [])),
    request_body = Value,
    responses(
        (status = 200, description = "Settings updated"),
        (status = 403, description = "Admin role required"),
        (status = 409, description = "Config schema version mismatch"),
    )
)]
pub async fn handle_update_global_settings(
    State(state): State<SharedState>,
    Extension(user_id): Extension<String>,
    Json(new_config): Json<Value>,
) -> impl IntoResponse {
    let db = state.context.db().await;
    let is_admin = match account::get_user_role(db.pool(), &user_id).await {
        Ok(role) => role.as_deref() == Some("admin"),
        Err(e) => {
            tawai_core::utils::logger::error(&format!(
                "update global settings role lookup failed: {e}"
            ));
            false
        }
    };
    if !is_admin {
        return (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({"error": "admin role required"})),
        )
            .into_response();
    }

    let expected = current_schema_version();
    if let Some(found) = new_config.get("schema_version").and_then(|v| v.as_u64()) {
        if found != expected {
            return (
                StatusCode::CONFLICT,
                Json(serde_json::json!({
                    "error": format!(
                        "config schema version {found} does not match this server's {expected}"
                    )
                })),
            )
                .into_response();
        }
    }

    // Device-local settings belong to each client and never reach this file.
    let mut filtered = strip_paths(&new_config, path_is_device);
    if let Value::Object(map) = &mut filtered {
        map.remove("schema_version");
    }

    if let Err(e) = state.context.update_config(&filtered).await {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response();
    }

    StatusCode::OK.into_response()
}
