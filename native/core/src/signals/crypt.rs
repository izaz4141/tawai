use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct RequestNewApiKey {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub user_id: String,
    pub master_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct NewApiKey {
    #[serde(default)]
    pub id: String,
    pub encrypted_api_key: String,
    pub decrypted_api_key: String,
    pub master_key: String,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct DecryptRequest {
    #[serde(default)]
    pub id: String,
    pub encrypted_key: String,
    #[serde(default)]
    pub master_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct DecryptResponse {
    #[serde(default)]
    pub id: String,
    pub decrypted_key: String,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct EncryptRequest {
    #[serde(default)]
    pub id: String,
    pub plain_key: String,
    pub master_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct EncryptResponse {
    #[serde(default)]
    pub id: String,
    pub encrypted_key: String,
    pub master_key: String,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct GenerateMasterKeyRequest {
    #[serde(default)]
    pub id: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct GenerateMasterKeyResponse {
    #[serde(default)]
    pub id: String,
    pub master_key: String,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct Login {
    #[serde(default)]
    pub id: String,
    pub iuser: String,
    pub ipass: String,
    pub ruser: String,
    pub rpass: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct LoginResult {
    #[serde(default)]
    pub id: String,
    pub success: bool,
    pub user_id: String,
    pub username: String,
}
