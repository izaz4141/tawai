pub mod start;
pub mod status;

pub use start::*;
pub use status::*;

use crate::server::SharedState;
use axum::{
    Router,
    routing::{get, post},
};

pub fn create_scan_router() -> Router<SharedState> {
    Router::new()
        .route("/", post(start::handle_scan))
        .route("/status", get(status::handle_scan_status))
}
