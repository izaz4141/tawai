use rinf::{DartSignal, RustSignal};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, DartSignal)]
pub struct GetCurrentVersionRequest {
    pub id: String,
    pub app: String,
}

#[derive(Serialize, RustSignal)]
pub struct GetCurrentVersionResponse {
    pub id: String,
    pub version: Option<String>,
    pub error: Option<String>,
}

#[derive(Deserialize, DartSignal)]
pub struct GetLatestVersionRequest {
    pub id: String,
    pub owner: String,
    pub repo: String,
    pub nightly: bool,
    pub atomic: bool,
}

#[derive(Serialize, RustSignal)]
pub struct GetLatestVersionResponse {
    pub id: String,
    pub version: Option<String>,
    pub tag_name: Option<String>,
    pub release_notes: Option<String>,
    pub published_at: Option<String>,
    pub error: Option<String>,
}

#[derive(Deserialize, DartSignal)]
pub struct CompareVersionsRequest {
    pub id: String,
    pub versions: Vec<String>,
}

#[derive(Serialize, RustSignal)]
pub struct CompareVersionsResponse {
    pub id: String,
    pub latest: Option<String>,
}
