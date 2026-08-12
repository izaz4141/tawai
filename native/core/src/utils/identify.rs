use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};

use crate::audio;
use crate::db::account::DEFAULT_USERNAME;
use crate::db::database::DatabasePool;
use crate::db::{library, library_source, user_settings};
use crate::libsources;
use crate::signals::library::TrackInfo;
use crate::tools::rename::move_file_into_source;

/// Parameters for applying a MusicBrainz identification result to a track or
/// a standalone (download-folder) audio file.
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
    pub album: Option<String>,
    pub album_mbid: Option<String>,
    pub album_disambiguation: Option<String>,
    pub release_date: Option<String>,
    pub track_num: Option<i32>,
    pub disc_num: Option<i32>,
    pub mbid_recording: Option<String>,
    pub lyrics: Option<String>,
    pub cover_bytes: Option<Vec<u8>>,
    pub total_discs: Option<i32>,
}

/// Result of applying an identification.
pub struct ApplyIdentificationOutcome {
    /// The new file path after the optional rename/move, if one happened.
    pub new_file_path: Option<String>,
    /// True when the apply targeted a standalone download-folder file. The
    /// caller should trigger an incremental scan of the target source to
    /// register it in the library.
    pub download_folder: bool,
}

/// Apply a MusicBrainz identification to a single audio file.
///
/// This is the single entry point for both flows:
/// - **Download folder** (`target_source_id` set): writes tags to the
///   standalone file at `file_path` and moves it into the target source
///   folder (rooted at the source url). No database row is created; the
///   caller triggers an incremental scan of the target source afterwards.
/// - **Library track** (`track_id` set): writes tags to the track's file,
///   moves it relative to its source root when a naming pattern is set, and
///   updates the library database row.
///
/// The user must have access to the affected source, and the source must be
/// local (non-local sources cannot have their tags rewritten or files moved).
pub async fn apply_identification(
    pool: &DatabasePool,
    user_id: &str,
    role: &str,
    params: &ApplyIdentificationParams,
) -> Result<ApplyIdentificationOutcome> {
    let (source, file_path, track, download_folder) =
        if let Some(source_id) = params.target_source_id.as_deref() {
            let source = library_source::get_source_by_id(pool, source_id)
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
            let source = library_source::get_source_info_by_track_id(pool, &params.track_id)
                .await?
                .ok_or_else(|| anyhow!("Library source for track not found"))?;
            (source, track.file_path.clone(), Some(track), false)
        };

    if !library_source::can_access_source(&source.owner_id, user_id, role, &source.access_rule) {
        return Err(anyhow!("No access to the library source: {}", source.name));
    }
    if source.source_type != "local" {
        return Err(anyhow!(
            "Cannot apply identification to non-local source: {}",
            source.source_type
        ));
    }

    // Start from the file's current on-disk tags. Only the explicitly provided
    // params override them; every omitted field keeps its existing value.
    let mut tag = audio::tags::read_audio_tags(Path::new(&file_path))?.0;

    if let Some(v) = params.title.as_deref() {
        if !v.is_empty() {
            tag.title = v.to_string();
        }
    }
    if let Some(v) = params.artist.as_deref() {
        if !v.is_empty() {
            tag.artist = v.to_string();
            tag.album_artist = v.to_string();
        }
    }
    if let Some(v) = params.album.as_deref() {
        if !v.is_empty() {
            tag.album = v.to_string();
        }
    }
    if let Some(v) = &params.artist_mbid {
        if !v.is_empty() {
            tag.mbid_artist = Some(v.clone());
            tag.mbid_release_artist = Some(v.clone());
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
    if let Some(v) = params.track_num {
        tag.track_number = v;
    }
    if let Some(v) = params.disc_num {
        tag.disc_number = v;
    }
    if let Some(v) = &params.mbid_recording {
        if !v.is_empty() {
            tag.mbid_recording = Some(v.clone());
        }
    }
    if let Some(v) = &params.lyrics {
        if !v.is_empty() {
            tag.lyrics = Some(v.clone());
        }
    }
    if let Some(v) = &params.cover_bytes {
        if !v.is_empty() {
            tag.cover = Some(v.clone());
        }
    }
    if let Some(v) = params.total_discs {
        tag.total_discs = v;
    }

    let file_path = PathBuf::from(&file_path);
    audio::tags::write_audio_tags(&file_path, &tag)?;

    if let Some(cover_bytes) = &params.cover_bytes {
        if let Some(track) = &track {
            if let Err(e) = library::update_album_cover(pool, &track.album_id, cover_bytes).await {
                crate::utils::logger::warn(&format!("update_album_cover failed (non-fatal): {e}"));
            }
        }
    }

    let pattern = user_settings::get_setting(pool, DEFAULT_USERNAME, "identify_naming_pattern")
        .await
        .filter(|s| !s.is_empty());

    let new_file_path = if download_folder {
        Some(
            move_file_into_source(&file_path, &source.url, pattern.as_deref(), &tag)?
                .to_string_lossy()
                .to_string(),
        )
    } else if let Some(pattern) = pattern {
        Some(
            move_file_into_source(&file_path, &source.url, Some(&pattern), &tag)?
                .to_string_lossy()
                .to_string(),
        )
    } else {
        None
    };

    if !download_folder {
        let track_row = track.as_ref().expect("library flow always has a track row");
        let title = params
            .title
            .as_deref()
            .filter(|v| !v.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| track_row.title.clone());
        let artist = params
            .artist
            .as_deref()
            .filter(|v| !v.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| track_row.artists_string.clone());
        let album = params
            .album
            .as_deref()
            .filter(|v| !v.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| track_row.album_title.clone());
        let artist_mbid = params
            .artist_mbid
            .clone()
            .or_else(|| track_row.artist_mbid.clone());
        let album_mbid = params
            .album_mbid
            .clone()
            .or_else(|| track_row.album_mbid.clone());
        let track_num = params.track_num.or(track_row.track_num);
        let disc_num = params.disc_num.or(track_row.disc_num);
        let mbid_recording = params
            .mbid_recording
            .clone()
            .or_else(|| track_row.mbid_recording.clone());
        let lyrics = params.lyrics.clone().or_else(|| track_row.lyrics.clone());

        let artist_pairs = [(artist.clone(), artist_mbid.clone())];
        let album_artist_pairs = [(artist.clone(), artist_mbid.clone())];
        library::update_track(
            pool,
            &params.track_id,
            &title,
            &artist_pairs,
            &album,
            &album_artist_pairs,
            album_mbid.as_deref(),
            params.release_date.clone(),
            track_num,
            disc_num,
            mbid_recording.as_deref(),
            lyrics.as_deref(),
            params.cover_bytes.as_deref(),
            new_file_path.as_deref(),
            params.album_disambiguation.clone(),
            params.total_discs.unwrap_or(0),
        )
        .await?;
    }

    Ok(ApplyIdentificationOutcome {
        new_file_path,
        download_folder,
    })
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
        album_mbid: tag.mbid_release.clone(),
        lyrics: tag.lyrics.clone(),
        release_date: tag.release_date.clone(),
        track_gain: tag.track_gain,
        track_peak: tag.track_peak,
        source: "Download folder".to_string(),
        source_type: "download_folder".to_string(),
        genres: tag.genres.clone(),
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
