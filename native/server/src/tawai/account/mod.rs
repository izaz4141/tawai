pub mod create;
pub mod delete;
pub mod list;
pub mod update;

pub use create::*;
pub use delete::*;
pub use list::*;
pub use update::*;

use crate::security::check_api_key;
use crate::server::{SharedState, auth_rate_limit_config};
use axum::Router;
use axum::middleware;
use axum::routing::{get, post};
use tower_governor::GovernorLayer;

pub fn create_account_router(state: SharedState) -> Router<SharedState> {
    Router::new()
        .route("/list", get(handle_list_users))
        .route("/create", post(handle_create_account))
        .route("/update", post(handle_update_account))
        .route("/delete", post(handle_delete_account))
        .layer(middleware::from_fn_with_state(state.clone(), check_api_key))
        .layer(GovernorLayer::new(auth_rate_limit_config()))
        .with_state(state)
}
