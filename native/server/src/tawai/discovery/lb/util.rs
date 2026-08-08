use axum::{
    Json,
    response::{IntoResponse, Response},
};
use tawai_core::db::{account, history};
use tawai_core::discovery::listenbrainz;

use crate::server::SharedState;

pub async fn resolve_lb_user(
    state: &SharedState,
    user_id: &str,
) -> Result<(String, String), Response> {
    let db = state.context.db().await;
    let mk = state.context.master_key.read().await.clone();

    let user = match account::get_user_by_id(db.pool(), user_id, &mk).await {
        Ok(u) => u,
        Err(_) => return Err(axum::http::StatusCode::UNAUTHORIZED.into_response()),
    };

    let token = match history::get_listenbrainz_token(db.pool(), &user.id, &mk).await {
        Some(t) => t,
        None => {
            return Err((
                axum::http::StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "ListenBrainz token not configured"})),
            )
                .into_response());
        }
    };

    let validated = listenbrainz::validate_token(state.context.client(), &token).await;
    let user_name = match validated {
        Ok(v) if v.valid => match v.user_name {
            Some(n) => n,
            None => {
                return Err((
                    axum::http::StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({"error": "Could not determine ListenBrainz user"})),
                )
                    .into_response());
            }
        },
        _ => {
            return Err((
                axum::http::StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Invalid ListenBrainz token"})),
            )
                .into_response());
        }
    };

    Ok((token, user_name))
}
