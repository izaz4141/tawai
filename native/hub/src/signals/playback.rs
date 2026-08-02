use rinf::{DartSignal, RustSignal, SignalPiece};
use serde::{Deserialize, Serialize};

use super::library::TrackInfo;

#[derive(Deserialize, DartSignal)]
pub struct PlayTrackRequest {
    pub id: String,
    pub track_id: Option<String>,
    pub track: Option<TrackInfo>,
}

#[derive(Serialize, RustSignal)]
pub struct PlayTrackResponse {
    pub id: String,
    pub file_path: String,
    pub error: Option<String>,
    pub headers: Option<Vec<(String, String)>>,
}

#[derive(Deserialize, DartSignal)]
pub struct PreviewTrackRequest {
    pub id: String,
    pub track: TrackInfo,
}

#[derive(Serialize, RustSignal)]
pub struct PreviewTrackResponse {
    pub id: String,
    pub url: Option<String>,
    pub source: Option<String>,
    pub error: Option<String>,
}

#[derive(Deserialize, DartSignal)]
pub struct ReportPlaybackRequest {
    pub id: String,
    pub user_id: String,
    pub track_id: String,
    pub played_at: String,
    pub source: String,
}

#[derive(Serialize, RustSignal)]
pub struct ReportPlaybackResponse {
    pub id: String,
    pub success: bool,
}

#[derive(Deserialize, DartSignal)]
pub struct GetHistoryRequest {
    pub id: String,
    pub user_id: String,
    pub limit: Option<i32>,
}

#[derive(Serialize, RustSignal)]
pub struct GetHistoryResponse {
    pub id: String,
    pub records: Vec<PlaybackRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize, SignalPiece)]
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

#[derive(Deserialize, DartSignal)]
pub struct UpdateNowPlayingRequest {
    pub id: String,
    pub user_id: String,
    pub track_id: String,
}

#[derive(Serialize, RustSignal)]
pub struct UpdateNowPlayingResponse {
    pub id: String,
    pub success: bool,
}
