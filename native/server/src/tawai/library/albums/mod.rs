pub mod cover;
pub mod list;

use crate::server::SharedState;
use axum::{Router, routing::get};

pub fn create_albums_router() -> Router<SharedState> {
    Router::new()
        .route("/", get(list::handle_list_albums))
        .route("/{id}/cover", get(cover::handle_album_cover))
}
