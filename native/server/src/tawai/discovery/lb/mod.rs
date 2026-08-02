pub mod now_playing;
pub mod playlist;
pub mod recommendations;
pub mod report;
pub mod util;
pub mod validate;

use crate::security::check_api_key;
use crate::server::SharedState;
use axum::{
    Router, middleware,
    routing::{get, post},
};

pub fn create_lb_router(state: SharedState) -> Router<SharedState> {
    Router::new()
        .route(
            "/recommendations",
            get(recommendations::handle_get_recommendations),
        )
        .route("/playlist", get(playlist::handle_get_playlist))
        .route("/validate", post(validate::handle_validate_lb_token))
        .route("/now-playing", post(now_playing::handle_update_now_playing))
        .route("/report", post(report::handle_report_playback))
        .layer(middleware::from_fn_with_state(state, check_api_key))
}
