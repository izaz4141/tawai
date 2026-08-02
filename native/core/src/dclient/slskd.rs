use anyhow::{Context, Result};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde_json::Value;
use tokio::time::{Duration, sleep};

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::db;
use crate::db::database::DatabasePool;
use crate::signals::download::{DlGlance, DlListResponse, DlSearchResponse, DlSearchResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SlskdSearchResultFile {
    pub bitrate: i32,
    pub code: i32,
    pub extension: String,
    pub filename: String,
    pub length: i32,
    pub size: u64,
    pub is_locked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SlskdSearchResult {
    pub file_count: i32,
    pub files: Vec<SlskdSearchResultFile>,
    pub has_free_upload_slot: bool,
    pub upload_speed: u64,
    pub username: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SlskdSearchResponse {
    #[serde(alias = "id")]
    pub search_id: String,
    pub is_complete: bool,
    pub responses: Vec<SlskdSearchResult>,
    pub state: String,
}

#[derive(Debug, Clone, Deserialize)]
struct SlskdDownloadRequest {
    pub username: String,
    pub filename: String,
    pub user_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct SlskdDownloadResponse {
    #[serde(alias = "id")]
    pub download_id: String,
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct SlskdCancelRequest {
    #[serde(alias = "id")]
    pub download_id: String,
    pub username: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SlskdDownloadInfo {
    pub download_id: Option<String>,
    pub username: Option<String>,
    pub filename: Option<String>,
    pub state: Option<String>,
    pub size: Option<i64>,
    pub started_at: Option<String>,
    pub bytes_downloaded: Option<i64>,
    pub bytes_remaining: Option<i64>,
    pub average_speed: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SlskdUserTransfer {
    username: String,
    directories: Vec<SlskdDirectoryTransfer>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SlskdDirectoryTransfer {
    directory: String,
    file_count: i32,
    files: Vec<SlskdFileTransfer>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SlskdFileTransfer {
    id: String,
    username: String,
    filename: String,
    size: i64,
    state: String,
    started_at: Option<String>,
    bytes_transferred: i64,
    bytes_remaining: i64,
    average_speed: f64,
}

pub struct SlskdClient {
    base_url: String,
    api_key: String,
    client: reqwest::Client,
}

impl SlskdClient {
    pub fn new(base_url: String, api_key: String, client: reqwest::Client) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key,
            client,
        }
    }

    fn headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        let header_name = HeaderName::from_static("x-api-key");
        if let Ok(value) = HeaderValue::from_str(&self.api_key) {
            headers.insert(header_name, value);
        }
        headers
    }

    pub async fn search_internal(&self, query: &str) -> Result<SlskdSearchResponse> {
        let url = format!("{}/api/v0/searches", self.base_url);
        let body = serde_json::json!({
            "searchText": query,
        });
        let resp = self
            .client
            .post(&url)
            .headers(self.headers())
            .json(&body)
            .send()
            .await
            .context("Failed to send slskd search request")?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("slskd search failed {}: {}", status, text);
        }
        let data: serde_json::Value = resp
            .json()
            .await
            .context("Failed to parse slskd search response")?;
        let result = self.search_complete(&data["id"].to_string()).await?;
        Ok(result)
    }

    pub async fn search_complete(&self, id: &str) -> Result<SlskdSearchResponse> {
        let mut complete = false;
        let mut result: SlskdSearchResponse = {
            let mut data = serde_json::Value::Null;
            while !complete {
                sleep(Duration::from_secs(5)).await;
                let url = format!("{}/api/v0/searches/{}", self.base_url, &id);
                let resp = self
                    .client
                    .post(&url)
                    .query(&[("includeResponses", "true")])
                    .headers(self.headers())
                    .send()
                    .await
                    .context("Failed to send slskd search result request")?;
                if !resp.status().is_success() {
                    let status = resp.status();
                    let text = resp.text().await.unwrap_or_default();
                    anyhow::bail!("slskd search failed {}: {}", status, text);
                }
                data = resp
                    .json()
                    .await
                    .context("Failed to parse slskd search response")?;
                complete = data["isComplete"].as_bool().unwrap_or(false);
            }
            serde_json::from_value(data)?
        };
        result.responses.sort_by(
            |a, b| match (a.has_free_upload_slot, b.has_free_upload_slot) {
                (true, false) => std::cmp::Ordering::Greater,
                (false, true) => std::cmp::Ordering::Less,
                _ => b.upload_speed.cmp(&a.upload_speed),
            },
        );
        Ok(result)
    }

    pub async fn enqueue_download(
        &self,
        request: SlskdDownloadRequest,
    ) -> Result<SlskdDownloadResponse> {
        let url = format!(
            "{}/api/v0/transfers/downloads/{}",
            self.base_url, request.username
        );
        let body = serde_json::json!({
            "filename": request.filename,
        });
        let resp = self
            .client
            .post(&url)
            .headers(self.headers())
            .json(&body)
            .send()
            .await
            .context("Failed to send slskd download request")?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("slskd download failed {}: {}", status, text);
        }
        let data: serde_json::Value = resp
            .json()
            .await
            .context("Failed to parse slskd download response")?;
        let result = serde_json::from_value(data)?;
        Ok(result)
    }

    pub async fn list_downloads(&self) -> Result<Vec<SlskdDownloadInfo>> {
        let url = format!("{}/api/v0/transfers/downloads", self.base_url);
        let resp = self
            .client
            .get(&url)
            .headers(self.headers())
            .send()
            .await
            .context("Failed to fetch slskd downloads")?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("slskd list downloads failed {}: {}", status, text);
        }
        let users: Vec<SlskdUserTransfer> = resp
            .json()
            .await
            .context("Failed to parse slskd downloads response")?;
        let flat = users
            .into_iter()
            .flat_map(|u| {
                u.directories.into_iter().flat_map(move |d| {
                    d.files.into_iter().map(move |f| SlskdDownloadInfo {
                        download_id: Some(f.id),
                        username: Some(f.username),
                        filename: Some(f.filename),
                        state: Some(f.state),
                        size: Some(f.size),
                        started_at: f.started_at,
                        bytes_downloaded: Some(f.bytes_transferred),
                        bytes_remaining: Some(f.bytes_remaining),
                        average_speed: Some(f.average_speed),
                    })
                })
            })
            .collect();
        Ok(flat)
    }

    pub async fn cancel_download(&self, request: SlskdCancelRequest) -> Result<()> {
        let url = format!(
            "{}/api/v0/transfers/downloads/{}/{}",
            self.base_url, request.username, request.download_id
        );
        let resp = self
            .client
            .delete(&url)
            .headers(self.headers())
            .send()
            .await
            .context("Failed to cancel slskd download")?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("slskd cancel download failed {}: {}", status, text);
        }
        Ok(())
    }

    pub async fn download(
        &self,
        request: SlskdDownloadRequest,
        pool: &DatabasePool,
        local_id: &str,
    ) -> Result<SlskdDownloadResponse> {
        let resp = self.enqueue_download(request).await?;
        let download_id = resp.download_id.clone();
        self.poll_download(&download_id, pool, local_id).await
    }

    pub async fn poll_download(
        &self,
        download_id: &str,
        pool: &DatabasePool,
        local_id: &str,
    ) -> Result<SlskdDownloadResponse> {
        loop {
            sleep(Duration::from_secs(10)).await;

            let transfers = match self.list_downloads().await {
                Ok(t) => t,
                Err(e) => {
                    eprintln!("slskd download tracker: list_downloads failed: {}", e);
                    continue;
                }
            };

            let found = transfers
                .into_iter()
                .find(|t| t.download_id.as_deref() == Some(download_id));

            let transfer = match found {
                Some(t) => t,
                None => continue,
            };

            let raw_state = transfer.state.as_deref().unwrap_or("");
            let tawai_state = map_slskd_state(raw_state);
            let downloaded = transfer.bytes_downloaded.unwrap_or(0);
            let total = transfer.size.unwrap_or(0);
            let err = if tawai_state == "error" {
                format!("slskd: {}", raw_state)
            } else {
                String::new()
            };

            let _ = db::download::update_download_state(
                pool,
                local_id,
                tawai_state,
                &err,
                downloaded,
                total,
            )
            .await;

            if is_terminal_slskd_state(raw_state) {
                let success = raw_state == "Completed";
                let error = if success {
                    None
                } else {
                    Some(format!("slskd: {}", raw_state))
                };
                return Ok(SlskdDownloadResponse {
                    download_id: download_id.to_string(),
                    success,
                    error,
                });
            }
        }
    }

    pub async fn test_connection(&self) -> Result<String> {
        let url = format!("{}/api/v0/application/version", self.base_url);
        let resp = self
            .client
            .get(&url)
            .headers(self.headers())
            .send()
            .await
            .context("Failed to connect to slskd — is the server running?")?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("slskd health check failed {}: {}", status, text);
        }
        let version = resp.text().await.context("Failed to get slskd version")?;
        Ok(version)
    }

    // --- generalized dispatch methods ---

    pub async fn create(&self, url: &str, _dest: &str, extra: Option<Value>) -> Result<String> {
        let username = extra
            .as_ref()
            .and_then(|e| e.get("username").and_then(|v| v.as_str()))
            .ok_or_else(|| anyhow::anyhow!("slskd create requires username in extra"))?;
        let req = SlskdDownloadRequest {
            username: username.to_string(),
            filename: url.to_string(),
            user_id: None,
        };
        let resp = self.enqueue_download(req).await?;
        Ok(resp.download_id)
    }

    pub async fn list(
        &self,
        _offset: u64,
        _limit: u64,
        _statuses: Vec<String>,
    ) -> Result<DlListResponse> {
        let transfers = self.list_downloads().await?;
        let glances: Vec<DlGlance> = transfers
            .into_iter()
            .map(|t| DlGlance {
                id: t.download_id.unwrap_or_default(),
                name: t.filename.unwrap_or_default(),
                total_size: t.size.unwrap_or(0),
                downloaded: t.bytes_downloaded.unwrap_or(0),
                state: map_slskd_state(t.state.as_deref().unwrap_or("")).to_string(),
                speed: t.average_speed,
            })
            .collect();
        let total_count = glances.len() as i64;
        Ok(DlListResponse {
            downloads: glances,
            total_count,
        })
    }

    pub async fn cancel(&self, id: &str, pool: Option<&DatabasePool>) -> Result<()> {
        let username = match pool {
            Some(p) => {
                let record = db::download::get_download_by_source(p, "slskd", id)
                    .await?
                    .ok_or_else(|| anyhow::anyhow!("slskd download not found: {}", id))?;
                record.url.split('/').next().unwrap_or("").to_string()
            }
            None => anyhow::bail!("slskd cancel requires database pool"),
        };
        let req = SlskdCancelRequest {
            download_id: id.to_string(),
            username: username.to_string(),
        };
        self.cancel_download(req).await
    }

    pub async fn search(&self, query: &str) -> Result<DlSearchResponse> {
        let resp = self.search_internal(query).await?;
        let results: Vec<DlSearchResult> = resp
            .responses
            .into_iter()
            .flat_map(|r| {
                let username = r.username;
                r.files.into_iter().map(move |f| DlSearchResult {
                    filename: f.filename.clone(),
                    size: f.size,
                    source_type: "slskd".to_string(),
                    username: Some(username.clone()),
                    title: None,
                    thumbnail: None,
                    duration: None,
                    channel: None,
                    bitrate: Some(f.bitrate),
                    extension: Some(f.extension.clone()),
                    webpage_url: None,
                })
            })
            .collect();
        Ok(DlSearchResponse { results })
    }

    pub async fn sync(&self, pool: &DatabasePool) -> Result<u32> {
        let transfers = self.list_downloads().await?;
        let mut synced = 0;
        for t in transfers {
            let source_id = match &t.download_id {
                Some(id) => id.clone(),
                None => continue,
            };
            match db::download::get_download_by_source(pool, "slskd", &source_id).await? {
                Some(record) => {
                    let raw_state = t.state.as_deref().unwrap_or("");
                    let state = map_slskd_state(raw_state);
                    let downloaded = t.bytes_downloaded.unwrap_or(0);
                    let total = t.size.unwrap_or(0);
                    let err = if state == "error" {
                        format!("slskd: {}", raw_state)
                    } else {
                        String::new()
                    };
                    db::download::update_download_state(
                        pool, &record.id, state, &err, downloaded, total,
                    )
                    .await?;
                    synced += 1;
                }
                None => {}
            }
        }
        Ok(synced)
    }
}

pub fn map_slskd_state(state: &str) -> &'static str {
    match state {
        "Queued" | "QueuedLocally" | "QueuedRemotely" => "queued",
        "Transferring" => "downloading",
        "Completed" => "completed",
        "Cancelled" | "Cancelling" | "Rejected" | "TimedOut" | "Errored" => "error",
        _ => "downloading",
    }
}

pub fn is_terminal_slskd_state(state: &str) -> bool {
    matches!(
        state,
        "Completed" | "Cancelled" | "Rejected" | "TimedOut" | "Errored"
    )
}
