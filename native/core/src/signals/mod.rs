use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

pub mod account;
pub mod crypt;
pub mod discovery;
pub mod download;
pub mod identify;
pub mod library;
pub mod metadata;
pub mod playback;
pub mod settings;
pub mod tools;
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
