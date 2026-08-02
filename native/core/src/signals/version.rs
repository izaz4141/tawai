use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GetCurrentVersionRequest {
    pub id: String,
    pub app: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GetCurrentVersionResponse {
    pub id: String,
    pub version: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GetLatestVersionRequest {
    pub id: String,
    pub owner: String,
    pub repo: String,
    pub nightly: bool,
    pub atomic: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GetLatestVersionResponse {
    pub id: String,
    pub version: Option<String>,
    pub tag_name: Option<String>,
    pub release_notes: Option<String>,
    pub published_at: Option<String>,
    pub error: Option<String>,
}
