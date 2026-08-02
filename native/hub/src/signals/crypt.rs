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
pub struct HashPassword {
    pub id: String,
    pub plain_text: String,
    pub salt: String,
}

#[derive(Serialize, RustSignal)]
pub struct HashingOutput {
    pub id: String,
    pub hashed_text: Option<String>,
}

#[derive(Deserialize, DartSignal)]
pub struct GenerateSalt {
    pub id: String,
}

#[derive(Serialize, RustSignal)]
pub struct SaltOutput {
    pub id: String,
    pub salt: String,
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

#[derive(Deserialize, DartSignal)]
pub struct VerifyPassword {
    pub id: String,
    pub input: String,
    pub reference: String,
}

#[derive(Serialize, RustSignal)]
pub struct VerifyPasswordResult {
    pub id: String,
    pub success: bool,
}
