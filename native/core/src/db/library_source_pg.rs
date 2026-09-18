use anyhow::Result;
use sqlx::PgPool;
use uuid::Uuid;

use crate::signals::library::LibrarySourceInfo;

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
    pool: &PgPool,
    user_id: &str,
    urls: &[String],
    name: &str,
    source_type: &str,
    access_rule: &str,
    master_key: &str,
) -> std::result::Result<String, crate::db::library_source::AddSourceError> {
    let urls = encrypt_urls(urls, master_key);
    let existing: Vec<(String, String)> = sqlx::query_as(
        "SELECT id, urls FROM library_sources WHERE owner_id = $1 AND source_type = $2",
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
    let urls_json = serde_json::to_string(&urls).unwrap_or_else(|_| "[]".to_string());
    sqlx::query(
        "INSERT INTO library_sources (id, source_type, urls, name, owner_id, access_rule, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, NOW(), NOW())",
    )
    .bind(&id)
    .bind(source_type)
    .bind(&urls_json)
    .bind(name)
    .bind(user_id)
    .bind(access_rule)
    .execute(pool)
    .await?;
    Ok(id)
}

pub async fn remove_source(pool: &PgPool, source_id: &str) -> Result<bool> {
    let source = sqlx::query_as::<_, (String, String, String, String, String, String, Option<String>, String, String)>(
        &format!(
            "SELECT id, source_type, urls, name, owner_id, access_rule, {}, {}, {} FROM library_sources WHERE id = $1",
            super::ts_utc("last_sync_at"),
            super::ts_utc("created_at"),
            super::ts_utc("updated_at"),
        ),
    )
    .bind(source_id)
    .fetch_optional(pool)
    .await?;

    let Some((_, source_type, _, name, owner_id, _, _, _, _)) = source else {
        return Ok(false);
    };

    super::library_pg::delete_tracks_by_source_id(pool, source_id).await?;

    if source_type.starts_with("recommendation:") {
        let collection_ids: Vec<String> = sqlx::query_scalar(
            "SELECT id FROM collections WHERE name = $1 AND user_id = $2 AND is_smart = FALSE",
        )
        .bind(&name)
        .bind(&owner_id)
        .fetch_all(pool)
        .await?;
        for collection_id in &collection_ids {
            let remaining: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM collection_tracks WHERE collection_id = $1",
            )
            .bind(collection_id)
            .fetch_one(pool)
            .await?;
            if remaining == 0 {
                sqlx::query("DELETE FROM collections WHERE id = $1")
                    .bind(collection_id)
                    .execute(pool)
                    .await?;
            }
        }
    }

    let rows = sqlx::query("DELETE FROM library_sources WHERE id = $1")
        .bind(source_id)
        .execute(pool)
        .await?
        .rows_affected();
    Ok(rows > 0)
}

pub async fn list_all_sources(pool: &PgPool, master_key: &str) -> Result<Vec<LibrarySourceInfo>> {
    let rows = sqlx::query_as::<_, (String, String, String, String, String, String, Option<String>, String, String)>(
        &format!(
            "SELECT id, source_type, urls, name, owner_id, access_rule, {}, {}, {} FROM library_sources ORDER BY created_at",
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
    pool: &PgPool,
    source_id: &str,
    master_key: &str,
) -> Result<Option<LibrarySourceInfo>> {
    let row = sqlx::query_as::<_, (String, String, String, String, String, String, Option<String>, String, String)>(
        &format!(
            "SELECT id, source_type, urls, name, owner_id, access_rule, {}, {}, {} FROM library_sources WHERE id = $1",
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
    pool: &PgPool,
    track_id: &str,
    master_key: &str,
) -> Result<Option<(String, String)>> {
    let row = sqlx::query_as::<_, (String, String)>(
        "SELECT ls.source_type, ls.urls FROM tracks t JOIN library_sources ls ON t.source_id = ls.id WHERE t.id = $1",
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
    pool: &PgPool,
    track_id: &str,
    master_key: &str,
) -> Result<Option<LibrarySourceInfo>> {
    let row = sqlx::query_as::<_, (String, String, String, String, String, String, Option<String>, String, String)>(
        &format!(
            "SELECT ls.id, ls.source_type, ls.urls, ls.name, ls.owner_id, ls.access_rule, {}, {}, {} FROM tracks t JOIN library_sources ls ON t.source_id = ls.id WHERE t.id = $1",
            super::ts_utc("ls.last_sync_at"),
            super::ts_utc("ls.created_at"),
            super::ts_utc("ls.updated_at"),
        ),
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

pub async fn get_urls_for_scan(pool: &PgPool, master_key: &str) -> Result<Vec<String>> {
    let rows = sqlx::query_scalar::<_, String>("SELECT urls FROM library_sources")
        .fetch_all(pool)
        .await?;
    Ok(rows
        .iter()
        .flat_map(|json| decrypt_urls(parse_urls(json), master_key))
        .collect())
}

pub async fn get_source_by_url_and_owner(
    pool: &PgPool,
    source_type: &str,
    url: &str,
    owner_id: &str,
    master_key: &str,
) -> Result<Option<LibrarySourceInfo>> {
    let row = sqlx::query_as::<_, (String, String, String, String, String, String, Option<String>, String, String)>(
        &format!(
            "SELECT id, source_type, urls, name, owner_id, access_rule, {}, {}, {} FROM library_sources WHERE source_type = $1 AND urls::jsonb ? $2 AND owner_id = $3",
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
    pool: &PgPool,
    source_type: &str,
    urls: &[String],
    name: &str,
    owner_id: &str,
) -> Result<String> {
    let id = Uuid::new_v4().to_string();
    let urls_json = serde_json::to_string(urls).unwrap_or_else(|_| "[]".to_string());
    let result: String = sqlx::query_scalar(
        "INSERT INTO library_sources (id, source_type, urls, name, owner_id, access_rule, created_at, updated_at, last_sync_at)
         VALUES ($1, $2, $3, $4, $5, $6, NOW(), NOW(), NOW())
         ON CONFLICT (source_type, owner_id)
         WHERE source_type LIKE 'recommendation:%'
         DO UPDATE SET urls = EXCLUDED.urls, name = EXCLUDED.name, updated_at = NOW(), last_sync_at = NOW()
         RETURNING id",
    )
    .bind(&id)
    .bind(source_type)
    .bind(&urls_json)
    .bind(name)
    .bind(owner_id)
    .bind("all")
    .fetch_one(pool)
    .await?;
    Ok(result)
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
