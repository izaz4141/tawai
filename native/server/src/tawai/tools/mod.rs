pub mod duplicates;
pub mod lyrics;
pub mod missing;
pub mod naming;
pub mod rename;
pub mod stats;

use crate::security::check_api_key;
use crate::server::SharedState;
use axum::{
    Router, middleware,
    routing::{get, post},
};

pub fn create_tools_router(state: SharedState) -> Router<SharedState> {
    Router::new()
        .route("/find-duplicates", post(duplicates::handle_find_duplicates))
        .route("/stats", get(stats::handle_get_library_stats))
        .route(
            "/missing-metadata",
            post(missing::handle_find_missing_metadata),
        )
        .route(
            "/format-naming-preview",
            post(naming::handle_format_naming_preview),
        )
        .route(
            "/check-convention",
            post(rename::handle_check_naming_convention),
        )
        .route("/rename-preview", post(rename::handle_batch_rename_preview))
        .route("/rename-apply", post(rename::handle_batch_rename_apply))
        .route("/romajize-lyrics", post(lyrics::handle_romajize_lyrics))
        .route("/write-lyrics", post(lyrics::handle_write_track_lyrics))
        .layer(middleware::from_fn_with_state(state, check_api_key))
}
