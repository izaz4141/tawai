use anyhow::Result;
use sqlx::PgPool;
use uuid::Uuid;

use crate::signals::library::LibrarySourceInfo;

pub async fn add_source(
    pool: &PgPool,
    user_id: &str,
    url: &str,
    name: &str,
    source_type: &str,
    access_rule: &str,
) -> Result<String> {
    let id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO library_sources (id, source_type, url, name, owner_id, access_rule, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, NOW(), NOW())",
    )
    .bind(&id)
    .bind(source_type)
    .bind(url)
    .bind(name)
    .bind(user_id)
    .bind(access_rule)
    .execute(pool)
    .await?;
    Ok(id)
}

pub async fn remove_source(pool: &PgPool, source_id: &str) -> Result<bool> {
    let rows = sqlx::query("DELETE FROM library_sources WHERE id = $1")
        .bind(source_id)
        .execute(pool)
        .await?
        .rows_affected();
    Ok(rows > 0)
}

pub async fn list_all_sources(pool: &PgPool) -> Result<Vec<LibrarySourceInfo>> {
    let rows = sqlx::query_as::<_, (String, String, String, String, String, String, Option<String>, String, String)>(
        &format!(
            "SELECT id, source_type, url, name, owner_id, access_rule, {}, {}, {} FROM library_sources ORDER BY created_at",
            super::ts_utc("last_sync_at"),
            super::ts_utc("created_at"),
            super::ts_utc("updated_at"),
        ),
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

pub async fn get_source_by_id(pool: &PgPool, source_id: &str) -> Result<Option<LibrarySourceInfo>> {
    let row = sqlx::query_as::<_, (String, String, String, String, String, String, Option<String>, String, String)>(
        &format!(
            "SELECT id, source_type, url, name, owner_id, access_rule, {}, {}, {} FROM library_sources WHERE id = $1",
            super::ts_utc("last_sync_at"),
            super::ts_utc("created_at"),
            super::ts_utc("updated_at"),
        ),
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
    pool: &PgPool,
    track_id: &str,
) -> Result<Option<(String, String)>> {
    let row = sqlx::query_as::<_, (String, String)>(
        "SELECT ls.source_type, ls.url FROM tracks t JOIN library_sources ls ON t.source_id = ls.id WHERE t.id = $1",
    )
    .bind(track_id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

pub async fn get_urls_for_scan(pool: &PgPool) -> Result<Vec<String>> {
    let rows = sqlx::query_scalar::<_, String>("SELECT url FROM library_sources")
        .fetch_all(pool)
        .await?;
    Ok(rows)
}

pub async fn get_source_by_url_and_owner(
    pool: &PgPool,
    source_type: &str,
    url: &str,
    owner_id: &str,
) -> Result<Option<LibrarySourceInfo>> {
    let row = sqlx::query_as::<_, (String, String, String, String, String, String, Option<String>, String, String)>(
        &format!(
            "SELECT id, source_type, url, name, owner_id, access_rule, {}, {}, {} FROM library_sources WHERE source_type = $1 AND url = $2 AND owner_id = $3",
            super::ts_utc("last_sync_at"),
            super::ts_utc("created_at"),
            super::ts_utc("updated_at"),
        ),
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
    pool: &PgPool,
    source_type: &str,
    url: &str,
    name: &str,
    owner_id: &str,
) -> Result<String> {
    let existing: Option<String> = sqlx::query_scalar(
        "SELECT id FROM library_sources WHERE source_type = $1 AND url = $2 AND owner_id = $3",
    )
    .bind(source_type)
    .bind(url)
    .bind(owner_id)
    .fetch_optional(pool)
    .await?;

    if let Some(id) = existing {
        sqlx::query("UPDATE library_sources SET name = $1, updated_at = NOW(), last_sync_at = NOW() WHERE id = $2")
            .bind(name)
            .bind(&id)
            .execute(pool)
            .await?;
        Ok(id)
    } else {
        let id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO library_sources (id, source_type, url, name, owner_id, access_rule, created_at, updated_at, last_sync_at) VALUES ($1, $2, $3, $4, $5, $6, NOW(), NOW(), NOW())"
        )
        .bind(&id)
        .bind(source_type)
        .bind(url)
        .bind(name)
        .bind(owner_id)
        .bind("all")
        .execute(pool)
        .await?;
        Ok(id)
    }
}

pub async fn touch_source_sync_at(pool: &PgPool, source_id: &str) -> Result<()> {
    sqlx::query(
        "UPDATE library_sources SET updated_at = NOW(), last_sync_at = NOW() WHERE id = $1",
    )
    .bind(source_id)
    .execute(pool)
    .await?;
    Ok(())
}
