use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

// ---------------------------------------------------------------------------
// Collection Statistics
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LibraryStats {
    pub total_tracks: i64,
    pub total_albums: i64,
    pub total_artists: i64,
    pub total_duration_secs: f64,
    pub average_bitrate: Option<f64>,
    pub most_common_genre: Option<String>,
    pub genre_count: i64,
    pub format_breakdown: Vec<FormatEntry>,
    pub decade_distribution: Vec<DecadeEntry>,
    pub largest_album_title: Option<String>,
    pub largest_album_tracks: i64,
    pub most_prolific_artist: Option<String>,
    pub most_prolific_artist_tracks: i64,
    pub naming_conformity_pct: Option<f64>,
    pub total_file_size: i64,
    pub tracks_with_cover: i64,
    pub tracks_without_cover: i64,
    pub tracks_with_lyrics: i64,
    pub tracks_without_lyrics: i64,
    pub average_track_duration_secs: f64,
    pub shortest_track_title: Option<String>,
    pub shortest_track_duration: Option<f64>,
    pub longest_track_title: Option<String>,
    pub longest_track_duration: Option<f64>,
    pub tracks_per_album_avg: f64,
    pub tracks_per_artist_avg: f64,
    pub tracks_with_mbid: i64,
    pub oldest_year: Option<String>,
    pub newest_year: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FormatEntry {
    pub format: String,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DecadeEntry {
    pub decade: String,
    pub count: i64,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct GetLibraryStatsRequest {
    #[serde(default)]
    pub id: String,
    pub naming_pattern: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct GetLibraryStatsResponse {
    #[serde(default)]
    pub id: String,
    pub stats: Option<LibraryStats>,
    pub error: Option<String>,
}

// ---------------------------------------------------------------------------
// Missing Metadata
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MissingMetadataEntry {
    pub track_id: String,
    pub file_path: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub missing_fields: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MissingMetadataCheck {
    pub check_title: bool,
    pub check_artist: bool,
    pub check_album: bool,
    pub check_genre: bool,
    pub check_year: bool,
    pub check_track_number: bool,
    pub check_cover: bool,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct FindMissingMetadataRequest {
    #[serde(default)]
    pub id: String,
    pub check_title: bool,
    pub check_artist: bool,
    pub check_album: bool,
    pub check_genre: bool,
    pub check_year: bool,
    pub check_track_number: bool,
    pub check_cover: bool,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct FindMissingMetadataResponse {
    #[serde(default)]
    pub id: String,
    pub tracks: Vec<MissingMetadataEntry>,
    pub error: Option<String>,
}

// ---------------------------------------------------------------------------
// Batch Rename
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RenamePreview {
    pub file_path: String,
    pub expected_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct NamingViolation {
    pub file_path: String,
    pub file_name: String,
    pub expected_name: String,
    pub track_id: String,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct BatchRenamePreviewRequest {
    #[serde(default)]
    pub id: String,
    pub file_paths: Vec<String>,
    pub pattern: String,
    pub source_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct BatchRenamePreviewResponse {
    #[serde(default)]
    pub id: String,
    pub previews: Vec<RenamePreview>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct BatchRenameApplyRequest {
    #[serde(default)]
    pub id: String,
    pub file_paths: Vec<String>,
    pub pattern: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct BatchRenameApplyResponse {
    #[serde(default)]
    pub id: String,
    pub results: Vec<RenamePreview>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CheckNamingConventionRequest {
    #[serde(default)]
    pub id: String,
    pub source_id: Option<String>,
    pub pattern: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CheckNamingConventionResponse {
    #[serde(default)]
    pub id: String,
    pub violations: Vec<NamingViolation>,
    pub error: Option<String>,
}

// ---------------------------------------------------------------------------
// Write Track Lyrics
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WriteLyricsResult {
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct WriteTrackLyricsRequest {
    #[serde(default)]
    pub id: String,
    pub track_id: String,
    pub lyrics: String,
    pub synced: bool,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct WriteTrackLyricsResponse {
    #[serde(default)]
    pub id: String,
    pub success: bool,
    pub error: Option<String>,
}

// ---------------------------------------------------------------------------
// Romajize Lyrics
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct RomajizeLyricsRequest {
    #[serde(default)]
    pub id: String,
    pub lyrics: String,
    pub synced: bool,
    pub lang: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct RomajizeLyricsResponse {
    #[serde(default)]
    pub id: String,
    pub romajized: String,
    pub synced: bool,
    pub error: Option<String>,
}

// ---------------------------------------------------------------------------
// Duplicate Finder
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DuplicateTrackEntry {
    pub track_id: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub file_path: String,
    pub file_size: Option<i64>,
    pub duration_secs: f64,
    pub mbid_recording: Option<String>,
    pub has_fingerprint: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DuplicateGroup {
    pub method: String,
    pub tracks: Vec<DuplicateTrackEntry>,
    pub confidence: f64,
    pub key: String,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct FindDuplicatesRequest {
    #[serde(default)]
    pub id: String,
    pub check_fingerprint: bool,
    pub check_mbid: bool,
    pub check_file_size_duration: bool,
    pub check_title_artist: bool,
    pub min_confidence: Option<f64>,
    pub source_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct FindDuplicatesResponse {
    #[serde(default)]
    pub id: String,
    pub groups: Vec<DuplicateGroup>,
    pub total_duplicates: u32,
    pub total_groups: u32,
    pub error: Option<String>,
}
