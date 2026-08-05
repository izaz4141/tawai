pub mod fs;
pub mod restart;
pub mod status;

pub use fs::*;
pub use restart::*;
pub use status::*;

use crate::security::{check_api_key, require_admin};
use crate::server::SharedState;
use axum::{
    Router, middleware,
    routing::{get, post},
};

pub fn create_system_router(state: SharedState) -> Router<SharedState> {
    // `/restart` is admin-only. `/fs` (folder browsing) is available to any
    // authenticated user, so it gets its own `check_api_key` layer.
    let restart_router = Router::new()
        .route("/restart", post(handle_restart))
        .layer(middleware::from_fn_with_state(state.clone(), require_admin));
    let fs_router = Router::new()
        .route("/fs/list", get(handle_list_dir))
        .layer(middleware::from_fn_with_state(state, check_api_key));
    Router::new().merge(restart_router).merge(fs_router)
}
