pub mod add;
pub mod editable;
pub mod list;
pub mod remove;
pub mod test;

pub use add::*;
pub use editable::*;
pub use list::*;
pub use remove::*;
pub use test::*;

use crate::server::SharedState;
use axum::{
    Router,
    routing::{delete, get, post},
};

pub fn create_sources_router() -> Router<SharedState> {
    Router::new()
        .route("/", get(list::handle_list_sources))
        .route("/", post(add::handle_add_source))
        .route("/editable", get(editable::handle_list_editable_sources))
        .route("/test-jellyfin", post(test::handle_test_jellyfin_source))
        .route("/{id}", delete(remove::handle_remove_source))
}
