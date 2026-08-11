use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::library::TrackInfo;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PlaybackRecord {
    pub id: String,
    pub track_id: String,
    pub track_title: String,
    pub album_title: String,
    pub artist_name: String,
    pub played_at: String,
    pub source: String,
    pub scrobbled: bool,
    pub duration_secs: Option<f64>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct PlayTrackRequest {
    pub id: String,
    pub track_id: Option<String>,
    pub track: Option<TrackInfo>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PlayTrackResponse {
    pub id: String,
    pub file_path: String,
    pub error: Option<String>,
    pub headers: Option<Vec<(String, String)>>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct PreviewTrackRequest {
    pub id: String,
    pub track: TrackInfo,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PreviewTrackResponse {
    pub id: String,
    pub url: Option<String>,
    pub source: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ReportPlaybackRequest {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub user_id: String,
    pub track_id: String,
    #[serde(default)]
    pub played_at: String,
    #[serde(default)]
    pub source: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ReportPlaybackResponse {
    pub id: String,
    pub success: bool,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct GetHistoryRequest {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub user_id: String,
    pub limit: Option<i32>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct GetHistoryResponse {
    pub id: String,
    pub records: Vec<PlaybackRecord>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct UpdateNowPlayingRequest {
    pub id: String,
    #[serde(default)]
    pub user_id: String,
    pub track_id: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct UpdateNowPlayingResponse {
    pub id: String,
    pub success: bool,
}
