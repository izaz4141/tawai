pub mod cancel;
pub mod client_list;
pub mod create;
pub mod delete;
pub mod get_info;
pub mod list;
pub mod pause;
pub mod poll;
pub mod resume;
pub mod search;
pub mod sync;
pub mod test_connection;

pub use cancel::handle_cancel;
pub use client_list::handle_client_list;
pub use create::handle_create;
pub use delete::handle_delete;
pub use get_info::handle_get_info;
pub use list::handle_list_downloads;
pub use pause::handle_pause;
pub use poll::handle_downloads_poll;
pub use resume::handle_resume;
pub use search::handle_search;
pub use sync::handle_sync;
pub use test_connection::handle_test_connection;

use crate::security::check_api_key;
use crate::server::SharedState;
use axum::{
    Router, middleware,
    routing::{get, post},
};

pub fn create_download_router(state: SharedState) -> Router<SharedState> {
    Router::new()
        .route("/list", get(handle_list_downloads))
        .route("/client-list", post(handle_client_list))
        .route("/create", post(handle_create))
        .route("/pause", post(handle_pause))
        .route("/poll", post(handle_downloads_poll))
        .route("/resume", post(handle_resume))
        .route("/cancel", post(handle_cancel))
        .route("/delete", post(handle_delete))
        .route("/sync", post(handle_sync))
        .route("/search", post(handle_search))
        .route("/test-connection", post(handle_test_connection))
        .route("/get-info", post(handle_get_info))
        .layer(middleware::from_fn_with_state(state, check_api_key))
}
