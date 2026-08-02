pub mod create;
pub mod delete;
pub mod list;
pub mod tracks;

pub use create::*;
pub use delete::*;
pub use list::*;
pub use tracks::*;

use crate::server::SharedState;
use axum::{
    Router,
    routing::{delete, get, post, put},
};

pub fn create_playlists_router() -> Router<SharedState> {
    Router::new()
        .route("/", get(list::handle_list_playlists))
        .route("/", post(create::handle_create_playlist))
        .route("/{id}", delete(delete::handle_delete_playlist))
        .route("/{id}/tracks", get(tracks::handle_get_playlist_tracks))
        .route("/{id}/tracks", post(tracks::handle_add_track_to_playlist))
        .route(
            "/{id}/tracks/{track_id}",
            delete(tracks::handle_remove_track_from_playlist),
        )
        .route(
            "/{id}/tracks/reorder",
            put(tracks::handle_reorder_playlist_tracks),
        )
}
