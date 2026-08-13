use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

// --- Generic download record / list ---

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
    #[serde(default)]
    pub user_id: String,
    pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ListDownloadsResponse {
    #[serde(default)]
    pub id: String,
    pub downloads: Vec<DownloadRecord>,
}

// --- generic download client types ---

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DlGlance {
    pub id: String,
    pub name: String,
    pub total_size: i64,
    pub downloaded: i64,
    pub state: String,
    pub speed: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DlListResponse {
    pub downloads: Vec<DlGlance>,
    pub total_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DlSearchItem {
    pub filename: String,
    pub size: u64,
    pub source_type: String,
    pub username: Option<String>,
    pub title: Option<String>,
    pub thumbnail: Option<String>,
    pub duration: Option<f64>,
    pub channel: Option<String>,
    pub bitrate: Option<i32>,
    pub extension: Option<String>,
    pub webpage_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DlSearchResponse {
    pub results: Vec<DlSearchItem>,
}

// --- Create ---

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DownloadCreateRequest {
    pub id: String,
    pub source_type: String,
    pub url: String,
    pub dest: String,
    pub user_id: String,
    pub extra: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DownloadCreateResponse {
    pub id: String,
    pub download_id: String,
    pub success: bool,
    pub error: Option<String>,
}

// --- Pause ---

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DownloadPauseRequest {
    pub id: String,
    pub source_type: String,
    pub download_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DownloadPauseResponse {
    pub id: String,
    pub success: bool,
    pub error: Option<String>,
}

// --- Resume ---

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DownloadResumeRequest {
    pub id: String,
    pub source_type: String,
    pub download_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DownloadResumeResponse {
    pub id: String,
    pub success: bool,
    pub error: Option<String>,
}

// --- Cancel ---

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DownloadCancelRequest {
    pub id: String,
    pub source_type: String,
    pub download_id: String,
    pub extra: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DownloadCancelResponse {
    pub id: String,
    pub success: bool,
    pub error: Option<String>,
}

// --- Delete ---

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DownloadDeleteRequest {
    pub id: String,
    pub source_type: String,
    pub download_id: String,
    pub delete_file: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DownloadDeleteResponse {
    pub id: String,
    pub success: bool,
    pub error: Option<String>,
}

// --- Client list ---

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DownloadClientListRequest {
    pub id: String,
    pub source_type: String,
    pub offset: Option<u64>,
    pub limit: Option<u64>,
    pub statuses: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DownloadClientListResponse {
    pub id: String,
    pub downloads: Vec<DlGlance>,
    pub total_count: i64,
    pub success: bool,
    pub error: Option<String>,
}

// --- Sync ---

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DownloadSyncRequest {
    pub id: String,
    pub source_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DownloadSyncResponse {
    pub id: String,
    pub synced: u32,
    pub success: bool,
    pub error: Option<String>,
}

// --- Search ---

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DownloadSearchRequest {
    pub id: String,
    pub source_type: String,
    pub query: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DownloadSearchResponse {
    pub id: String,
    pub results: Vec<DlSearchItem>,
    pub success: bool,
    pub error: Option<String>,
}

// --- Test connection ---

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DownloadTestConnectionRequest {
    pub id: String,
    pub source_type: String,
    pub url: Option<String>,
    pub token: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DownloadTestConnectionResponse {
    pub id: String,
    pub success: bool,
    pub version: Option<String>,
    pub error: Option<String>,
}

// --- Get info ---

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DownloadGetInfoRequest {
    pub id: String,
    pub source_type: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DownloadGetInfoResponse {
    pub id: String,
    pub info: String,
    pub success: bool,
    pub error: Option<String>,
}

// --- Poll (sync all sources + list) ---

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DownloadsPollRequest {
    pub id: String,
    pub user_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DownloadsPollResponse {
    pub id: String,
    pub downloads: Vec<DownloadRecord>,
}
