use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use tawai_core::app_context::AppContext;
use tawai_core::signals::library::ScanProgress;
use tawai_core::utils::config::{get_path, set_path};
use tawai_core::utils::master_key::{self, KeySource, MasterKeyError};
use tawai_core::utils::{logger, paths};

use tawai_server::utils::{observability, server};

fn main() {
    observability::apply_env_aliases();

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("failed to build Tokio runtime");

    runtime.block_on(async {
        let (guard, tracing_ready) = observability::init();
        if tracing_ready {
            logger::enable_tracing();
        }

        run(guard).await;
    });
}

async fn run(guard: init_tracing_opentelemetry::Guard) {
    let _guard = guard;

    logger::debug("Initializing Tawai Server...");

    let device_id = std::env::var("TAWAI_DEVICE_ID").unwrap_or_default();
    let (master_key, source) = match master_key::resolve(Path::new(paths::tawai_home()), &device_id)
    {
        Ok(value) => value,
        Err(MasterKeyError::MissingDeviceId) => {
            logger::error("TAWAI_SERVER_MASTER_KEY is not set and TAWAI_DEVICE_ID is empty");
            std::process::exit(1);
        }
        Err(e) => {
            logger::error(&format!("Failed to resolve master key: {e}"));
            std::process::exit(1);
        }
    };

    match source {
        KeySource::Created => logger::info(&format!(
            "created master key at {}",
            paths::master_key_path()
        )),
        KeySource::Environment => {
            logger::debug("loaded master key from TAWAI_SERVER_MASTER_KEY");
        }
        KeySource::File => logger::debug(&format!(
            "loaded master key from {}",
            paths::master_key_path()
        )),
    }

    let client = tawai_core::utils::url::build_browser_client().await;

    let context = AppContext::new(client);

    if let Err(e) = context.set_master_key(master_key) {
        logger::error(&format!("Failed to set master key: {e}"));
        std::process::exit(1);
    }

    let db_url = std::env::var("TAWAI_DATABASE_URL")
        .unwrap_or_else(|_| format!("{}/tawai.db", paths::data_dir()));
    match context.init_database(&db_url).await {
        Ok(()) => {
            let loop_ctx = Arc::clone(&context);
            tokio::spawn(async move { loop_ctx.run_database_loop().await });
        }
        Err(e) => logger::error(&format!("Failed to start database manager: {:?}", e)),
    }

    // Initialize config
    if let Err(e) = context.init_config(paths::config_path()).await {
        logger::error(&format!("Failed to load config: {:?}", e));
    }

    let port = get_path(&context.cfg().await.value, "server.port")
        .and_then(|v| v.as_u64())
        .and_then(|v| u16::try_from(v).ok())
        .unwrap_or(8181);
    logger::debug(&format!("Server port: {port}"));

    // Default the download folder to {tawai_home}/downloads when unset
    let mut config_value = context.cfg().await.value.clone();
    let existing = get_path(&config_value, "downloads.folder")
        .and_then(|v| v.as_str())
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false);
    if !existing {
        set_path(
            &mut config_value,
            "downloads.folder",
            serde_json::json!(paths::downloads_dir()),
        );
        let _ = context.save_config(&config_value).await;
    }

    let state = server::AppState::new(context);

    state.context.spawn_periodic_scan(
        Duration::from_secs(3600 * 3),
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

    let context = state.context.clone();
    let server_task = tokio::spawn(async move {
        server::run_server_loop(state).await;
    });

    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            logger::debug("Ctrl-C received");
            context.operation_signals.request_shutdown();
        }
        _ = context.operation_signals.wait_for_shutdown() => {
            logger::debug("Shutdown notification received");
        }
    }

    let _ = server_task.await;
}
