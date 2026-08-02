use crate::db::database::DatabasePool;

pub async fn set_setting(
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

pub async fn get_setting(pool: &DatabasePool, user_id: &str, key: &str) -> Option<String> {
    match pool {
        DatabasePool::Sqlite(p) => super::user_settings_sq::get_setting(p, user_id, key).await,
        DatabasePool::Postgres(p) => super::user_settings_pg::get_setting(p, user_id, key).await,
    }
}

pub async fn get_all_settings(pool: &DatabasePool, user_id: &str) -> Vec<(String, String)> {
    match pool {
        DatabasePool::Sqlite(p) => super::user_settings_sq::get_all_settings(p, user_id).await,
        DatabasePool::Postgres(p) => super::user_settings_pg::get_all_settings(p, user_id).await,
    }
}
