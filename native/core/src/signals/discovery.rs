use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::signals::metadata::RecordingInfo;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DiscoveryRecording {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub artist_id: Option<String>,
    pub duration_secs: Option<f64>,
    pub album_title: Option<String>,
    pub cover: Option<String>,
    pub score: f64,
    #[serde(default)]
    pub is_owned: bool,
}

impl From<RecordingInfo> for DiscoveryRecording {
    fn from(r: RecordingInfo) -> Self {
        Self {
            id: r.id,
            title: r.title,
            artist: r.artist,
            artist_id: r.artist_id,
            duration_secs: r.duration_secs,
            album_title: r.releases.into_iter().next().map(|rel| rel.title),
            cover: r.cover,
            score: r.score,
            is_owned: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct JellyfinLibraryInfo {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct TestJellyfinSourceRequest {
    pub url: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct TestJellyfinSourceResponse {
    pub libraries: Vec<JellyfinLibraryInfo>,
    pub error: Option<String>,
}

// ---------------------------------------------------------------------------
// ListenBrainz Discovery
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ValidateTokenResponse {
    pub code: i32,
    pub message: String,
    pub valid: bool,
    pub user_name: Option<String>,
}

// ---------------------------------------------------------------------------
// Sync Recommendation Tracks (consolidated)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SyncRecsRequest {
    pub user_id: String,
    pub included_keys: String,
}

#[derive(Debug, Clone, Serialize, Default, ToSchema)]
pub struct SyncRecsResponse {
    pub success: bool,
    pub added_sources: Vec<String>,
    pub removed_sources: Vec<String>,
    pub tracks_added: u32,
    pub tracks_removed: u32,
    pub error: Option<String>,
}
