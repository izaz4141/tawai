use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::library::TrackInfo;

// ---------------------------------------------------------------------------
// Identify / Tag Editor
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MatchCandidate {
    pub score: f64,
    pub title: String,
    pub artist: String,
    pub artist_id: Option<String>,
    pub album: String,
    pub album_id: Option<String>,
    pub recording_id: Option<String>,
    pub release_date: Option<String>,
    pub acoust_id: Option<String>,
    pub duration_secs: Option<f64>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ListUnidentifiedTracksRequest {
    #[serde(default)]
    pub id: String,
    pub source_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ListUnidentifiedTracksResponse {
    pub id: String,
    pub tracks: Vec<TrackInfo>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct IdentifySingleTrackRequest {
    pub id: String,
    pub track_id: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct IdentifySingleTrackResponse {
    pub id: String,
    pub track_id: String,
    pub candidates: Vec<MatchCandidate>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct SearchMusicBrainzRequest {
    #[serde(default)]
    pub id: String,
    pub query: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SearchMusicBrainzResponse {
    pub id: String,
    pub candidates: Vec<MatchCandidate>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ApplyIdentificationRequest {
    pub id: String,
    #[serde(default)]
    pub user_id: String,
    pub track_id: String,
    pub file_path: Option<String>,
    pub target_source_id: Option<String>,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub artist_mbid: Option<String>,
    pub album: Option<String>,
    pub album_mbid: Option<String>,
    pub album_disambiguation: Option<String>,
    pub release_date: Option<String>,
    pub track_num: Option<i32>,
    pub disc_num: Option<i32>,
    pub mbid_recording: Option<String>,
    pub lyrics: Option<String>,
    pub cover_bytes: Option<Vec<u8>>,
    pub total_discs: Option<i32>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ApplyIdentificationResponse {
    pub id: String,
    pub track_id: String,
    pub success: bool,
    pub error: Option<String>,
    pub new_file_path: Option<String>,
}
