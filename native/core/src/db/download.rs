use crate::db::database::DatabasePool;
use crate::signals::DownloadRecord;

pub async fn insert_download(
    pool: &DatabasePool,
    user_id: &str,
    source: &str,
    source_id: &str,
    url: &str,
    dest_path: &str,
    filename: &str,
) -> anyhow::Result<String> {
    match pool {
        DatabasePool::Sqlite(p) => {
            super::download_sq::insert_download(
                p, user_id, source, source_id, url, dest_path, filename,
            )
            .await
        }
        DatabasePool::Postgres(p) => {
            super::download_pg::insert_download(
                p, user_id, source, source_id, url, dest_path, filename,
            )
            .await
        }
    }
}

pub async fn update_download_state(
    pool: &DatabasePool,
    id: &str,
    state: &str,
    error: &str,
    downloaded: i64,
    total_size: i64,
) -> anyhow::Result<()> {
    match pool {
        DatabasePool::Sqlite(p) => {
            super::download_sq::update_download_state(p, id, state, error, downloaded, total_size)
                .await
        }
        DatabasePool::Postgres(p) => {
            super::download_pg::update_download_state(p, id, state, error, downloaded, total_size)
                .await
        }
    }
}

pub async fn list_downloads(
    pool: &DatabasePool,
    user_id: &str,
    source: Option<&str>,
) -> anyhow::Result<Vec<DownloadRecord>> {
    match pool {
        DatabasePool::Sqlite(p) => super::download_sq::list_downloads(p, user_id, source).await,
        DatabasePool::Postgres(p) => super::download_pg::list_downloads(p, user_id, source).await,
    }
}

pub async fn get_download(pool: &DatabasePool, id: &str) -> anyhow::Result<Option<DownloadRecord>> {
    match pool {
        DatabasePool::Sqlite(p) => super::download_sq::get_download(p, id).await,
        DatabasePool::Postgres(p) => super::download_pg::get_download(p, id).await,
    }
}

pub async fn update_download_filename(
    pool: &DatabasePool,
    id: &str,
    filename: &str,
) -> anyhow::Result<()> {
    match pool {
        DatabasePool::Sqlite(p) => {
            super::download_sq::update_download_filename(p, id, filename).await
        }
        DatabasePool::Postgres(p) => {
            super::download_pg::update_download_filename(p, id, filename).await
        }
    }
}

pub async fn get_download_by_source(
    pool: &DatabasePool,
    source: &str,
    source_id: &str,
) -> anyhow::Result<Option<DownloadRecord>> {
    match pool {
        DatabasePool::Sqlite(p) => {
            super::download_sq::get_download_by_source(p, source, source_id).await
        }
        DatabasePool::Postgres(p) => {
            super::download_pg::get_download_by_source(p, source, source_id).await
        }
    }
}
