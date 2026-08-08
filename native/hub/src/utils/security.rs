use std::sync::Arc;

extern crate tawai_core as core;
use crate::signals::crypt::{
    GenerateMasterKeyRequest, GenerateMasterKeyResponse, Login, LoginResult,
};
use crate::utils::logger;
use core::db::account;
use core::utils::encryption;
use core::utils::security::validate_password;
use rinf::{DartSignal, RustSignal};
use tawai_core::app_context::AppContext;

pub async fn handle_login(context: Arc<AppContext>) {
    let receiver = Login::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;

        let success =
            validate_password(&msg.rpass, &msg.ipass).unwrap_or(false) & (msg.iuser == msg.ruser);

        let mk = context.master_key.read().await.clone();
        let db = context.db().await;
        let (user_id, username) =
            match account::get_user_by_username(db.pool(), &msg.iuser, &mk).await {
                Ok(u) => (u.id, u.username),
                Err(_) => (String::new(), msg.iuser.clone()),
            };

        LoginResult {
            id: msg.id.clone(),
            success,
            user_id,
            username,
        }
        .send_signal_to_dart();
    }
}

pub async fn handle_generate_master_key() {
    let receiver = GenerateMasterKeyRequest::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let master_key = encryption::generate_master_key();
        GenerateMasterKeyResponse {
            id: signal_pack.message.id,
            master_key,
        }
        .send_signal_to_dart();
    }
}
