use rinf::{DartSignal, RustSignal, SignalPiece};
use serde::{Deserialize, Serialize};

use crate::signals::library::ArtistInfo;

#[derive(Debug, Clone, Serialize, Deserialize, SignalPiece)]
pub struct ReleaseInfo {
    pub id: String,
    pub title: String,
    pub date: Option<String>,
    pub country: Option<String>,
    pub artist: String,
    pub artist_id: Option<String>,
    pub artists: Vec<ArtistInfo>,
    pub genres: Vec<String>,
    pub tracks: Vec<ReleaseTrackInfo>,
    pub disambiguation: Option<String>,
    pub total_discs: Option<i32>,
    pub total_tracks: Option<i32>,
    pub release_group_mbid: Option<String>,
    pub release_type: Option<String>,
    pub record_label: Option<String>,
    pub media: Option<String>,
    pub script: Option<String>,
    pub original_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, SignalPiece)]
pub struct RecordingInfo {
    pub id: String,
    pub title: String,
    pub score: f64,
    pub artist: String,
    pub artist_id: Option<String>,
    pub artists: Vec<ArtistInfo>,
    pub isrcs: Vec<String>,
    pub duration_secs: Option<f64>,
    pub acoust_id: Option<String>,
    pub releases: Vec<ReleaseInfo>,
    pub cover: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, SignalPiece)]
pub struct MBSearchInfo {
    pub recordings: Vec<RecordingInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, SignalPiece)]
pub struct ReleaseTrackInfo {
    pub id: String,
    pub title: String,
    pub position: Option<i32>,
    pub disc_number: Option<i32>,
    pub duration_secs: Option<f64>,
    pub lyrics: Option<String>,
    pub artists: Vec<ArtistInfo>,
    pub mbid_track: Option<String>,
    pub isrcs: Vec<String>,
    pub genres: Vec<String>,
}

#[derive(Deserialize, DartSignal)]
pub struct EnhancedSearchRequest {
    pub id: String,
    pub query: String,
}

#[derive(Serialize, RustSignal)]
pub struct EnhancedSearchResponse {
    pub id: String,
    pub recordings: Vec<RecordingInfo>,
}

#[derive(Deserialize, DartSignal)]
pub struct GetReleaseTracksRequest {
    pub id: String,
    pub release_id: String,
}

#[derive(Serialize, RustSignal)]
pub struct GetReleaseTracksResponse {
    pub id: String,
    pub release_id: String,
    pub release_title: String,
    pub release_date: Option<String>,
    pub artist: String,
    pub artist_id: Option<String>,
    pub artists: Vec<ArtistInfo>,
    pub genres: Vec<String>,
    pub disambiguation: Option<String>,
    pub tracks: Vec<ReleaseTrackInfo>,
    pub release_country: Option<String>,
    pub release_group_mbid: Option<String>,
    pub release_type: Option<String>,
    pub record_label: Option<String>,
    pub media: Option<String>,
    pub script: Option<String>,
    pub original_date: Option<String>,
    pub total_tracks: Option<i32>,
    pub total_discs: Option<i32>,
}

// ---------------------------------------------------------------------------
// From impls
// ---------------------------------------------------------------------------

impl From<tawai_core::signals::metadata::ReleaseInfo> for ReleaseInfo {
    fn from(r: tawai_core::signals::metadata::ReleaseInfo) -> Self {
        Self {
            id: r.id,
            title: r.title,
            date: r.date,
            country: r.country,
            artist: r.artist,
            artist_id: r.artist_id,
            artists: r.artists.into_iter().map(Into::into).collect(),
            genres: r.genres,
            tracks: r.tracks.into_iter().map(Into::into).collect(),
            disambiguation: r.disambiguation,
            total_discs: r.total_discs,
            total_tracks: r.total_tracks,
            release_group_mbid: r.release_group_mbid,
            release_type: r.release_type,
            record_label: r.record_label,
            media: r.media,
            script: r.script,
            original_date: r.original_date,
        }
    }
}

impl From<tawai_core::signals::metadata::RecordingInfo> for RecordingInfo {
    fn from(r: tawai_core::signals::metadata::RecordingInfo) -> Self {
        Self {
            id: r.id,
            title: r.title,
            score: r.score,
            artist: r.artist,
            artist_id: r.artist_id,
            artists: r.artists.into_iter().map(Into::into).collect(),
            isrcs: r.isrcs,
            duration_secs: r.duration_secs,
            acoust_id: r.acoust_id,
            releases: r.releases.into_iter().map(Into::into).collect(),
            cover: r.cover,
        }
    }
}

impl From<tawai_core::signals::metadata::MBSearchInfo> for MBSearchInfo {
    fn from(r: tawai_core::signals::metadata::MBSearchInfo) -> Self {
        Self {
            recordings: r.recordings.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<tawai_core::signals::metadata::ReleaseTrackInfo> for ReleaseTrackInfo {
    fn from(t: tawai_core::signals::metadata::ReleaseTrackInfo) -> Self {
        Self {
            id: t.id,
            title: t.title,
            position: t.position,
            disc_number: t.disc_number,
            duration_secs: t.duration_secs,
            lyrics: t.lyrics,
            artists: t.artists.into_iter().map(Into::into).collect(),
            mbid_track: t.mbid_track,
            isrcs: t.isrcs,
            genres: t.genres,
        }
    }
}

// ---------------------------------------------------------------------------
// Fingerprint track
// ---------------------------------------------------------------------------

#[derive(Deserialize, DartSignal)]
pub struct FingerprintTrackRequest {
    pub id: String,
    pub track_id: Option<String>,
    /// When set, fingerprint the audio file at this path directly instead of
    /// looking up a stored fingerprint by track id (download-folder flow).
    pub file_path: Option<String>,
}

#[derive(Serialize, RustSignal)]
pub struct FingerprintTrackResponse {
    pub id: String,
    pub track_id: Option<String>,
    pub recording: Option<RecordingInfo>,
}

// ---------------------------------------------------------------------------
// Fetch recording by MBID
// ---------------------------------------------------------------------------

#[derive(Deserialize, DartSignal)]
pub struct FetchRecordingRequest {
    pub id: String,
    pub mbid: String,
}

#[derive(Serialize, RustSignal)]
pub struct FetchRecordingResponse {
    pub id: String,
    pub recording: Option<RecordingInfo>,
    pub error: Option<String>,
}

// ---------------------------------------------------------------------------
// Fetch lyrics from LRCLIB
// ---------------------------------------------------------------------------

#[derive(Deserialize, DartSignal)]
pub struct FetchLyricsRequest {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub album: Option<String>,
    pub duration: Option<f64>,
    pub prefer_sync: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, SignalPiece)]
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

impl From<tawai_core::signals::metadata::LyricsResult> for LyricsResult {
    fn from(r: tawai_core::signals::metadata::LyricsResult) -> Self {
        Self {
            id: r.id,
            title: r.title,
            artist: r.artist,
            album: r.album,
            duration: r.duration,
            instrumental: r.instrumental,
            lyrics: r.lyrics,
            synced: r.synced,
        }
    }
}

#[derive(Serialize, RustSignal)]
pub struct FetchLyricsResponse {
    pub id: String,
    pub result: Option<LyricsResult>,
    pub error: Option<String>,
}

#[derive(Deserialize, DartSignal)]
pub struct SearchLyricsRequest {
    pub id: String,
    pub query: String,
    pub prefer_sync: bool,
}

#[derive(Serialize, RustSignal)]
pub struct SearchLyricsResponse {
    pub id: String,
    pub results: Vec<LyricsResult>,
    pub error: Option<String>,
}

// ---------------------------------------------------------------------------
// Standalone file tag reader/writer
// ---------------------------------------------------------------------------

#[derive(Deserialize, DartSignal)]
pub struct ReadFileTagsRequest {
    pub id: String,
    pub path: String,
}

#[derive(Debug, Clone, Default, Serialize, RustSignal)]
pub struct ReadFileTagsResponse {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub artists: Vec<String>,
    pub album: String,
    pub album_artist: String,
    pub album_artists: Vec<String>,
    pub artist_sort: String,
    pub album_artist_sort: String,
    pub genres: Vec<String>,
    pub track_number: Option<i32>,
    pub disc_number: Option<i32>,
    pub total_tracks: Option<i32>,
    pub total_discs: Option<i32>,
    pub release_date: Option<String>,
    pub original_date: Option<String>,
    pub release_country: Option<String>,
    pub release_type: Vec<String>,
    pub record_label: Option<String>,
    pub media: Option<String>,
    pub script: Option<String>,
    pub isrc: Option<String>,
    pub mbid_recording: Option<String>,
    pub mbid_track: Option<String>,
    pub mbid_artist: Option<String>,
    pub mbid_release_artist: Option<String>,
    pub mbid_release: Option<String>,
    pub mbid_release_group: Option<String>,
    pub comment: Option<String>,
    pub copyright: Option<String>,
    pub lyrics: Option<String>,
    pub track_gain: Option<f64>,
    pub track_peak: Option<f64>,
    pub encoder: Option<String>,
    pub cover: Option<Vec<u8>>,
    pub duration_secs: f64,
    pub error: Option<String>,
}

impl From<tawai_core::signals::metadata::ReadFileTagsResponse> for ReadFileTagsResponse {
    fn from(r: tawai_core::signals::metadata::ReadFileTagsResponse) -> Self {
        Self {
            id: r.id,
            title: r.title,
            artist: r.artist,
            artists: r.artists,
            album: r.album,
            album_artist: r.album_artist,
            album_artists: r.album_artists,
            artist_sort: r.artist_sort,
            album_artist_sort: r.album_artist_sort,
            genres: r.genres,
            track_number: r.track_number,
            disc_number: r.disc_number,
            total_tracks: r.total_tracks,
            total_discs: r.total_discs,
            release_date: r.release_date,
            original_date: r.original_date,
            release_country: r.release_country,
            release_type: r.release_type,
            record_label: r.record_label,
            media: r.media,
            script: r.script,
            isrc: r.isrc,
            mbid_recording: r.mbid_recording,
            mbid_track: r.mbid_track,
            mbid_artist: r.mbid_artist,
            mbid_release_artist: r.mbid_release_artist,
            mbid_release: r.mbid_release,
            mbid_release_group: r.mbid_release_group,
            comment: r.comment,
            copyright: r.copyright,
            lyrics: r.lyrics,
            track_gain: r.track_gain,
            track_peak: r.track_peak,
            encoder: r.encoder,
            cover: r.cover,
            duration_secs: r.duration_secs,
            error: r.error,
        }
    }
}

#[derive(Deserialize, DartSignal)]
pub struct WriteFileTagsRequest {
    pub id: String,
    pub path: String,
    /// Update the matching library track row (matched by file path) after a
    /// successful write. Ignored by bytes-based writes.
    pub sync_database: bool,
    pub title: String,
    pub artist: String,
    pub artists: Vec<String>,
    pub album: String,
    pub album_artist: String,
    pub album_artists: Vec<String>,
    pub genres: Vec<String>,
    pub track_number: i32,
    pub disc_number: i32,
    pub total_tracks: i32,
    pub total_discs: i32,
    pub release_date: Option<String>,
    pub original_date: Option<String>,
    pub media: Option<String>,
    pub script: Option<String>,
    pub record_label: Option<String>,
    pub release_country: Option<String>,
    pub release_type: Vec<String>,
    pub isrc: Option<String>,
    pub mbid_recording: Option<String>,
    pub mbid_track: Option<String>,
    pub mbid_artist: Option<String>,
    pub mbid_release: Option<String>,
    pub mbid_release_artist: Option<String>,
    pub mbid_release_group: Option<String>,
    pub artist_sort: String,
    pub album_artist_sort: String,
    pub comment: Option<String>,
    pub copyright: Option<String>,
    pub lyrics: Option<String>,
    pub cover: Option<Vec<u8>>,
    pub clear_cover: bool,
}

#[derive(Serialize, RustSignal)]
pub struct WriteFileTagsResponse {
    pub id: String,
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Deserialize, DartSignal)]
pub struct ReadFileTagsBytesRequest {
    pub id: String,
    pub filename: String,
    pub bytes: Vec<u8>,
}

#[derive(Deserialize, DartSignal)]
pub struct WriteFileTagsBytesRequest {
    pub id: String,
    pub filename: String,
    pub bytes: Vec<u8>,
    pub title: String,
    pub artist: String,
    pub artists: Vec<String>,
    pub album: String,
    pub album_artist: String,
    pub album_artists: Vec<String>,
    pub genres: Vec<String>,
    pub track_number: i32,
    pub disc_number: i32,
    pub total_tracks: i32,
    pub total_discs: i32,
    pub release_date: Option<String>,
    pub original_date: Option<String>,
    pub media: Option<String>,
    pub script: Option<String>,
    pub record_label: Option<String>,
    pub release_country: Option<String>,
    pub release_type: Vec<String>,
    pub isrc: Option<String>,
    pub mbid_recording: Option<String>,
    pub mbid_track: Option<String>,
    pub mbid_artist: Option<String>,
    pub mbid_release: Option<String>,
    pub mbid_release_artist: Option<String>,
    pub mbid_release_group: Option<String>,
    pub artist_sort: String,
    pub album_artist_sort: String,
    pub comment: Option<String>,
    pub copyright: Option<String>,
    pub lyrics: Option<String>,
    pub cover: Option<Vec<u8>>,
    pub clear_cover: bool,
}

#[derive(Serialize, RustSignal)]
pub struct WriteFileTagsBytesResponse {
    pub id: String,
    pub success: bool,
    pub bytes: Vec<u8>,
    pub error: Option<String>,
}

// ---------------------------------------------------------------------------
// Naming format preview
// ---------------------------------------------------------------------------

#[derive(Deserialize, DartSignal)]
pub struct FormatNamingPreviewRequest {
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
    /// Library track to preview the destination for. When set, the source root
    /// and file extension are resolved the same way `apply_identification` does.
    pub track_id: Option<String>,
    /// Download-folder flow: the library source the file will be moved into.
    pub target_source_id: Option<String>,
    /// Audio file being identified; supplies the extension and the current path
    /// shown when Apply would not move the file.
    pub file_path: Option<String>,
    /// User requesting the preview. Used to authorize the source whose root is
    /// resolved; this is local identity context, not a server credential.
    pub user_id: String,
}

#[derive(Serialize, RustSignal)]
pub struct FormatNamingPreviewResponse {
    pub id: String,
    pub result: String,
}
