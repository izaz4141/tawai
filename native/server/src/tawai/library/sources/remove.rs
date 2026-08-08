use axum::{
    extract::{Extension, Path, State},
    response::IntoResponse,
};
use tawai_core::db::{account, library_source};

use crate::server::SharedState;

pub async fn handle_remove_source(
    State(state): State<SharedState>,
    Extension(user_id): Extension<String>,
    Path(source_id): Path<String>,
) -> impl IntoResponse {
    let db = state.context.db().await;
    let mk = state.context.master_key.read().await.clone();
    let user = match account::get_user_by_id(db.pool(), &user_id, &mk).await {
        Ok(u) => u,
        Err(_) => return axum::http::StatusCode::UNAUTHORIZED.into_response(),
    };

    let source = match library_source::get_source_by_id(db.pool(), &source_id).await {
        Ok(Some(s)) => s,
        Ok(None) => return axum::http::StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tawai_core::utils::logger::error(&format!("get source failed: {}", e));
            return axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    if source.owner_id != user.id && user.role != "admin" {
        return axum::http::StatusCode::FORBIDDEN.into_response();
    }

    match library_source::remove_source(db.pool(), &source_id).await {
        Ok(true) => axum::http::StatusCode::OK.into_response(),
        Ok(false) => axum::http::StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tawai_core::utils::logger::error(&format!("remove source failed: {}", e));
            axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
