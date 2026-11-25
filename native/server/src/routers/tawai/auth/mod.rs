pub mod api;
pub mod decrypt;
pub mod encrypt;
pub mod login;
pub mod logout;
pub mod verify_password;

use crate::utils::security::check_api_key;
use crate::utils::server::SharedState;
use axum::middleware;
use axum::{
    Router,
    routing::{get, post},
};

pub fn create_auth_router(state: SharedState) -> Router<SharedState> {
    let public_router = Router::new().route("/login", post(login::handle_login));

    let protected_router = Router::new()
        .route("/generate-api", get(api::handle_generate_api))
        .route("/decrypt", post(decrypt::handle_decrypt))
        .route("/encrypt", post(encrypt::handle_encrypt))
        .route("/logout", post(logout::handle_logout))
        .route(
            "/verify-password",
            post(verify_password::handle_verify_password),
        )
        .layer(middleware::from_fn_with_state(state.clone(), check_api_key));

    public_router.merge(protected_router).with_state(state)
}
