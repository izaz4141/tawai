pub mod history;
pub mod play;
pub mod preview;
pub mod stream;

pub use history::*;
pub use play::*;
pub use preview::*;
pub use stream::*;

use crate::security::check_api_key;
use crate::server::SharedState;
use axum::{
    Router, middleware,
    routing::{get, post},
};

pub fn create_playback_router(state: SharedState) -> Router<SharedState> {
    Router::new()
        .route("/play", post(handle_play_track))
        .route("/preview", post(handle_preview_track))
        .route("/stream/{id}", get(handle_stream_track))
        .route("/history", get(handle_get_history))
        .layer(middleware::from_fn_with_state(state, check_api_key))
}
