pub mod edit;
pub mod lyrics;
pub mod track;

pub use edit::*;
pub use lyrics::*;
pub use track::*;

use crate::server::SharedState;
use axum::{
    Router,
    routing::{get, post},
};

pub fn create_library_identify_router() -> Router<SharedState> {
    Router::new()
        .route("/unidentified", get(track::handle_list_unidentified))
        .route("/apply", post(track::handle_apply_identification))
        .route("/lyrics", get(lyrics::handle_get_lyrics))
        .route("/lyrics/search", get(lyrics::handle_search_lyrics))
        .route("/tags/read", post(edit::handle_read_file_tags))
        .route("/tags/write", post(edit::handle_write_file_tags))
}
