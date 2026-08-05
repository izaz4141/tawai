use rinf::{DartSignal, RustSignal, SignalPiece};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, DartSignal)]
pub struct ListTracksRequest {
    pub id: String,
    pub album_id: Option<String>,
}

#[derive(Serialize, RustSignal)]
pub struct ListTracksResponse {
    pub id: String,
    pub tracks: Vec<TrackInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, SignalPiece, Default)]
pub struct TrackInfo {
    pub id: String,
    pub title: String,
    pub album_id: String,
    pub album_title: String,
    pub artists_string: String,
    pub artists: Vec<ArtistInfo>,
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

#[derive(Deserialize, DartSignal)]
pub struct ListAlbumsRequest {
    pub id: String,
    pub artist_id: Option<String>,
}

#[derive(Serialize, RustSignal)]
pub struct ListAlbumsResponse {
    pub id: String,
    pub albums: Vec<AlbumInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, SignalPiece)]
pub struct AlbumInfo {
    pub id: String,
    pub title: String,
    pub sort_name: Option<String>,
    pub artists_string: String,
    pub artists: Vec<ArtistInfo>,
    pub release_date: Option<String>,
    pub track_count: i64,
}

#[derive(Deserialize, DartSignal)]
pub struct ListArtistsRequest {
    pub id: String,
}

#[derive(Serialize, RustSignal)]
pub struct ListArtistsResponse {
    pub id: String,
    pub artists: Vec<ArtistInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, SignalPiece)]
pub struct ArtistInfo {
    pub id: String,
    pub name: String,
    pub sort_name: Option<String>,
    pub mbid: Option<String>,
    pub thumbnail_url: Option<String>,
    pub album_count: i64,
    pub track_count: i64,
}

#[derive(Deserialize, DartSignal)]
pub struct ListPlaylistsRequest {
    pub id: String,
}

#[derive(Serialize, RustSignal)]
pub struct ListPlaylistsResponse {
    pub id: String,
    pub playlists: Vec<PlaylistInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, SignalPiece)]
pub struct PlaylistInfo {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub is_smart: bool,
    pub track_count: i64,
    pub created_at: Option<String>,
}

#[derive(Deserialize, DartSignal)]
pub struct CreatePlaylistRequest {
    pub id: String,
    pub name: String,
}

#[derive(Serialize, RustSignal)]
pub struct CreatePlaylistResponse {
    pub id: String,
    pub playlist_id: String,
}

#[derive(Deserialize, DartSignal)]
pub struct DeletePlaylistRequest {
    pub id: String,
    pub playlist_id: String,
}

#[derive(Serialize, RustSignal)]
pub struct DeletePlaylistResponse {
    pub id: String,
    pub success: bool,
}

// ---------------------------------------------------------------------------
// Get Track
// ---------------------------------------------------------------------------

#[derive(Deserialize, DartSignal)]
pub struct GetTrackRequest {
    pub id: String,
    pub track_id: String,
}

#[derive(Serialize, RustSignal)]
pub struct GetTrackResponse {
    pub id: String,
    pub track: TrackInfo,
    pub error: Option<String>,
}

// ---------------------------------------------------------------------------
// Cover
// ---------------------------------------------------------------------------

#[derive(Deserialize, DartSignal)]
pub struct GetAlbumCoverRequest {
    pub id: String,
    pub album_id: String,
}

#[derive(Serialize, RustSignal)]
pub struct GetAlbumCoverResponse {
    pub id: String,
    pub cover: Option<Vec<u8>>,
}

#[derive(Deserialize, DartSignal)]
pub struct GetTrackCoverRequest {
    pub id: String,
    pub track_id: String,
}

#[derive(Serialize, RustSignal)]
pub struct GetTrackCoverResponse {
    pub id: String,
    pub cover: Option<Vec<u8>>,
}

// ---------------------------------------------------------------------------
// Scan
// ---------------------------------------------------------------------------

#[derive(Deserialize, DartSignal)]
pub struct ScanLibraryRequest {
    pub id: String,
    pub user_id: String,
    pub force: bool,
}

#[derive(Serialize, RustSignal)]
pub struct ScanLibraryResponse {
    pub id: String,
    pub started: bool,
    pub error: Option<String>,
}

#[derive(Deserialize, DartSignal)]
pub struct ScanStatusRequest {
    pub id: String,
}

#[derive(Serialize, RustSignal)]
pub struct ScanStatusResponse {
    pub id: String,
    pub running: bool,
    pub progress: Option<ScanProgressSignal>,
}

impl From<tawai_core::signals::library::TrackInfo> for TrackInfo {
    fn from(t: tawai_core::signals::library::TrackInfo) -> Self {
        Self {
            id: t.id,
            title: t.title,
            album_id: t.album_id,
            album_title: t.album_title,
            artists_string: t.artists_string,
            artists: t.artists.into_iter().map(|a| a.into()).collect(),
            track_num: t.track_num,
            disc_num: t.disc_num,
            duration_secs: t.duration_secs,
            file_path: t.file_path,
            file_size: t.file_size,
            bitrate: t.bitrate,
            mbid_recording: t.mbid_recording,
            artist_mbid: t.artist_mbid,
            album_mbid: t.album_mbid,
            lyrics: t.lyrics,
            release_date: t.release_date,
            track_gain: t.track_gain,
            track_peak: t.track_peak,
            source: t.source,
            source_type: t.source_type,
            genres: t.genres,
        }
    }
}

impl From<tawai_core::signals::library::AlbumInfo> for AlbumInfo {
    fn from(a: tawai_core::signals::library::AlbumInfo) -> Self {
        Self {
            id: a.id,
            title: a.title,
            sort_name: a.sort_name,
            artists_string: a.artists_string,
            artists: a.artists.into_iter().map(|a| a.into()).collect(),
            release_date: a.release_date,
            track_count: a.track_count,
        }
    }
}

impl From<tawai_core::signals::library::ArtistInfo> for ArtistInfo {
    fn from(a: tawai_core::signals::library::ArtistInfo) -> Self {
        Self {
            id: a.id,
            name: a.name,
            sort_name: a.sort_name,
            mbid: a.mbid,
            thumbnail_url: a.thumbnail_url,
            album_count: a.album_count,
            track_count: a.track_count,
        }
    }
}

impl From<tawai_core::signals::library::PlaylistInfo> for PlaylistInfo {
    fn from(p: tawai_core::signals::library::PlaylistInfo) -> Self {
        Self {
            id: p.id,
            name: p.name,
            description: p.description,
            is_smart: p.is_smart,
            track_count: p.track_count,
            created_at: p.created_at,
        }
    }
}

#[derive(Serialize, RustSignal, SignalPiece, Default)]
pub struct ScanProgressSignal {
    pub id: String,
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

// ---------------------------------------------------------------------------
// Periodic Scan
// ---------------------------------------------------------------------------

#[derive(Deserialize, DartSignal)]
pub struct StartPeriodicScanRequest {
    pub id: String,
}

#[derive(Serialize, RustSignal)]
pub struct StartPeriodicScanResponse {
    pub id: String,
}

// ---------------------------------------------------------------------------
// List tracks by source
// ---------------------------------------------------------------------------

#[derive(Deserialize, DartSignal)]
pub struct ListTracksBySourceRequest {
    pub id: String,
    pub source_id: String,
}

#[derive(Serialize, RustSignal)]
pub struct ListTracksBySourceResponse {
    pub id: String,
    pub tracks: Vec<TrackInfo>,
}

// ---------------------------------------------------------------------------
// List download folder tracks
// ---------------------------------------------------------------------------

#[derive(Deserialize, DartSignal)]
pub struct ListDownloadFolderTracksRequest {
    pub id: String,
    pub path: String,
}

#[derive(Serialize, RustSignal)]
pub struct ListDownloadFolderTracksResponse {
    pub id: String,
    pub tracks: Vec<TrackInfo>,
}

// ---------------------------------------------------------------------------
// Scan a single library source
// ---------------------------------------------------------------------------

#[derive(Deserialize, DartSignal)]
pub struct ScanSourceRequest {
    pub id: String,
    pub user_id: String,
    pub source_id: String,
    pub force: bool,
}

#[derive(Serialize, RustSignal)]
pub struct ScanSourceResponse {
    pub id: String,
    pub started: bool,
    pub error: Option<String>,
}

// ---------------------------------------------------------------------------
// Get album MBID
// ---------------------------------------------------------------------------

#[derive(Deserialize, DartSignal)]
pub struct GetAlbumMbidRequest {
    pub id: String,
    pub album_id: String,
}

#[derive(Serialize, RustSignal)]
pub struct GetAlbumMbidResponse {
    pub id: String,
    pub mbid: Option<String>,
}

// ---------------------------------------------------------------------------
// Library Source Management
// ---------------------------------------------------------------------------

#[derive(Deserialize, DartSignal)]
pub struct AddLibrarySourceRequest {
    pub id: String,
    pub user_id: String,
    pub url: String,
    pub name: String,
    pub source_type: String,
}

#[derive(Serialize, RustSignal)]
pub struct AddLibrarySourceResponse {
    pub id: String,
    pub source_id: String,
    pub success: bool,
}

#[derive(Deserialize, DartSignal)]
pub struct RemoveLibrarySourceRequest {
    pub id: String,
    pub user_id: String,
    pub source_id: String,
}

#[derive(Serialize, RustSignal)]
pub struct RemoveLibrarySourceResponse {
    pub id: String,
    pub success: bool,
}

#[derive(Deserialize, DartSignal)]
pub struct ListLibrarySourcesRequest {
    pub id: String,
    pub user_id: String,
}

#[derive(Serialize, RustSignal)]
pub struct ListLibrarySourcesResponse {
    pub id: String,
    pub sources: Vec<LibrarySourceInfo>,
}

impl From<tawai_core::signals::library::LibrarySourceInfo> for LibrarySourceInfo {
    fn from(s: tawai_core::signals::library::LibrarySourceInfo) -> Self {
        Self {
            id: s.id,
            source_type: s.source_type,
            url: s.url,
            name: s.name,
            last_sync_at: s.last_sync_at,
            owner_id: s.owner_id,
            access_rule: s.access_rule,
            created_at: s.created_at,
            updated_at: s.updated_at,
        }
    }
}

#[derive(Deserialize, DartSignal)]
pub struct ListEditableSourcesRequest {
    pub id: String,
    pub user_id: String,
}

#[derive(Serialize, RustSignal)]
pub struct ListEditableSourcesResponse {
    pub id: String,
    pub sources: Vec<LibrarySourceInfo>,
}

// ---------------------------------------------------------------------------
// Playlist tracks
// ---------------------------------------------------------------------------

#[derive(Deserialize, DartSignal)]
pub struct GetPlaylistTracksRequest {
    pub id: String,
    pub playlist_id: String,
}

#[derive(Serialize, RustSignal)]
pub struct GetPlaylistTracksResponse {
    pub id: String,
    pub tracks: Vec<TrackInfo>,
}

#[derive(Deserialize, DartSignal)]
pub struct AddTrackToPlaylistRequest {
    pub id: String,
    pub playlist_id: String,
    pub track_id: String,
}

#[derive(Serialize, RustSignal)]
pub struct AddTrackToPlaylistResponse {
    pub id: String,
    pub success: bool,
}

#[derive(Deserialize, DartSignal)]
pub struct RemoveTrackFromPlaylistRequest {
    pub id: String,
    pub playlist_id: String,
    pub track_id: String,
}

#[derive(Serialize, RustSignal)]
pub struct RemoveTrackFromPlaylistResponse {
    pub id: String,
    pub success: bool,
}

#[derive(Deserialize, DartSignal)]
pub struct ReorderPlaylistTracksRequest {
    pub id: String,
    pub playlist_id: String,
    pub track_ids: Vec<String>,
}

#[derive(Serialize, RustSignal)]
pub struct ReorderPlaylistTracksResponse {
    pub id: String,
    pub success: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, SignalPiece)]
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
