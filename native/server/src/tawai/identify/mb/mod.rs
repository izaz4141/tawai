pub mod recording;
pub mod release;
pub mod track;

pub use recording::*;
pub use release::*;
pub use track::*;

use crate::server::SharedState;
use axum::{Router, routing::get, routing::post};

pub fn create_mb_router() -> Router<SharedState> {
    Router::new()
        .route("/search", get(recording::handle_enhanced_search))
        .route("/recording/{mbid}", get(recording::handle_fetch_recording))
        .route(
            "/release/{id}/tracks",
            get(release::handle_get_release_tracks),
        )
        .route("/track/{id}", get(track::handle_identify_track))
        .route(
            "/track/{id}/fingerprint",
            get(track::handle_fingerprint_track),
        )
        .route("/fingerprint", post(track::handle_fingerprint_path))
}
