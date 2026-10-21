use std::path::{Path, PathBuf};
use std::sync::Weak;

use anyhow::{Result, anyhow};
use serde::Serialize;

use crate::app_context::AppContext;
use crate::audio;
use crate::db::database::DatabasePool;
use crate::db::{library, library_source};
use crate::libsources;
use crate::signals::library::{LibrarySourceInfo, TrackInfo};
use crate::tools::rename::{
    DEFAULT_PATTERN, destination_for, format_naming_pattern, move_file_into_source,
};

/// Parameters for applying a MusicBrainz identification result to a track or
/// a standalone (download-folder) audio file.
///
/// Field names mirror the HTTP `ApplyIdentificationRequest` body so a params
/// value can be serialized straight into a remote apply mirror.
#[derive(Debug, Clone, Serialize)]
pub struct ApplyIdentificationParams {
    pub track_id: String,
    /// When set (download-folder flow), the audio file to write tags to and
    /// move into the target library source.
    pub file_path: Option<String>,
    /// When set, the library source to move the (download-folder) file into.
    pub target_source_id: Option<String>,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub artist_mbid: Option<String>,
    /// Album artist (release artist) name/mbid, kept separate from the track
    /// artist. Falls back to `artist`/`artist_mbid` when absent.
    pub album_artist: Option<String>,
    pub album_artist_mbid: Option<String>,
    /// Multi-value artist lists (`TrackArtists` / `AlbumArtists`).
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

/// Result of applying an identification.
pub struct ApplyIdentificationOutcome {
    /// The file path after the rename/move.
    pub new_file_path: Option<String>,
    /// True when the apply targeted a standalone download-folder file. The
    /// caller should trigger an incremental scan of the target source to
    /// register it in the library.
    pub download_folder: bool,
}

/// Where `apply_identification` will move a file, without performing the move.
struct MovePlan {
    source_root: String,
    pattern: String,
    local_path: PathBuf,
}

/// Resolve a naming pattern for a preview or an apply.
///
/// A non-empty `pattern_override` is used as-is. Otherwise the global pattern
/// resolved by [`crate::tools::rename::naming_pattern_from_config`] is used,
/// falling back to [`DEFAULT_PATTERN`] when that is empty too.
fn resolve_preview_pattern<'a>(
    pattern_override: Option<&'a str>,
    global_pattern: &'a str,
) -> &'a str {
    if let Some(pattern) = pattern_override.filter(|p| !p.trim().is_empty()) {
        return pattern;
    }
    if global_pattern.trim().is_empty() {
        DEFAULT_PATTERN
    } else {
        global_pattern
    }
}

/// Resolve the pattern and source root, probing the root before any write.
///
/// `pattern_override` wins when it is non-empty; otherwise the global pattern
/// resolved by [`crate::tools::rename::naming_pattern_from_config`] is used.
///
/// For `tawai` sources the editable copy is the local backup directory, so its
/// local root is preferred over URL probing: preview and apply must never
/// resolve a remote root for a backup path.
async fn resolve_move_plan(
    source: &LibrarySourceInfo,
    local_path: &Path,
    pattern_override: Option<&str>,
    global_pattern: &str,
) -> Result<MovePlan> {
    let pattern = resolve_preview_pattern(pattern_override, global_pattern).to_string();

    let source_root = match tawai_local_root(source) {
        Some(root) => root,
        None => {
            let mut resolver = crate::libsources::SourceUrlResolver::new();
            resolver.resolve(&source.urls, None, None).await?
        }
    };

    Ok(MovePlan {
        source_root,
        pattern,
        local_path: local_path.to_path_buf(),
    })
}

/// The local backup directory of a `tawai` source, when it exists on disk.
fn tawai_local_root(source: &LibrarySourceInfo) -> Option<String> {
    if source.source_type != "tawai" {
        return None;
    }
    crate::libsources::tawai::local_root(&source.urls)
        .filter(|root| Path::new(root).is_dir())
        .map(str::to_string)
}

/// Apply a MusicBrainz identification to a single audio file.
///
/// This is the single entry point for both flows:
/// - **Download folder** (`target_source_id` set): writes tags to the
///   standalone file at `file_path` and moves it into the target source
///   folder (rooted at the source url). No database row is created; the
///   caller triggers an incremental scan of the target source afterwards.
/// - **Library track** (`track_id` set): writes tags to the track's file,
///   moves it relative to its source root, and updates the library database
///   row.
///
/// The user must have access to the affected source, and the source must be
/// local (non-local sources cannot have their tags rewritten or files moved).
/// The destination is built from `global_pattern`, resolved by the caller with
/// [`crate::tools::rename::naming_pattern_from_config`].
///
/// `context` is the live app context handle; the tawai mirror resolves the
/// backing remote track through it when no explicit pattern applies.
pub async fn apply_identification(
    pool: &DatabasePool,
    user_id: &str,
    role: &str,
    params: &ApplyIdentificationParams,
    master_key: &str,
    global_pattern: &str,
    client: &reqwest::Client,
    context: Weak<AppContext>,
) -> Result<ApplyIdentificationOutcome> {
    let (source, file_path, track, download_folder) =
        if let Some(source_id) = params.target_source_id.as_deref() {
            let source = library_source::get_source_by_id(pool, source_id, master_key)
                .await?
                .ok_or_else(|| anyhow!("Target library source not found"))?;
            let file_path = params
                .file_path
                .as_deref()
                .ok_or_else(|| anyhow!("file_path is required for download-folder apply"))?;
            (source, file_path.to_string(), None, true)
        } else {
            let track = library::lookup_track(pool, &params.track_id)
                .await?
                .ok_or_else(|| anyhow!("Track not found"))?;
            let source =
                library_source::get_source_info_by_track_id(pool, &params.track_id, master_key)
                    .await?
                    .ok_or_else(|| anyhow!("Library source for track not found"))?;
            (source, track.file_path.clone(), Some(track), false)
        };

    if !library_source::can_access_source(&source.owner_id, user_id, role, &source.access_rule) {
        return Err(anyhow!("No access to the library source: {}", source.name));
    }
    if !crate::libsources::is_editable(&source.source_type) {
        return Err(anyhow!(
            "Cannot apply identification to non-editable source: {}",
            source.source_type
        ));
    }

    // Tawai tracks live in a local backup of a remote tawai library. Resolve
    // the remote track backing this file *before* any local retag/rename:
    // matching recomputes `dest_for(root, pattern, remote metadata)` and the
    // pre-apply path/tags are the only state that still lines up. The mirror
    // itself is best-effort — any failure below just drops it.
    let remote_backing: Option<(crate::libsources::tawai::RemoteConn, TrackInfo)> =
        if source.source_type == "tawai" && params.target_source_id.is_none() {
            match crate::libsources::tawai::resolve_remote_track(
                pool,
                client,
                &source.urls,
                &file_path,
                Some(global_pattern),
                &context,
            )
            .await
            {
                Ok(pair) => Some(pair),
                Err(e) => {
                    crate::utils::logger::debug(&format!(
                        "tawai apply mirror skipped for '{}': {e}",
                        source.name
                    ));
                    None
                }
            }
        } else {
            None
        };

    let move_plan = resolve_move_plan(&source, Path::new(&file_path), None, global_pattern)
        .await
        .map_err(|e| anyhow!("No reachable source directory for '{}': {}", source.name, e))?;

    // Start from the file's current on-disk tags. Only the explicitly provided
    // params override them; every omitted field keeps its existing value.
    let mut tag = audio::tags::read_audio_tags(Path::new(&file_path))?.0;

    if let Some(v) = params.title.as_deref().filter(|v| !v.is_empty()) {
        tag.title = v.to_string();
    }
    if let Some(v) = params.artist.as_deref().filter(|v| !v.is_empty()) {
        tag.artist = v.to_string();
    }
    // Album artist falls back to the track artist when only the track artist
    // was supplied, which preserves the previous single-artist behaviour.
    if let Some(v) = params.album_artist.as_deref().filter(|v| !v.is_empty()) {
        tag.album_artist = v.to_string();
    } else if let Some(v) = params.artist.as_deref().filter(|v| !v.is_empty()) {
        tag.album_artist = v.to_string();
    }
    if let Some(v) = params.album.as_deref().filter(|v| !v.is_empty()) {
        tag.album = v.to_string();
    }
    if let Some(v) = params.artist_mbid.as_deref().filter(|v| !v.is_empty()) {
        tag.mbid_artist = Some(v.to_string());
    }
    if let Some(v) = params
        .album_artist_mbid
        .as_deref()
        .filter(|v| !v.is_empty())
    {
        tag.mbid_release_artist = Some(v.to_string());
    } else if params.album_artist_mbid.is_none() {
        // Keep the previous behaviour of mirroring the track artist MBID.
        if let Some(v) = params.artist_mbid.as_deref().filter(|v| !v.is_empty()) {
            tag.mbid_release_artist = Some(v.to_string());
        }
    }
    if let Some(v) = &params.album_mbid {
        if !v.is_empty() {
            tag.mbid_release = Some(v.clone());
        }
    }
    if let Some(v) = &params.album_disambiguation {
        if !v.is_empty() {
            tag.album_disambiguation = Some(v.clone());
        }
    }
    if let Some(v) = &params.release_date {
        if !v.is_empty() {
            tag.release_date = Some(v.clone());
        }
    }
    if let Some(v) = &params.original_date {
        if !v.is_empty() {
            tag.original_date = Some(v.clone());
        }
    }
    if let Some(v) = &params.release_group_mbid {
        if !v.is_empty() {
            tag.mbid_release_group = Some(v.clone());
        }
    }
    if let Some(v) = &params.release_country {
        if !v.is_empty() {
            tag.release_country = Some(v.clone());
        }
    }
    if let Some(v) = &params.release_type {
        if !v.is_empty() {
            tag.release_type = v.clone();
        }
    }
    if let Some(v) = &params.record_label {
        if !v.is_empty() {
            tag.record_label = Some(v.clone());
        }
    }
    if let Some(v) = &params.media {
        if !v.is_empty() {
            tag.media = Some(v.clone());
        }
    }
    if let Some(v) = &params.artist_sort {
        if !v.is_empty() {
            tag.artist_sort = v.clone();
        }
    }
    if let Some(v) = &params.album_artist_sort {
        if !v.is_empty() {
            tag.album_artist_sort = v.clone();
        }
    }
    if let Some(v) = params.track_num {
        if v > 0 {
            tag.track_number = v;
        }
    }
    if let Some(v) = params.disc_num {
        if v > 0 {
            tag.disc_number = v;
        }
    }
    if let Some(v) = params.total_tracks {
        if v > 0 {
            tag.total_tracks = v;
        }
    }
    if let Some(v) = params.total_discs {
        if v > 0 {
            tag.total_discs = v;
        }
    }
    if let Some(v) = &params.mbid_recording {
        if !v.is_empty() {
            tag.mbid_recording = Some(v.clone());
        }
    }
    if let Some(v) = &params.mbid_track {
        if !v.is_empty() {
            tag.mbid_track = Some(v.clone());
        }
    }
    if let Some(v) = &params.isrc {
        if !v.is_empty() {
            tag.isrc = Some(v.clone());
        }
    }
    if let Some(v) = &params.comment {
        if !v.is_empty() {
            tag.comment = Some(v.clone());
        }
    }
    if let Some(v) = &params.copyright {
        if !v.is_empty() {
            tag.copyright = Some(v.clone());
        }
    }
    if let Some(v) = &params.script {
        if !v.is_empty() {
            tag.script = Some(v.clone());
        }
    }
    if let Some(v) = &params.lyrics {
        if !v.is_empty() {
            tag.lyrics = Some(v.clone());
        }
    }
    let mut explicit_artists = false;
    if let Some(v) = &params.artists {
        if !v.is_empty() {
            tag.artists = v.clone();
            explicit_artists = true;
        } else {
            tag.artists.clear();
        }
    } else {
        tag.artists.clear();
    }
    let mut explicit_album_artists = false;
    if let Some(v) = &params.album_artists {
        if !v.is_empty() {
            tag.album_artists = v.clone();
            explicit_album_artists = true;
        } else {
            tag.album_artists.clear();
        }
    } else {
        tag.album_artists.clear();
    }
    if let Some(v) = &params.genres {
        if !v.is_empty() {
            tag.genres = v.clone();
        }
    }
    if let Some(v) = &params.cover_bytes {
        if !v.is_empty() {
            tag.cover = Some(v.clone());
        }
    }

    let file_path = PathBuf::from(&file_path);
    // The identify flow patches the on-disk tags but may carry explicit
    // artist/album-artist lists from the comparison sheet.
    audio::tags::write_audio_tags(
        &file_path,
        &tag,
        audio::tags::TagWritePolicy::patch_explicit_artists(),
    )?;

    if let Some(cover_bytes) = &params.cover_bytes {
        if let Some(track) = &track {
            if let Err(e) = library::update_album_cover(pool, &track.album_id, cover_bytes).await {
                crate::utils::logger::warn(&format!("update_album_cover failed (non-fatal): {e}"));
            }
        }
    }

    let new_file_path = Some(
        move_file_into_source(
            &move_plan.local_path,
            &move_plan.source_root,
            Some(&move_plan.pattern),
            &tag,
        )?
        .to_string_lossy()
        .to_string(),
    );

    if !download_folder {
        let track_row = track.as_ref().expect("library flow always has a track row");

        // Prefer what was written to disk (`tag`, already merged with the
        // provided params) and fall back to the existing DB row so a field the
        // file had nothing for is never blanked.
        let title = if tag.title.is_empty() {
            track_row.title.clone()
        } else {
            tag.title.clone()
        };
        let artist = if tag.artist.is_empty() {
            track_row.artists_string.clone()
        } else {
            tag.artist.clone()
        };
        let album = if tag.album.is_empty() {
            track_row.album_title.clone()
        } else {
            tag.album.clone()
        };
        let album_artist = if tag.album_artist.is_empty() {
            artist.clone()
        } else {
            tag.album_artist.clone()
        };

        // Artist / album-artist links, primary entry carrying the MBID and sort.
        let artist_names = if tag.artists.is_empty() {
            vec![artist.clone()]
        } else {
            tag.artists.clone()
        };
        let artists: Vec<(String, Option<String>, Option<String>)> = artist_names
            .iter()
            .enumerate()
            .map(|(i, name)| {
                let mbid = (i == 0 || artist_names.len() == 1)
                    .then(|| tag.mbid_artist.clone())
                    .flatten();
                let sort = (i == 0 && !tag.artist_sort.is_empty()).then(|| tag.artist_sort.clone());
                (name.clone(), mbid, sort)
            })
            .collect();

        let album_artist_names = if tag.album_artists.is_empty() {
            vec![album_artist.clone()]
        } else {
            tag.album_artists.clone()
        };
        let album_artists: Vec<(String, Option<String>, Option<String>)> = album_artist_names
            .iter()
            .enumerate()
            .map(|(i, name)| {
                let mbid = (i == 0 || album_artist_names.len() == 1)
                    .then(|| tag.mbid_release_artist.clone())
                    .flatten();
                let sort = (i == 0 && !tag.album_artist_sort.is_empty())
                    .then(|| tag.album_artist_sort.clone());
                (name.clone(), mbid, sort)
            })
            .collect();

        let album_meta = library::AlbumWriteMeta {
            date: tag.release_date.clone(),
            release_mbid: tag.mbid_release.clone(),
            release_group_mbid: tag.mbid_release_group.clone(),
            release_artist_mbid: tag.mbid_release_artist.clone(),
            release_country: tag.release_country.clone(),
            release_type: (!tag.release_type.is_empty()).then(|| tag.release_type.join(";")),
            record_label: tag.record_label.clone(),
            media: tag.media.clone(),
            original_date: tag.original_date.clone(),
            disambiguation: tag.album_disambiguation.clone(),
            total_tracks: tag.total_tracks,
            total_discs: tag.total_discs,
        };
        let track_meta = library::TrackTagMeta {
            mbid_recording: tag.mbid_recording.clone(),
            mbid_track: tag.mbid_track.clone(),
            isrc: tag.isrc.clone(),
            comment: tag.comment.clone(),
            copyright: tag.copyright.clone(),
            // Encoder is read-only; apply never writes it.
            encoder: None,
            script: tag.script.clone(),
        };

        let track_num = (tag.track_number > 0)
            .then_some(tag.track_number)
            .or(track_row.track_num);
        let disc_num = (tag.disc_number > 0)
            .then_some(tag.disc_number)
            .or(track_row.disc_num);
        let lyrics = tag.lyrics.clone().or_else(|| track_row.lyrics.clone());

        library::update_track(
            pool,
            &params.track_id,
            &title,
            &artists,
            &album,
            &album_artists,
            &album_meta,
            &track_meta,
            track_num,
            disc_num,
            lyrics.as_deref(),
            params.cover_bytes.as_deref(),
            new_file_path.as_deref(),
            params
                .genres
                .as_deref()
                .filter(|g| !g.is_empty())
                .map(|_| tag.genres.as_slice()),
        )
        .await?;
    }

    if let Some((conn, rt)) = &remote_backing {
        mirror_apply_to_remote(client, conn, params, &rt.id).await;
    }

    Ok(ApplyIdentificationOutcome {
        new_file_path,
        download_folder,
    })
}

/// Best-effort mirror of an identification apply to the remote tawai server
/// backing a local backup track, analogous to delete's remote mirror. The
/// remote re-runs its own apply with its own configured pattern (no pattern
/// crosses the wire); a failure only logs a warning, never failing the
/// already-successful local apply.
async fn mirror_apply_to_remote(
    client: &reqwest::Client,
    conn: &crate::libsources::tawai::RemoteConn,
    params: &ApplyIdentificationParams,
    remote_track_id: &str,
) {
    let mut body = match serde_json::to_value(params) {
        Ok(body) => body,
        Err(e) => {
            crate::utils::logger::warn(&format!("tawai remote apply mirror failed: {e}"));
            return;
        }
    };
    // `id` is required by the remote's request body (it only echoes it);
    // track flow addresses the remote library track by its own row id.
    body["id"] = serde_json::Value::String(String::new());
    body["track_id"] = serde_json::Value::String(remote_track_id.to_string());
    body["file_path"] = serde_json::Value::Null;
    body["target_source_id"] = serde_json::Value::Null;

    let url = format!("{}/api/tawai/library/identify/apply", conn.http_base);
    match client
        .post(&url)
        .header("X-API-Key", &conn.api_key)
        .json(&body)
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => {}
        Ok(resp) => crate::utils::logger::warn(&format!(
            "tawai remote apply mirror failed: {}",
            resp.status()
        )),
        Err(e) => crate::utils::logger::warn(&format!("tawai remote apply mirror failed: {e}")),
    }
}

/// Naming preview entry point shared by the hub and HTTP handlers.
///
/// Pattern resolution, for both cases: a non-empty `pattern_override` is used
/// as-is, otherwise the global naming pattern, falling back to
/// `DEFAULT_PATTERN` when that is missing/empty.
///
/// With no `file_path` (naming-format help dialog, download previews that join
/// their own root) this renders the resolved pattern relative to nothing,
/// without an extension. With a file it computes the destination
/// [`apply_identification`] would produce.
///
/// Resolution ladder for the file case, matching what Apply would do:
/// - Unresolved role (unknown user), a file without a source context
///   (download-folder row before a target source is picked), or a source the
///   caller cannot access: the current file path, since nothing would move.
/// - Unresolvable, non-editable, or unreachable source: the current path.
/// - Otherwise the full destination path.
pub async fn preview_naming(
    pool: &DatabasePool,
    master_key: &str,
    user_id: &str,
    role: Option<&str>,
    track_id: Option<&str>,
    target_source_id: Option<&str>,
    file_path: Option<&str>,
    pattern_override: Option<&str>,
    global_pattern: &str,
    tag: &audio::tags::AudioTag,
) -> String {
    let Some(file_path) = file_path else {
        return format_naming_pattern(
            resolve_preview_pattern(pattern_override, global_pattern),
            tag,
        );
    };
    // Without a resolved role the source can't be authorized; don't resolve or
    // disclose its root.
    let Some(role) = role else {
        return file_path.to_string();
    };

    let (source, local_path) = if let Some(source_id) = target_source_id {
        match library_source::get_source_by_id(pool, source_id, master_key).await {
            Ok(Some(source)) => (source, file_path.to_string()),
            _ => return file_path.to_string(),
        }
    } else if let Some(track_id) = track_id {
        let track = match library::lookup_track(pool, track_id).await {
            Ok(Some(track)) => track,
            _ => return file_path.to_string(),
        };
        match library_source::get_source_info_by_track_id(pool, track_id, master_key).await {
            Ok(Some(source)) => (source, track.file_path),
            _ => return track.file_path,
        }
    } else {
        // A standalone file with no target source chosen yet: nothing moves.
        return file_path.to_string();
    };

    if !library_source::can_access_source(&source.owner_id, user_id, role, &source.access_rule) {
        return file_path.to_string();
    }

    if !libsources::is_editable(&source.source_type) {
        return local_path;
    }

    match resolve_move_plan(
        &source,
        Path::new(&local_path),
        pattern_override,
        global_pattern,
    )
    .await
    {
        Ok(plan) => destination_for(
            &plan.source_root,
            Some(&plan.pattern),
            &plan.local_path,
            tag,
        )
        .to_string_lossy()
        .to_string(),
        Err(_) => local_path,
    }
}

/// Build a `TrackInfo` from a scanned audio file. Used when listing download
/// folder contents, where tracks have no database row yet.
pub fn parsed_track_to_info(track: libsources::ParsedTrack) -> TrackInfo {
    let tag = &track.tag;
    let path = Path::new(&track.file_path);
    let parent_dir = path
        .parent()
        .and_then(|p| p.to_str())
        .unwrap_or("")
        .to_string();
    let folder_name = path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("Unknown Album")
        .to_string();

    let mut title = tag.title.clone();
    let mut artist = tag.artist.clone();
    if title.is_empty() {
        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
            let (parsed_artist, parsed_title, _) = audio::tags::parse_filename_tags(stem);
            if title.is_empty() {
                title = parsed_title.unwrap_or_else(|| stem.to_string());
            }
            if artist.is_empty() {
                artist = parsed_artist.unwrap_or_default();
            }
        }
    }

    let album_title = if tag.album.is_empty() {
        folder_name
    } else {
        tag.album.clone()
    };
    let album_id = tag.mbid_release.clone().unwrap_or_else(|| parent_dir);

    TrackInfo {
        id: track.file_path.clone(),
        title,
        album_id,
        album_title,
        artists_string: artist.clone(),
        artists: vec![crate::signals::library::ArtistInfo {
            id: artist.clone(),
            name: artist.clone(),
            sort_name: None,
            mbid: tag.mbid_artist.clone(),
            thumbnail_url: None,
            album_count: 0,
            track_count: 0,
        }],
        track_num: (tag.track_number > 0).then_some(tag.track_number),
        disc_num: (tag.disc_number > 0).then_some(tag.disc_number),
        duration_secs: (track.duration_secs > 0.0).then_some(track.duration_secs),
        file_path: track.file_path.clone(),
        file_size: (track.file_size > 0).then_some(track.file_size as i64),
        bitrate: track.bitrate.map(|b| b as i32),
        mbid_recording: tag.mbid_recording.clone(),
        artist_mbid: tag.mbid_artist.clone(),
        release_mbid: tag.mbid_release.clone(),
        lyrics: tag.lyrics.clone(),
        release_date: tag.release_date.clone(),
        track_gain: tag.track_gain,
        track_peak: tag.track_peak,
        source: "Download folder".to_string(),
        source_type: "download_folder".to_string(),
        genres: tag.genres.clone(),
        file_hash: track.file_hash.clone(),
        sample_rate: track.sample_rate.map(|s| s as i32),
        acoust_id_fingerprint: tag.acoust_id_fingerprint.clone(),
        acoust_id: tag.acoust_id.clone(),
        mbid_track: tag.mbid_track.clone(),
        isrc: tag.isrc.clone(),
        comment: tag.comment.clone(),
        copyright: tag.copyright.clone(),
        encoder: tag.encoder.clone(),
        script: tag.script.clone(),
    }
}

/// Recursively walk `dir`, read only the external tags of every audio file,
/// and return them as `TrackInfo` entries. No hashing or fingerprinting is
/// performed here — that happens during the incremental scan after apply.
/// Files that fail to read tags are skipped with a warning.
pub fn list_download_folder_tracks(dir: &Path) -> Result<Vec<TrackInfo>> {
    let mut tracks = Vec::new();
    for file_path in libsources::local::walk_directory(dir) {
        match audio::tags::read_audio_tags(&file_path) {
            Ok((tag, duration_secs, sample_rate, bitrate)) => {
                let file_size = std::fs::metadata(&file_path).map(|m| m.len()).unwrap_or(0);
                let parsed = libsources::ParsedTrack {
                    tag,
                    file_path: file_path.to_string_lossy().to_string(),
                    file_hash: None,
                    duration_secs,
                    sample_rate,
                    bitrate,
                    file_size,
                };
                tracks.push(parsed_track_to_info(parsed));
            }
            Err(e) => {
                crate::utils::logger::warn(&format!(
                    "Failed to read tags of download folder file {}: {}",
                    file_path.display(),
                    e
                ));
            }
        }
    }
    Ok(tracks)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_pattern_prefers_non_empty_override() {
        assert_eq!(
            resolve_preview_pattern(Some("{artist}/{title}"), "{album}/{title}"),
            "{artist}/{title}"
        );
    }

    #[test]
    fn preview_pattern_uses_override_as_is() {
        assert_eq!(
            resolve_preview_pattern(Some(" {artist}/{title} "), "{album}/{title}"),
            " {artist}/{title} "
        );
    }

    #[test]
    fn preview_pattern_missing_override_uses_global() {
        assert_eq!(
            resolve_preview_pattern(None, "{album}/{title}"),
            "{album}/{title}"
        );
    }

    #[test]
    fn preview_pattern_empty_override_uses_global() {
        assert_eq!(
            resolve_preview_pattern(Some(""), "{album}/{title}"),
            "{album}/{title}"
        );
    }

    #[test]
    fn preview_pattern_whitespace_override_uses_global() {
        assert_eq!(
            resolve_preview_pattern(Some("  \t\n"), "{album}/{title}"),
            "{album}/{title}"
        );
    }

    #[test]
    fn preview_pattern_empty_global_falls_back_to_default() {
        assert_eq!(resolve_preview_pattern(None, ""), DEFAULT_PATTERN);
        assert_eq!(resolve_preview_pattern(Some("   "), ""), DEFAULT_PATTERN);
    }
}
