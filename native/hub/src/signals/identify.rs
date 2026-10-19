use rinf::{DartSignal, RustSignal, SignalPiece};
use serde::{Deserialize, Serialize};

use super::library::TrackInfo;

// ---------------------------------------------------------------------------
// Identify / Tag Editor
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, SignalPiece)]
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

#[derive(Deserialize, DartSignal)]
pub struct ListUnidentifiedTracksRequest {
    pub id: String,
    pub source_id: Option<String>,
}

#[derive(Serialize, RustSignal)]
pub struct ListUnidentifiedTracksResponse {
    pub id: String,
    pub tracks: Vec<TrackInfo>,
}

#[derive(Deserialize, DartSignal)]
pub struct IdentifySingleTrackRequest {
    pub id: String,
    pub track_id: String,
}

#[derive(Serialize, RustSignal)]
pub struct IdentifySingleTrackResponse {
    pub id: String,
    pub track_id: String,
    pub candidates: Vec<MatchCandidate>,
}

#[derive(Deserialize, DartSignal)]
pub struct SearchMusicBrainzRequest {
    pub id: String,
    pub query: String,
}

#[derive(Serialize, RustSignal)]
pub struct SearchMusicBrainzResponse {
    pub id: String,
    pub candidates: Vec<MatchCandidate>,
}

#[derive(Deserialize, DartSignal)]
pub struct ApplyIdentificationRequest {
    pub id: String,
    pub user_id: String,
    pub track_id: String,
    /// When set (download-folder flow), the audio file to write tags to and
    /// move into the target library source.
    pub file_path: Option<String>,
    /// When set, the library source to move the (download-folder) file into.
    pub target_source_id: Option<String>,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub artist_mbid: Option<String>,
    pub album_artist: Option<String>,
    pub album_artist_mbid: Option<String>,
    pub artists: Option<Vec<String>>,
    pub album_artists: Option<Vec<String>>,
    pub artist_sort: Option<String>,
    pub album_artist_sort: Option<String>,
    pub album: Option<String>,
    pub album_mbid: Option<String>,
    pub album_disambiguation: Option<String>,
    pub release_date: Option<String>,
    pub original_date: Option<String>,
    pub release_group_mbid: Option<String>,
    pub release_country: Option<String>,
    pub release_type: Option<Vec<String>>,
    pub record_label: Option<String>,
    pub media: Option<String>,
    pub track_num: Option<i32>,
    pub disc_num: Option<i32>,
    pub total_tracks: Option<i32>,
    pub total_discs: Option<i32>,
    pub mbid_recording: Option<String>,
    pub mbid_track: Option<String>,
    pub isrc: Option<String>,
    pub comment: Option<String>,
    pub copyright: Option<String>,
    pub script: Option<String>,
    pub lyrics: Option<String>,
    pub genres: Option<Vec<String>>,
    pub cover_bytes: Option<Vec<u8>>,
}

#[derive(Serialize, RustSignal)]
pub struct ApplyIdentificationResponse {
    pub id: String,
    pub track_id: String,
    pub success: bool,
    pub error: Option<String>,
    pub new_file_path: Option<String>,
}
