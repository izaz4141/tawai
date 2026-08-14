pub mod delete;
pub mod detail;
pub mod list;
pub mod mbid;
pub mod source;

pub use delete::*;
pub use detail::*;
pub use list::*;
pub use mbid::*;
pub use source::*;

use crate::server::SharedState;
use axum::{
    Router,
    routing::{delete, get},
};

pub fn create_tracks_router() -> Router<SharedState> {
    Router::new()
        .route("/", get(list::handle_list_tracks))
        .route("/{id}", get(detail::handle_get_track))
        .route("/{id}", delete(delete::handle_delete_track))
        .route(
            "/by-source/{source_id}",
            get(source::handle_list_tracks_by_source),
        )
        .route("/album-mbid/{album_id}", get(mbid::handle_get_album_mbid))
        .route("/{id}/cover", get(detail::handle_track_cover))
}
