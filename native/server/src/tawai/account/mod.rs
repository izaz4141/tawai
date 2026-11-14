pub mod create;
pub mod delete;
pub mod list;
pub mod update;

use crate::security::check_api_key;
use crate::server::{SharedState, auth_rate_limit_config};
use axum::Router;
use axum::middleware;
use axum::routing::{get, post};
use tower_governor::GovernorLayer;

pub fn create_account_router(state: SharedState) -> Router<SharedState> {
    Router::new()
        .route("/create", post(create::handle_create_account))
        .route("/delete", post(delete::handle_delete_account))
        .route("/list", get(list::handle_list_users))
        .route("/update", post(update::handle_update_account))
        .layer(middleware::from_fn_with_state(state.clone(), check_api_key))
        .layer(GovernorLayer::new(auth_rate_limit_config()))
        .with_state(state)
}
