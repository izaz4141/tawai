pub mod restart;
pub mod status;

pub use restart::*;
pub use status::*;

use crate::security::require_admin;
use crate::server::SharedState;
use axum::{Router, middleware, routing::post};

pub fn create_system_router(state: SharedState) -> Router<SharedState> {
    Router::new()
        .route("/restart", post(handle_restart))
        .layer(middleware::from_fn_with_state(state, require_admin))
}
