use sqlx::SqlitePool;

pub async fn set_setting(
    pool: &SqlitePool,
    user_id: &str,
    key: &str,
    value: &str,
) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO user_settings (user_id, key, value) VALUES (?, ?, ?)
         ON CONFLICT(user_id, key) DO UPDATE SET value = excluded.value, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')",
    )
    .bind(user_id)
    .bind(key)
    .bind(value)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_setting(pool: &SqlitePool, user_id: &str, key: &str) -> Option<String> {
    sqlx::query_scalar::<_, String>("SELECT value FROM user_settings WHERE user_id = ? AND key = ?")
        .bind(user_id)
        .bind(key)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
        .filter(|v| !v.is_empty())
}

pub async fn get_all_settings(pool: &SqlitePool, user_id: &str) -> Vec<(String, String)> {
    sqlx::query_as::<_, (String, String)>("SELECT key, value FROM user_settings WHERE user_id = ?")
        .bind(user_id)
        .fetch_all(pool)
        .await
        .unwrap_or_default()
}
