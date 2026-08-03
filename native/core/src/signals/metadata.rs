use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ReleaseInfo {
    pub id: String,
    pub title: String,
    pub date: Option<String>,
    pub country: Option<String>,
    pub artist: String,
    pub artist_id: Option<String>,
    pub tracks: Vec<ReleaseTrackInfo>,
    #[serde(default)]
    pub disambiguation: Option<String>,
    #[serde(default)]
    pub total_discs: Option<i32>,
    #[serde(default)]
    pub total_tracks: Option<i32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct RecordingInfo {
    pub id: String,
    pub title: String,
    pub score: f64,
    pub artist: String,
    pub artist_id: Option<String>,
    pub duration_secs: Option<f64>,
    pub acoust_id: Option<String>,
    pub releases: Vec<ReleaseInfo>,
    pub cover: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MBSearchInfo {
    pub recordings: Vec<RecordingInfo>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct ReleaseTrackInfo {
    pub id: String,
    pub title: String,
    pub position: Option<i32>,
    pub disc_number: Option<i32>,
    pub duration_secs: Option<f64>,
    pub lyrics: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LyricsResult {
    pub id: u64,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration: f64,
    pub instrumental: bool,
    pub lyrics: String,
    pub synced: bool,
}
