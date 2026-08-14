use std::collections::{HashMap, HashSet};
use std::io::Cursor;

use image::GenericImageView;

use crate::audio::ffmpeg;
use crate::audio::tags::derive_sort_name;
use crate::db::database::DatabasePool;
use crate::db::library;
use crate::libsources;
use crate::signals::library::{LibrarySourceInfo, ScanProgress, ScanResult};
use crate::utils::logger;

fn send_progress(tx: &Option<tokio::sync::watch::Sender<ScanProgress>>, p: ScanProgress) {
    if let Some(tx) = tx {
        let _ = tx.send(p);
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
    source_id: &str,
    force: bool,
    stale_fp_to_path: &mut HashMap<String, String>,
    total_new: &mut u32,
    total_duplicates: &mut u32,
) -> InsertOutcome {
    // Fingerprint-based duplicate detection within the new set
    if !force {
        if let Some(fp) = track.acoust_id_fingerprint.as_deref() {
            match library::track_exists_by_fingerprint(pool, fp).await {
                Ok(Some(_)) => {
                    if let Some(stale_path) = stale_fp_to_path.remove(fp) {
                        // The existing copy is being removed this scan (its file is gone),
                        // so adopt this file as the kept copy. remove() ensures only ONE
                        // replacement happens — later duplicates hit the branch below.
                        if let Err(e) = library::delete_track_by_file_path(pool, &stale_path).await
                        {
                            logger::error(&format!(
                                "Failed to remove stale track '{}' before keep-one insert: {}",
                                stale_path, e
                            ));
                        }
                    } else {
                        *total_duplicates += 1;
                        return InsertOutcome::Duplicate;
                    }
                }
                Err(e) => {
                    logger::error(&format!(
                        "track_exists_by_fingerprint failed for {}: {}",
                        track.file_path, e
                    ));
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
                logger::error(&format!("Failed to insert album artist '{}': {}", name, e));
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
                logger::error(&format!("Failed to insert track artist '{}': {}", name, e));
                return InsertOutcome::Failed;
            }
        }
    }
    let album_id = match library::insert_album(
        pool,
        &track.album,
        &album_artist_ids[0],
        track.release_date.clone(),
        track.mbid_release.clone(),
        None,
        track.album_disambiguation.clone(),
        0,
    )
    .await
    {
        Ok(id) => {
            let _ = library::insert_album_artists(pool, &id, &album_artist_ids).await;
            id
        }
        Err(e) => {
            logger::error(&format!("Failed to insert album '{}': {}", track.album, e));
            return InsertOutcome::Failed;
        }
    };

    let track_id = match library::insert_track(
        pool,
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
        track.mbid_recording.clone(),
        track.cover.as_deref(),
        track.lyrics.as_deref(),
        track.track_gain,
        track.track_peak,
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
            logger::error(&format!("Failed to insert track '{}': {}", track.title, e));
            return InsertOutcome::Failed;
        }
    };

    if let Some(fp) = track.acoust_id_fingerprint.as_deref() {
        if let Err(e) =
            library::insert_fingerprint(pool, &track_id, fp, track.acoust_id.as_deref()).await
        {
            logger::error(&format!(
                "Failed to insert fingerprint for '{}': {}",
                track.title, e
            ));
        }
    }

    InsertOutcome::Inserted { track_id, album_id }
}

pub async fn run_scan(
    pool: &DatabasePool,
    client: reqwest::Client,
    sources: &[LibrarySourceInfo],
    force: bool,
    progress: Option<tokio::sync::watch::Sender<ScanProgress>>,
) -> ScanResult {
    let sources: Vec<LibrarySourceInfo> = sources
        .iter()
        .filter(|s| !s.source_type.starts_with("recommendation:"))
        .cloned()
        .collect();
    if force {
        if let Err(e) = library::delete_all_library(pool).await {
            logger::error(&format!("delete_all_library failed: {}", e));
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
    let mut source_paths: HashMap<String, Vec<String>> = HashMap::new();

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

        let parser = match libsources::get_parser(&source.source_type, client.clone()) {
            Some(p) => p,
            None => {
                logger::error(&format!("Unknown source type: {}", source.source_type));
                continue;
            }
        };

        let paths = match parser.enumerate_paths(&source.url).await {
            Ok(p) => p,
            Err(e) => {
                logger::error(&format!(
                    "Failed to enumerate source '{}' ({}): {}",
                    source.name, source.url, e
                ));
                continue;
            }
        };

        for p in &paths {
            filesystem_paths.insert(p.clone());
        }
        source_paths.insert(source.id.clone(), paths);
    }

    let total_found = filesystem_paths.len() as u32;
    logger::info(&format!("Phase 1 complete: {} files found", total_found));

    // -----------------------------------------------------------------------
    // Phase 2: Get all DB file paths (single query)
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

    let db_paths: HashSet<String> = match library::all_track_file_paths(pool).await {
        Ok(paths) => paths.into_iter().collect(),
        Err(e) => {
            logger::error(&format!("Failed to list track file paths: {}", e));
            HashSet::new()
        }
    };

    logger::info(&format!("Phase 2 complete: {} paths in DB", db_paths.len()));

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
    let (to_scan_set, to_delete_set) = if force {
        let to_delete: HashSet<String> = db_paths
            .difference(&filesystem_paths)
            .cloned()
            .filter(|p| !p.starts_with("recommendation://") && !p.is_empty())
            .collect();
        (filesystem_paths, to_delete)
    } else {
        let to_scan: HashSet<String> = filesystem_paths.difference(&db_paths).cloned().collect();
        let to_delete: HashSet<String> = db_paths
            .difference(&filesystem_paths)
            .cloned()
            .filter(|p| !p.starts_with("recommendation://") && !p.is_empty())
            .collect();
        (to_scan, to_delete)
    };

    logger::info(&format!(
        "Phase 3 complete: {} to scan, {} to delete",
        to_scan_set.len(),
        to_delete_set.len()
    ));

    // -----------------------------------------------------------------------
    // Phase 3b: Fingerprints of DB tracks that will be removed this scan.
    // When a new file matches one of these, it is the replacement copy — keep
    // it instead of deleting it ("keep one when all copies would be uninserted").
    // -----------------------------------------------------------------------
    let mut stale_fp_to_path: HashMap<String, String> =
        match library::fingerprint_paths_of(pool, &to_delete_set).await {
            Ok(map) => map,
            Err(e) => {
                logger::error(&format!("fingerprint_paths_of failed: {}", e));
                HashMap::new()
            }
        };
    if !stale_fp_to_path.is_empty() {
        logger::info(&format!(
            "Phase 3b complete: {} stale fingerprints will be replaced",
            stale_fp_to_path.len()
        ));
    }

    // -----------------------------------------------------------------------
    // Phase 4: Scan only new files (tag parsing + DB insert)
    // -----------------------------------------------------------------------
    let mut total_new: u32 = 0;
    let mut total_duplicates: u32 = 0;
    let mut total_duplicates_deleted: u32 = 0;
    let mut total_deleted: u32 = 0;
    let mut best_covers: HashMap<String, (Vec<u8>, u32)> = HashMap::new();
    let to_scan_count = to_scan_set.len() as u32;
    let mut scanned_count: u32 = 0;

    for source in &sources {
        let Some(source_paths) = source_paths.get(&source.id) else {
            continue;
        };
        let new_paths: Vec<String> = source_paths
            .iter()
            .filter(|p| to_scan_set.contains(*p))
            .cloned()
            .collect();

        if new_paths.is_empty() {
            logger::info(&format!(
                "Source '{}' has no new files — skipping tag parsing entirely",
                source.name
            ));
            continue;
        }

        let parser = match libsources::get_parser(&source.source_type, client.clone()) {
            Some(p) => p,
            None => continue,
        };

        for file_path in &new_paths {
            let mut track = match parser.scan_file(&source.url, file_path).await {
                Ok(t) => t,
                Err(e) => {
                    logger::warn(&format!("Failed to scan '{}': {}", file_path, e));
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
                && source.source_type == "local"
            {
                match ffmpeg::measure_loudness(&track.file_path).await {
                    Ok(m) => {
                        track.tag.track_gain = Some(-18.0 - m.integrated_lufs);
                        track.tag.track_peak =
                            Some((10f64.powf(m.peak_dbfs / 20.0)).clamp(0.0, 1.0));
                    }
                    Err(e) => {
                        logger::warn(&format!(
                            "Failed to measure loudness for '{}': {}",
                            track.file_path, e
                        ));
                    }
                }
            }

            scanned_count += 1;
            send_progress(
                &progress,
                ScanProgress {
                    current_file: track.file_path.clone(),
                    files_scanned: scanned_count,
                    total_files: to_scan_count,
                    stage: "scanning".to_string(),
                    current_source: source.name.clone(),
                    ..Default::default()
                },
            );

            let outcome = insert_track_to_db(
                pool,
                &track,
                &source.id,
                force,
                &mut stale_fp_to_path,
                &mut total_new,
                &mut total_duplicates,
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
                    match parser.delete(file_path, &source.url).await {
                        Ok(()) => {
                            total_duplicates_deleted += 1;
                            logger::info(&format!("Deleted duplicate file '{}'", file_path));
                        }
                        Err(e) => {
                            logger::warn(&format!(
                                "Failed to delete duplicate file '{}': {}",
                                file_path, e
                            ));
                        }
                    }
                }
                InsertOutcome::Failed => {}
            }
        }
    }

    // -----------------------------------------------------------------------
    // Phase 5: Delete stale DB entries
    // -----------------------------------------------------------------------
    send_progress(
        &progress,
        ScanProgress {
            current_file: String::new(),
            files_scanned: 0,
            total_files: to_delete_set.len() as u32,
            stage: "cleaning".to_string(),
            ..Default::default()
        },
    );

    for file_path in &to_delete_set {
        if let Err(e) = library::delete_track_by_file_path(pool, file_path).await {
            logger::error(&format!(
                "Failed to delete missing track '{}': {}",
                file_path, e
            ));
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
            logger::error(&format!(
                "Failed to update album cover for '{}': {}",
                album_id, e
            ));
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
