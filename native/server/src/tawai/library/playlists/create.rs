use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use serde::{Deserialize, Serialize};
use tawai_core::db::library;
use utoipa::ToSchema;

use crate::server::SharedState;

#[derive(Deserialize, ToSchema)]
pub struct CreatePlaylistBody {
    pub name: String,
}

#[derive(Serialize, ToSchema)]
pub struct CreatePlaylistResponse {
    pub playlist_id: String,
}

pub async fn handle_create_playlist(
    State(state): State<SharedState>,
    Json(body): Json<CreatePlaylistBody>,
) -> impl IntoResponse {
    let db = state.context.db().await;
    match library::create_playlist(db.pool(), &body.name).await {
        Ok(playlist_id) => Json(CreatePlaylistResponse { playlist_id }).into_response(),
        Err(e) => {
            tawai_core::utils::logger::error(&format!("create playlist failed: {}", e));
            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response()
        }
    }
}
