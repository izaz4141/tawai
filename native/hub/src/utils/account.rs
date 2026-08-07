use std::sync::Arc;

use rinf::{DartSignal, RustSignal};

use crate::signals;
use crate::utils::logger;
use tawai_core::app_context::AppContext;
use tawai_core::utils::helper;
use tawai_core::utils::{encryption, security};

pub async fn handle_get_user_by_id(context: Arc<AppContext>) {
    use signals::account::*;
    let receiver = GetUserByIdRequest::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;
        let db = context.db().await;
        let mk = context.master_key.read().await.clone();

        let result = tawai_core::db::account::get_user_by_id(db.pool(), &msg.user_id, &mk).await;

        match result {
            Ok(Some(user)) => {
                GetUserByIdResponse {
                    id: msg.id,
                    user_id: user.id,
                    username: user.username,
                    display_name: user.display_name,
                    role: user.role,
                    found: true,
                }
                .send_signal_to_dart();
            }
            _ => {
                GetUserByIdResponse {
                    id: msg.id,
                    user_id: msg.user_id,
                    username: String::new(),
                    display_name: String::new(),
                    role: String::new(),
                    found: false,
                }
                .send_signal_to_dart();
            }
        }
    }
}

pub async fn handle_list_users(context: Arc<AppContext>) {
    use signals::account::*;
    let receiver = ListUsersRequest::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;
        let db = context.db().await;
        let mk = context.master_key.read().await.clone();

        let core_users = tawai_core::db::account::get_all_users_with_keys(db.pool(), &mk)
            .await
            .unwrap_or_default();

        let users: Vec<UserListItem> = core_users.into_iter().map(Into::into).collect();

        ListUsersResponse { id: msg.id, users }.send_signal_to_dart();
    }
}

pub async fn handle_get_user_by_username(context: Arc<AppContext>) {
    use signals::account::*;
    let receiver = GetUserByUsernameRequest::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;
        let db = context.db().await;
        let mk = context.master_key.read().await.clone();

        let result =
            tawai_core::db::account::get_user_by_username(db.pool(), &msg.username, &mk).await;

        match result {
            Ok(Some(user)) => {
                GetUserByUsernameResponse {
                    id: msg.id,
                    user_id: user.id,
                    username: user.username,
                    display_name: user.display_name,
                    role: user.role,
                    found: true,
                }
                .send_signal_to_dart();
            }
            _ => {
                GetUserByUsernameResponse {
                    id: msg.id,
                    user_id: String::new(),
                    username: msg.username,
                    display_name: String::new(),
                    role: String::new(),
                    found: false,
                }
                .send_signal_to_dart();
            }
        }
    }
}

fn sanitize_role(role: Option<&str>) -> &'static str {
    match role {
        Some("admin") => "admin",
        _ => "user",
    }
}

pub async fn handle_update_account(context: Arc<AppContext>) {
    use signals::account::*;
    let receiver = UpdateAccountRequest::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;
        let db = context.db().await;
        let mk = context.master_key.read().await.clone();

        let authorizer =
            tawai_core::db::account::get_user_by_username(db.pool(), &msg.username, &mk).await;

        let (success, username) = match authorizer {
            Ok(Some(authorizer)) => {
                let valid =
                    security::validate_password(&authorizer.password_hash, &msg.current_password)
                        .unwrap_or(false);
                if !valid {
                    (false, msg.target_username.clone())
                } else {
                    let is_admin = authorizer.role == "admin";
                    if msg.target_username != msg.username && !is_admin {
                        (false, msg.target_username.clone())
                    } else {
                        let target = tawai_core::db::account::get_user_by_username(
                            db.pool(),
                            &msg.target_username,
                            &mk,
                        )
                        .await;
                        match target {
                            Ok(Some(target)) => {
                                let mut result_username = target.username.clone();
                                if let Some(new_username) = &msg.new_username {
                                    if !new_username.is_empty() {
                                        match tawai_core::db::account::change_username(
                                            db.pool(),
                                            &target.id,
                                            new_username,
                                        )
                                        .await
                                        {
                                            Ok(()) => result_username = new_username.clone(),
                                            Err(e) => {
                                                logger::error(&format!(
                                                    "Failed to change username: {}",
                                                    e
                                                ));
                                            }
                                        }
                                    }
                                }
                                if let Some(new_password) = &msg.new_password {
                                    if !new_password.is_empty() {
                                        match security::hash_password(
                                            new_password,
                                            &target.password_hash,
                                        ) {
                                            Ok(hashed) => {
                                                if let Err(e) =
                                                    tawai_core::db::account::change_password(
                                                        db.pool(),
                                                        &target.id,
                                                        &hashed,
                                                    )
                                                    .await
                                                {
                                                    logger::error(&format!(
                                                        "Failed to change password: {}",
                                                        e
                                                    ));
                                                }
                                            }
                                            Err(e) => {
                                                logger::error(&format!(
                                                    "Failed to hash password: {}",
                                                    e
                                                ));
                                            }
                                        }
                                    }
                                }
                                if is_admin {
                                    let role = sanitize_role(msg.role.as_deref());
                                    if role != target.role {
                                        if let Err(e) = tawai_core::db::account::change_role(
                                            db.pool(),
                                            &target.id,
                                            role,
                                        )
                                        .await
                                        {
                                            logger::error(&format!("Failed to change role: {}", e));
                                        }
                                    }
                                }
                                if let Some(display_name) = &msg.display_name {
                                    if !display_name.is_empty() {
                                        if let Err(e) =
                                            tawai_core::db::account::change_display_name(
                                                db.pool(),
                                                &target.id,
                                                display_name,
                                            )
                                            .await
                                        {
                                            logger::error(&format!(
                                                "Failed to change display name: {}",
                                                e
                                            ));
                                        }
                                    }
                                }
                                (true, result_username)
                            }
                            _ => (false, msg.target_username.clone()),
                        }
                    }
                }
            }
            _ => (false, msg.target_username.clone()),
        };

        let mut user_id = String::new();
        let mut display_name = String::new();
        let mut role = String::new();
        let mut api_key = String::new();
        if success {
            if let Ok(Some(u)) =
                tawai_core::db::account::get_user_by_username(db.pool(), &username, &mk).await
            {
                user_id = u.id;
                display_name = u.display_name;
                role = u.role;
                api_key = u.api_key;
            }
        }

        UpdateAccountResponse {
            id: msg.id,
            success,
            username,
            user_id,
            display_name,
            role,
            api_key,
        }
        .send_signal_to_dart();
    }
}

pub async fn handle_create_account(context: Arc<AppContext>) {
    use signals::account::*;
    let receiver = CreateAccountRequest::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;
        let db = context.db().await;
        let mk = context.master_key.read().await.clone();

        let (success, user_id, username, display_name, role, api_key) = if msg
            .username
            .trim()
            .is_empty()
        {
            (
                false,
                String::new(),
                msg.username.clone(),
                String::new(),
                String::new(),
                String::new(),
            )
        } else {
            let admin =
                tawai_core::db::account::get_user_by_username(db.pool(), &msg.admin_username, &mk)
                    .await
                    .unwrap_or(None);

            let is_admin = match &admin {
                Some(u) => {
                    u.role == "admin"
                        && security::validate_password(&u.password_hash, &msg.admin_password)
                            .unwrap_or(false)
                }
                None => false,
            };

            if !is_admin {
                (
                    false,
                    String::new(),
                    msg.username.clone(),
                    String::new(),
                    String::new(),
                    String::new(),
                )
            } else {
                let taken =
                    tawai_core::db::account::get_user_by_username(db.pool(), &msg.username, &mk)
                        .await
                        .unwrap_or(None)
                        .is_some();
                if taken {
                    (
                        false,
                        String::new(),
                        msg.username.clone(),
                        String::new(),
                        String::new(),
                        String::new(),
                    )
                } else {
                    let salt = security::generate_salt();
                    match security::hash_password(&msg.password, &salt) {
                        Ok(hash) => {
                            let api_key = uuid::Uuid::new_v4().to_string();
                            let encrypted = encryption::encrypt(&api_key, &mk)
                                .unwrap_or_else(|_| api_key.clone());
                            let api_key_hash = helper::sha256_hex(&api_key);
                            let display_name = msg.display_name.clone().unwrap_or_default();
                            let role = sanitize_role(msg.role.as_deref());
                            match tawai_core::db::account::create_user(
                                db.pool(),
                                &msg.username,
                                &display_name,
                                &hash,
                                &encrypted,
                                &api_key_hash,
                                role,
                            )
                            .await
                            {
                                Ok(uid) => (
                                    true,
                                    uid,
                                    msg.username.clone(),
                                    display_name,
                                    role.to_string(),
                                    api_key.clone(),
                                ),
                                Err(e) => {
                                    logger::error(&format!("Failed to create user: {}", e));
                                    (
                                        false,
                                        String::new(),
                                        msg.username.clone(),
                                        String::new(),
                                        String::new(),
                                        String::new(),
                                    )
                                }
                            }
                        }
                        Err(e) => {
                            logger::error(&format!("Failed to hash password: {}", e));
                            (
                                false,
                                String::new(),
                                msg.username.clone(),
                                String::new(),
                                String::new(),
                                String::new(),
                            )
                        }
                    }
                }
            }
        };

        CreateAccountResponse {
            id: msg.id,
            success,
            user_id,
            username,
            display_name,
            role,
            api_key,
        }
        .send_signal_to_dart();
    }
}

pub async fn handle_delete_account(context: Arc<AppContext>) {
    use signals::account::*;
    let receiver = DeleteAccountRequest::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;
        let db = context.db().await;
        let mk = context.master_key.read().await.clone();

        let admin =
            tawai_core::db::account::get_user_by_username(db.pool(), &msg.admin_username, &mk)
                .await
                .unwrap_or(None);

        let is_admin = match &admin {
            Some(u) => {
                u.role == "admin"
                    && security::validate_password(&u.password_hash, &msg.admin_password)
                        .unwrap_or(false)
            }
            None => false,
        };

        let mut success = false;
        if is_admin {
            let target =
                tawai_core::db::account::get_user_by_username(db.pool(), &msg.target_username, &mk)
                    .await
                    .unwrap_or(None);

            if let Some(target) = target {
                let mut allow = true;
                if target.role == "admin" {
                    let users = tawai_core::db::account::get_all_users(db.pool())
                        .await
                        .unwrap_or_default();
                    let admin_count = users.iter().filter(|u| u.role == "admin").count();
                    allow = admin_count > 1;
                }
                if allow {
                    success = tawai_core::db::account::delete_user(db.pool(), &target.id)
                        .await
                        .unwrap_or(false);
                }
            }
        }

        DeleteAccountResponse {
            id: msg.id,
            success,
            username: msg.target_username,
        }
        .send_signal_to_dart();
    }
}

pub async fn handle_verify_current_password(context: Arc<AppContext>) {
    use signals::account::*;
    let receiver = VerifyCurrentPasswordRequest::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;
        let db = context.db().await;
        let mk = context.master_key.read().await.clone();

        let user =
            tawai_core::db::account::get_user_by_id(db.pool(), &msg.user_id, &mk).await;

        let valid = match user {
            Ok(Some(u)) => {
                security::validate_password(&u.password_hash, &msg.password).unwrap_or(false)
            }
            _ => false,
        };

        VerifyCurrentPasswordResponse { id: msg.id, valid }.send_signal_to_dart();
    }
}
