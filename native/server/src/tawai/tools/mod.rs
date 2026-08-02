pub mod duplicates;
pub mod lyrics;
pub mod missing;
pub mod naming;
pub mod rename;
pub mod stats;

pub use duplicates::*;
pub use lyrics::*;
pub use missing::*;
pub use naming::*;
pub use rename::*;
pub use stats::*;

use crate::security::check_api_key;
use crate::server::SharedState;
use axum::{
    Router, middleware,
    routing::{get, post},
};

pub fn create_tools_router(state: SharedState) -> Router<SharedState> {
    Router::new()
        .route("/find-duplicates", post(handle_find_duplicates))
        .route("/stats", get(handle_get_library_stats))
        .route("/missing-metadata", post(handle_find_missing_metadata))
        .route("/format-naming-preview", post(handle_format_naming_preview))
        .route("/rename-preview", post(handle_batch_rename_preview))
        .route("/rename-apply", post(handle_batch_rename_apply))
        .route("/check-convention", post(handle_check_naming_convention))
        .route("/romajize-lyrics", post(handle_romajize_lyrics))
        .route("/write-lyrics", post(handle_write_track_lyrics))
        .layer(middleware::from_fn_with_state(state, check_api_key))
}
