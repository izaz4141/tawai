use anyhow::Result;

use crate::db::database::DatabasePool;
use crate::signals::library::LibrarySourceInfo;

#[derive(Debug)]
pub enum AddSourceError {
    Duplicate { source_id: String },
    Db(anyhow::Error),
}

impl std::fmt::Display for AddSourceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Duplicate { source_id } => {
                write!(f, "duplicate source (already exists: {source_id})")
            }
            Self::Db(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for AddSourceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Duplicate { .. } => None,
            Self::Db(e) => Some(e.as_ref()),
        }
    }
}

impl From<anyhow::Error> for AddSourceError {
    fn from(e: anyhow::Error) -> Self {
        Self::Db(e)
    }
}

impl From<sqlx::Error> for AddSourceError {
    fn from(e: sqlx::Error) -> Self {
        Self::Db(e.into())
    }
}

pub fn can_access_source(owner_id: &str, user_id: &str, role: &str, access_rule: &str) -> bool {
    if role == "admin" {
        return true;
    }
    if owner_id == user_id {
        return true;
    }
    match access_rule {
        "" | "all" => true,
        "owner" => false,
        r if r.starts_with("role:") => role == &r[5..],
        json => serde_json::from_str::<Vec<String>>(json)
            .ok()
            .map(|ids| ids.iter().any(|id| id == user_id))
            .unwrap_or(false),
    }
}

pub async fn add_source(
    pool: &DatabasePool,
    user_id: &str,
    urls: &[String],
    name: &str,
    source_type: &str,
    access_rule: &str,
    master_key: &str,
) -> std::result::Result<String, AddSourceError> {
    match pool {
        DatabasePool::Sqlite(p) => {
            super::library_source_sq::add_source(p, user_id, urls, name, source_type, access_rule, master_key)
                .await
        }
        DatabasePool::Postgres(p) => {
            super::library_source_pg::add_source(p, user_id, urls, name, source_type, access_rule, master_key)
                .await
        }
    }
}

pub async fn remove_source(pool: &DatabasePool, source_id: &str) -> Result<bool> {
    match pool {
        DatabasePool::Sqlite(p) => super::library_source_sq::remove_source(p, source_id).await,
        DatabasePool::Postgres(p) => super::library_source_pg::remove_source(p, source_id).await,
    }
}

pub async fn list_editable_sources(
    pool: &DatabasePool,
    user_id: &str,
    role: &str,
    master_key: &str,
) -> Result<Vec<LibrarySourceInfo>> {
    let accessible = list_accessible_sources(pool, user_id, role, master_key).await?;
    Ok(accessible
        .into_iter()
        .filter(|s| crate::libsources::is_editable(&s.source_type))
        .collect())
}

pub async fn list_accessible_sources(
    pool: &DatabasePool,
    user_id: &str,
    role: &str,
    master_key: &str,
) -> Result<Vec<LibrarySourceInfo>> {
    let all = list_all_sources(pool, master_key).await?;
    Ok(all
        .into_iter()
        .filter(|s| can_access_source(&s.owner_id, user_id, role, &s.access_rule))
        .collect())
}

pub async fn list_all_sources(pool: &DatabasePool, master_key: &str) -> Result<Vec<LibrarySourceInfo>> {
    match pool {
        DatabasePool::Sqlite(p) => super::library_source_sq::list_all_sources(p, master_key).await,
        DatabasePool::Postgres(p) => super::library_source_pg::list_all_sources(p, master_key).await,
    }
}

pub async fn get_source_by_track_id(
    pool: &DatabasePool,
    track_id: &str,
    master_key: &str,
) -> Result<Option<(String, String)>> {
    match pool {
        DatabasePool::Sqlite(p) => {
            super::library_source_sq::get_source_by_track_id(p, track_id, master_key).await
        }
        DatabasePool::Postgres(p) => {
            super::library_source_pg::get_source_by_track_id(p, track_id, master_key).await
        }
    }
}

pub async fn get_source_by_id(
    pool: &DatabasePool,
    source_id: &str,
    master_key: &str,
) -> Result<Option<LibrarySourceInfo>> {
    match pool {
        DatabasePool::Sqlite(p) => super::library_source_sq::get_source_by_id(p, source_id, master_key).await,
        DatabasePool::Postgres(p) => super::library_source_pg::get_source_by_id(p, source_id, master_key).await,
    }
}

/// Look up the full library source row backing a track, including access
/// metadata (`owner_id`, `access_rule`).
pub async fn get_source_info_by_track_id(
    pool: &DatabasePool,
    track_id: &str,
    master_key: &str,
) -> Result<Option<LibrarySourceInfo>> {
    match pool {
        DatabasePool::Sqlite(p) => {
            super::library_source_sq::get_source_info_by_track_id(p, track_id, master_key).await
        }
        DatabasePool::Postgres(p) => {
            super::library_source_pg::get_source_info_by_track_id(p, track_id, master_key).await
        }
    }
}

pub async fn get_source_by_url_and_owner(
    pool: &DatabasePool,
    source_type: &str,
    url: &str,
    owner_id: &str,
    master_key: &str,
) -> Result<Option<LibrarySourceInfo>> {
    match pool {
        DatabasePool::Sqlite(p) => {
            super::library_source_sq::get_source_by_url_and_owner(p, source_type, url, owner_id, master_key)
                .await
        }
        DatabasePool::Postgres(p) => {
            super::library_source_pg::get_source_by_url_and_owner(p, source_type, url, owner_id, master_key)
                .await
        }
    }
}

pub async fn get_urls_for_scan(pool: &DatabasePool, master_key: &str) -> Result<Vec<String>> {
    match pool {
        DatabasePool::Sqlite(p) => super::library_source_sq::get_urls_for_scan(p, master_key).await,
        DatabasePool::Postgres(p) => super::library_source_pg::get_urls_for_scan(p, master_key).await,
    }
}

pub async fn upsert_source(
    pool: &DatabasePool,
    source_type: &str,
    urls: &[String],
    name: &str,
    owner_id: &str,
) -> Result<String> {
    match pool {
        DatabasePool::Sqlite(p) => {
            super::library_source_sq::upsert_source(p, source_type, urls, name, owner_id).await
        }
        DatabasePool::Postgres(p) => {
            super::library_source_pg::upsert_source(p, source_type, urls, name, owner_id).await
        }
    }
}

pub async fn touch_source_sync_at(pool: &DatabasePool, source_id: &str) -> Result<()> {
    match pool {
        DatabasePool::Sqlite(p) => {
            super::library_source_sq::touch_source_sync_at(p, source_id).await
        }
        DatabasePool::Postgres(p) => {
            super::library_source_pg::touch_source_sync_at(p, source_id).await
        }
    }
}
