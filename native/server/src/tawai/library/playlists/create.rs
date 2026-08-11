use axum::{Extension, Json, extract::State, http::StatusCode, response::IntoResponse};
use tawai_core::db::library;
use tawai_core::signals::library::{CreatePlaylistRequest, CreatePlaylistResponse};

use crate::server::SharedState;

pub async fn handle_create_playlist(
    State(state): State<SharedState>,
    Extension(user_id): Extension<String>,
    Json(body): Json<CreatePlaylistRequest>,
) -> impl IntoResponse {
    let db = state.context.db().await;
    match library::create_playlist(db.pool(), &user_id, &body.name).await {
        Ok(playlist_id) => Json(CreatePlaylistResponse {
            id: body.id,
            playlist_id,
        })
        .into_response(),
        Err(e) => {
            tawai_core::utils::logger::error(&format!("create playlist failed: {}", e));
            (
                StatusCode::BAD_REQUEST,
                Json(CreatePlaylistResponse {
                    id: body.id,
                    playlist_id: String::new(),
                }),
            )
                .into_response()
        }
    }
}
