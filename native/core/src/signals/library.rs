use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TrackInfo {
    pub id: String,
    pub title: String,
    pub album_id: String,
    pub album_title: String,
    pub artists: Vec<ArtistInfo>,
    pub artists_string: String,
    pub track_num: Option<i32>,
    pub disc_num: Option<i32>,
    pub duration_secs: Option<f64>,
    pub file_path: String,
    pub file_size: Option<i64>,
    pub bitrate: Option<i32>,
    pub mbid_recording: Option<String>,
    pub artist_mbid: Option<String>,
    pub album_mbid: Option<String>,
    pub lyrics: Option<String>,
    pub release_date: Option<String>,
    pub track_gain: Option<f64>,
    pub track_peak: Option<f64>,
    pub source: String,
    pub source_type: String,
    pub genres: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AlbumInfo {
    pub id: String,
    pub title: String,
    pub sort_name: Option<String>,
    pub artists: Vec<ArtistInfo>,
    pub artists_string: String,
    pub release_date: Option<String>,
    pub track_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ArtistInfo {
    pub id: String,
    pub name: String,
    pub sort_name: Option<String>,
    pub mbid: Option<String>,
    pub thumbnail_url: Option<String>,
    pub album_count: i64,
    pub track_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PlaylistInfo {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub is_smart: bool,
    pub track_count: i64,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ListTracksRequest {
    #[serde(default)]
    pub id: String,
    pub album_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ListTracksResponse {
    pub id: String,
    pub tracks: Vec<TrackInfo>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ListAlbumsRequest {
    #[serde(default)]
    pub id: String,
    pub artist_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ListAlbumsResponse {
    pub id: String,
    pub albums: Vec<AlbumInfo>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ListArtistsRequest {
    #[serde(default)]
    pub id: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ListArtistsResponse {
    pub id: String,
    pub artists: Vec<ArtistInfo>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ListPlaylistsRequest {
    #[serde(default)]
    pub id: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ListPlaylistsResponse {
    pub id: String,
    pub playlists: Vec<PlaylistInfo>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreatePlaylistRequest {
    pub id: String,
    pub user_id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CreatePlaylistResponse {
    pub id: String,
    pub playlist_id: String,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct DeletePlaylistRequest {
    pub id: String,
    pub playlist_id: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct DeletePlaylistResponse {
    pub id: String,
    pub success: bool,
}

// Scan / library folder management

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ScanResult {
    pub success: bool,
    pub tracks_found: u32,
    pub new_tracks: u32,
    pub duplicates: u32,
    pub deleted: u32,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
pub struct ScanProgress {
    pub current_file: String,
    pub files_scanned: u32,
    pub total_files: u32,
    pub stage: String,
    pub complete: bool,
    pub tracks_found: u32,
    pub new_tracks: u32,
    pub duplicates: u32,
    pub deleted: u32,
    pub current_source: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LibrarySourceInfo {
    pub id: String,
    pub source_type: String,
    pub url: String,
    pub name: String,
    pub last_sync_at: Option<String>,
    pub owner_id: String,
    pub access_rule: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ListEditableSourcesRequest {
    pub id: String,
    pub user_id: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ListEditableSourcesResponse {
    pub id: String,
    pub sources: Vec<LibrarySourceInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LibraryFolderInfo {
    pub id: String,
    pub path: String,
    pub last_scanned_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ScanLibraryRequest {
    pub id: String,
    #[serde(default)]
    pub user_id: String,
    pub force: bool,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ScanLibraryResponse {
    pub id: String,
    pub started: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ScanStatusRequest {
    pub id: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ScanStatusResponse {
    pub id: String,
    pub running: bool,
    pub progress: Option<ScanProgress>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct AddLibraryFolderRequest {
    pub id: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AddLibraryFolderResponse {
    pub id: String,
    pub folder_id: String,
    pub success: bool,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct RemoveLibraryFolderRequest {
    pub id: String,
    pub folder_id: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct RemoveLibraryFolderResponse {
    pub id: String,
    pub success: bool,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ListLibraryFoldersRequest {
    pub id: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ListLibraryFoldersResponse {
    pub id: String,
    pub folders: Vec<LibraryFolderInfo>,
}

// ---------------------------------------------------------------------------
// Playlist tracks
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct GetPlaylistTracksRequest {
    pub id: String,
    pub playlist_id: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct GetPlaylistTracksResponse {
    pub id: String,
    pub tracks: Vec<TrackInfo>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct AddTrackToPlaylistRequest {
    pub id: String,
    pub playlist_id: String,
    pub track_id: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AddTrackToPlaylistResponse {
    pub id: String,
    pub success: bool,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct RemoveTrackFromPlaylistRequest {
    pub id: String,
    pub playlist_id: String,
    pub track_id: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct RemoveTrackFromPlaylistResponse {
    pub id: String,
    pub success: bool,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ReorderPlaylistTracksRequest {
    pub id: String,
    pub playlist_id: String,
    pub track_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ReorderPlaylistTracksResponse {
    pub id: String,
    pub success: bool,
}

// ---------------------------------------------------------------------------
// Get Track / Covers
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct GetTrackRequest {
    pub id: String,
    pub track_id: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct GetTrackResponse {
    pub id: String,
    pub track: TrackInfo,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct GetAlbumCoverRequest {
    pub id: String,
    pub album_id: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct GetAlbumCoverResponse {
    pub id: String,
    pub cover: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct GetTrackCoverRequest {
    pub id: String,
    pub track_id: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct GetTrackCoverResponse {
    pub id: String,
    pub cover: Option<Vec<u8>>,
}

// ---------------------------------------------------------------------------
// Periodic Scan
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct StartPeriodicScanRequest {
    pub id: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct StartPeriodicScanResponse {
    pub id: String,
}

// ---------------------------------------------------------------------------
// Tracks by source / download folder
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ListTracksBySourceRequest {
    pub id: String,
    pub source_id: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ListTracksBySourceResponse {
    pub id: String,
    pub tracks: Vec<TrackInfo>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ListDownloadFolderTracksRequest {
    pub id: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ListDownloadFolderTracksResponse {
    pub id: String,
    pub tracks: Vec<TrackInfo>,
}

// ---------------------------------------------------------------------------
// Scan a single library source
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ScanSourceRequest {
    pub id: String,
    #[serde(default)]
    pub user_id: String,
    pub source_id: String,
    pub force: bool,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ScanSourceResponse {
    pub id: String,
    pub started: bool,
    pub error: Option<String>,
}

// ---------------------------------------------------------------------------
// Get album MBID
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct GetAlbumMbidRequest {
    pub id: String,
    pub album_id: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct GetAlbumMbidResponse {
    pub id: String,
    pub mbid: Option<String>,
}

// ---------------------------------------------------------------------------
// Library Source Management
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct AddLibrarySourceRequest {
    pub id: String,
    #[serde(default)]
    pub user_id: String,
    pub url: String,
    pub name: String,
    pub source_type: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AddLibrarySourceResponse {
    pub id: String,
    pub source_id: String,
    pub success: bool,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct RemoveLibrarySourceRequest {
    pub id: String,
    #[serde(default)]
    pub user_id: String,
    pub source_id: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct RemoveLibrarySourceResponse {
    pub id: String,
    pub success: bool,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ListLibrarySourcesRequest {
    pub id: String,
    #[serde(default)]
    pub user_id: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ListLibrarySourcesResponse {
    pub id: String,
    pub sources: Vec<LibrarySourceInfo>,
}
