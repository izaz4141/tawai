use rinf::{DartSignal, RustSignal, SignalPiece};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, SignalPiece)]
pub struct ReleaseInfo {
    pub id: String,
    pub title: String,
    pub date: Option<String>,
    pub country: Option<String>,
    pub artist: String,
    pub artist_id: Option<String>,
    pub tracks: Vec<ReleaseTrackInfo>,
    pub disambiguation: Option<String>,
    pub total_discs: Option<i32>,
    pub total_tracks: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, SignalPiece)]
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
    pub disambiguation: Option<String>,
    pub tracks: Vec<ReleaseTrackInfo>,
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
            tracks: r.tracks.into_iter().map(Into::into).collect(),
            disambiguation: r.disambiguation,
            total_discs: r.total_discs,
            total_tracks: r.total_tracks,
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
        }
    }
}

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
    pub tracks: Vec<super::library::TrackInfo>,
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
    pub track_id: String,
    pub title: String,
    pub artist: String,
    pub artist_mbid: Option<String>,
    pub album: String,
    pub album_mbid: Option<String>,
    pub album_disambiguation: Option<String>,
    pub release_date: Option<String>,
    pub track_num: Option<i32>,
    pub disc_num: Option<i32>,
    pub mbid_recording: Option<String>,
    pub lyrics: Option<String>,
    pub cover_bytes: Option<Vec<u8>>,
    pub total_discs: i32,
}

#[derive(Serialize, RustSignal)]
pub struct ApplyIdentificationResponse {
    pub id: String,
    pub track_id: String,
    pub success: bool,
    pub error: Option<String>,
}

// ---------------------------------------------------------------------------
// Fingerprint track
// ---------------------------------------------------------------------------

#[derive(Deserialize, DartSignal)]
pub struct FingerprintTrackRequest {
    pub id: String,
    pub track_id: String,
}

#[derive(Serialize, RustSignal)]
pub struct FingerprintTrackResponse {
    pub id: String,
    pub track_id: String,
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

#[derive(Serialize, RustSignal)]
pub struct ReadFileTagsResponse {
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

#[derive(Deserialize, DartSignal)]
pub struct WriteFileTagsRequest {
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

#[derive(Serialize, RustSignal)]
pub struct WriteFileTagsResponse {
    pub id: String,
    pub success: bool,
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
}

#[derive(Serialize, RustSignal)]
pub struct FormatNamingPreviewResponse {
    pub id: String,
    pub result: String,
}
