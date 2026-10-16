use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::audio::tags::AudioTag;

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
    #[serde(default)]
    pub track_id: Option<String>,
    pub file_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct FingerprintTrackResponse {
    #[serde(default)]
    pub id: String,
    pub track_id: Option<String>,
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

#[derive(Debug, Clone, Default, Serialize, ToSchema)]
pub struct ReadFileTagsResponse {
    #[serde(default)]
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

impl ReadFileTagsResponse {
    /// Build the full display response from a read `AudioTag`.
    ///
    /// `AudioTag` keeps an `i32` sentinel (`0` = absent) for numeric fields;
    /// the response exposes them as `Option<i32>` (`None` when `<= 0`).
    pub fn from_audio_tag(id: String, tag: &AudioTag, duration_secs: f64) -> Self {
        Self {
            id,
            title: tag.title.clone(),
            artist: tag.artist.clone(),
            artists: tag.artists.clone(),
            album: tag.album.clone(),
            album_artist: tag.album_artist.clone(),
            album_artists: tag.album_artists.clone(),
            artist_sort: tag.artist_sort.clone(),
            album_artist_sort: tag.album_artist_sort.clone(),
            genres: tag.genres.clone(),
            track_number: (tag.track_number > 0).then_some(tag.track_number),
            disc_number: (tag.disc_number > 0).then_some(tag.disc_number),
            total_tracks: (tag.total_tracks > 0).then_some(tag.total_tracks),
            total_discs: (tag.total_discs > 0).then_some(tag.total_discs),
            release_date: tag.release_date.clone(),
            original_date: tag.original_date.clone(),
            release_country: tag.release_country.clone(),
            release_type: tag.release_type.clone(),
            record_label: tag.record_label.clone(),
            media: tag.media.clone(),
            script: tag.script.clone(),
            isrc: tag.isrc.clone(),
            mbid_recording: tag.mbid_recording.clone(),
            mbid_track: tag.mbid_track.clone(),
            mbid_artist: tag.mbid_artist.clone(),
            mbid_release_artist: tag.mbid_release_artist.clone(),
            mbid_release: tag.mbid_release.clone(),
            mbid_release_group: tag.mbid_release_group.clone(),
            comment: tag.comment.clone(),
            copyright: tag.copyright.clone(),
            lyrics: tag.lyrics.clone(),
            track_gain: tag.track_gain,
            track_peak: tag.track_peak,
            encoder: tag.encoder.clone(),
            cover: tag.cover.clone(),
            duration_secs,
            error: None,
        }
    }
}

/// Serde default for [`WriteFileTagsRequest::sync_database`]: path-based saves
/// sync the library row unless a caller explicitly opts out.
fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct WriteFileTagsRequest {
    #[serde(default)]
    pub id: String,
    pub path: String,
    /// Update the matching library track row (matched by file path) after a
    /// successful write. Ignored by bytes-based writes. Defaults to `true`.
    #[serde(default = "default_true")]
    pub sync_database: bool,
    pub title: String,
    pub artist: String,
    #[serde(default)]
    pub artists: Vec<String>,
    pub album: String,
    pub album_artist: String,
    #[serde(default)]
    pub album_artists: Vec<String>,
    pub genres: Vec<String>,
    pub track_number: i32,
    pub disc_number: i32,
    #[serde(default)]
    pub total_tracks: i32,
    #[serde(default)]
    pub total_discs: i32,
    pub release_date: Option<String>,
    #[serde(default)]
    pub original_date: Option<String>,
    #[serde(default)]
    pub media: Option<String>,
    #[serde(default)]
    pub script: Option<String>,
    #[serde(default)]
    pub record_label: Option<String>,
    #[serde(default)]
    pub release_country: Option<String>,
    #[serde(default)]
    pub release_type: Vec<String>,
    #[serde(default)]
    pub isrc: Option<String>,
    #[serde(default)]
    pub mbid_recording: Option<String>,
    #[serde(default)]
    pub mbid_track: Option<String>,
    #[serde(default)]
    pub mbid_artist: Option<String>,
    #[serde(default)]
    pub mbid_release: Option<String>,
    #[serde(default)]
    pub mbid_release_artist: Option<String>,
    #[serde(default)]
    pub mbid_release_group: Option<String>,
    #[serde(default)]
    pub artist_sort: String,
    #[serde(default)]
    pub album_artist_sort: String,
    #[serde(default)]
    pub comment: Option<String>,
    #[serde(default)]
    pub copyright: Option<String>,
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
    #[serde(default)]
    pub artists: Vec<String>,
    pub album: String,
    pub album_artist: String,
    #[serde(default)]
    pub album_artists: Vec<String>,
    pub genres: Vec<String>,
    pub track_number: i32,
    pub disc_number: i32,
    #[serde(default)]
    pub total_tracks: i32,
    #[serde(default)]
    pub total_discs: i32,
    pub release_date: Option<String>,
    #[serde(default)]
    pub original_date: Option<String>,
    #[serde(default)]
    pub media: Option<String>,
    #[serde(default)]
    pub script: Option<String>,
    #[serde(default)]
    pub record_label: Option<String>,
    #[serde(default)]
    pub release_country: Option<String>,
    #[serde(default)]
    pub release_type: Vec<String>,
    #[serde(default)]
    pub isrc: Option<String>,
    #[serde(default)]
    pub mbid_recording: Option<String>,
    #[serde(default)]
    pub mbid_track: Option<String>,
    #[serde(default)]
    pub mbid_artist: Option<String>,
    #[serde(default)]
    pub mbid_release: Option<String>,
    #[serde(default)]
    pub mbid_release_artist: Option<String>,
    #[serde(default)]
    pub mbid_release_group: Option<String>,
    #[serde(default)]
    pub artist_sort: String,
    #[serde(default)]
    pub album_artist_sort: String,
    #[serde(default)]
    pub comment: Option<String>,
    #[serde(default)]
    pub copyright: Option<String>,
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
