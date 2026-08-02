pub mod lb;
pub mod sync_recs;

pub use lb::*;

use crate::server::SharedState;
use axum::{Router, routing::post};

pub fn create_discovery_router(state: SharedState) -> Router<SharedState> {
    Router::new()
        .nest("/lb", lb::create_lb_router(state.clone()))
        .route("/sync-recs", post(sync_recs::handle_sync_recs))
}
