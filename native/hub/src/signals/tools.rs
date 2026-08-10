use rinf::{DartSignal, RustSignal, SignalPiece};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Batch Rename
// ---------------------------------------------------------------------------

#[derive(Deserialize, DartSignal)]
pub struct BatchRenamePreviewRequest {
    pub id: String,
    pub file_paths: Vec<String>,
    pub pattern: String,
    pub source_id: Option<String>,
}

#[derive(Serialize, RustSignal)]
pub struct BatchRenamePreviewResponse {
    pub id: String,
    pub previews: Vec<RenamePreview>,
    pub error: Option<String>,
}

#[derive(Deserialize, DartSignal)]
pub struct BatchRenameApplyRequest {
    pub id: String,
    pub file_paths: Vec<String>,
    pub track_ids: Vec<String>,
    pub pattern: String,
}

#[derive(Serialize, RustSignal)]
pub struct BatchRenameApplyResponse {
    pub id: String,
    pub results: Vec<RenamePreview>,
    pub error: Option<String>,
}

#[derive(Deserialize, DartSignal)]
pub struct CheckNamingConventionRequest {
    pub id: String,
    pub source_id: Option<String>,
    pub pattern: String,
}

#[derive(Serialize, RustSignal)]
pub struct CheckNamingConventionResponse {
    pub id: String,
    pub violations: Vec<NamingViolation>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, SignalPiece)]
pub struct RenamePreview {
    pub file_path: String,
    pub expected_path: String,
    pub track_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, SignalPiece)]
pub struct NamingViolation {
    pub file_path: String,
    pub file_name: String,
    pub expected_name: String,
    pub track_id: String,
}

impl From<tawai_core::signals::tools::RenamePreview> for RenamePreview {
    fn from(r: tawai_core::signals::tools::RenamePreview) -> Self {
        Self {
            file_path: r.file_path,
            expected_path: r.expected_path,
            track_id: r.track_id,
        }
    }
}

impl From<tawai_core::signals::tools::NamingViolation> for NamingViolation {
    fn from(v: tawai_core::signals::tools::NamingViolation) -> Self {
        Self {
            file_path: v.file_path,
            file_name: v.file_name,
            expected_name: v.expected_name,
            track_id: v.track_id,
        }
    }
}

// ---------------------------------------------------------------------------
// Missing Metadata
// ---------------------------------------------------------------------------

#[derive(Deserialize, DartSignal)]
pub struct FindMissingMetadataRequest {
    pub id: String,
    pub check_title: bool,
    pub check_artist: bool,
    pub check_album: bool,
    pub check_genre: bool,
    pub check_year: bool,
    pub check_track_number: bool,
    pub check_cover: bool,
}

#[derive(Serialize, RustSignal)]
pub struct FindMissingMetadataResponse {
    pub id: String,
    pub tracks: Vec<MissingMetadataEntry>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, SignalPiece)]
pub struct MissingMetadataEntry {
    pub track_id: String,
    pub file_path: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub missing_fields: Vec<String>,
}

impl From<tawai_core::signals::tools::MissingMetadataEntry> for MissingMetadataEntry {
    fn from(e: tawai_core::signals::tools::MissingMetadataEntry) -> Self {
        Self {
            track_id: e.track_id,
            file_path: e.file_path,
            title: e.title,
            artist: e.artist,
            album: e.album,
            missing_fields: e.missing_fields,
        }
    }
}

// ---------------------------------------------------------------------------
// Collection Stats
// ---------------------------------------------------------------------------

#[derive(Deserialize, DartSignal)]
pub struct GetLibraryStatsRequest {
    pub id: String,
    pub naming_pattern: Option<String>,
}

#[derive(Serialize, RustSignal)]
pub struct GetLibraryStatsResponse {
    pub id: String,
    pub stats: Option<LibraryStats>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, SignalPiece)]
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

#[derive(Debug, Clone, Serialize, Deserialize, SignalPiece)]
pub struct FormatEntry {
    pub format: String,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, SignalPiece)]
pub struct DecadeEntry {
    pub decade: String,
    pub count: i64,
}

impl From<tawai_core::signals::tools::LibraryStats> for LibraryStats {
    fn from(s: tawai_core::signals::tools::LibraryStats) -> Self {
        Self {
            total_tracks: s.total_tracks,
            total_albums: s.total_albums,
            total_artists: s.total_artists,
            total_duration_secs: s.total_duration_secs,
            average_bitrate: s.average_bitrate,
            most_common_genre: s.most_common_genre,
            genre_count: s.genre_count,
            format_breakdown: s.format_breakdown.into_iter().map(Into::into).collect(),
            decade_distribution: s.decade_distribution.into_iter().map(Into::into).collect(),
            largest_album_title: s.largest_album_title,
            largest_album_tracks: s.largest_album_tracks,
            most_prolific_artist: s.most_prolific_artist,
            most_prolific_artist_tracks: s.most_prolific_artist_tracks,
            naming_conformity_pct: s.naming_conformity_pct,
            total_file_size: s.total_file_size,
            tracks_with_cover: s.tracks_with_cover,
            tracks_without_cover: s.tracks_without_cover,
            tracks_with_lyrics: s.tracks_with_lyrics,
            tracks_without_lyrics: s.tracks_without_lyrics,
            average_track_duration_secs: s.average_track_duration_secs,
            shortest_track_title: s.shortest_track_title,
            shortest_track_duration: s.shortest_track_duration,
            longest_track_title: s.longest_track_title,
            longest_track_duration: s.longest_track_duration,
            tracks_per_album_avg: s.tracks_per_album_avg,
            tracks_per_artist_avg: s.tracks_per_artist_avg,
            tracks_with_mbid: s.tracks_with_mbid,
            oldest_year: s.oldest_year,
            newest_year: s.newest_year,
        }
    }
}

impl From<tawai_core::signals::tools::FormatEntry> for FormatEntry {
    fn from(f: tawai_core::signals::tools::FormatEntry) -> Self {
        Self {
            format: f.format,
            count: f.count,
        }
    }
}

impl From<tawai_core::signals::tools::DecadeEntry> for DecadeEntry {
    fn from(d: tawai_core::signals::tools::DecadeEntry) -> Self {
        Self {
            decade: d.decade,
            count: d.count,
        }
    }
}

// ---------------------------------------------------------------------------
// Romajize Lyrics
// ---------------------------------------------------------------------------

#[derive(Deserialize, DartSignal)]
pub struct RomajizeLyricsRequest {
    pub id: String,
    pub lyrics: String,
    pub synced: bool,
    pub lang: Option<String>,
}

#[derive(Serialize, RustSignal)]
pub struct RomajizeLyricsResponse {
    pub id: String,
    pub romajized: String,
    pub synced: bool,
    pub error: Option<String>,
}

// ---------------------------------------------------------------------------
// Write Track Lyrics
// ---------------------------------------------------------------------------

#[derive(Deserialize, DartSignal)]
pub struct WriteTrackLyricsRequest {
    pub id: String,
    pub track_id: String,
    pub lyrics: String,
    pub synced: bool,
}

#[derive(Serialize, RustSignal)]
pub struct WriteTrackLyricsResponse {
    pub id: String,
    pub success: bool,
    pub error: Option<String>,
}

// ---------------------------------------------------------------------------
// Duplicate Finder
// ---------------------------------------------------------------------------

#[derive(Deserialize, DartSignal)]
pub struct FindDuplicatesRequest {
    pub id: String,
    pub check_fingerprint: bool,
    pub check_mbid: bool,
    pub check_file_size_duration: bool,
    pub check_title_artist: bool,
    pub min_confidence: Option<f64>,
    pub source_id: Option<String>,
}

#[derive(Serialize, RustSignal)]
pub struct FindDuplicatesResponse {
    pub id: String,
    pub groups: Vec<DuplicateGroup>,
    pub total_duplicates: u32,
    pub total_groups: u32,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, SignalPiece)]
pub struct DuplicateGroup {
    pub method: String,
    pub tracks: Vec<DuplicateTrackEntry>,
    pub confidence: f64,
    pub key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, SignalPiece)]
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

impl From<tawai_core::signals::tools::DuplicateGroup> for DuplicateGroup {
    fn from(g: tawai_core::signals::tools::DuplicateGroup) -> Self {
        Self {
            method: g.method,
            tracks: g.tracks.into_iter().map(Into::into).collect(),
            confidence: g.confidence,
            key: g.key,
        }
    }
}

impl From<tawai_core::signals::tools::DuplicateTrackEntry> for DuplicateTrackEntry {
    fn from(e: tawai_core::signals::tools::DuplicateTrackEntry) -> Self {
        Self {
            track_id: e.track_id,
            title: e.title,
            artist: e.artist,
            album: e.album,
            file_path: e.file_path,
            file_size: e.file_size,
            duration_secs: e.duration_secs,
            mbid_recording: e.mbid_recording,
            has_fingerprint: e.has_fingerprint,
        }
    }
}
