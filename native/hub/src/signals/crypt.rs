use rinf::{DartSignal, RustSignal};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, DartSignal)]
pub struct RequestNewApiKey {
    pub id: String,
    pub user_id: String,
    pub master_key: Option<String>,
}

#[derive(Serialize, RustSignal)]
pub struct NewApiKey {
    pub id: String,
    pub encrypted_api_key: String,
    pub decrypted_api_key: String,
    pub master_key: String,
}

#[derive(Serialize, RustSignal)]
pub struct ApiKeyResponse {
    pub id: String,
    pub api_key: String,
    pub access_token: String,
    pub csrf_token: String,
    pub expires_in: u64,
}

#[derive(Deserialize, DartSignal)]
pub struct DecryptRequest {
    pub id: String,
    pub encrypted_key: String,
    pub master_key: Option<String>,
}

#[derive(Serialize, RustSignal)]
pub struct DecryptResponse {
    pub id: String,
    pub decrypted_key: String,
}

#[derive(Deserialize, DartSignal)]
pub struct EncryptRequest {
    pub id: String,
    pub plain_key: String,
    pub master_key: Option<String>,
}

#[derive(Serialize, RustSignal)]
pub struct EncryptResponse {
    pub id: String,
    pub encrypted_key: String,
    pub master_key: String,
}

#[derive(Deserialize, DartSignal)]
pub struct GenerateMasterKeyRequest {
    pub id: String,
}

#[derive(Serialize, RustSignal)]
pub struct GenerateMasterKeyResponse {
    pub id: String,
    pub master_key: String,
}

#[derive(Deserialize, DartSignal)]
pub struct Login {
    pub id: String,
    pub iuser: String,
    pub ipass: String,
    pub ruser: String,
    pub rpass: String,
}

#[derive(Serialize, RustSignal)]
pub struct LoginResult {
    pub id: String,
    pub success: bool,
    pub user_id: String,
    pub username: String,
}
