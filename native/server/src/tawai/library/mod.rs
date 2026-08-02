pub mod albums;
pub mod artists;
pub mod identify;
pub mod playlists;
pub mod scan;
pub mod sources;
pub mod tracks;

#[allow(ambiguous_glob_reexports)]
pub use albums::*;
pub use artists::*;
pub use identify::*;
pub use playlists::*;
pub use scan::*;
pub use sources::*;
pub use tracks::*;

use crate::security::check_api_key;
use crate::server::SharedState;
use axum::{Router, middleware};

pub fn create_library_router(state: SharedState) -> Router<SharedState> {
    Router::new()
        .nest("/tracks", tracks::create_tracks_router())
        .nest("/albums", albums::create_albums_router())
        .nest("/artists", artists::create_artists_router())
        .nest("/playlists", playlists::create_playlists_router())
        .nest("/sources", sources::create_sources_router())
        .nest("/scan", scan::create_scan_router())
        .nest("/identify", identify::create_library_identify_router())
        .layer(middleware::from_fn_with_state(state, check_api_key))
}
