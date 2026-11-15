use std::{
    path::PathBuf,
    sync::{Arc, Weak},
};

use crate::{app_context::AppContext, utils::logger};

#[derive(Debug)]
pub enum DatabasePool {
    Sqlite(sqlx::Pool<sqlx::Sqlite>),
    Postgres(sqlx::Pool<sqlx::Postgres>),
}

impl DatabasePool {
    pub async fn close(&self) {
        match self {
            DatabasePool::Sqlite(pool) => {
                let _ = sqlx::query("PRAGMA wal_checkpoint(TRUNCATE)")
                    .execute(pool)
                    .await;
                pool.close().await;
            }
            DatabasePool::Postgres(pool) => {
                pool.close().await;
            }
        }
    }
}

#[derive(Debug)]
pub struct DatabaseManager {
    pub pool: Arc<DatabasePool>,
    context: Weak<AppContext>,
}

impl DatabaseManager {
    pub async fn new(pool: Arc<DatabasePool>, context: Weak<AppContext>) -> Arc<Self> {
        Arc::new(Self { pool, context })
    }

    pub fn pool(&self) -> &DatabasePool {
        &self.pool
    }

    pub fn context(&self) -> Arc<AppContext> {
        self.context.upgrade().unwrap_or_else(|| {
            let bt = std::backtrace::Backtrace::force_capture();
            panic!("AppContext missing from heap; called from:\n{bt}")
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

    pub async fn run_loop(self: Arc<Self>) {
        let shutdown_signal = self.context().operation_signals.shutdown_signal.clone();

        tokio::select! {
            _ = shutdown_signal.notified() => {
                self.flush_and_signal().await;
            }
        }
    }

    async fn flush_and_signal(&self) {
        self.pool.close().await;
        self.context().operation_signals.database_done.notify_one();
    }
}
