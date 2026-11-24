use tawai_core::app_context::AppContext;
use tawai_core::db::account;
use tawai_core::utils::master_key::MasterKey;
use tawai_server::routers::create_router;
use tawai_server::utils::server::{AppState, run_server};

use crate::signals::account::{ApiKeyResponse, GenerateApiKeyRequest};
use crate::signals::crypt::{
    DecryptRequest, DecryptResponse, EncryptRequest, EncryptResponse, NewApiKey, RequestNewApiKey,
};
use crate::signals::server::{
    InitConfig, InitConfigResponse, SaveConfigRequest, SaveConfigResponse, StartServer,
};
use crate::signals::settings::{GetGlobalSettingsRequest, GlobalSettingsResponse};
use crate::utils::logger;
use crate::utils::master_key_or_log;
use rinf::{DartSignal, RustSignal};
use std::sync::Arc;
use tokio::spawn;

pub async fn handle_generate_api_key(context: Arc<AppContext>) {
    let receiver = GenerateApiKeyRequest::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;
        let db = context.db().await;
        let Some(mk) = master_key_or_log(&context) else {
            continue;
        };
        let api_key = account::regenerate_user_api_key(db.pool(), &msg.user_id, &mk)
            .await
            .unwrap_or_default();
        ApiKeyResponse {
            id: msg.id,
            api_key,
        }
        .send_signal_to_dart();
    }
}

pub async fn handle_api_key_generation(context: Arc<AppContext>) {
    let receiver = RequestNewApiKey::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;
        let Some(master_key) = master_key_or_log(&context) else {
            NewApiKey {
                id: msg.id,
                encrypted_api_key: String::new(),
                decrypted_api_key: String::new(),
                master_key: String::new(),
            }
            .send_signal_to_dart();
            continue;
        };

        let db = context.db().await;
        let api_key = account::regenerate_user_api_key(db.pool(), &msg.user_id, &master_key)
            .await
            .unwrap_or_else(|_| uuid::Uuid::new_v4().to_string());

        let encrypted_api_key = match master_key.encrypt(&api_key) {
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
            master_key: String::new(),
        }
        .send_signal_to_dart();
    }
}

pub async fn handle_decrypt_request(context: Arc<AppContext>) {
    let receiver = DecryptRequest::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;
        let Some(master_key) = master_key_or_log(&context) else {
            DecryptResponse {
                id: msg.id,
                decrypted_key: String::new(),
            }
            .send_signal_to_dart();
            continue;
        };
        let decrypted_key = match master_key.decrypt(&msg.encrypted_key) {
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
        let Some(master_key) = master_key_or_log(&context) else {
            EncryptResponse {
                id: msg.id,
                encrypted_key: String::new(),
                master_key: String::new(),
            }
            .send_signal_to_dart();
            continue;
        };
        let encrypted_key = match master_key.encrypt(&msg.plain_key) {
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
            master_key: String::new(),
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

pub async fn handle_get_global_settings(context: Arc<AppContext>) {
    let receiver = GetGlobalSettingsRequest::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;
        let settings_json = serde_json::to_string(&context.client_config().await).ok();
        GlobalSettingsResponse {
            id: msg.id,
            settings_json,
        }
        .send_signal_to_dart();
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
    let mut current_server: Option<tokio::task::JoinHandle<()>> = None;
    let receiver = StartServer::get_dart_signal_receiver();

    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;
        logger::debug(&format!("Starting server on port {}", msg.port));

        if let Some(old_handle) = current_server.take() {
            context.operation_signals.request_restart();
            let _ = old_handle.await;
        }

        if !context.has_master_key()
            && let Ok(key) = MasterKey::from_hex(&msg.master_key)
        {
            let _ = context.set_master_key(key);
        }

        // Initialize config if not already done
        if context.cfg().await.value.is_null() {
            let _ = context.init_config(msg.config_path.clone()).await;
        }

        let state = AppState::new(context.clone());

        let router = create_router(state);

        let operation_signals = context.operation_signals.clone();
        let new_handle = spawn(async move {
            run_server(router, msg.port, operation_signals).await;
        });
        current_server = Some(new_handle);
    }
}
