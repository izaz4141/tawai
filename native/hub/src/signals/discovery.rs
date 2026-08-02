use rinf::{DartSignal, RustSignal, SignalPiece};
use serde::{Deserialize, Serialize};
use tawai_core::signals::discovery as core_disc;

// ---------------------------------------------------------------------------
// Discovery-track struct (slim, no ReleaseInfo/general-purpose fields)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, SignalPiece)]
pub struct DiscoveryRecording {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub artist_id: Option<String>,
    pub duration_secs: Option<f64>,
    pub album_title: Option<String>,
    pub cover: Option<String>,
    pub score: f64,
    pub is_owned: bool,
}

impl From<core_disc::DiscoveryRecording> for DiscoveryRecording {
    fn from(r: core_disc::DiscoveryRecording) -> Self {
        Self {
            id: r.id,
            title: r.title,
            artist: r.artist,
            artist_id: r.artist_id,
            duration_secs: r.duration_secs,
            album_title: r.album_title,
            cover: r.cover,
            score: r.score,
            is_owned: r.is_owned,
        }
    }
}

#[derive(Deserialize, DartSignal)]
pub struct TestJellyfinSourceRequest {
    pub id: String,
    pub url: String,
}

#[derive(Serialize, RustSignal)]
pub struct TestJellyfinSourceResponse {
    pub id: String,
    pub libraries: Vec<JellyfinLibraryInfo>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, SignalPiece)]
pub struct JellyfinLibraryInfo {
    pub id: String,
    pub name: String,
}

impl From<core_disc::JellyfinLibraryInfo> for JellyfinLibraryInfo {
    fn from(s: core_disc::JellyfinLibraryInfo) -> Self {
        Self {
            id: s.id,
            name: s.name,
        }
    }
}

// ---------------------------------------------------------------------------
// ListenBrainz Discovery
// ---------------------------------------------------------------------------

#[derive(Deserialize, DartSignal)]
pub struct GetLBRecommendationsRequest {
    pub id: String,
    pub user_id: String,
    pub rec_type: String,
    pub count: Option<i32>,
    pub offset: Option<i32>,
    pub index: Option<u32>,
}

#[derive(Serialize, RustSignal)]
pub struct GetLBRecommendationsResponse {
    pub id: String,
    pub recommendations: Vec<DiscoveryRecording>,
    pub playlist_title: Option<String>,
    pub playlist_id: Option<String>,
    pub playlist_count: Option<u32>,
    pub error: Option<String>,
}

#[derive(Deserialize, DartSignal)]
pub struct ValidateLBTokenRequest {
    pub id: String,
    pub token: String,
}

#[derive(Serialize, RustSignal)]
pub struct ValidateLBTokenResponse {
    pub id: String,
    pub valid: bool,
    pub user_name: Option<String>,
    pub message: String,
}

// ---------------------------------------------------------------------------
// Sync Recommendation Tracks (consolidated)
// ---------------------------------------------------------------------------

#[derive(Deserialize, DartSignal)]
pub struct SyncRecsRequest {
    pub id: String,
    pub user_id: String,
    pub included_keys: String,
}

#[derive(Serialize, RustSignal)]
pub struct SyncRecsResponse {
    pub id: String,
    pub success: bool,
    pub added_sources: Vec<String>,
    pub removed_sources: Vec<String>,
    pub tracks_added: u32,
    pub tracks_removed: u32,
    pub error: Option<String>,
}
