use sqlx::PgPool;

pub async fn set_setting(
    pool: &PgPool,
    user_id: &str,
    key: &str,
    value: &str,
) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO user_settings (user_id, key, value) VALUES ($1, $2, $3)
         ON CONFLICT(user_id, key) DO UPDATE SET value = EXCLUDED.value, updated_at = NOW()",
    )
    .bind(user_id)
    .bind(key)
    .bind(value)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_setting(pool: &PgPool, user_id: &str, key: &str) -> Option<String> {
    sqlx::query_scalar::<_, String>(
        "SELECT value FROM user_settings WHERE user_id = $1 AND key = $2",
    )
    .bind(user_id)
    .bind(key)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
    .filter(|v| !v.is_empty())
}

pub async fn get_all_settings(pool: &PgPool, user_id: &str) -> Vec<(String, String)> {
    sqlx::query_as::<_, (String, String)>("SELECT key, value FROM user_settings WHERE user_id = $1")
        .bind(user_id)
        .fetch_all(pool)
        .await
        .unwrap_or_default()
}
