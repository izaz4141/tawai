pub mod mb;

pub use mb::*;

use crate::server::SharedState;
use axum::Router;

pub fn create_identify_router() -> Router<SharedState> {
    Router::new().nest("/mb", mb::create_mb_router())
}
