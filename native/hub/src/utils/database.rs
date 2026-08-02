use crate::signals::server::{InitDatabase, InitDatabaseResponse};
use crate::utils::logger;

use rinf::{DartSignal, RustSignal};
use std::sync::Arc;
use tawai_core::app_context::AppContext;
use tokio::{spawn, sync::Notify};

pub async fn init_database_handler(context: Arc<AppContext>, db_done_signal: Arc<Notify>) {
    let receiver = InitDatabase::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let id = signal_pack.message.id;
        let db_url = signal_pack.message.path;
        let master_key = signal_pack.message.master_key.clone();

        let context_clone = context.clone();
        let sig = db_done_signal.clone();

        let init_result = context_clone.init_database(&db_url, sig, &master_key).await;

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
