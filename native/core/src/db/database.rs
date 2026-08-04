use std::{
    path::PathBuf,
    sync::{Arc, Weak},
};
use tokio::sync::Notify;

use crate::{app_context::AppContext, utils::logger};

#[derive(Debug)]
pub enum DatabasePool {
    Sqlite(sqlx::Pool<sqlx::Sqlite>),
    Postgres(sqlx::Pool<sqlx::Postgres>),
}

#[derive(Debug)]
pub struct DatabaseManager {
    pool: DatabasePool,
    context: Weak<AppContext>,
    shutdown_signal: Arc<Notify>,
    db_done_signal: Arc<Notify>,
}

impl DatabaseManager {
    pub async fn new(
        pool: DatabasePool,
        context: Weak<AppContext>,
        shutdown_signal: Arc<Notify>,
        db_done_signal: Arc<Notify>,
    ) -> Arc<Self> {
        Arc::new(Self {
            pool,
            context,
            shutdown_signal,
            db_done_signal,
        })
    }

    pub async fn init_db(db_url: &str) -> anyhow::Result<DatabasePool> {
        if db_url.starts_with("postgres:") || db_url.starts_with("postgresql:") {
            let pool = sqlx::postgres::PgPoolOptions::new()
                .max_connections(15)
                .connect(db_url)
                .await?;
            sqlx::migrate!("./migrations_pg").run(&pool).await?;
            Ok(DatabasePool::Postgres(pool))
        } else {
            let db_path = PathBuf::from(
                db_url
                    .trim_start_matches("sqlite://")
                    .trim_start_matches("sqlite:"),
            );
            if let Some(parent) = db_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            if !db_path.exists()
                && let Err(e) = std::fs::File::create(&db_path)
            {
                logger::error(&format!(
                    "Failed to create db file at {}: {}",
                    db_path.display(),
                    e
                ));
                return Err(e.into());
            }
            let sql_url = format!("sqlite://{}", db_path.display());
            logger::debug(&format!("SQL URL: {}", &sql_url));
            let pool = sqlx::sqlite::SqlitePoolOptions::new()
                .max_connections(10)
                .after_connect(|conn, _metadata| {
                    Box::pin(async move {
                        sqlx::query("PRAGMA journal_mode=WAL")
                            .execute(&mut *conn)
                            .await?;
                        sqlx::query("PRAGMA busy_timeout=5000")
                            .execute(&mut *conn)
                            .await?;
                        sqlx::query("PRAGMA foreign_keys=ON")
                            .execute(&mut *conn)
                            .await?;
                        Ok(())
                    })
                })
                .connect(&sql_url)
                .await?;
            logger::debug("SQL Connected");
            sqlx::migrate!("./migrations").run(&pool).await?;
            logger::debug("SQL Migrated");
            Ok(DatabasePool::Sqlite(pool))
        }
    }

    pub fn pool(&self) -> &DatabasePool {
        &self.pool
    }

    pub async fn run_loop(self: Arc<Self>) {
        use tokio::time::{Duration, sleep};
        loop {
            tokio::select! {
                _ = sleep(Duration::from_secs(5)) => {}
                _ = self.shutdown_signal.notified() => {
                    self.db_done_signal.notify_waiters();
                    break;
                }
            }
        }
    }
}
