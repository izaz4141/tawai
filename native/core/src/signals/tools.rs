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
    /// An explicit naming-pattern override. When absent the server resolves
    /// the global pattern from the live config, so the naming-conformity
    /// figure is computed against the admin-configured pattern instead of
    /// being silently skipped.
    ///
    /// This is a read-only route that never mutates anything, so an override
    /// is safe here.
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
    #[serde(default)]
    pub track_id: String,
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
    /// The caller of the request. The server ignores this field and uses the
    /// authenticated user instead.
    #[serde(default)]
    pub user_id: String,
    /// An explicit naming-pattern override. When absent the server resolves
    /// the global pattern from the live config.
    ///
    /// This is a dry-run route, so an override is safe here: the mutating
    /// apply takes no pattern at all.
    #[serde(default)]
    pub pattern: Option<String>,
    pub source_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct BatchRenamePreviewResponse {
    #[serde(default)]
    pub id: String,
    pub previews: Vec<RenamePreview>,
    /// The naming pattern the server actually used.
    ///
    /// This is how the client learns the effective pattern: a client may not
    /// be allowed to read the global settings (`naming_pattern` is
    /// deliberately absent from the non-admin allowlist), so this response is
    /// the only source of truth for it. The renamer page shows it read-only,
    /// and the rename apply uses the very same pattern — what it previews is
    /// what gets renamed.
    pub pattern: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct BatchRenameApplyRequest {
    #[serde(default)]
    pub id: String,
    pub file_paths: Vec<String>,
    #[serde(default)]
    pub track_ids: Vec<String>,
    /// The caller of the request. The server ignores this field and uses the
    /// authenticated user instead.
    #[serde(default)]
    pub user_id: String,
    // No pattern: the apply always renames with the global naming pattern
    // from the live config, so a client snapshot can never drive a mutation.
    //
    // The client learns the effective pattern from
    // [`BatchRenamePreviewResponse::pattern`] and previews with it, so what
    // it previews is what gets renamed. A pattern sent here by an older
    // client is ignored: the field does not exist.
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
    /// The caller of the request. The server ignores this field and uses the
    /// authenticated user instead.
    #[serde(default)]
    pub user_id: String,
    /// An explicit naming-pattern override. When absent the server resolves
    /// the global pattern from the live config.
    ///
    /// This is a read-only route that never mutates anything, so an override
    /// is safe here — unlike the rename apply, which takes no pattern.
    #[serde(default)]
    pub pattern: Option<String>,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn batch_rename_apply_request_ignores_a_client_pattern() {
        // An older client still sends its cached pattern on the apply. The
        // field does not exist on the request, so serde drops it: a client
        // snapshot can never drive a mutation.
        let req: BatchRenameApplyRequest = serde_json::from_str(
            r#"{"id":"1","file_paths":["/music/a.mp3"],"track_ids":["t1"],"user_id":"u","pattern":"{artist}/{title}"}"#,
        )
        .expect("an apply request carrying a stale pattern still deserializes");
        assert_eq!(req.file_paths, vec!["/music/a.mp3".to_string()]);
        assert_eq!(req.track_ids, vec!["t1".to_string()]);
    }

    #[test]
    fn batch_rename_preview_request_pattern_is_optional() {
        let without: BatchRenamePreviewRequest =
            serde_json::from_str(r#"{"id":"1","file_paths":[]}"#).expect("no pattern");
        assert!(without.pattern.is_none());

        let with: BatchRenamePreviewRequest =
            serde_json::from_str(r#"{"id":"1","file_paths":[],"pattern":"{artist}/{title}"}"#)
                .expect("an explicit override");
        assert_eq!(with.pattern.as_deref(), Some("{artist}/{title}"));
    }

    #[test]
    fn check_naming_convention_request_pattern_is_optional() {
        let without: CheckNamingConventionRequest =
            serde_json::from_str(r#"{"id":"1"}"#).expect("no pattern");
        assert!(without.pattern.is_none());

        let with: CheckNamingConventionRequest =
            serde_json::from_str(r#"{"id":"1","pattern":"{artist}/{title}"}"#)
                .expect("an explicit override");
        assert_eq!(with.pattern.as_deref(), Some("{artist}/{title}"));
    }

    #[test]
    fn get_library_stats_request_pattern_is_optional() {
        let without: GetLibraryStatsRequest =
            serde_json::from_str(r#"{"id":"1"}"#).expect("no pattern");
        assert!(without.naming_pattern.is_none());

        let with: GetLibraryStatsRequest =
            serde_json::from_str(r#"{"id":"1","naming_pattern":"{artist}/{title}"}"#)
                .expect("an explicit override");
        assert_eq!(with.naming_pattern.as_deref(), Some("{artist}/{title}"));
    }

    #[test]
    fn batch_rename_preview_response_carries_the_effective_pattern() {
        // The client cannot read the global settings, so the response is the
        // only way it learns which pattern is in use. It must not be dropped.
        let response = BatchRenamePreviewResponse {
            id: "1".to_string(),
            previews: vec![],
            pattern: "{artist}/{title}".to_string(),
            error: None,
        };
        let value = serde_json::to_value(&response).expect("the response serializes");
        assert_eq!(value["pattern"], serde_json::json!("{artist}/{title}"));
    }
}
