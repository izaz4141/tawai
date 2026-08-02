use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::sync::RwLock;

use tawai_core::app_context::AppContext;
use tawai_core::signals::library::ScanProgress;
use tawai_core::utils::logger;

use tawai_server::server;

#[tokio::main]
async fn main() {
    logger::debug("Initializing Tawai Server...");

    let config_path = server::get_config_path();

    let master_key =
        std::env::var("TAWAI_SERVER_MASTER_KEY").expect("TAWAI_SERVER_MASTER_KEY is not set");

    let shutdown_signal = Arc::new(tokio::sync::Notify::new());
    let client = tawai_core::utils::url::build_browser_client().await;

    let context = AppContext::new(shutdown_signal, client);

    // Initialize config
    if let Err(e) = context.init_config(config_path).await {
        logger::error(&format!("Failed to load config: {:?}", e));
    }

    // Initialize database
    let db_done_signal = Arc::new(tokio::sync::Notify::new());
    let db_url = std::env::var("TAWAI_DATABASE_URL")
        .unwrap_or_else(|_| format!("{}/data/tawai.db", server::tawai_home()));
    {
        let ctx = context.clone();
        let sig = db_done_signal.clone();
        if let Err(e) = ctx.init_database(&db_url, sig, &master_key).await {
            logger::error(&format!("Failed to start database manager: {:?}", e));
        }
        let loop_ctx = Arc::clone(&context);
        tokio::spawn(async move { loop_ctx.run_database_loop().await });
    }

    let port: u16 = std::env::var("TAWAI_SERVER_PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse()
        .unwrap_or(8080);

    // Set server_port in config
    let mut config_value = context.cfg().await.value.clone();
    config_value["server_port"] = serde_json::json!(port);

    // Inject optional downstream service connection details from the environment.
    // Environment variables override whatever is already stored in the config file.
    // save_config encrypts the slskd/nadekodon API keys with the master key.
    let env_overrides = [
        ("TAWAI_SLSKD_URL", "slskd_url"),
        ("TAWAI_SLSKD_API_KEY", "slskd_api_key"),
        ("TAWAI_NADEKODON_URL", "nadekodon_url"),
        ("TAWAI_NADEKODON_API_KEY", "nadekodon_api_key"),
    ];
    for (env_var, config_key) in env_overrides {
        if let Ok(value) = std::env::var(env_var)
            && !value.trim().is_empty()
        {
            config_value[config_key] = serde_json::json!(value.trim());
        }
    }

    let _ = context.save_config(&config_value).await;

    let state = Arc::new(server::AppState {
        context,
        restart_signal: Arc::new(tokio::sync::Notify::new()),
        shutdown_signal: Arc::new(tokio::sync::Notify::new()),
        shutdown_requested: Arc::new(AtomicBool::new(false)),
        version: Arc::new(RwLock::new(None)),
    });

    state.context.spawn_periodic_scan(
        Duration::from_secs(300),
        Arc::new(|p: ScanProgress| {
            if p.complete {
                logger::info(&format!(
                    "Periodic scan complete: {} files, {} new, {} deleted",
                    p.tracks_found, p.new_tracks, p.deleted,
                ));
            } else {
                logger::debug(&format!(
                    "Periodic scan {}: {}/{}",
                    p.stage, p.files_scanned, p.total_files,
                ));
            }
        }),
    );

    let state_clone = state.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        logger::debug("Shutdown signal received...");
        state_clone.shutdown_signal.notify_waiters();
        state_clone.shutdown_requested.store(true, Ordering::SeqCst);
    });

    server::run_server_loop(state).await;
}
