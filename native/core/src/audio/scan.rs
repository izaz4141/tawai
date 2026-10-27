use std::collections::{HashMap, HashSet};
use std::io::Cursor;
use std::sync::Weak;

use image::GenericImageView;

use crate::app_context::AppContext;
use crate::audio::ffmpeg;
use crate::audio::tags::derive_sort_name;
use crate::db::database::DatabasePool;
use crate::db::library;
use crate::db::library::{AlbumWriteMeta, TrackTagMeta};
use crate::libsources;
use crate::signals::library::{LibrarySourceInfo, ScanProgress, ScanResult};
use crate::utils::logger;

fn send_progress(tx: &Option<tokio::sync::watch::Sender<ScanProgress>>, p: ScanProgress) {
    if let Some(tx) = tx {
        let _ = tx.send(p);
    }
}

pub(crate) type LogSender = tokio::sync::mpsc::UnboundedSender<(String, String)>;

/// Forward a (level, message) pair to the live log channel, or log locally
/// when absent. Levels map to Dart error/warning.
pub(crate) fn stream_log(log_tx: &Option<LogSender>, level: &str, msg: String) {
    if let Some(tx) = log_tx {
        let _ = tx.send((level.to_string(), msg));
        return;
    }
    match level {
        "ERROR" => logger::error(&msg),
        "WARN" => logger::warn(&msg),
        "INFO" => logger::info(&msg),
        _ => logger::debug(&msg),
    }
}

enum InsertOutcome {
    Inserted { track_id: String, album_id: String },
    Duplicate,
    Failed,
}

async fn insert_track_to_db(
    pool: &DatabasePool,
    track: &libsources::ParsedTrack,
    track_id: Option<&str>,
    source_id: &str,
    force: bool,
    stale_fp_to_track: &mut HashMap<String, String>,
    total_new: &mut u32,
    total_duplicates: &mut u32,
    log_tx: &Option<LogSender>,
) -> InsertOutcome {
    // Fingerprint-based duplicate detection within the new set
    if !force {
        if let Some(fp) = track.acoust_id_fingerprint.as_deref() {
            match library::track_exists_by_fingerprint(pool, fp).await {
                Ok(Some(_)) => {
                    if let Some(stale_id) = stale_fp_to_track.remove(fp) {
                        // The existing copy is being removed this scan (its file is gone),
                        // so adopt this file as the kept copy. remove() ensures only ONE
                        // replacement happens — later duplicates hit the branch below.
                        if let Err(e) = library::delete_track_by_id(pool, &stale_id).await {
                            stream_log(
                                log_tx,
                                "ERROR",
                                format!(
                                    "Failed to remove stale track '{}' before keep-one insert: {}",
                                    stale_id, e
                                ),
                            );
                        }
                    } else {
                        *total_duplicates += 1;
                        return InsertOutcome::Duplicate;
                    }
                }
                Err(e) => {
                    stream_log(
                        log_tx,
                        "ERROR",
                        format!(
                            "track_exists_by_fingerprint failed for {}: {}",
                            track.file_path, e
                        ),
                    );
                }
                _ => {}
            }
        }
    }

    // Insert each individual album artist (MBID only for the primary/combined)
    let mut album_artist_ids: Vec<String> = Vec::new();
    for (i, name) in track.album_artists.iter().enumerate() {
        let sort_name = derive_sort_name(name);
        let mbid = if i == 0 || track.album_artists.len() == 1 {
            track.mbid_release_artist.clone()
        } else {
            None
        };
        match library::insert_artist(pool, name, &sort_name, mbid).await {
            Ok(id) => album_artist_ids.push(id),
            Err(e) => {
                stream_log(
                    log_tx,
                    "ERROR",
                    format!("Failed to insert album artist '{}': {}", name, e),
                );
                return InsertOutcome::Failed;
            }
        }
    }
    // Insert each individual track artist (MBID only for the primary/combined)
    let mut track_artist_ids: Vec<String> = Vec::new();
    for (i, name) in track.artists.iter().enumerate() {
        let sort_name = derive_sort_name(name);
        let mbid = if i == 0 || track.artists.len() == 1 {
            track.mbid_artist.clone()
        } else {
            None
        };
        match library::insert_artist(pool, name, &sort_name, mbid).await {
            Ok(id) => track_artist_ids.push(id),
            Err(e) => {
                stream_log(
                    log_tx,
                    "ERROR",
                    format!("Failed to insert track artist '{}': {}", name, e),
                );
                return InsertOutcome::Failed;
            }
        }
    }
    let album_meta = AlbumWriteMeta {
        date: track.release_date.clone(),
        release_mbid: track.mbid_release.clone(),
        release_group_mbid: track.mbid_release_group.clone(),
        release_artist_mbid: track.mbid_release_artist.clone(),
        release_country: track.release_country.clone(),
        release_type: (!track.release_type.is_empty()).then(|| track.release_type.join(";")),
        record_label: track.record_label.clone(),
        media: track.media.clone(),
        original_date: track.original_date.clone(),
        disambiguation: track.album_disambiguation.clone(),
        total_tracks: track.total_tracks,
        total_discs: track.total_discs,
    };
    let album_id =
        match library::insert_album(pool, &track.album, &album_artist_ids[0], None, &album_meta)
            .await
        {
            Ok(id) => {
                let _ = library::insert_album_artists(pool, &id, &album_artist_ids).await;
                id
            }
            Err(e) => {
                stream_log(
                    log_tx,
                    "ERROR",
                    format!("Failed to insert album '{}': {}", track.album, e),
                );
                return InsertOutcome::Failed;
            }
        };

    let track_id = match library::insert_track(
        pool,
        track_id,
        &track.title,
        &album_id,
        &track_artist_ids[0],
        track.disc_number,
        track.track_number,
        track.duration_secs,
        &track.file_path,
        track.file_hash.as_deref(),
        source_id,
        Some(track.file_size as i64),
        track.bitrate.map(|b| b as i32),
        track.sample_rate.map(|s| s as i32),
        track.cover.as_deref(),
        track.lyrics.as_deref(),
        track.track_gain,
        track.track_peak,
        &TrackTagMeta {
            mbid_recording: track.mbid_recording.clone(),
            mbid_track: track.mbid_track.clone(),
            isrc: track.isrc.clone(),
            comment: track.comment.clone(),
            copyright: track.copyright.clone(),
            encoder: track.encoder.clone(),
            script: track.script.clone(),
        },
    )
    .await
    {
        Ok(id) => {
            let _ = library::insert_track_artists(pool, &id, &track_artist_ids).await;
            for genre_name in &track.genres {
                if let Ok(genre_id) = library::insert_genre(pool, genre_name).await {
                    let _ = library::insert_track_genre(pool, &id, &genre_id).await;
                }
            }
            *total_new += 1;
            id
        }
        Err(e) => {
            stream_log(
                log_tx,
                "ERROR",
                format!("Failed to insert track '{}': {}", track.title, e),
            );
            return InsertOutcome::Failed;
        }
    };

    if let Some(fp) = track.acoust_id_fingerprint.as_deref() {
        if let Err(e) =
            library::insert_fingerprint(pool, &track_id, fp, track.acoust_id.as_deref()).await
        {
            stream_log(
                log_tx,
                "ERROR",
                format!("Failed to insert fingerprint for '{}': {}", track.title, e),
            );
        }
    }

    InsertOutcome::Inserted { track_id, album_id }
}

/// Scan every source and reconcile the library with what is on disk.
///
/// `context` is the live app context handle: parsers that derive paths from
/// the naming pattern (tawai) read it from the config at every use, so a
/// pattern change mid-scan is picked up by the next phase.
pub async fn run_scan(
    pool: &DatabasePool,
    client: reqwest::Client,
    sources: &[LibrarySourceInfo],
    force: bool,
    progress: Option<tokio::sync::watch::Sender<ScanProgress>>,
    master_key: &str,
    log_tx: Option<LogSender>,
    context: Weak<AppContext>,
) -> ScanResult {
    let sources: Vec<LibrarySourceInfo> = sources
        .iter()
        .filter(|s| !s.source_type.starts_with("recommendation:"))
        .cloned()
        .collect();
    if force {
        if let Err(e) = library::delete_all_library(pool).await {
            stream_log(
                &log_tx,
                "ERROR",
                format!("delete_all_library failed: {}", e),
            );
            let result = ScanResult {
                success: false,
                tracks_found: 0,
                new_tracks: 0,
                duplicates: 0,
                deleted: 0,
                error: Some(format!("Failed to clear library: {}", e)),
            };
            send_progress(
                &progress,
                ScanProgress {
                    stage: "error".to_string(),
                    complete: true,
                    tracks_found: result.tracks_found,
                    new_tracks: result.new_tracks,
                    duplicates: result.duplicates,
                    deleted: result.deleted,
                    error: result.error.clone(),
                    ..Default::default()
                },
            );
            return result;
        }
    }

    // -----------------------------------------------------------------------
    // Phase 1: Enumerate all file paths from all sources (cheap — no tag parsing)
    // -----------------------------------------------------------------------
    let mut filesystem_paths: HashSet<String> = HashSet::new();
    let mut source_entries: HashMap<String, Vec<libsources::EnumeratedTrack>> = HashMap::new();

    for source in &sources {
        send_progress(
            &progress,
            ScanProgress {
                current_file: format!("Scanning {} ({})...", source.name, source.source_type),
                files_scanned: 0,
                total_files: 0,
                stage: "enumerating".to_string(),
                current_source: source.name.clone(),
                ..Default::default()
            },
        );

        let parser =
            match libsources::get_parser(&source.source_type, client.clone(), context.clone()) {
                Some(p) => p,
                None => {
                    stream_log(
                        &log_tx,
                        "ERROR",
                        format!("Unknown source type: {}", source.source_type),
                    );
                    continue;
                }
            };

        let mut resolver = libsources::SourceUrlResolver::new();
        let url = match resolver.resolve(&source.urls, Some(&client), None).await {
            Ok(u) => u,
            Err(e) => {
                stream_log(
                    &log_tx,
                    "ERROR",
                    format!(
                        "No reachable URL for source '{}' ({:?}): {}",
                        source.name, source.urls, e
                    ),
                );
                continue;
            }
        };

        let entries = match parser
            .enumerate_tracks(pool, &url, &source.urls, master_key)
            .await
        {
            Ok(p) => p,
            Err(e) => {
                stream_log(
                    &log_tx,
                    "ERROR",
                    format!(
                        "Failed to enumerate source '{}' ({}): {}",
                        source.name, url, e
                    ),
                );
                continue;
            }
        };

        for entry in &entries {
            filesystem_paths.insert(entry.path.clone());
        }
        source_entries.insert(source.id.clone(), entries);
    }

    let total_found = filesystem_paths.len() as u32;
    logger::info(&format!("Phase 1 complete: {} files found", total_found));

    // -----------------------------------------------------------------------
    // Phase 2: Load track rows grouped by owning source (single query)
    // -----------------------------------------------------------------------
    send_progress(
        &progress,
        ScanProgress {
            current_file: String::new(),
            files_scanned: 0,
            total_files: total_found,
            stage: "comparing".to_string(),
            current_source: "All".to_string(),
            ..Default::default()
        },
    );

    // Detection is per source, so a source's stale paths can never include
    // another source's live rows. Ids are collected globally as well so a
    // source can detect that a remote id already belongs to someone else.
    let source_keys = match library::all_track_source_keys(pool).await {
        Ok(keys) => keys,
        Err(e) => {
            stream_log(
                &log_tx,
                "ERROR",
                format!("Failed to list track source keys: {}", e),
            );
            Vec::new()
        }
    };
    let mut own_rows: HashMap<String, Vec<(String, String)>> = HashMap::new();
    let mut all_db_ids: HashSet<String> = HashSet::new();
    for (source_id, track_id, file_path) in source_keys {
        all_db_ids.insert(track_id.clone());
        own_rows
            .entry(source_id)
            .or_default()
            .push((track_id, file_path));
    }
    let db_row_count: usize = own_rows.values().map(Vec::len).sum();

    logger::info(&format!("Phase 2 complete: {} rows in DB", db_row_count));

    // -----------------------------------------------------------------------
    // Phase 3: Compute set difference
    // -----------------------------------------------------------------------
    send_progress(
        &progress,
        ScanProgress {
            current_file: String::new(),
            files_scanned: 0,
            total_files: total_found,
            stage: "diffing".to_string(),
            current_source: "All".to_string(),
            ..Default::default()
        },
    );
    let mut to_scan_set: HashSet<String> = HashSet::new();
    // Paths of stale rows, kept for the fingerprint pass below.
    let mut stale_path_set: HashSet<String> = HashSet::new();
    // Rows to drop, keyed by track id — a path is not unique, so id is the only
    // key that can't take another track with it.
    let mut to_delete_id_set: HashSet<String> = HashSet::new();
    // Stale paths that resolved to no row at all (defensive: nothing else keys
    // a delete on a path).
    let mut unresolved_delete_paths: HashSet<String> = HashSet::new();
    let mut replace_ids_set: HashSet<String> = HashSet::new();
    for source in &sources {
        let Some(parser) =
            libsources::get_parser(&source.source_type, client.clone(), context.clone())
        else {
            continue;
        };
        let Some(entries) = source_entries.get(&source.id) else {
            continue;
        };
        let own: Vec<(String, String)> = own_rows.get(&source.id).cloned().unwrap_or_default();
        let own_ids: HashSet<String> = own.iter().map(|(id, _)| id.clone()).collect();
        let foreign_ids: HashSet<String> = all_db_ids.difference(&own_ids).cloned().collect();
        // path -> ids still available for a stale delete, so a parser that only
        // reports paths still gets its rows dropped by id.
        let mut ids_by_path: HashMap<String, Vec<String>> = HashMap::new();
        for (id, path) in &own {
            ids_by_path
                .entry(path.clone())
                .or_default()
                .push(id.clone());
        }
        match parser.detect_difference(entries, &own, &foreign_ids).await {
            Ok(diff) => {
                for (id, path) in diff.conflicts {
                    stream_log(
                        &log_tx,
                        "WARN",
                        format!(
                            "Track id '{}' from source '{}' is already owned by another library source — skipping '{}'",
                            id, source.name, path
                        ),
                    );
                }
                to_scan_set.extend(diff.to_scan);
                for path in diff.to_delete {
                    // Recommendation rows are synced, not on disk; their
                    // lifecycle is the sync's, not the scan's.
                    if path.starts_with("recommendation://") || path.is_empty() {
                        continue;
                    }
                    match ids_by_path.get_mut(&path).and_then(Vec::pop) {
                        Some(id) => {
                            to_delete_id_set.insert(id);
                        }
                        None => {
                            unresolved_delete_paths.insert(path.clone());
                        }
                    }
                    stale_path_set.insert(path);
                }
                to_delete_id_set.extend(diff.to_delete_ids);
                replace_ids_set.extend(diff.replace_ids);
            }
            Err(e) => stream_log(
                &log_tx,
                "ERROR",
                format!(
                    "detect_difference failed for source '{}': {}",
                    source.name, e
                ),
            ),
        }
    }

    logger::info(&format!(
        "Phase 3 complete: {} to scan, {} to delete ({} unresolved by path)",
        to_scan_set.len(),
        to_delete_id_set.len() + unresolved_delete_paths.len(),
        unresolved_delete_paths.len()
    ));

    // -----------------------------------------------------------------------
    // Phase 3b: Fingerprints of DB tracks that will be removed this scan.
    // When a new file matches one of these, it is the replacement copy — keep
    // it instead of deleting it ("keep one when all copies would be uninserted").
    // Stale rows are normally already gone by phase 4 (phase 3c deletes them
    // first), so this is the safety net for a delete that did not land.
    // -----------------------------------------------------------------------
    let mut stale_fp_to_track: HashMap<String, String> =
        match library::fingerprint_ids_of(pool, &stale_path_set).await {
            Ok(map) => map,
            Err(e) => {
                stream_log(
                    &log_tx,
                    "ERROR",
                    format!("fingerprint_ids_of failed: {}", e),
                );
                HashMap::new()
            }
        };
    if !stale_fp_to_track.is_empty() {
        logger::info(&format!(
            "Phase 3b complete: {} stale fingerprints will be replaced",
            stale_fp_to_track.len()
        ));
    }

    let mut total_deleted: u32 = 0;

    // -----------------------------------------------------------------------
    // Phase 3c: Drop rows whose federated id is gone, before anything is
    // inserted. These are deleted by id, never by path: a path can be shared
    // with another row (including one inserted later this scan), so a
    // path-keyed delete could remove the wrong track. Doing it ahead of phase 4
    // also means a file that is now enumerated by another source can be
    // inserted instead of being clobbered by the cleanup.
    // -----------------------------------------------------------------------
    for track_id in &to_delete_id_set {
        match library::delete_track_by_id(pool, track_id).await {
            Ok(()) => total_deleted += 1,
            Err(e) => stream_log(
                &log_tx,
                "ERROR",
                format!("Failed to delete track '{}' by id: {}", track_id, e),
            ),
        }
    }
    if !to_delete_id_set.is_empty() {
        logger::info(&format!(
            "Phase 3c complete: {} rows deleted by id",
            to_delete_id_set.len()
        ));
    }

    // -----------------------------------------------------------------------
    // Phase 4: Scan only new files (tag parsing + DB insert)
    // -----------------------------------------------------------------------
    let mut total_new: u32 = 0;
    let mut total_duplicates: u32 = 0;
    let mut total_duplicates_deleted: u32 = 0;
    let mut best_covers: HashMap<String, (Vec<u8>, u32)> = HashMap::new();
    let to_scan_count = to_scan_set.len() as u32;
    let mut scanned_count: u32 = 0;

    for source in &sources {
        let Some(entries) = source_entries.get(&source.id) else {
            continue;
        };
        let new_entries: Vec<&libsources::EnumeratedTrack> = entries
            .iter()
            .filter(|entry| to_scan_set.contains(&entry.path))
            .collect();

        if new_entries.is_empty() {
            logger::info(&format!(
                "Source '{}' has no new files — skipping tag parsing entirely",
                source.name
            ));
            continue;
        }

        let parser =
            match libsources::get_parser(&source.source_type, client.clone(), context.clone()) {
                Some(p) => p,
                None => continue,
            };

        let mut resolver = libsources::SourceUrlResolver::new();
        for entry in new_entries {
            let file_path = &entry.path;
            send_progress(
                &progress,
                ScanProgress {
                    current_file: file_path.clone(),
                    files_scanned: scanned_count,
                    total_files: to_scan_count,
                    stage: "scanning".to_string(),
                    current_source: source.name.clone(),
                    ..Default::default()
                },
            );
            let url = match resolver
                .resolve(&source.urls, Some(&client), Some(file_path.as_str()))
                .await
            {
                Ok(u) => u,
                Err(e) => {
                    stream_log(
                        &log_tx,
                        "WARN",
                        format!("No reachable URL for '{}': {}", source.name, e),
                    );
                    continue;
                }
            };

            let mut track = match parser
                .scan_file(&url, &source.urls, file_path, entry.id.as_deref(), &log_tx)
                .await
            {
                Ok(t) => t,
                Err(e) => {
                    stream_log(
                        &log_tx,
                        "WARN",
                        format!("Failed to scan '{}': {}", file_path, e),
                    );
                    continue;
                }
            };

            // Resize cover (longest side = 500, maintains aspect ratio)
            let mut cover_shortest = 0u32;
            if let Some(bytes) = track.tag.cover.as_ref() {
                if let Ok(img) = image::load_from_memory(bytes) {
                    let (w, h) = img.dimensions();
                    let (new_w, new_h) = if w > h {
                        (500u32, (h * 500 / w).max(1))
                    } else {
                        ((w * 500 / h).max(1), 500u32)
                    };
                    let resized =
                        img.resize_exact(new_w, new_h, image::imageops::FilterType::Lanczos3);
                    let mut buf = Vec::new();
                    if resized
                        .write_to(&mut Cursor::new(&mut buf), image::ImageFormat::Jpeg)
                        .is_ok()
                    {
                        track.tag.cover = Some(buf);
                    }
                    cover_shortest = new_w.min(new_h);
                }
            }

            if (track.track_gain.is_none() || track.track_peak.is_none())
                && crate::libsources::is_editable(&source.source_type)
            {
                match ffmpeg::measure_loudness(&track.file_path).await {
                    Ok(m) => {
                        track.tag.track_gain = Some(-18.0 - m.integrated_lufs);
                        track.tag.track_peak =
                            Some((10f64.powf(m.peak_dbfs / 20.0)).clamp(0.0, 1.0));
                    }
                    Err(e) => {
                        stream_log(
                            &log_tx,
                            "WARN",
                            format!(
                                "Failed to measure loudness for '{}': {}",
                                track.file_path, e
                            ),
                        );
                    }
                }
            }

            scanned_count += 1;

            // Replacement re-provision: drop the stale row by its federated id
            // first, so the path stays unique and the old fingerprint can't
            // veto the new copy.
            if let Some(track_id) = entry.id.as_deref() {
                if replace_ids_set.contains(track_id) {
                    if let Err(e) = library::delete_track_by_id(pool, track_id).await {
                        stream_log(
                            &log_tx,
                            "ERROR",
                            format!(
                                "Failed to remove replaced track '{}' before re-insert: {}",
                                track_id, e
                            ),
                        );
                    }
                }
            }

            let outcome = insert_track_to_db(
                pool,
                &track,
                entry.id.as_deref(),
                &source.id,
                force,
                &mut stale_fp_to_track,
                &mut total_new,
                &mut total_duplicates,
                &log_tx,
            )
            .await;

            match &outcome {
                InsertOutcome::Inserted { album_id, .. } => {
                    // Pick best album cover (prefer shortest side ≥ 500, then larger)
                    if let Some(resized) = track.tag.cover.as_ref() {
                        let shortest = cover_shortest;

                        if !best_covers.contains_key(album_id) {
                            if let Ok(Some(db_cover)) =
                                library::get_album_cover(pool, album_id).await
                            {
                                if let Ok(img) = image::load_from_memory(&db_cover) {
                                    let (dw, dh) = img.dimensions();
                                    best_covers.insert(album_id.clone(), (db_cover, dw.min(dh)));
                                }
                            }
                        }

                        let should_replace = match best_covers.get(album_id) {
                            None => true,
                            Some((_, existing_short)) => {
                                if *existing_short >= 500 {
                                    false
                                } else {
                                    shortest > *existing_short
                                }
                            }
                        };

                        if should_replace {
                            best_covers.insert(album_id.clone(), (resized.to_vec(), shortest));
                        }
                    }
                }
                InsertOutcome::Duplicate => {
                    // A surviving copy exists (or was kept this scan) — remove this file.
                    // `mirror_remote = false`: scan-time cleanup must never propagate
                    // the delete back to a remote server's shared copy.
                    match parser
                        .delete(pool, file_path, &url, &source.urls, false)
                        .await
                    {
                        Ok(()) => {
                            total_duplicates_deleted += 1;
                            logger::info(&format!("Deleted duplicate file '{}'", file_path));
                        }
                        Err(e) => {
                            stream_log(
                                &log_tx,
                                "WARN",
                                format!("Failed to delete duplicate file '{}': {}", file_path, e),
                            );
                        }
                    }
                }
                InsertOutcome::Failed => {}
            }
        }
    }

    // -----------------------------------------------------------------------
    // Phase 5: Delete stale DB entries that could not be resolved to an id.
    // -----------------------------------------------------------------------
    send_progress(
        &progress,
        ScanProgress {
            current_file: String::new(),
            files_scanned: 0,
            total_files: unresolved_delete_paths.len() as u32,
            stage: "cleaning".to_string(),
            ..Default::default()
        },
    );

    for file_path in &unresolved_delete_paths {
        if let Err(e) = library::delete_track_by_file_path(pool, file_path).await {
            stream_log(
                &log_tx,
                "ERROR",
                format!("Failed to delete missing track '{}': {}", file_path, e),
            );
        } else {
            total_deleted += 1;
        }
    }

    // -----------------------------------------------------------------------
    // Phase 6: Apply best covers
    // -----------------------------------------------------------------------
    send_progress(
        &progress,
        ScanProgress {
            current_file: String::new(),
            files_scanned: 0,
            total_files: best_covers.len() as u32,
            stage: "cover_update".to_string(),
            ..Default::default()
        },
    );
    for (album_id, (cover_bytes, _)) in &best_covers {
        if let Err(e) = library::update_album_cover(pool, album_id, cover_bytes).await {
            stream_log(
                &log_tx,
                "ERROR",
                format!("Failed to update album cover for '{}': {}", album_id, e),
            );
        }
    }

    // -----------------------------------------------------------------------
    // Done
    // -----------------------------------------------------------------------
    let result = ScanResult {
        success: true,
        tracks_found: total_found,
        new_tracks: total_new,
        duplicates: total_duplicates,
        deleted: total_deleted,
        error: None,
    };

    logger::info(&format!(
        "Scan complete: {} found, {} new, {} duplicates ({} files deleted), {} deleted",
        result.tracks_found,
        result.new_tracks,
        result.duplicates,
        total_duplicates_deleted,
        result.deleted
    ));

    send_progress(
        &progress,
        ScanProgress {
            current_file: String::new(),
            files_scanned: result.tracks_found,
            total_files: result.tracks_found,
            stage: "done".to_string(),
            complete: true,
            tracks_found: result.tracks_found,
            new_tracks: result.new_tracks,
            duplicates: result.duplicates,
            deleted: result.deleted,
            current_source: String::new(),
            error: None,
        },
    );

    result
}
