use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

pub mod account;
pub mod crypt;
pub mod discovery;
pub mod download;
pub mod library;
pub mod metadata;
pub mod playback;
pub mod tools;
pub mod user_settings;
pub mod version;

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
    #[serde(default)]
    pub id: String,
    pub user_id: String,
    pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ListDownloadsResponse {
    #[serde(default)]
    pub id: String,
    pub downloads: Vec<DownloadRecord>,
}
