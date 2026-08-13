use anyhow::Result;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::signals::download::DownloadRecord;

pub async fn insert_download(
    pool: &PgPool,
    user_id: &str,
    source: &str,
    source_id: &str,
    url: &str,
    dest_path: &str,
    filename: &str,
) -> Result<String> {
    let id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO downloads (id, user_id, source, source_id, url, dest_path, filename, state) VALUES ($1, $2, $3, $4, $5, $6, $7, 'queued')",
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
    pool: &PgPool,
    id: &str,
    state: &str,
    error: &str,
    downloaded: i64,
    total_size: i64,
) -> Result<()> {
    sqlx::query(
        "UPDATE downloads SET state = $1, error = $2, downloaded = $3, total_size = $4, updated_at = NOW() WHERE id = $5",
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
    pool: &PgPool,
    user_id: &str,
    source: Option<&str>,
) -> Result<Vec<DownloadRecord>> {
    let rows = if let Some(src) = source {
        sqlx::query(&format!(
            "SELECT id, source, source_id, url, dest_path, filename, total_size, downloaded, state, error, {}, {} FROM downloads WHERE user_id = $1 AND source = $2 ORDER BY downloads.added_at DESC",
            super::ts_utc("added_at"),
            super::ts_utc("updated_at"),
        ))
        .bind(user_id)
        .bind(src)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query(&format!(
            "SELECT id, source, source_id, url, dest_path, filename, total_size, downloaded, state, error, {}, {} FROM downloads WHERE user_id = $1 ORDER BY downloads.added_at DESC",
            super::ts_utc("added_at"),
            super::ts_utc("updated_at"),
        ))
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

pub async fn get_download(pool: &PgPool, id: &str) -> Result<Option<DownloadRecord>> {
    let row = sqlx::query(&format!(
        "SELECT id, source, source_id, url, dest_path, filename, total_size, downloaded, state, error, {}, {} FROM downloads WHERE id = $1",
        super::ts_utc("added_at"),
        super::ts_utc("updated_at"),
    ))
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

pub async fn update_download_filename(pool: &PgPool, id: &str, filename: &str) -> Result<()> {
    sqlx::query("UPDATE downloads SET filename = $1, updated_at = NOW() WHERE id = $2")
        .bind(filename)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn get_download_by_source(
    pool: &PgPool,
    source: &str,
    source_id: &str,
) -> Result<Option<DownloadRecord>> {
    let row = sqlx::query(&format!(
        "SELECT id, source, source_id, url, dest_path, filename, total_size, downloaded, state, error, {}, {} FROM downloads WHERE source = $1 AND source_id = $2 LIMIT 1",
        super::ts_utc("added_at"),
        super::ts_utc("updated_at"),
    ))
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
