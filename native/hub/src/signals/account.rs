use rinf::{DartSignal, RustSignal, SignalPiece};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, DartSignal)]
pub struct GetUserByUsernameRequest {
    pub id: String,
    pub username: String,
}

#[derive(Serialize, RustSignal)]
pub struct GetUserByUsernameResponse {
    pub id: String,
    pub user_id: String,
    pub username: String,
    pub display_name: String,
    pub role: String,
    pub found: bool,
}

#[derive(Deserialize, DartSignal)]
pub struct GetUserByIdRequest {
    pub id: String,
    pub user_id: String,
}

#[derive(Serialize, RustSignal)]
pub struct GetUserByIdResponse {
    pub id: String,
    pub user_id: String,
    pub username: String,
    pub display_name: String,
    pub role: String,
    pub found: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, SignalPiece)]
pub struct UserListItem {
    pub id: String,
    pub username: String,
    pub display_name: String,
    pub role: String,
    pub api_key: String,
}

impl From<tawai_core::signals::account::UserListItem> for UserListItem {
    fn from(u: tawai_core::signals::account::UserListItem) -> Self {
        Self {
            id: u.id,
            username: u.username,
            display_name: u.display_name,
            role: u.role,
            api_key: u.api_key,
        }
    }
}

#[derive(Deserialize, DartSignal)]
pub struct ListUsersRequest {
    pub id: String,
}

#[derive(Serialize, RustSignal)]
pub struct ListUsersResponse {
    pub id: String,
    pub users: Vec<UserListItem>,
}

#[derive(Deserialize, DartSignal)]
pub struct UpdateAccountRequest {
    pub id: String,
    pub username: String,
    pub current_password: String,
    pub target_username: String,
    pub new_username: Option<String>,
    pub new_password: Option<String>,
    pub display_name: Option<String>,
    pub role: Option<String>,
}

#[derive(Serialize, RustSignal)]
pub struct UpdateAccountResponse {
    pub id: String,
    pub success: bool,
    pub user_id: String,
    pub username: String,
    pub display_name: String,
    pub role: String,
    pub api_key: String,
}

#[derive(Deserialize, DartSignal)]
pub struct CreateAccountRequest {
    pub id: String,
    pub admin_username: String,
    pub admin_password: String,
    pub username: String,
    pub password: String,
    pub display_name: Option<String>,
    pub role: Option<String>,
}

#[derive(Serialize, RustSignal)]
pub struct CreateAccountResponse {
    pub id: String,
    pub success: bool,
    pub user_id: String,
    pub username: String,
    pub display_name: String,
    pub role: String,
    pub api_key: String,
}

#[derive(Deserialize, DartSignal)]
pub struct DeleteAccountRequest {
    pub id: String,
    pub admin_username: String,
    pub admin_password: String,
    pub target_username: String,
}

#[derive(Serialize, RustSignal)]
pub struct DeleteAccountResponse {
    pub id: String,
    pub success: bool,
    pub username: String,
}

#[derive(Deserialize, DartSignal)]
pub struct VerifyCurrentPasswordRequest {
    pub id: String,
    pub user_id: String,
    pub password: String,
}

#[derive(Serialize, RustSignal)]
pub struct VerifyCurrentPasswordResponse {
    pub id: String,
    pub valid: bool,
}
