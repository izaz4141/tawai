pub mod global;
pub mod user;

pub use global::*;
pub use user::*;

use crate::security::{check_api_key, require_admin};
use crate::server::SharedState;
use axum::{
    Router, middleware,
    routing::{get, put},
};

pub fn create_settings_router(state: SharedState) -> Router<SharedState> {
    // Any authenticated user may read the global settings (with a reduced key
    // set for non-admins); only admins may change them.
    let global_read_router = Router::new()
        .route("/global", get(handle_get_global_settings))
        .layer(middleware::from_fn_with_state(state.clone(), check_api_key));

    let global_write_router = Router::new()
        .route("/global", put(handle_update_global_settings))
        .layer(middleware::from_fn_with_state(state.clone(), require_admin));

    let user_router = Router::new()
        .route("/user", get(handle_get_all_user_settings))
        .route(
            "/user/{key}",
            get(handle_get_user_setting).put(handle_set_user_setting),
        )
        .layer(middleware::from_fn_with_state(state.clone(), check_api_key));

    Router::new()
        .merge(global_read_router)
        .merge(global_write_router)
        .merge(user_router)
        .with_state(state)
}
