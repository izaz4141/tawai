use crate::db::database::DatabasePool;
use crate::db::library;
use crate::utils::config::AppConfig;
use crate::utils::config::dash_cache_dir;
use crate::utils::logger;
use std::path::Path;

/// Remove stale per-track DASH cache directories under the cache root.
///
/// A directory named `{track_id}` is removed when the track no longer exists
/// in the database or its source file is no longer present on disk. Database
/// errors never delete anything; they are logged and skipped.
pub async fn cleanup_dash_cache(pool: &DatabasePool, cfg: &AppConfig) -> anyhow::Result<()> {
    let root = dash_cache_dir(cfg);
    let mut entries = match tokio::fs::read_dir(&root).await {
        Ok(entries) => entries,
        Err(_) => return Ok(()),
    };

    let mut removed = 0u32;
    while let Some(entry) = entries.next_entry().await? {
        let Ok(file_type) = entry.file_type().await else {
            continue;
        };
        if !file_type.is_dir() {
            continue;
        }
        let track_id = entry.file_name().to_string_lossy().to_string();
        match library::lookup_track(pool, &track_id).await {
            Ok(Some(track)) => {
                if Path::new(&track.file_path).is_file() {
                    continue;
                }
            }
            Ok(None) => {}
            Err(e) => {
                logger::warn(&format!(
                    "dash cache cleanup: skipping {} (lookup failed: {})",
                    track_id, e
                ));
                continue;
            }
        }
        match tokio::fs::remove_dir_all(entry.path()).await {
            Ok(()) => {
                removed += 1;
                logger::info(&format!(
                    "dash cache cleanup: removed stale cache for track {}",
                    track_id
                ));
            }
            Err(e) => {
                logger::warn(&format!(
                    "dash cache cleanup: failed to remove {} ({})",
                    track_id, e
                ));
            }
        }
    }

    if removed > 0 {
        logger::info(&format!(
            "dash cache cleanup: removed {} directories",
            removed
        ));
    }
    Ok(())
}
