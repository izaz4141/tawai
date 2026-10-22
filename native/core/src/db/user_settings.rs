use std::collections::HashMap;

use crate::db::database::DatabasePool;
use crate::utils::config::{decode_user_setting, encode_user_setting};

/// Stores a per-user setting, encrypting the value first when the key holds a
/// secret.
pub async fn set_user_setting(
    pool: &DatabasePool,
    user_id: &str,
    key: &str,
    value: &str,
    master_key: &str,
) -> anyhow::Result<()> {
    let stored = encode_user_setting(key, value, master_key);
    set_setting(pool, user_id, key, &stored).await
}

/// Reads a single per-user setting, decrypting the value when the key holds a
/// secret. Returns `None` when the setting is not set.
pub async fn get_user_setting(
    pool: &DatabasePool,
    user_id: &str,
    key: &str,
    master_key: &str,
) -> Option<String> {
    let raw = get_setting(pool, user_id, key).await?;
    Some(decode_user_setting(key, &raw, master_key))
}

/// Reads every per-user setting of a user, decrypting secret values.
pub async fn get_all_user_settings(
    pool: &DatabasePool,
    user_id: &str,
    master_key: &str,
) -> HashMap<String, String> {
    get_all_settings(pool, user_id)
        .await
        .into_iter()
        .map(|(k, v)| {
            let v = decode_user_setting(&k, &v, master_key);
            (k, v)
        })
        .collect()
}

/// Raw storage access. Does not apply any secret handling, use the functions
/// above instead.
pub(crate) async fn set_setting(
    pool: &DatabasePool,
    user_id: &str,
    key: &str,
    value: &str,
) -> anyhow::Result<()> {
    match pool {
        DatabasePool::Sqlite(p) => {
            super::user_settings_sq::set_setting(p, user_id, key, value).await
        }
        DatabasePool::Postgres(p) => {
            super::user_settings_pg::set_setting(p, user_id, key, value).await
        }
    }
}

/// Raw storage access. Does not apply any secret handling, use
/// `get_user_setting` instead.
pub(crate) async fn get_setting(pool: &DatabasePool, user_id: &str, key: &str) -> Option<String> {
    match pool {
        DatabasePool::Sqlite(p) => super::user_settings_sq::get_setting(p, user_id, key).await,
        DatabasePool::Postgres(p) => super::user_settings_pg::get_setting(p, user_id, key).await,
    }
}

/// Raw storage access. Does not apply any secret handling, use
/// `get_all_user_settings` instead.
pub(crate) async fn get_all_settings(pool: &DatabasePool, user_id: &str) -> Vec<(String, String)> {
    match pool {
        DatabasePool::Sqlite(p) => super::user_settings_sq::get_all_settings(p, user_id).await,
        DatabasePool::Postgres(p) => super::user_settings_pg::get_all_settings(p, user_id).await,
    }
}
