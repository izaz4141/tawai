use axum::{Extension, Json, extract::State, http::StatusCode, response::IntoResponse};
use tawai_core::db;
use tawai_core::dclient::DownloadClient;
use tawai_core::signals::download::{DownloadsPollRequest, DownloadsPollResponse};

use crate::server::SharedState;

#[utoipa::path(
    post,
    path = "/api/tawai/download/poll",
    tags = ["tawai.download"],
    security(("ApiKeyAuth" = [])),
    request_body = DownloadsPollRequest,
    responses(
        (status = 200, description = "Downloads synced and listed", body = DownloadsPollResponse),
        (status = 400, description = "Poll failed"),
        (status = 403, description = "Forbidden"),
    )
)]
pub async fn handle_downloads_poll(
    State(state): State<SharedState>,
    Extension(username): Extension<String>,
    Json(req): Json<DownloadsPollRequest>,
) -> impl IntoResponse {
    let db = state.context.db().await;
    let pool = db.pool();

    let auth_user_id = match db::account::get_user_id_by_username(pool, &username).await {
        Ok(Some(uid)) => uid,
        _ => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "Failed to resolve user" })),
            )
                .into_response();
        }
    };

    let role = db::account::get_user_role(pool, &auth_user_id)
        .await
        .unwrap_or(None);

    let target_user_id = match role.as_deref() {
        Some("admin") => req.user_id,
        _ => {
            if auth_user_id != req.user_id {
                return (
                    StatusCode::FORBIDDEN,
                    Json(serde_json::json!({ "error": "Forbidden" })),
                )
                    .into_response();
            }
            auth_user_id
        }
    };

    let cfg = state.context.cfg().await;
    for source_type in &["slskd", "nadekodon"] {
        if let Ok(client) = DownloadClient::from_config(source_type, &cfg, state.context.client()) {
            let _ = client.sync(pool).await;
        }
    }

    match db::download::list_downloads(pool, &target_user_id, None).await {
        Ok(downloads) => Json(DownloadsPollResponse {
            id: req.id,
            downloads,
        })
        .into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}
