use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use tawai_core::db::library;

use crate::server::SharedState;

#[utoipa::path(
    delete,
    path = "/api/tawai/library/tracks/{id}",
    tags = ["tawai.library"],
    security(("ApiKeyAuth" = [])),
    params(
        ("id" = String, Path, description = "Track ID"),
    ),
    responses(
        (status = 200, description = "Track deleted"),
        (status = 403, description = "Forbidden: user cannot access the track's source"),
        (status = 404, description = "Track not found"),
        (status = 500, description = "Deletion failed")
    )
)]
pub async fn handle_delete_track(
    State(state): State<SharedState>,
    Extension(user_id): Extension<String>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let db = state.context.db().await;
    let client = state.context.client().clone();
    match library::delete_track(db.pool(), &client, &user_id, &id).await {
        Ok(()) => StatusCode::OK.into_response(),
        Err(e) => {
            tawai_core::utils::logger::error(&format!("delete track failed: {}", e));
            let message = e.to_string();
            let status = if message.contains("not found") || message.contains("has no source") {
                StatusCode::NOT_FOUND
            } else if message.contains("does not have access") {
                StatusCode::FORBIDDEN
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            (status, axum::Json(serde_json::json!({"error": message}))).into_response()
        }
    }
}
