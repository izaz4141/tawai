use std::collections::HashSet;
use std::fmt;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::time::Duration;

use anyhow::Result;
use tokio::sync::{Notify, RwLock, watch};

use crate::audio;
use crate::db::{account, database::DatabaseManager, library_source, user_settings};
use crate::discovery::sync::{SyncRecsParams, sync_recs};
use crate::signals::library::ScanProgress;
use crate::utils::config::{
    AppConfig, DEFAULT_SETTINGS, config_file_mtime, get_path, get_path_mut, is_local_host,
    is_version_mismatch, load_config, load_default_config, merge_into, normalize_secret,
    path_is_secret, readonly_default_config, remove_path, set_path, strip_paths,
};
use crate::utils::{cache, encryption::encrypt, encryption::valid_encryption_format, logger};

#[derive(Clone)]
pub struct AppContext {
    db: Arc<RwLock<Option<Arc<DatabaseManager>>>>,
    client: reqwest::Client,
    config: Arc<RwLock<Option<Arc<AppConfig>>>>,
    pub shutdown_signal: Arc<Notify>,
    pub master_key: Arc<RwLock<String>>,
    pub scan_running: Arc<AtomicBool>,
    pub scan_progress: Arc<RwLock<Option<ScanProgress>>>,
}

impl fmt::Debug for AppContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AppContext")
            .field("shutdown_signal", &self.shutdown_signal)
            .finish()
    }
}

impl AppContext {
    pub fn new(shutdown_signal: Arc<Notify>, client: reqwest::Client) -> Arc<Self> {
        Arc::new(AppContext {
            db: Arc::new(RwLock::new(None)),
            client,
            config: Arc::new(RwLock::new(None)),
            shutdown_signal,
            master_key: Arc::new(RwLock::new(String::new())),
            scan_running: Arc::new(AtomicBool::new(false)),
            scan_progress: Arc::new(RwLock::new(None)),
        })
    }

    pub fn client(&self) -> &reqwest::Client {
        &self.client
    }

    pub async fn db(&self) -> Arc<DatabaseManager> {
        self.db
            .read()
            .await
            .as_ref()
            .unwrap_or_else(|| {
                let bt = std::backtrace::Backtrace::force_capture();
                panic!("DatabaseManager not initialized; called from:\n{bt}")
            })
            .clone()
    }

    pub async fn cfg(&self) -> Arc<AppConfig> {
        let current_config = self
            .config
            .read()
            .await
            .as_ref()
            .unwrap_or_else(|| {
                let bt = std::backtrace::Backtrace::force_capture();
                panic!("No config loaded; called from:\n{bt}")
            })
            .clone();
        let current_path = current_config.path.clone();
        let current_mtime = current_config.mtime;

        let observed_mtime = config_file_mtime(&current_path).await;

        let need_reload = match observed_mtime {
            Some(mtime) => mtime != current_mtime,
            None => {
                // The rejected file is gone (or unreadable). Forget the cached
                // rejection so a file that reappears is loaded again.
                if !current_config.writable && current_mtime != 0 {
                    let refreshed = readonly_default_config(
                        current_path.to_string_lossy().into_owned(),
                        0,
                    );
                    *self.config.write().await = Some(refreshed.clone());
                    return refreshed;
                }
                false
            }
        };

        if !need_reload {
            return current_config;
        }

        let mut write = self.config.write().await;
        match load_config(
            current_path
                .to_str()
                .unwrap_or_else(|| {
                    let bt = std::backtrace::Backtrace::force_capture();
                    panic!("Invalid config path character; called from:\n{bt}")
                })
                .to_string(),
            self.master_key.read().await.clone(),
        )
        .await
        {
            Ok(new_config) => {
                *write = Some(new_config.clone());
                new_config
            }
            Err(e) => {
                logger::error(&format!("Failed to reload config: {}", e));
                if is_version_mismatch(&e) {
                    // Never keep retrying a file that is still on another
                    // schema version; remember its mtime and fall back to
                    // defaults until the file changes or disappears.
                    let rejected = readonly_default_config(
                        current_path.to_string_lossy().into_owned(),
                        observed_mtime.unwrap_or(0),
                    );
                    *write = Some(rejected.clone());
                    rejected
                } else {
                    current_config // fallback to old config
                }
            }
        }
    }

    /// Loads the config file at `path`, or bootstraps a default config when the
    /// file is missing. Returns `true` when a default config was created
    /// (first run).
    pub async fn init_config(&self, path: String) -> Result<bool> {
        let (app_config, is_first_run) = match load_config(
            path.clone(),
            self.master_key.read().await.clone(),
        )
        .await
        {
            Ok(aconfig) => (aconfig, false),
            Err(e) if is_version_mismatch(&e) => {
                // The document belongs to another schema version. Start on
                // in-memory defaults, but never write them over the file.
                // Remember the rejected file's mtime so reloads are skipped
                // until the file changes or disappears.
                logger::error(&format!(
                    "Refusing to load config: {e}. Running on defaults; the file will not be written."
                ));
                let mtime = config_file_mtime(Path::new(&path)).await.unwrap_or(0);
                (readonly_default_config(path.clone(), mtime), false)
            }
            Err(e) => {
                logger::warn(&format!("Cant load provided config: {}", e.to_string()));
                let aconfig = match load_default_config(path.clone()).await {
                    Ok(c) => c,
                    Err(e) => {
                        logger::error(&format!(
                            "Cant load default config: {e}\n ConfigPath: {path}"
                        ));
                        anyhow::bail!("Cant load default config: {e}\n ConfigPath: {path}")
                    }
                };
                (aconfig, true)
            }
        };
        *self.config.write().await = Some(app_config);
        if let Some(db) = self.db.read().await.as_ref().cloned() {
            let pool = db.pool().clone();
            let cfg = self.cfg().await;
            tokio::spawn(async move {
                if let Err(e) = cache::cleanup_dash_cache(&pool, &cfg).await {
                    logger::warn(&format!("Failed to clean up dash cache: {}", e));
                }
            });
        }
        if let Err(e) = self.sync_accounts_from_db().await {
            logger::warn(&format!("Failed to sync accounts from db: {}", e));
        }
        Ok(is_first_run)
    }

    /// Syncs localhost accounts in config.json with the users table in the DB.
    /// The DB is the source of truth: localhost accounts whose `id` no longer
    /// exists in the DB are removed, and DB users without a matching localhost
    /// account are added. Existing localhost accounts get their username,
    /// display name, role and api key refreshed from the DB while preserving
    /// their host, port and label. Non-localhost accounts are left untouched.
    pub async fn sync_accounts_from_db(&self) -> Result<()> {
        let db = match self.db.read().await.as_ref() {
            Some(db) => db.clone(),
            None => {
                logger::warn("Database not initialized; skipping account sync");
                return Ok(());
            }
        };

        let master_key = self.master_key.read().await.clone();
        let users = account::get_all_users_with_keys(db.pool(), &master_key).await?;
        let mut cfg = self.cfg().await.value.clone();

        let server_host = get_path(&cfg, "server.host")
            .and_then(|v| v.as_str())
            .unwrap_or("127.0.0.1")
            .to_string();
        let server_port = get_path(&cfg, "server.port")
            .and_then(|v| v.as_u64())
            .unwrap_or(8181);

        let mut accounts: Vec<serde_json::Value> = get_path(&cfg, "accounts.saved")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        let db_ids: HashSet<&str> = users.iter().map(|u| u.id.as_str()).collect();
        accounts.retain(|acc| {
            let host = acc["host"].as_str().unwrap_or("");
            if !is_local_host(host) {
                return true;
            }
            match acc["id"].as_str() {
                Some(id) => db_ids.contains(id),
                None => false,
            }
        });

        for user in &users {
            let mut found = None;
            for acc in accounts.iter_mut() {
                let host = acc["host"].as_str().unwrap_or("");
                if is_local_host(host) && acc["id"].as_str() == Some(user.id.as_str()) {
                    acc["username"] = serde_json::Value::from(user.username.clone());
                    acc["display_name"] = serde_json::Value::from(user.display_name.clone());
                    acc["role"] = serde_json::Value::from(user.role.clone());
                    acc["api_key"] = serde_json::Value::from(user.api_key.clone());
                    found = Some(());
                    break;
                }
            }
            if found.is_none() {
                accounts.push(serde_json::json!({
                    "id": user.id,
                    "host": server_host,
                    "port": server_port,
                    "username": user.username,
                    "display_name": user.display_name,
                    "label": format!("{}@localhost", user.username),
                    "role": user.role,
                    "api_key": user.api_key,
                }));
            }
        }

        set_path(
            &mut cfg,
            "accounts.saved",
            serde_json::Value::Array(accounts),
        );
        self.save_config(&cfg).await
    }

    /// Returns the current config with secrets stripped, suitable for sending
    /// to the client.
    pub async fn client_config(&self) -> serde_json::Value {
        strip_paths(&self.cfg().await.value, path_is_secret)
    }

    /// Merges `partial` into the current config and persists it.
    ///
    /// Objects merge recursively, arrays replace wholesale, and `null` resets a
    /// path to its schema default. An empty string for a secret path is
    /// dropped, so a partial save can't wipe a stored credential.
    pub async fn update_config(&self, partial: &serde_json::Value) -> Result<()> {
        let current = self.cfg().await.value.clone();

        let mut incoming = partial.clone();
        for (path, spec) in &crate::utils::config::SCHEMA.entries {
            if !spec.secret {
                continue;
            }
            let empty = get_path(&incoming, path)
                .and_then(|v| v.as_str())
                .map(|s| s.trim().is_empty())
                .unwrap_or(false);
            if empty {
                remove_path(&mut incoming, path);
            }
        }

        let mut cfg = merge_into(&current, &incoming, &DEFAULT_SETTINGS);

        // When the server port changes on a local host, keep every local
        // account's port in sync so saved accounts still match the server.
        if let Some(new_port) = get_path(&incoming, "server.port").and_then(|v| v.as_u64()) {
            let host = get_path(&incoming, "server.host")
                .and_then(|v| v.as_str())
                .map(str::to_string)
                .or_else(|| {
                    get_path(&current, "server.host")
                        .and_then(|v| v.as_str())
                        .map(str::to_string)
                })
                .unwrap_or_default();
            let old_port = get_path(&current, "server.port")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            if is_local_host(&host) && new_port != old_port {
                if let Some(accounts) =
                    get_path_mut(&mut cfg, "accounts.saved").and_then(|v| v.as_array_mut())
                {
                    for acc in accounts.iter_mut() {
                        if let Some(acc_host) = acc.get("host").and_then(|v| v.as_str()) {
                            if is_local_host(acc_host) {
                                acc["port"] = serde_json::Value::from(new_port);
                            }
                        }
                    }
                }
            }
        }

        self.save_config(&cfg).await
    }

    pub async fn save_config(&self, settings: &serde_json::Value) -> Result<()> {
        let app_config = self
            .config
            .read()
            .await
            .clone()
            .ok_or_else(|| anyhow::anyhow!("Config not initialized"))?;

        if !app_config.writable {
            logger::warn(
                "Config is read-only because it belongs to another schema version; skipping save",
            );
            return Ok(());
        }

        let mut new_settings = settings.clone();
        let master_key = self.master_key.read().await.clone();
        for path in [
            "integrations.slskd.api_key",
            "integrations.nadekodon.api_key",
        ] {
            let stored = get_path(settings, path)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let plain = normalize_secret(&stored).to_string();
            if valid_encryption_format(&plain) {
                set_path(&mut new_settings, path, serde_json::Value::String(plain));
                continue;
            }
            if let Ok(encrypted) = encrypt(plain.as_str(), master_key.as_str()) {
                set_path(
                    &mut new_settings,
                    path,
                    serde_json::Value::String(encrypted),
                );
            }
        }

        if let Some(accounts) =
            get_path_mut(&mut new_settings, "accounts.saved").and_then(|v| v.as_array_mut())
        {
            for acc in accounts.iter_mut() {
                if let Some(api_key) = acc.get("api_key").and_then(|v| v.as_str()) {
                    let dapi = normalize_secret(api_key).to_string();
                    if dapi.is_empty() || valid_encryption_format(&dapi) {
                        continue;
                    }
                    if let Ok(eapi) = encrypt(dapi.as_str(), master_key.as_str()) {
                        acc["api_key"] = serde_json::Value::String(eapi);
                    }
                }
            }
        }

        let path = app_config.path.clone();
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        let json_str = serde_json::to_string_pretty(&new_settings)?;
        tokio::fs::write(&path, &json_str).await?;

        // Update in-memory cache so subsequent cfg() calls see the new values
        let new_config = load_config(
            path.to_str()
                .ok_or_else(|| anyhow::anyhow!("[Save Config] Invalid Config Path"))?
                .to_string(),
            self.master_key.read().await.clone(),
        )
        .await?;
        *self.config.write().await = Some(new_config);
        Ok(())
    }

    pub async fn init_database(
        self: &Arc<Self>,
        db_url: &str,
        db_done_signal: Arc<Notify>,
        master_key: &str,
    ) -> Result<()> {
        let pool = DatabaseManager::init_db(db_url).await?;
        let weak_ctx = Arc::downgrade(self);
        let db = DatabaseManager::new(pool, weak_ctx, self.shutdown_signal.clone(), db_done_signal)
            .await;

        *self.db.write().await = Some(db.clone());
        *self.master_key.write().await = master_key.to_owned();

        account::ensure_default_user(db.pool(), master_key).await?;

        Ok(())
    }

    pub async fn run_database_loop(self: Arc<Self>) {
        let db = self.db().await;
        db.run_loop().await;
    }

    pub fn spawn_periodic_scan(
        self: &Arc<Self>,
        interval: Duration,
        on_progress: Arc<dyn Fn(ScanProgress) + Send + Sync + 'static>,
    ) -> tokio::task::JoinHandle<()> {
        let ctx = self.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(interval).await;

                let db = ctx.db().await;
                let scan_mk = ctx.master_key.read().await.clone();
                let sources = library_source::list_all_sources(db.pool(), &scan_mk)
                    .await
                    .unwrap_or_default();

                if sources.is_empty() {
                    continue;
                }

                if ctx.scan_running.swap(true, Ordering::SeqCst) {
                    continue;
                }

                *ctx.scan_progress.write().await = None;

                let client = ctx.client().clone();
                let sync_client = client.clone();
                let sources2 = sources.clone();
                let inner_ctx = ctx.clone();
                let cb = on_progress.clone();
                let db_inner = db.clone();

                tokio::spawn(async move {
                    let pool = db_inner.pool();
                    let (tx, mut rx) = watch::channel(ScanProgress::default());
                    let inner_ctx2 = inner_ctx.clone();

                    tokio::spawn(async move {
                        while rx.changed().await.is_ok() {
                            let p = rx.borrow_and_update().clone();
                            *inner_ctx2.scan_progress.write().await = Some(p.clone());
                            cb(p);
                        }
                    });

                    let mk_scan = inner_ctx.master_key.read().await.clone();
                    let result = audio::scan::run_scan(
                        &pool,
                        client,
                        &sources2,
                        false,
                        Some(tx),
                        &mk_scan,
                        None,
                        Arc::downgrade(&inner_ctx),
                    )
                    .await;

                    *inner_ctx.scan_progress.write().await = Some(ScanProgress {
                        complete: true,
                        tracks_found: result.tracks_found,
                        new_tracks: result.new_tracks,
                        duplicates: result.duplicates,
                        deleted: result.deleted,
                        error: result.error.clone(),
                        ..Default::default()
                    });

                    inner_ctx.scan_running.store(false, Ordering::SeqCst);

                    // Sync stale recommendation sources for all users
                    let mk = inner_ctx.master_key.read().await.clone();
                    if let Ok(users) = account::get_all_users(pool).await {
                        for user in &users {
                            let included = user_settings::get_user_setting(
                                pool,
                                &user.id,
                                "included_recommendations",
                                &mk,
                            )
                            .await
                            .unwrap_or_default();
                            let _ = sync_recs(SyncRecsParams {
                                pool,
                                client: &sync_client,
                                master_key: &mk,
                                user_id: &user.id,
                                included_keys: &included,
                                context: Arc::downgrade(&inner_ctx),
                            })
                            .await;
                        }
                    }

                    logger::info(&format!(
                        "Periodic scan complete: {} found, {} new, {} deleted",
                        result.tracks_found, result.new_tracks, result.deleted,
                    ));
                });
            }
        })
    }

    pub async fn shutdown(&self) {
        self.shutdown_signal.notify_waiters();
    }
}
