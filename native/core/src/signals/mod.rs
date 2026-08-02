use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

pub mod account;
pub mod discovery;
pub mod download;
pub mod library;
pub mod metadata;
pub mod playback;
pub mod tools;
pub mod version;

#[derive(Debug, Clone, Serialize)]
pub struct LogSignal {
    pub level: String,
    pub message: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct InitDatabase {
    pub id: String,
    pub path: String,
    pub master_key: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct InitDatabaseResponse {
    pub id: String,
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct User {
    pub id: String,
    pub username: String,
    pub display_name: String,
    pub password_hash: String,
    pub api_key: String,
    pub role: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StartServer {
    pub port: u16,
    pub master_key: String,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct RequestNewApiKey {
    pub master_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct NewApiKey {
    pub encrypted_api_key: String,
    pub decrypted_api_key: String,
    pub master_key: String,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct DecryptRequest {
    pub encrypted_key: String,
    pub master_key: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct DecryptResponse {
    pub decrypted_key: String,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct EncryptRequest {
    pub plain_key: String,
    pub master_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct EncryptResponse {
    pub encrypted_key: String,
    pub master_key: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RequestFfmpeg {
    pub id: String,
    pub args: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FfmpegResult {
    pub id: String,
    pub success: bool,
    pub log: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GetCategories {}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct UpdateCategories {
    pub categories: Vec<CategoryDisplay>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CategoriesOutput {
    pub categories: Vec<CategoryDisplay>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CategoryDisplay {
    pub name: String,
    pub save_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DownloadRecord {
    pub id: String,
    pub source: String,
    pub source_id: String,
    pub url: String,
    pub dest_path: String,
    pub filename: String,
    pub total_size: i64,
    pub downloaded: i64,
    pub state: String,
    pub error: String,
    pub added_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ListDownloadsRequest {
    pub user_id: String,
    pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ListDownloadsResponse {
    pub downloads: Vec<DownloadRecord>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct InsertDownloadRequest {
    pub id: String,
    pub user_id: String,
    pub source: String,
    pub source_id: String,
    pub url: String,
    pub dest_path: String,
    pub filename: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct InsertDownloadResponse {
    pub id: String,
    pub download_id: String,
    pub success: bool,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct UpdateDownloadStateRequest {
    pub id: String,
    pub download_id: String,
    pub state: String,
    pub error: String,
    pub downloaded: i64,
    pub total_size: i64,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct UpdateDownloadStateResponse {
    pub id: String,
    pub success: bool,
}
