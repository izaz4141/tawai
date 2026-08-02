pub mod cover;
pub mod list;

pub use cover::*;
pub use list::*;

use crate::server::SharedState;
use axum::{Router, routing::get};

pub fn create_albums_router() -> Router<SharedState> {
    Router::new()
        .route("/", get(list::handle_list_albums))
        .route("/{id}/cover", get(cover::handle_album_cover))
}
