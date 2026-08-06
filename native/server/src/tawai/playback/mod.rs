pub mod history;
pub mod play;
pub mod preview;
pub mod stream;

pub use history::*;
pub use play::*;
pub use preview::*;
pub use stream::*;

use crate::security::{auth_query, check_api_key};
use crate::server::SharedState;
use axum::{
    Router, middleware,
    routing::{get, post},
};

fn create_streaming_router(state: SharedState) -> Router<SharedState> {
    Router::new()
        .route("/stream/{id}", get(handle_stream_track))
        .layer(middleware::from_fn_with_state(state, auth_query))
}

pub fn create_playback_router(state: SharedState) -> Router<SharedState> {
    let streaming_router = create_streaming_router(state.clone());
    Router::new()
        .route("/play", post(handle_play_track))
        .route("/preview", post(handle_preview_track))
        .route("/history", get(handle_get_history))
        .layer(middleware::from_fn_with_state(state, check_api_key))
        .merge(streaming_router)
}
