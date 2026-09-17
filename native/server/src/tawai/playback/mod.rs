pub mod dash;
pub mod history;
pub mod play;
pub mod preview;
pub mod stream;

pub use dash::*;
pub use history::*;
pub use play::*;
pub use preview::*;
pub use stream::*;

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

use tawai_core::db::database::DatabasePool;
use tawai_core::db::{account, library_source};

use crate::security::{auth_query, auth_stream, check_api_key};
use crate::server::SharedState;
use axum::{
    Router, middleware,
    routing::{get, post},
};

/// Result of checking whether a user may read a track's source. Mirrors the
/// access semantics used by the delete endpoint.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TrackAccess {
    Allowed,
    /// Track exists but has no library source row.
    NoSource,
    /// User no longer exists in the account table.
    NoUser,
    /// User exists but fails `can_access_source` for the track's source.
    Denied,
}

struct AccessEntry {
    access: TrackAccess,
    checked_at: Instant,
}

/// Short-lived cache of per-(user, track) readability decisions so DASH
/// init/segment bursts don't each run source + role lookups.
static ACCESS_CACHE: LazyLock<Mutex<HashMap<(String, String), AccessEntry>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

const ACCESS_TTL: Duration = Duration::from_secs(60);

/// Check whether `user_id` may read `track_id`'s source. Denying a deleted
/// user (or a source that vanished) is conservative; `Denied` is also used
/// when a DB lookup itself fails.
pub async fn user_can_read_track(
    pool: &DatabasePool,
    user_id: &str,
    track_id: &str,
) -> TrackAccess {
    let now = Instant::now();
    let key = (user_id.to_string(), track_id.to_string());
    {
        let cache = ACCESS_CACHE.lock().unwrap();
        if let Some(entry) = cache.get(&key) {
            if entry.checked_at.elapsed() < ACCESS_TTL {
                return entry.access;
            }
        }
    }

    let access = match library_source::get_source_info_by_track_id(pool, track_id).await {
        Ok(Some(source)) => match account::get_user_role(pool, user_id).await {
            Ok(Some(role)) => {
                if library_source::can_access_source(
                    &source.owner_id,
                    user_id,
                    &role,
                    &source.access_rule,
                ) {
                    TrackAccess::Allowed
                } else {
                    TrackAccess::Denied
                }
            }
            Ok(None) => TrackAccess::NoUser,
            Err(_) => TrackAccess::Denied,
        },
        Ok(None) => TrackAccess::NoSource,
        Err(_) => TrackAccess::Denied,
    };

    {
        let mut cache = ACCESS_CACHE.lock().unwrap();
        cache.insert(
            key,
            AccessEntry {
                access,
                checked_at: now,
            },
        );
    }
    access
}

fn create_streaming_router(state: SharedState) -> Router<SharedState> {
    // `/stream/{id}` is used by API-key clients (libsource backup/download) as
    // well as token-authenticated web players, so it accepts either.
    let stream_router = Router::new()
        .route("/stream/{id}", get(handle_stream_track))
        .layer(middleware::from_fn_with_state(state.clone(), auth_stream));
    // DASH embeds the JWT into the manifest, so it must stay token-only.
    let dash_router = Router::new()
        .route("/dash/{id}/{file}", get(handle_dash_file))
        .layer(middleware::from_fn_with_state(state, auth_query));
    stream_router.merge(dash_router)
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
