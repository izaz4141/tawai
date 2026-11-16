use std::sync::Arc;
use std::time::Duration;

use tawai_core::app_context::AppContext;
use tawai_core::signals::library::ScanProgress;
use tawai_core::utils::config::{get_path, set_path};
use tawai_core::utils::{logger, paths};

use tawai_server::utils::{observability, server};

fn main() {
    observability::apply_env_aliases();

    let guard = observability::init();
    logger::enable_tracing();

    run(guard);
}

#[tokio::main]
async fn run(guard: init_tracing_opentelemetry::Guard) {
    let _guard = guard;

    logger::debug("Initializing Tawai Server...");

    let config_path = paths::config_path();

    let master_key =
        std::env::var("TAWAI_SERVER_MASTER_KEY").expect("TAWAI_SERVER_MASTER_KEY is not set");

    let client = tawai_core::utils::url::build_browser_client().await;

    let context = AppContext::new(client);

    let db_url = std::env::var("TAWAI_DATABASE_URL")
        .unwrap_or_else(|_| format!("{}/tawai.db", paths::data_dir()));
    match context.init_database(&db_url, &master_key).await {
        Ok(()) => {
            let loop_ctx = Arc::clone(&context);
            tokio::spawn(async move { loop_ctx.run_database_loop().await });
        }
        Err(e) => logger::error(&format!("Failed to start database manager: {:?}", e)),
    }

    // Initialize config
    if let Err(e) = context.init_config(config_path).await {
        logger::error(&format!("Failed to load config: {:?}", e));
    }

    let port: u16 = std::env::var("TAWAI_SERVER_PORT")
        .unwrap_or_else(|_| "8181".to_string())
        .parse()
        .unwrap_or(8181);

    // Set the server port in config
    let mut config_value = context.cfg().await.value.clone();
    set_path(&mut config_value, "server.port", serde_json::json!(port));

    // Default the download folder to {tawai_home}/downloads when unset
    let existing = get_path(&config_value, "downloads.folder")
        .and_then(|v| v.as_str())
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false);
    let downloads_dir = paths::downloads_dir();
    if !existing {
        set_path(
            &mut config_value,
            "downloads.folder",
            serde_json::json!(downloads_dir),
        );
    }

    // Inject optional downstream service connection details from the environment.
    // Environment variables override whatever is already stored in the config file.
    // save_config encrypts the slskd/nadekodon API keys with the master key.
    let env_overrides = [
        ("TAWAI_SLSKD_URL", "integrations.slskd.url"),
        ("TAWAI_SLSKD_API_KEY", "integrations.slskd.api_key"),
        ("TAWAI_NADEKODON_URL", "integrations.nadekodon.url"),
        ("TAWAI_NADEKODON_API_KEY", "integrations.nadekodon.api_key"),
    ];
    for (env_var, config_path) in env_overrides {
        if let Ok(value) = std::env::var(env_var)
            && !value.trim().is_empty()
        {
            set_path(
                &mut config_value,
                config_path,
                serde_json::json!(value.trim()),
            );
        }
    }

    let _ = context.save_config(&config_value).await;

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
