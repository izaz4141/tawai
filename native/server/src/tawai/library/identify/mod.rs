pub mod edit;
pub mod lyrics;
pub mod track;

pub use edit::*;
pub use lyrics::*;
pub use track::*;

use crate::server::SharedState;
use axum::{
    Router,
    extract::DefaultBodyLimit,
    routing::{get, post},
};

const MAX_BYTES_UPLOAD: usize = 64 * 1024 * 1024;

pub fn create_library_identify_router() -> Router<SharedState> {
    Router::new()
        .route("/unidentified", get(track::handle_list_unidentified))
        .route("/apply", post(track::handle_apply_identification))
        .route(
            "/download-folder",
            get(track::handle_list_download_folder_tracks),
        )
        .route("/lyrics", get(lyrics::handle_get_lyrics))
        .route("/lyrics/search", get(lyrics::handle_search_lyrics))
        .route("/tags/read", post(edit::handle_read_file_tags))
        .route("/tags/write", post(edit::handle_write_file_tags))
        .route(
            "/tags/read-bytes",
            post(edit::handle_read_file_tags_bytes).layer(DefaultBodyLimit::max(MAX_BYTES_UPLOAD)),
        )
        .route(
            "/tags/write-bytes",
            post(edit::handle_write_file_tags_bytes).layer(DefaultBodyLimit::max(MAX_BYTES_UPLOAD)),
        )
}
