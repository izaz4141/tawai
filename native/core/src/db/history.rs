use crate::db::database::DatabasePool;
use crate::signals::playback::PlaybackRecord;

pub async fn record_playback(
    pool: &DatabasePool,
    user_id: &str,
    track_id: &str,
    source: &str,
) -> anyhow::Result<String> {
    match pool {
        DatabasePool::Sqlite(p) => {
            super::history_sq::record_playback(p, user_id, track_id, source).await
        }
        DatabasePool::Postgres(p) => {
            super::history_pg::record_playback(p, user_id, track_id, source).await
        }
    }
}

pub async fn get_recent_history(
    pool: &DatabasePool,
    user_id: &str,
    limit: i32,
) -> anyhow::Result<Vec<PlaybackRecord>> {
    match pool {
        DatabasePool::Sqlite(p) => super::history_sq::get_recent_history(p, user_id, limit).await,
        DatabasePool::Postgres(p) => super::history_pg::get_recent_history(p, user_id, limit).await,
    }
}

pub async fn mark_scrobbled(pool: &DatabasePool, history_id: &str) -> anyhow::Result<()> {
    match pool {
        DatabasePool::Sqlite(p) => super::history_sq::mark_scrobbled(p, history_id).await,
        DatabasePool::Postgres(p) => super::history_pg::mark_scrobbled(p, history_id).await,
    }
}

pub async fn get_listenbrainz_token(
    pool: &DatabasePool,
    user_id: &str,
    master_key: &str,
) -> Option<String> {
    match pool {
        DatabasePool::Sqlite(p) => {
            super::history_sq::get_listenbrainz_token(p, user_id, master_key).await
        }
        DatabasePool::Postgres(p) => {
            super::history_pg::get_listenbrainz_token(p, user_id, master_key).await
        }
    }
}
