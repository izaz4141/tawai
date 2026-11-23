use crate::signals::server::{InitDatabase, InitDatabaseResponse};
use crate::utils::logger;

use rinf::{DartSignal, RustSignal};
use std::sync::Arc;
use tawai_core::app_context::AppContext;
use tawai_core::utils::master_key::MasterKey;
use tokio::spawn;

pub async fn init_database_handler(context: Arc<AppContext>) {
    let receiver = InitDatabase::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let id = signal_pack.message.id;
        let db_url = signal_pack.message.path;

        let key = match MasterKey::from_hex(&signal_pack.message.master_key) {
            Ok(key) => key,
            Err(e) => {
                logger::error(&format!("invalid master key: {e}"));
                InitDatabaseResponse {
                    id,
                    success: false,
                    error: Some(e.to_string()),
                }
                .send_signal_to_dart();
                continue;
            }
        };

        if context.has_master_key() {
            logger::debug("master key already set; ignoring duplicate");
        } else if let Err(e) = context.set_master_key(key) {
            logger::error(&format!("failed to set master key: {e}"));
        }

        let context_clone = context.clone();

        let init_result = context_clone.init_database(&db_url).await;

        match init_result {
            Ok(()) => {
                InitDatabaseResponse {
                    id,
                    success: true,
                    error: None,
                }
                .send_signal_to_dart();

                spawn(async move { context_clone.run_database_loop().await });
            }
            Err(e) => {
                logger::error(&format!("Failed to start database manager: {:?}", e));
                InitDatabaseResponse {
                    id,
                    success: false,
                    error: Some(e.to_string()),
                }
                .send_signal_to_dart();
            }
        }
    }
}
