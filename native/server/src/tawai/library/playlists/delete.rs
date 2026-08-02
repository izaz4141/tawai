use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use tawai_core::db::library;

use crate::server::SharedState;

pub async fn handle_delete_playlist(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let db = state.context.db().await;
    match library::delete_playlist(db.pool(), &id).await {
        Ok(_) => StatusCode::OK.into_response(),
        Err(e) => {
            tawai_core::utils::logger::error(&format!("delete playlist failed: {}", e));
            (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response()
        }
    }
}
