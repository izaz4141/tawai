use anyhow::Result;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::signals::library::LibrarySourceInfo;

fn now() -> String {
    time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_default()
}

fn parse_urls(json: &str) -> Vec<String> {
    serde_json::from_str(json).unwrap_or_default()
}

fn encrypt_urls(urls: &[String], master_key: &str) -> Vec<String> {
    urls.iter()
        .map(|u| crate::libsources::tawai::encrypt_url(u, master_key))
        .collect()
}

fn decrypt_urls(urls: Vec<String>, master_key: &str) -> Vec<String> {
    urls.into_iter()
        .map(|u| crate::libsources::tawai::decrypt_url(&u, master_key))
        .collect()
}

pub async fn add_source(
    pool: &SqlitePool,
    user_id: &str,
    urls: &[String],
    name: &str,
    source_type: &str,
    access_rule: &str,
    master_key: &str,
) -> std::result::Result<String, crate::db::library_source::AddSourceError> {
    let urls = encrypt_urls(urls, master_key);
    let existing: Vec<(String, String)> = sqlx::query_as(
        "SELECT id, urls FROM library_sources WHERE owner_id = ? AND source_type = ?",
    )
    .bind(user_id)
    .bind(source_type)
    .fetch_all(pool)
    .await?;
    for (existing_id, existing_urls_json) in existing {
        let existing_urls = parse_urls(&existing_urls_json);
        if urls.iter().any(|u| existing_urls.contains(u)) {
            return Err(crate::db::library_source::AddSourceError::Duplicate {
                source_id: existing_id,
            });
        }
    }

    let id = Uuid::new_v4().to_string();
    let now = now();
    let urls_json = serde_json::to_string(&urls).unwrap_or_else(|_| "[]".to_string());
    sqlx::query(
        "INSERT INTO library_sources (id, source_type, urls, name, owner_id, access_rule, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(source_type)
    .bind(&urls_json)
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
    let source = sqlx::query_as::<_, (String, String, String, String, String, String, Option<String>, String, String)>(
        "SELECT id, source_type, urls, name, owner_id, access_rule, last_sync_at, created_at, updated_at FROM library_sources WHERE id = ?",
    )
    .bind(source_id)
    .fetch_optional(pool)
    .await?;

    let Some((_, source_type, _, name, owner_id, _, _, _, _)) = source else {
        return Ok(false);
    };

    super::library_sq::delete_tracks_by_source_id(pool, source_id).await?;

    if source_type.starts_with("recommendation:") {
        let collection_ids: Vec<String> = sqlx::query_scalar(
            "SELECT id FROM collections WHERE name = ? AND user_id = ? AND is_smart = 0",
        )
        .bind(&name)
        .bind(&owner_id)
        .fetch_all(pool)
        .await?;
        for collection_id in &collection_ids {
            let remaining: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM collection_tracks WHERE collection_id = ?",
            )
            .bind(collection_id)
            .fetch_one(pool)
            .await?;
            if remaining == 0 {
                sqlx::query("DELETE FROM collections WHERE id = ?")
                    .bind(collection_id)
                    .execute(pool)
                    .await?;
            }
        }
    }

    let rows = sqlx::query("DELETE FROM library_sources WHERE id = ?")
        .bind(source_id)
        .execute(pool)
        .await?
        .rows_affected();
    Ok(rows > 0)
}

pub async fn list_all_sources(pool: &SqlitePool, master_key: &str) -> Result<Vec<LibrarySourceInfo>> {
    let rows = sqlx::query_as::<_, (String, String, String, String, String, String, Option<String>, String, String)>(
        "SELECT id, source_type, urls, name, owner_id, access_rule, last_sync_at, created_at, updated_at FROM library_sources ORDER BY created_at",
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(
            |(
                id,
                source_type,
                urls_json,
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
                    urls: decrypt_urls(parse_urls(&urls_json), master_key),
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
    master_key: &str,
) -> Result<Option<LibrarySourceInfo>> {
    let row = sqlx::query_as::<_, (String, String, String, String, String, String, Option<String>, String, String)>(
        "SELECT id, source_type, urls, name, owner_id, access_rule, last_sync_at, created_at, updated_at FROM library_sources WHERE id = ?",
    )
    .bind(source_id)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(
        |(
            id,
            source_type,
            urls_json,
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
                urls: decrypt_urls(parse_urls(&urls_json), master_key),
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
    master_key: &str,
) -> Result<Option<(String, String)>> {
    let row = sqlx::query_as::<_, (String, String)>(
        "SELECT ls.source_type, ls.urls FROM tracks t JOIN library_sources ls ON t.source_id = ls.id WHERE t.id = ?",
    )
    .bind(track_id)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|(source_type, urls_json)| {
        let urls = decrypt_urls(parse_urls(&urls_json), master_key);
        (
            source_type,
            serde_json::to_string(&urls).unwrap_or(urls_json),
        )
    }))
}

pub async fn get_source_info_by_track_id(
    pool: &SqlitePool,
    track_id: &str,
    master_key: &str,
) -> Result<Option<LibrarySourceInfo>> {
    let row = sqlx::query_as::<_, (String, String, String, String, String, String, Option<String>, String, String)>(
        "SELECT ls.id, ls.source_type, ls.urls, ls.name, ls.owner_id, ls.access_rule, ls.last_sync_at, ls.created_at, ls.updated_at FROM tracks t JOIN library_sources ls ON t.source_id = ls.id WHERE t.id = ?",
    )
    .bind(track_id)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(
        |(
            id,
            source_type,
            urls_json,
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
                urls: decrypt_urls(parse_urls(&urls_json), master_key),
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

pub async fn get_urls_for_scan(pool: &SqlitePool, master_key: &str) -> Result<Vec<String>> {
    let rows = sqlx::query_scalar::<_, String>("SELECT urls FROM library_sources")
        .fetch_all(pool)
        .await?;
    Ok(rows
        .iter()
        .flat_map(|json| decrypt_urls(parse_urls(json), master_key))
        .collect())
}

pub async fn get_source_by_url_and_owner(
    pool: &SqlitePool,
    source_type: &str,
    url: &str,
    owner_id: &str,
    master_key: &str,
) -> Result<Option<LibrarySourceInfo>> {
    let row = sqlx::query_as::<_, (String, String, String, String, String, String, Option<String>, String, String)>(
        "SELECT id, source_type, urls, name, owner_id, access_rule, last_sync_at, created_at, updated_at FROM library_sources WHERE source_type = ? AND owner_id = ? AND EXISTS (SELECT 1 FROM json_each(library_sources.urls) WHERE json_each.value = ?)",
    )
    .bind(source_type)
    .bind(owner_id)
    .bind(url)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(
        |(
            id,
            source_type,
            urls_json,
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
                urls: decrypt_urls(parse_urls(&urls_json), master_key),
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
    urls: &[String],
    name: &str,
    owner_id: &str,
) -> Result<String> {
    let id = Uuid::new_v4().to_string();
    let now = now();
    let urls_json = serde_json::to_string(urls).unwrap_or_else(|_| "[]".to_string());
    let result: String = sqlx::query_scalar(
        "INSERT INTO library_sources (id, source_type, urls, name, owner_id, access_rule, created_at, updated_at, last_sync_at)
         VALUES (?, ?, ?, ?, ?, 'all', ?, ?, ?)
         ON CONFLICT (source_type, owner_id)
         WHERE source_type LIKE 'recommendation:%'
         DO UPDATE SET urls = excluded.urls, name = excluded.name, updated_at = excluded.updated_at, last_sync_at = excluded.last_sync_at
         RETURNING id",
    )
    .bind(&id)
    .bind(source_type)
    .bind(&urls_json)
    .bind(name)
    .bind(owner_id)
    .bind(&now)
    .bind(&now)
    .bind(&now)
    .fetch_one(pool)
    .await?;
    Ok(result)
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
