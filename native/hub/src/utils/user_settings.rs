use std::collections::HashMap;
use std::sync::Arc;

use rinf::{DartSignal, RustSignal};

use crate::signals;
use crate::utils::logger;
use tawai_core::app_context::AppContext;
use tawai_core::utils::encryption;

pub async fn handle_set_user_setting(context: Arc<AppContext>) {
    use signals::settings::*;
    let receiver = SetUserSettingRequest::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;
        let db = context.db().await;

        let value = if msg.key == "listenbrainz_token" && !msg.value.is_empty() {
            let mk = context.master_key.read().await.clone();
            encryption::encrypt(&msg.value, &mk).unwrap_or(msg.value.clone())
        } else {
            msg.value.clone()
        };

        let result =
            tawai_core::db::user_settings::set_setting(db.pool(), &msg.user_id, &msg.key, &value)
                .await;

        match result {
            Ok(()) => {
                SetUserSettingResponse {
                    id: msg.id,
                    success: true,
                }
                .send_signal_to_dart();
            }
            Err(e) => {
                logger::error(&format!("set user setting failed: {}", e));
                SetUserSettingResponse {
                    id: msg.id,
                    success: false,
                }
                .send_signal_to_dart();
            }
        }
    }
}

pub async fn handle_get_user_setting(context: Arc<AppContext>) {
    use signals::settings::*;
    let receiver = GetUserSettingRequest::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;
        let db = context.db().await;

        let value =
            tawai_core::db::user_settings::get_setting(db.pool(), &msg.user_id, &msg.key).await;

        GetUserSettingResponse {
            id: msg.id,
            key: msg.key,
            value: value.unwrap_or_default(),
        }
        .send_signal_to_dart();
    }
}

pub async fn handle_get_all_user_settings(context: Arc<AppContext>) {
    use signals::settings::*;
    let receiver = GetAllUserSettingsRequest::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;
        let db = context.db().await;

        let pairs = tawai_core::db::user_settings::get_all_settings(db.pool(), &msg.user_id).await;
        let settings: HashMap<_, _> = pairs.into_iter().collect();

        GetAllUserSettingsResponse {
            id: msg.id,
            settings,
        }
        .send_signal_to_dart();
    }
}
