use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GetCurrentVersionRequest {
    #[serde(default)]
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
    #[serde(default)]
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

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CompareVersionsRequest {
    #[serde(default)]
    pub id: String,
    pub versions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CompareVersionsResponse {
    #[serde(default)]
    pub id: String,
    pub latest: Option<String>,
}
