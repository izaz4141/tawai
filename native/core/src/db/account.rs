use crate::db::database::DatabasePool;
use crate::signals::account::UserListItem;

pub const DEFAULT_USERNAME: &str = "admin";
pub const DEFAULT_PASSWORD: &str = "admin";

pub async fn ensure_default_user(pool: &DatabasePool, master_key: &str) -> anyhow::Result<()> {
    match pool {
        DatabasePool::Sqlite(p) => super::account_sq::ensure_default_user(p, master_key).await,
        DatabasePool::Postgres(p) => super::account_pg::ensure_default_user(p, master_key).await,
    }
}

pub async fn create_user(
    pool: &DatabasePool,
    username: &str,
    display_name: &str,
    password_hash: &str,
    api_key_encrypted: &str,
    api_key_hash: &str,
    role: &str,
) -> anyhow::Result<String> {
    match pool {
        DatabasePool::Sqlite(p) => {
            super::account_sq::create_user(
                p,
                username,
                display_name,
                password_hash,
                api_key_encrypted,
                api_key_hash,
                role,
            )
            .await
        }
        DatabasePool::Postgres(p) => {
            super::account_pg::create_user(
                p,
                username,
                display_name,
                password_hash,
                api_key_encrypted,
                api_key_hash,
                role,
            )
            .await
        }
    }
}

pub async fn change_password(
    pool: &DatabasePool,
    user_id: &str,
    new_hash: &str,
) -> anyhow::Result<()> {
    match pool {
        DatabasePool::Sqlite(p) => super::account_sq::change_password(p, user_id, new_hash).await,
        DatabasePool::Postgres(p) => super::account_pg::change_password(p, user_id, new_hash).await,
    }
}

pub async fn change_username(
    pool: &DatabasePool,
    user_id: &str,
    new_username: &str,
) -> anyhow::Result<()> {
    match pool {
        DatabasePool::Sqlite(p) => {
            super::account_sq::change_username(p, user_id, new_username).await
        }
        DatabasePool::Postgres(p) => {
            super::account_pg::change_username(p, user_id, new_username).await
        }
    }
}

pub async fn change_role(pool: &DatabasePool, user_id: &str, role: &str) -> anyhow::Result<()> {
    match pool {
        DatabasePool::Sqlite(p) => super::account_sq::change_role(p, user_id, role).await,
        DatabasePool::Postgres(p) => super::account_pg::change_role(p, user_id, role).await,
    }
}

pub async fn change_display_name(
    pool: &DatabasePool,
    user_id: &str,
    display_name: &str,
) -> anyhow::Result<()> {
    match pool {
        DatabasePool::Sqlite(p) => {
            super::account_sq::change_display_name(p, user_id, display_name).await
        }
        DatabasePool::Postgres(p) => {
            super::account_pg::change_display_name(p, user_id, display_name).await
        }
    }
}

pub async fn delete_user(pool: &DatabasePool, user_id: &str) -> anyhow::Result<bool> {
    match pool {
        DatabasePool::Sqlite(p) => super::account_sq::delete_user(p, user_id).await,
        DatabasePool::Postgres(p) => super::account_pg::delete_user(p, user_id).await,
    }
}

pub async fn get_user_id_by_username(
    pool: &DatabasePool,
    username: &str,
) -> anyhow::Result<Option<String>> {
    match pool {
        DatabasePool::Sqlite(p) => super::account_sq::get_user_id_by_username(p, username).await,
        DatabasePool::Postgres(p) => super::account_pg::get_user_id_by_username(p, username).await,
    }
}

pub async fn get_user_role(pool: &DatabasePool, user_id: &str) -> anyhow::Result<Option<String>> {
    match pool {
        DatabasePool::Sqlite(p) => super::account_sq::get_user_role(p, user_id).await,
        DatabasePool::Postgres(p) => super::account_pg::get_user_role(p, user_id).await,
    }
}

pub async fn get_user_by_id(
    pool: &DatabasePool,
    user_id: &str,
    master_key: &str,
) -> anyhow::Result<Option<crate::signals::User>> {
    match pool {
        DatabasePool::Sqlite(p) => super::account_sq::get_user_by_id(p, user_id, master_key).await,
        DatabasePool::Postgres(p) => {
            super::account_pg::get_user_by_id(p, user_id, master_key).await
        }
    }
}

pub async fn get_user_by_username(
    pool: &DatabasePool,
    username: &str,
    master_key: &str,
) -> anyhow::Result<Option<crate::signals::User>> {
    match pool {
        DatabasePool::Sqlite(p) => {
            super::account_sq::get_user_by_username(p, username, master_key).await
        }
        DatabasePool::Postgres(p) => {
            super::account_pg::get_user_by_username(p, username, master_key).await
        }
    }
}

pub async fn get_user_id_by_api_key(
    pool: &DatabasePool,
    api_key: &str,
) -> anyhow::Result<Option<String>> {
    let hash = crate::utils::helper::sha256_hex(api_key);
    match pool {
        DatabasePool::Sqlite(p) => super::account_sq::get_user_id_by_api_key_hash(p, &hash).await,
        DatabasePool::Postgres(p) => super::account_pg::get_user_id_by_api_key_hash(p, &hash).await,
    }
}

pub async fn get_user_api_key(
    pool: &DatabasePool,
    user_id: &str,
    master_key: &str,
) -> anyhow::Result<Option<String>> {
    match pool {
        DatabasePool::Sqlite(p) => {
            super::account_sq::get_user_api_key(p, user_id, master_key).await
        }
        DatabasePool::Postgres(p) => {
            super::account_pg::get_user_api_key(p, user_id, master_key).await
        }
    }
}

pub async fn regenerate_user_api_key(
    pool: &DatabasePool,
    user_id: &str,
    master_key: &str,
) -> anyhow::Result<String> {
    match pool {
        DatabasePool::Sqlite(p) => {
            super::account_sq::regenerate_user_api_key(p, user_id, master_key).await
        }
        DatabasePool::Postgres(p) => {
            super::account_pg::regenerate_user_api_key(p, user_id, master_key).await
        }
    }
}

pub async fn get_all_users(pool: &DatabasePool) -> anyhow::Result<Vec<UserListItem>> {
    match pool {
        DatabasePool::Sqlite(p) => super::account_sq::get_all_users(p).await,
        DatabasePool::Postgres(p) => super::account_pg::get_all_users(p).await,
    }
}

pub async fn get_all_users_with_keys(
    pool: &DatabasePool,
    master_key: &str,
) -> anyhow::Result<Vec<UserListItem>> {
    match pool {
        DatabasePool::Sqlite(p) => super::account_sq::get_all_users_with_keys(p, master_key).await,
        DatabasePool::Postgres(p) => {
            super::account_pg::get_all_users_with_keys(p, master_key).await
        }
    }
}
