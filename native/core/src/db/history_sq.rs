use anyhow::Result;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::signals::playback::PlaybackRecord;
use crate::utils::encryption;

pub async fn record_playback(
    pool: &SqlitePool,
    user_id: &str,
    track_id: &str,
    source: &str,
) -> Result<String> {
    let id = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO playback_history (id, user_id, track_id, source) VALUES (?, ?, ?, ?)")
        .bind(&id)
        .bind(user_id)
        .bind(track_id)
        .bind(source)
        .execute(pool)
        .await?;
    Ok(id)
}

pub async fn get_recent_history(
    pool: &SqlitePool,
    user_id: &str,
    limit: i32,
) -> Result<Vec<PlaybackRecord>> {
    let rows = sqlx::query(
        r#"SELECT ph.id, ph.track_id, COALESCE(t.title, 'Unknown') AS track_title,
                  COALESCE(a.title, 'Unknown Album') AS album_title,
                  COALESCE(ar.name, 'Unknown Artist') AS artist_name,
                  ph.played_at, ph.source, ph.scrobbled, t.duration_secs
           FROM playback_history ph
           LEFT JOIN tracks t ON ph.track_id = t.id
           LEFT JOIN albums a ON t.album_id = a.id
           LEFT JOIN artists ar ON t.artist_id = ar.id
           WHERE ph.user_id = ?
           ORDER BY ph.played_at DESC
           LIMIT ?"#,
    )
    .bind(user_id)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| PlaybackRecord {
            id: row.get("id"),
            track_id: row.get("track_id"),
            track_title: row.get("track_title"),
            album_title: row.get("album_title"),
            artist_name: row.get("artist_name"),
            played_at: row.get("played_at"),
            source: row.get("source"),
            scrobbled: row.get::<bool, _>("scrobbled"),
            duration_secs: row.get("duration_secs"),
        })
        .collect())
}

pub async fn mark_scrobbled(pool: &SqlitePool, history_id: &str) -> Result<()> {
    sqlx::query("UPDATE playback_history SET scrobbled = 1 WHERE id = ?")
        .bind(history_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn get_listenbrainz_token(
    pool: &SqlitePool,
    user_id: &str,
    master_key: &str,
) -> Option<String> {
    let stored: Option<String> = sqlx::query_scalar::<_, String>(
        "SELECT value FROM user_settings WHERE user_id = ? AND key = 'listenbrainz_token'",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
    .filter(|v| !v.is_empty());
    match stored {
        Some(val) if val.starts_with("NDK:") => encryption::decrypt(&val, master_key).ok(),
        other => other,
    }
}
