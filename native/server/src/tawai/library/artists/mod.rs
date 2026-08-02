pub mod list;
pub use list::*;

use crate::server::SharedState;
use axum::{Router, routing::get};

pub fn create_artists_router() -> Router<SharedState> {
    Router::new().route("/", get(list::handle_list_artists))
}
