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

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct EnhancedSearchRequest {
    #[serde(default)]
    pub id: String,
    pub query: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct EnhancedSearchResponse {
    #[serde(default)]
    pub id: String,
    pub recordings: Vec<RecordingInfo>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct GetReleaseTracksRequest {
    #[serde(default)]
    pub id: String,
    pub release_id: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct GetReleaseTracksResponse {
    #[serde(default)]
    pub id: String,
    pub release_id: String,
    pub release_title: String,
    pub release_date: Option<String>,
    pub artist: String,
    pub artist_id: Option<String>,
    pub disambiguation: Option<String>,
    pub tracks: Vec<ReleaseTrackInfo>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct FingerprintTrackRequest {
    #[serde(default)]
    pub id: String,
    pub track_id: String,
    pub file_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct FingerprintTrackResponse {
    #[serde(default)]
    pub id: String,
    pub track_id: String,
    pub recording: Option<RecordingInfo>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct FetchRecordingRequest {
    #[serde(default)]
    pub id: String,
    pub mbid: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct FetchRecordingResponse {
    #[serde(default)]
    pub id: String,
    pub recording: Option<RecordingInfo>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct FetchLyricsRequest {
    #[serde(default)]
    pub id: String,
    pub title: String,
    pub artist: String,
    pub album: Option<String>,
    pub duration: Option<f64>,
    pub prefer_sync: bool,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct FetchLyricsResponse {
    #[serde(default)]
    pub id: String,
    pub result: Option<LyricsResult>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct SearchLyricsRequest {
    #[serde(default)]
    pub id: String,
    pub query: String,
    pub prefer_sync: bool,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SearchLyricsResponse {
    #[serde(default)]
    pub id: String,
    pub results: Vec<LyricsResult>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ReadFileTagsRequest {
    #[serde(default)]
    pub id: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ReadFileTagsResponse {
    #[serde(default)]
    pub id: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub album_artist: String,
    pub genres: Vec<String>,
    pub track_number: i32,
    pub disc_number: i32,
    pub release_date: Option<String>,
    pub lyrics: Option<String>,
    pub cover: Option<Vec<u8>>,
    pub duration_secs: f64,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct WriteFileTagsRequest {
    #[serde(default)]
    pub id: String,
    pub path: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub album_artist: String,
    pub genres: Vec<String>,
    pub track_number: i32,
    pub disc_number: i32,
    pub release_date: Option<String>,
    pub lyrics: Option<String>,
    pub cover: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct WriteFileTagsResponse {
    #[serde(default)]
    pub id: String,
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ReadFileTagsBytesRequest {
    #[serde(default)]
    pub id: String,
    pub filename: String,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct WriteFileTagsBytesRequest {
    #[serde(default)]
    pub id: String,
    pub filename: String,
    pub bytes: Vec<u8>,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub album_artist: String,
    pub genres: Vec<String>,
    pub track_number: i32,
    pub disc_number: i32,
    pub release_date: Option<String>,
    pub lyrics: Option<String>,
    pub cover: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct WriteFileTagsBytesResponse {
    #[serde(default)]
    pub id: String,
    pub success: bool,
    pub bytes: Vec<u8>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct FormatNamingPreviewRequest {
    #[serde(default)]
    pub id: String,
    pub pattern: String,
    pub title: String,
    pub artist: String,
    pub album_artist: String,
    pub album: String,
    pub release_date: Option<String>,
    pub track_number: i32,
    pub disc_number: i32,
    pub album_disambiguation: Option<String>,
    pub total_discs: i32,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct FormatNamingPreviewResponse {
    #[serde(default)]
    pub id: String,
    pub result: String,
}
