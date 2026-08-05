use axum::{
    Json,
    extract::{Extension, State},
    response::IntoResponse,
};
use tawai_core::db::{account, library_source};

use crate::server::SharedState;

pub async fn handle_list_sources(
    State(state): State<SharedState>,
    Extension(user_id): Extension<String>,
) -> impl IntoResponse {
    let db = state.context.db().await;
    let mk = state.context.master_key.read().await.clone();
    let user = match account::get_user_by_id(db.pool(), &user_id, &mk).await {
        Ok(Some(u)) => u,
        _ => return axum::http::StatusCode::UNAUTHORIZED.into_response(),
    };

    match library_source::list_accessible_sources(db.pool(), &user.id, &user.role).await {
        Ok(sources) => Json(serde_json::json!({ "sources": sources })).into_response(),
        Err(e) => {
            tawai_core::utils::logger::error(&format!("list sources failed: {}", e));
            axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
