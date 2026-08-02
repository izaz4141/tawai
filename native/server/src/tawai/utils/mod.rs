pub mod img;

pub use img::*;

use crate::security::check_api_key;
use crate::server::SharedState;
use axum::middleware;
use axum::{Router, routing::get};

pub fn create_utils_router(state: SharedState) -> Router<SharedState> {
    Router::new()
        .route("/img", get(handle_proxy_image))
        .layer(middleware::from_fn_with_state(state, check_api_key))
}
