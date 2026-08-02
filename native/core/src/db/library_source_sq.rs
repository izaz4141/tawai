use anyhow::Result;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::signals::library::LibrarySourceInfo;

fn now() -> String {
    time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_default()
}

pub async fn add_source(
    pool: &SqlitePool,
    user_id: &str,
    url: &str,
    name: &str,
    source_type: &str,
    access_rule: &str,
) -> Result<String> {
    let id = Uuid::new_v4().to_string();
    let now = now();
    sqlx::query(
        "INSERT INTO library_sources (id, source_type, url, name, owner_id, access_rule, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(source_type)
    .bind(url)
    .bind(name)
    .bind(user_id)
    .bind(access_rule)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;
    Ok(id)
}

pub async fn remove_source(pool: &SqlitePool, source_id: &str) -> Result<bool> {
    let rows = sqlx::query("DELETE FROM library_sources WHERE id = ?")
        .bind(source_id)
        .execute(pool)
        .await?
        .rows_affected();
    Ok(rows > 0)
}

pub async fn list_all_sources(pool: &SqlitePool) -> Result<Vec<LibrarySourceInfo>> {
    let rows = sqlx::query_as::<_, (String, String, String, String, String, String, Option<String>, String, String)>(
        "SELECT id, source_type, url, name, owner_id, access_rule, last_sync_at, created_at, updated_at FROM library_sources ORDER BY created_at",
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(
            |(
                id,
                source_type,
                url,
                name,
                owner_id,
                access_rule,
                last_sync_at,
                created_at,
                updated_at,
            )| {
                LibrarySourceInfo {
                    id,
                    source_type,
                    url,
                    name,
                    last_sync_at,
                    owner_id,
                    access_rule,
                    created_at,
                    updated_at,
                }
            },
        )
        .collect())
}

pub async fn get_source_by_id(
    pool: &SqlitePool,
    source_id: &str,
) -> Result<Option<LibrarySourceInfo>> {
    let row = sqlx::query_as::<_, (String, String, String, String, String, String, Option<String>, String, String)>(
        "SELECT id, source_type, url, name, owner_id, access_rule, last_sync_at, created_at, updated_at FROM library_sources WHERE id = ?",
    )
    .bind(source_id)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(
        |(
            id,
            source_type,
            url,
            name,
            owner_id,
            access_rule,
            last_sync_at,
            created_at,
            updated_at,
        )| {
            LibrarySourceInfo {
                id,
                source_type,
                url,
                name,
                last_sync_at,
                owner_id,
                access_rule,
                created_at,
                updated_at,
            }
        },
    ))
}

pub async fn get_source_by_track_id(
    pool: &SqlitePool,
    track_id: &str,
) -> Result<Option<(String, String)>> {
    let row = sqlx::query_as::<_, (String, String)>(
        "SELECT ls.source_type, ls.url FROM tracks t JOIN library_sources ls ON t.source_id = ls.id WHERE t.id = ?",
    )
    .bind(track_id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

pub async fn get_urls_for_scan(pool: &SqlitePool) -> Result<Vec<String>> {
    let rows = sqlx::query_scalar::<_, String>("SELECT url FROM library_sources")
        .fetch_all(pool)
        .await?;
    Ok(rows)
}

pub async fn get_source_by_url_and_owner(
    pool: &SqlitePool,
    source_type: &str,
    url: &str,
    owner_id: &str,
) -> Result<Option<LibrarySourceInfo>> {
    let row = sqlx::query_as::<_, (String, String, String, String, String, String, Option<String>, String, String)>(
        "SELECT id, source_type, url, name, owner_id, access_rule, last_sync_at, created_at, updated_at FROM library_sources WHERE source_type = ? AND url = ? AND owner_id = ?",
    )
    .bind(source_type)
    .bind(url)
    .bind(owner_id)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(
        |(
            id,
            source_type,
            url,
            name,
            owner_id,
            access_rule,
            last_sync_at,
            created_at,
            updated_at,
        )| {
            LibrarySourceInfo {
                id,
                source_type,
                url,
                name,
                last_sync_at,
                owner_id,
                access_rule,
                created_at,
                updated_at,
            }
        },
    ))
}

pub async fn upsert_source(
    pool: &SqlitePool,
    source_type: &str,
    url: &str,
    name: &str,
    owner_id: &str,
) -> Result<String> {
    let existing: Option<String> = sqlx::query_scalar(
        "SELECT id FROM library_sources WHERE source_type = ? AND url = ? AND owner_id = ?",
    )
    .bind(source_type)
    .bind(url)
    .bind(owner_id)
    .fetch_optional(pool)
    .await?;

    if let Some(id) = existing {
        let now = now();
        sqlx::query(
            "UPDATE library_sources SET name = ?, updated_at = ?, last_sync_at = ? WHERE id = ?",
        )
        .bind(name)
        .bind(&now)
        .bind(&now)
        .bind(&id)
        .execute(pool)
        .await?;
        Ok(id)
    } else {
        let id = Uuid::new_v4().to_string();
        let now = now();
        sqlx::query(
            "INSERT INTO library_sources (id, source_type, url, name, owner_id, access_rule, created_at, updated_at, last_sync_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&id)
        .bind(source_type)
        .bind(url)
        .bind(name)
        .bind(owner_id)
        .bind("all")
        .bind(&now)
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await?;
        Ok(id)
    }
}

pub async fn touch_source_sync_at(pool: &SqlitePool, source_id: &str) -> Result<()> {
    let now = now();
    sqlx::query("UPDATE library_sources SET updated_at = ?, last_sync_at = ? WHERE id = ?")
        .bind(&now)
        .bind(&now)
        .bind(source_id)
        .execute(pool)
        .await?;
    Ok(())
}
