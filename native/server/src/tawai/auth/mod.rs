pub mod api;
pub mod decrypt;
pub mod encrypt;
pub mod login;
pub mod logout;
pub mod master_key;
pub mod verify_password;

use crate::security::check_api_key;
use crate::server::{SharedState, auth_rate_limit_config};
use axum::middleware;
use axum::{
    Router,
    routing::{get, post},
};
use tower_governor::GovernorLayer;

pub fn create_auth_router(state: SharedState) -> Router<SharedState> {
    let public_router = Router::new()
        .route("/login", post(login::handle_login))
        .layer(GovernorLayer::new(auth_rate_limit_config()));

    let protected_router = Router::new()
        .route("/generate-api", get(api::handle_generate_api))
        .route("/decrypt", post(decrypt::handle_decrypt))
        .route("/encrypt", post(encrypt::handle_encrypt))
        .route("/logout", post(logout::handle_logout))
        .route(
            "/generate-master-key",
            get(master_key::handle_generate_master_key),
        )
        .route(
            "/verify-password",
            post(verify_password::handle_verify_password),
        )
        .layer(middleware::from_fn_with_state(state.clone(), check_api_key))
        .layer(GovernorLayer::new(auth_rate_limit_config()));

    public_router.merge(protected_router).with_state(state)
}
