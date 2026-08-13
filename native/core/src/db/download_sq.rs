use anyhow::Result;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::signals::download::DownloadRecord;

pub async fn insert_download(
    pool: &SqlitePool,
    user_id: &str,
    source: &str,
    source_id: &str,
    url: &str,
    dest_path: &str,
    filename: &str,
) -> Result<String> {
    let id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO downloads (id, user_id, source, source_id, url, dest_path, filename, state) VALUES (?, ?, ?, ?, ?, ?, ?, 'queued')",
    )
    .bind(&id)
    .bind(user_id)
    .bind(source)
    .bind(source_id)
    .bind(url)
    .bind(dest_path)
    .bind(filename)
    .execute(pool)
    .await?;
    Ok(id)
}

pub async fn update_download_state(
    pool: &SqlitePool,
    id: &str,
    state: &str,
    error: &str,
    downloaded: i64,
    total_size: i64,
) -> Result<()> {
    sqlx::query(
        "UPDATE downloads SET state = ?, error = ?, downloaded = ?, total_size = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = ?",
    )
    .bind(state)
    .bind(error)
    .bind(downloaded)
    .bind(total_size)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn list_downloads(
    pool: &SqlitePool,
    user_id: &str,
    source: Option<&str>,
) -> Result<Vec<DownloadRecord>> {
    let rows = if let Some(src) = source {
        sqlx::query(
            "SELECT id, source, source_id, url, dest_path, filename, total_size, downloaded, state, error, added_at, updated_at FROM downloads WHERE user_id = ? AND source = ? ORDER BY added_at DESC",
        )
        .bind(user_id)
        .bind(src)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query(
            "SELECT id, source, source_id, url, dest_path, filename, total_size, downloaded, state, error, added_at, updated_at FROM downloads WHERE user_id = ? ORDER BY added_at DESC",
        )
        .bind(user_id)
        .fetch_all(pool)
        .await?
    };

    Ok(rows
        .into_iter()
        .map(|row| DownloadRecord {
            id: row.get("id"),
            source: row.get("source"),
            source_id: row.get("source_id"),
            url: row.get("url"),
            dest_path: row.get("dest_path"),
            filename: row.get("filename"),
            total_size: row.get("total_size"),
            downloaded: row.get("downloaded"),
            state: row.get("state"),
            error: row.get("error"),
            added_at: row.get("added_at"),
            updated_at: row.get("updated_at"),
        })
        .collect())
}

pub async fn get_download(pool: &SqlitePool, id: &str) -> Result<Option<DownloadRecord>> {
    let row = sqlx::query(
        "SELECT id, source, source_id, url, dest_path, filename, total_size, downloaded, state, error, added_at, updated_at FROM downloads WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|row| DownloadRecord {
        id: row.get("id"),
        source: row.get("source"),
        source_id: row.get("source_id"),
        url: row.get("url"),
        dest_path: row.get("dest_path"),
        filename: row.get("filename"),
        total_size: row.get("total_size"),
        downloaded: row.get("downloaded"),
        state: row.get("state"),
        error: row.get("error"),
        added_at: row.get("added_at"),
        updated_at: row.get("updated_at"),
    }))
}

pub async fn update_download_filename(pool: &SqlitePool, id: &str, filename: &str) -> Result<()> {
    sqlx::query(
        "UPDATE downloads SET filename = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = ?",
    )
    .bind(filename)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_download_by_source(
    pool: &SqlitePool,
    source: &str,
    source_id: &str,
) -> Result<Option<DownloadRecord>> {
    let row = sqlx::query(
        "SELECT id, source, source_id, url, dest_path, filename, total_size, downloaded, state, error, added_at, updated_at FROM downloads WHERE source = ? AND source_id = ? LIMIT 1",
    )
    .bind(source)
    .bind(source_id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|row| DownloadRecord {
        id: row.get("id"),
        source: row.get("source"),
        source_id: row.get("source_id"),
        url: row.get("url"),
        dest_path: row.get("dest_path"),
        filename: row.get("filename"),
        total_size: row.get("total_size"),
        downloaded: row.get("downloaded"),
        state: row.get("state"),
        error: row.get("error"),
        added_at: row.get("added_at"),
        updated_at: row.get("updated_at"),
    }))
}
