use anyhow::Result;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::db::account;
use crate::signals::account::UserListItem;
use crate::utils::{encryption, helper, security};

pub async fn ensure_default_user(pool: &SqlitePool, master_key: &str) -> Result<()> {
    let exists: bool = sqlx::query_scalar("SELECT COUNT(*) > 0 FROM users WHERE role = 'admin'")
        .fetch_one(pool)
        .await?;

    if !exists {
        let user_id = Uuid::new_v4().to_string();
        let salt = security::generate_salt();
        let hash = security::hash_password(account::DEFAULT_PASSWORD, &salt)?;
        let api_key = Uuid::new_v4().to_string();
        let encrypted = encryption::encrypt(&api_key, master_key)?;
        let api_key_hash = helper::sha256_hex(&api_key);
        sqlx::query(
            "INSERT INTO users (id, username, display_name, password_hash, api_key, api_key_hash, role) VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&user_id)
        .bind(account::DEFAULT_USERNAME)
        .bind("Admin")
        .bind(&hash)
        .bind(&encrypted)
        .bind(&api_key_hash)
        .bind("admin")
        .execute(pool)
        .await?;
    }

    Ok(())
}

pub async fn create_user(
    pool: &SqlitePool,
    username: &str,
    display_name: &str,
    password_hash: &str,
    api_key_encrypted: &str,
    api_key_hash: &str,
    role: &str,
) -> Result<String> {
    let user_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO users (id, username, display_name, password_hash, api_key, api_key_hash, role) VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&user_id)
    .bind(username)
    .bind(display_name)
    .bind(password_hash)
    .bind(api_key_encrypted)
    .bind(api_key_hash)
    .bind(role)
    .execute(pool)
    .await?;
    Ok(user_id)
}

pub async fn change_password(pool: &SqlitePool, user_id: &str, new_hash: &str) -> Result<()> {
    sqlx::query("UPDATE users SET password_hash = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = ?")
        .bind(new_hash)
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn change_username(pool: &SqlitePool, user_id: &str, new_username: &str) -> Result<()> {
    sqlx::query("UPDATE users SET username = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = ?")
        .bind(new_username)
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn change_role(pool: &SqlitePool, user_id: &str, role: &str) -> Result<()> {
    sqlx::query("UPDATE users SET role = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = ?")
        .bind(role)
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn change_display_name(pool: &SqlitePool, user_id: &str, display_name: &str) -> Result<()> {
    sqlx::query("UPDATE users SET display_name = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = ?")
        .bind(display_name)
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn delete_user(pool: &SqlitePool, user_id: &str) -> Result<bool> {
    let result = sqlx::query("DELETE FROM users WHERE id = ?")
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn get_user_by_username(
    pool: &SqlitePool,
    username: &str,
    master_key: &str,
) -> Result<Option<crate::signals::User>> {
    use crate::signals::User;
    let row: Option<(String, String, String, String, String, String, String, String)> = sqlx::query_as(
        "SELECT id, username, display_name, password_hash, api_key, role, created_at, updated_at FROM users WHERE username = ?",
    )
    .bind(username)
    .fetch_optional(pool)
    .await?;
    match row {
        Some((
            id,
            username,
            display_name,
            password_hash,
            api_key,
            role,
            created_at,
            updated_at,
        )) => {
            let api_key = if !api_key.is_empty() && api_key.starts_with("NDK:") {
                encryption::decrypt(&api_key, master_key).unwrap_or(api_key)
            } else {
                api_key
            };
            Ok(Some(User {
                id,
                username,
                display_name,
                password_hash,
                api_key,
                role,
                created_at,
                updated_at,
            }))
        }
        None => Ok(None),
    }
}

pub async fn get_user_id_by_username(pool: &SqlitePool, username: &str) -> Result<Option<String>> {
    let user_id: Option<String> = sqlx::query_scalar("SELECT id FROM users WHERE username = ?")
        .bind(username)
        .fetch_optional(pool)
        .await?;
    Ok(user_id)
}

pub async fn get_user_role(pool: &SqlitePool, user_id: &str) -> Result<Option<String>> {
    let role: Option<String> = sqlx::query_scalar("SELECT role FROM users WHERE id = ?")
        .bind(user_id)
        .fetch_optional(pool)
        .await?;
    Ok(role)
}

pub async fn get_user_id_by_api_key_hash(pool: &SqlitePool, hash: &str) -> Result<Option<String>> {
    let user_id: Option<String> = sqlx::query_scalar("SELECT id FROM users WHERE api_key_hash = ?")
        .bind(hash)
        .fetch_optional(pool)
        .await?;
    Ok(user_id)
}

pub async fn get_user_by_id(
    pool: &SqlitePool,
    user_id: &str,
    master_key: &str,
) -> Result<Option<crate::signals::User>> {
    use crate::signals::User;
    let row: Option<(String, String, String, String, String, String, String, String)> = sqlx::query_as(
        "SELECT id, username, display_name, password_hash, api_key, role, created_at, updated_at FROM users WHERE id = ?",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?;
    match row {
        Some((
            id,
            username,
            display_name,
            password_hash,
            api_key,
            role,
            created_at,
            updated_at,
        )) => {
            let api_key = if !api_key.is_empty() && api_key.starts_with("NDK:") {
                encryption::decrypt(&api_key, master_key).unwrap_or(api_key)
            } else {
                api_key
            };
            Ok(Some(User {
                id,
                username,
                display_name,
                password_hash,
                api_key,
                role,
                created_at,
                updated_at,
            }))
        }
        None => Ok(None),
    }
}

pub async fn get_user_api_key(
    pool: &SqlitePool,
    user_id: &str,
    master_key: &str,
) -> Result<Option<String>> {
    let stored: Option<String> = sqlx::query_scalar("SELECT api_key FROM users WHERE id = ?")
        .bind(user_id)
        .fetch_optional(pool)
        .await?;
    match stored {
        Some(val) if !val.is_empty() && val.starts_with("NDK:") => {
            encryption::decrypt(&val, master_key)
                .map(Some)
                .or(Ok(Some(val)))
        }
        other => Ok(other),
    }
}

pub async fn regenerate_user_api_key(
    pool: &SqlitePool,
    user_id: &str,
    master_key: &str,
) -> Result<String> {
    let new_key = Uuid::new_v4().to_string();
    let encrypted = encryption::encrypt(&new_key, master_key)?;
    let api_key_hash = helper::sha256_hex(&new_key);
    sqlx::query(
        "UPDATE users SET api_key = ?, api_key_hash = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = ?",
    )
    .bind(&encrypted)
    .bind(&api_key_hash)
    .bind(user_id)
    .execute(pool)
    .await?;
    Ok(new_key)
}

pub async fn get_all_users(pool: &SqlitePool) -> Result<Vec<UserListItem>> {
    let rows: Vec<(String, String, String, String)> =
        sqlx::query_as("SELECT id, username, display_name, role FROM users ORDER BY username")
            .fetch_all(pool)
            .await?;
    Ok(rows
        .into_iter()
        .map(|(id, username, display_name, role)| UserListItem {
            id,
            username,
            display_name,
            role,
        })
        .collect())
}
