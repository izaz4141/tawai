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

use crate::security::check_api_key;
use crate::server::SharedState;
use axum::{
    Router, middleware,
    routing::{get, post},
};

pub fn create_download_router(state: SharedState) -> Router<SharedState> {
    Router::new()
        .route("/list", get(list::handle_list_downloads))
        .route("/client-list", post(client_list::handle_client_list))
        .route("/create", post(create::handle_create))
        .route("/pause", post(pause::handle_pause))
        .route("/poll", post(poll::handle_downloads_poll))
        .route("/resume", post(resume::handle_resume))
        .route("/cancel", post(cancel::handle_cancel))
        .route("/delete", post(delete::handle_delete))
        .route("/sync", post(sync::handle_sync))
        .route("/search", post(search::handle_search))
        .route(
            "/test-connection",
            post(test_connection::handle_test_connection),
        )
        .route("/get-info", post(get_info::handle_get_info))
        .layer(middleware::from_fn_with_state(state, check_api_key))
}
