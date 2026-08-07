use tawai_core::app_context::AppContext;
use tawai_core::db::account;
use tawai_core::utils::encryption;
use tawai_server::{
    docs::create_docs_router,
    server::{AppState, global_rate_limit_config, run_server},
    tawai::{create_tawai_router, system::handle_status},
};

use crate::signals::crypt::{
    DecryptRequest, DecryptResponse, EncryptRequest, EncryptResponse, NewApiKey, RequestNewApiKey,
};
use crate::signals::server::{
    InitConfig, InitConfigResponse, SaveConfigRequest, SaveConfigResponse, StartServer,
};
use crate::utils::logger;
use axum::{Router, routing::get};
use rinf::{DartSignal, RustSignal};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;
use std::time::Duration;
use tokio::{
    spawn,
    sync::{Notify, RwLock},
};
use tower_governor::GovernorLayer;

pub async fn handle_api_key_generation(context: Arc<AppContext>) {
    let receiver = RequestNewApiKey::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;
        let master_key = match &msg.master_key {
            Some(key) if encryption::is_valid_master_key(key) => key.to_owned(),
            _ => {
                logger::warn("Invalid master key provided, generating new one");
                encryption::generate_master_key()
            }
        };

        let db = context.db().await;
        let api_key = account::regenerate_user_api_key(db.pool(), &msg.user_id, &master_key)
            .await
            .unwrap_or_else(|_| uuid::Uuid::new_v4().to_string());

        let encrypted_api_key = match encryption::encrypt(&api_key, &master_key) {
            Ok(encrypted) => encrypted,
            Err(e) => {
                logger::error(&format!("Encryption failed: {}", e));
                NewApiKey {
                    id: msg.id,
                    encrypted_api_key: String::new(),
                    decrypted_api_key: String::new(),
                    master_key: String::new(),
                }
                .send_signal_to_dart();
                continue;
            }
        };
        NewApiKey {
            id: msg.id,
            encrypted_api_key,
            decrypted_api_key: api_key,
            master_key,
        }
        .send_signal_to_dart();
    }
}

pub async fn handle_decrypt_request(context: Arc<AppContext>) {
    let receiver = DecryptRequest::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;
        let master_key = match msg.master_key {
            Some(key) => key,
            None => context.master_key.read().await.clone(),
        };
        let decrypted_key = match encryption::decrypt(&msg.encrypted_key, &master_key) {
            Ok(key) => key,
            Err(e) => {
                logger::error(&format!("Decryption failed: {}", e));
                String::new()
            }
        };
        DecryptResponse {
            id: msg.id,
            decrypted_key,
        }
        .send_signal_to_dart();
    }
}

pub async fn handle_encrypt_request(context: Arc<AppContext>) {
    let receiver = EncryptRequest::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;
        let master_key = match msg.master_key {
            Some(key) => key,
            None => context.master_key.read().await.clone(),
        };
        let encrypted_key = match encryption::encrypt(&msg.plain_key, &master_key) {
            Ok(encrypted) => encrypted,
            Err(e) => {
                logger::error(&format!("Encryption failed: {}", e));
                EncryptResponse {
                    id: msg.id,
                    encrypted_key: String::new(),
                    master_key: String::new(),
                }
                .send_signal_to_dart();
                continue;
            }
        };
        EncryptResponse {
            id: msg.id,
            encrypted_key,
            master_key,
        }
        .send_signal_to_dart();
    }
}

pub async fn handle_init_config(context: Arc<AppContext>) {
    let receiver = InitConfig::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;
        match context.init_config(msg.path).await {
            Ok(is_first_run) => {
                let settings_json = serde_json::to_string(&context.client_config().await).ok();
                InitConfigResponse {
                    id: msg.id,
                    success: true,
                    error: None,
                    settings_json,
                    is_first_run,
                }
                .send_signal_to_dart();
            }
            Err(e) => {
                logger::error(&format!("init config failed: {}", e));
                InitConfigResponse {
                    id: msg.id,
                    success: false,
                    error: Some(e.to_string()),
                    settings_json: None,
                    is_first_run: false,
                }
                .send_signal_to_dart();
            }
        }
    }
}

pub async fn handle_save_config(context: Arc<AppContext>) {
    let receiver = SaveConfigRequest::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;
        let settings = match serde_json::from_str::<serde_json::Value>(&msg.settings_json) {
            Ok(value) => value,
            Err(e) => {
                logger::error(&format!("save config: invalid settings json: {}", e));
                SaveConfigResponse {
                    id: msg.id,
                    success: false,
                    error: Some(e.to_string()),
                }
                .send_signal_to_dart();
                continue;
            }
        };
        match context.update_config(&settings).await {
            Ok(()) => SaveConfigResponse {
                id: msg.id,
                success: true,
                error: None,
            }
            .send_signal_to_dart(),
            Err(e) => {
                logger::error(&format!("save config failed: {}", e));
                SaveConfigResponse {
                    id: msg.id,
                    success: false,
                    error: Some(e.to_string()),
                }
                .send_signal_to_dart();
            }
        }
    }
}

pub async fn start_server_listener(context: Arc<AppContext>) {
    let mut current_server: Option<(tokio::task::JoinHandle<()>, Arc<tokio::sync::Notify>)> = None;
    let mut cleanup_handle: Option<tokio::task::JoinHandle<()>> = None;
    let receiver = StartServer::get_dart_signal_receiver();

    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;
        logger::debug(&format!("Starting server on port {}", msg.port));

        if let Some(handle) = cleanup_handle.take() {
            handle.abort();
        }

        if let Some((old_handle, old_notify)) = current_server.take() {
            old_notify.notify_one();
            let _ = old_handle.await;
        }

        // Initialize config if not already done
        if context.cfg().await.value.is_null() {
            let _ = context.init_config(msg.config_path.clone()).await;
        }

        let restart_signal = Arc::new(Notify::new());
        let shutdown_signal = Arc::new(Notify::new());
        let shutdown_requested = Arc::new(AtomicBool::new(false));

        *context.master_key.write().await = msg.master_key.clone();

        let state = Arc::new(AppState {
            context: context.clone(),
            restart_signal: restart_signal.clone(),
            shutdown_signal: shutdown_signal.clone(),
            shutdown_requested: shutdown_requested.clone(),
            version: Arc::new(RwLock::new(None)),
            dash_generation_locks: Arc::new(Mutex::new(HashMap::new())),
        });

        let governor_conf = global_rate_limit_config();
        let governor_limiter = governor_conf.limiter().clone();

        cleanup_handle = Some(spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(60)).await;
                // logger::debug(&format!(
                //     "Rate limiting storage size: {}",
                //     governor_limiter.len()
                // ));
                governor_limiter.retain_recent();
            }
        }));

        let tawai_router = create_tawai_router(state.clone());
        let docs_router = create_docs_router(state.clone());
        let router = Router::new()
            .nest("/api/tawai", tawai_router)
            .merge(docs_router)
            .layer(GovernorLayer::new(governor_conf))
            .route("/api/tawai/system/status", get(handle_status))
            .with_state(state);

        let rs_clone = restart_signal.clone();
        let ss_clone = shutdown_signal.clone();
        let new_handle = spawn(async move {
            run_server(router, msg.port, rs_clone, ss_clone).await;
        });
        current_server = Some((new_handle, restart_signal));
    }
}
