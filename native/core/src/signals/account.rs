use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserListItem {
    pub id: String,
    pub username: String,
    pub display_name: String,
    pub role: String,
    pub api_key: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ApiKeyResponse {
    pub api_key: String,
    pub access_token: String,
    pub csrf_token: String,
    pub expires_in: u64,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct GetUserByUsernameRequest {
    #[serde(default)]
    pub id: String,
    pub username: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct GetUserByUsernameResponse {
    #[serde(default)]
    pub id: String,
    pub user_id: String,
    pub username: String,
    pub display_name: String,
    pub role: String,
    pub found: bool,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct GetUserByIdRequest {
    #[serde(default)]
    pub id: String,
    pub user_id: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct GetUserByIdResponse {
    #[serde(default)]
    pub id: String,
    pub user_id: String,
    pub username: String,
    pub display_name: String,
    pub role: String,
    pub found: bool,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ListUsersRequest {
    #[serde(default)]
    pub id: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ListUsersResponse {
    #[serde(default)]
    pub id: String,
    pub users: Vec<UserListItem>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct UpdateAccountRequest {
    #[serde(default)]
    pub id: String,
    pub operator_user_id: String,
    pub operator_password: String,
    pub target_user_id: String,
    pub new_username: Option<String>,
    pub new_password: Option<String>,
    pub display_name: Option<String>,
    pub role: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct UpdateAccountResponse {
    #[serde(default)]
    pub id: String,
    pub success: bool,
    pub user_id: String,
    pub username: String,
    pub display_name: String,
    pub role: String,
    pub api_key: String,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateAccountRequest {
    #[serde(default)]
    pub id: String,
    pub admin_username: String,
    pub admin_password: String,
    pub username: String,
    pub password: String,
    pub display_name: Option<String>,
    pub role: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CreateAccountResponse {
    #[serde(default)]
    pub id: String,
    pub success: bool,
    pub user_id: String,
    pub username: String,
    pub display_name: String,
    pub role: String,
    pub api_key: String,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct DeleteAccountRequest {
    #[serde(default)]
    pub id: String,
    pub admin_username: String,
    pub admin_password: String,
    pub target_username: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct DeleteAccountResponse {
    #[serde(default)]
    pub id: String,
    pub success: bool,
    pub username: String,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct VerifyCurrentPasswordRequest {
    #[serde(default)]
    pub id: String,
    pub user_id: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct VerifyCurrentPasswordResponse {
    #[serde(default)]
    pub id: String,
    pub valid: bool,
}
