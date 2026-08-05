pub mod source;
pub mod start;
pub mod status;

pub use source::*;
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
        .route("/source", post(source::handle_scan_source))
        .route("/status", post(status::handle_scan_status))
}
