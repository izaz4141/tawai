use std::sync::Arc;

use rinf::{DartSignal, RustSignal};

use crate::signals;
use crate::utils::logger;
use tawai_core::app_context::AppContext;
use tawai_core::utils::account as caccount;

pub async fn handle_get_user_by_id(context: Arc<AppContext>) {
    use signals::account::*;
    let receiver = GetUserByIdRequest::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;
        let db = context.db().await;
        let mk = context.master_key.read().await.clone();

        match caccount::get_user_by_id(&db, &mk, &msg.user_id).await {
            Ok(user) => {
                GetUserByIdResponse {
                    id: msg.id,
                    user_id: user.user_id,
                    username: user.username,
                    display_name: user.display_name,
                    role: user.role,
                    found: true,
                }
                .send_signal_to_dart();
            }
            Err(_) => {
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

        let core_users = caccount::list_users(&db, &mk, true)
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

        match caccount::get_user_by_username(&db, &mk, &msg.username).await {
            Ok(user) => {
                GetUserByUsernameResponse {
                    id: msg.id,
                    user_id: user.user_id,
                    username: user.username,
                    display_name: user.display_name,
                    role: user.role,
                    found: true,
                }
                .send_signal_to_dart();
            }
            Err(_) => {
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

pub async fn handle_update_account(context: Arc<AppContext>) {
    use signals::account::*;
    let receiver = UpdateAccountRequest::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;
        let db = context.db().await;
        let mk = context.master_key.read().await.clone();

        let req = tawai_core::signals::account::UpdateAccountRequest {
            id: msg.id.clone(),
            operator_user_id: msg.operator_user_id,
            operator_password: msg.operator_password,
            target_user_id: msg.target_user_id,
            new_username: msg.new_username,
            new_password: msg.new_password,
            display_name: msg.display_name,
            role: msg.role,
        };

        match caccount::update_user(&db, &mk, &req).await {
            Ok(response) => {
                UpdateAccountResponse {
                    id: msg.id,
                    success: true,
                    user_id: response.user_id,
                    username: response.username,
                    display_name: response.display_name,
                    role: response.role,
                    api_key: response.api_key,
                }
                .send_signal_to_dart();
            }
            Err(e) => {
                logger::error(&format!("update account failed: {}", e));
                UpdateAccountResponse {
                    id: msg.id,
                    success: false,
                    user_id: String::new(),
                    username: String::new(),
                    display_name: String::new(),
                    role: String::new(),
                    api_key: String::new(),
                }
                .send_signal_to_dart();
            }
        }
    }
}

pub async fn handle_create_account(context: Arc<AppContext>) {
    use signals::account::*;
    let receiver = CreateAccountRequest::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;
        let db = context.db().await;
        let mk = context.master_key.read().await.clone();

        let req = tawai_core::signals::account::CreateAccountRequest {
            id: msg.id.clone(),
            admin_username: msg.admin_username,
            admin_password: msg.admin_password,
            username: msg.username,
            password: msg.password,
            display_name: msg.display_name,
            role: msg.role,
        };

        match caccount::create_user(&db, &mk, &req).await {
            Ok(response) => {
                CreateAccountResponse {
                    id: msg.id,
                    success: true,
                    user_id: response.user_id,
                    username: response.username,
                    display_name: response.display_name,
                    role: response.role,
                    api_key: response.api_key,
                }
                .send_signal_to_dart();
            }
            Err(e) => {
                logger::error(&format!("create account failed: {}", e));
                CreateAccountResponse {
                    id: msg.id,
                    success: false,
                    user_id: String::new(),
                    username: req.username,
                    display_name: String::new(),
                    role: String::new(),
                    api_key: String::new(),
                }
                .send_signal_to_dart();
            }
        }
    }
}

pub async fn handle_delete_account(context: Arc<AppContext>) {
    use signals::account::*;
    let receiver = DeleteAccountRequest::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;
        let db = context.db().await;
        let mk = context.master_key.read().await.clone();

        let req = tawai_core::signals::account::DeleteAccountRequest {
            id: msg.id.clone(),
            admin_username: msg.admin_username,
            admin_password: msg.admin_password,
            target_username: msg.target_username,
        };

        match caccount::delete_user(&db, &mk, &req).await {
            Ok(_) => {
                DeleteAccountResponse {
                    id: msg.id,
                    success: true,
                    username: req.target_username,
                }
                .send_signal_to_dart();
            }
            Err(_) => {
                DeleteAccountResponse {
                    id: msg.id,
                    success: false,
                    username: req.target_username,
                }
                .send_signal_to_dart();
            }
        }
    }
}

pub async fn handle_verify_password(context: Arc<AppContext>) {
    use signals::account::*;
    let receiver = VerifyPasswordRequest::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;
        let db = context.db().await;
        let mk = context.master_key.read().await.clone();

        let valid = caccount::verify_current_password(&db, &mk, &msg.user_id, &msg.password).await;

        VerifyPasswordResponse { id: msg.id, valid }.send_signal_to_dart();
    }
}
