use anyhow::Result;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde_json::Value;
use std::time::Instant;
use tokio::time::{Duration, sleep};

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::audio::tags::audio_extensions;
use crate::db;
use crate::db::database::DatabasePool;
use crate::signals::download::{DlGlance, DlListResponse, DlSearchItem, DlSearchResponse};

fn is_audio_extension(ext: &str) -> bool {
    audio_extensions()
        .iter()
        .any(|&ae| ae.eq_ignore_ascii_case(ext))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SlskdSearchResultFile {
    // slskd serializes `File.BitRate`, and omits the field entirely when it is null.
    #[serde(rename = "bitRate", alias = "bitrate", default)]
    pub bitrate: Option<i32>,
    #[serde(default)]
    pub code: i32,
    #[serde(default)]
    pub extension: String,
    #[serde(default)]
    pub filename: String,
    #[serde(default)]
    pub length: Option<i32>,
    #[serde(default)]
    pub size: u64,
    #[serde(default)]
    pub is_locked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SlskdSearchResult {
    #[serde(default)]
    pub file_count: i32,
    #[serde(default)]
    pub files: Vec<SlskdSearchResultFile>,
    #[serde(default)]
    pub has_free_upload_slot: bool,
    #[serde(default)]
    pub upload_speed: u64,
    #[serde(default)]
    pub username: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SlskdSearchResponse {
    #[serde(alias = "id")]
    pub search_id: String,
    #[serde(default)]
    pub is_complete: bool,
    #[serde(default)]
    pub responses: Vec<SlskdSearchResult>,
    #[serde(default)]
    pub state: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SlskdDownloadRequest {
    pub username: String,
    pub filename: String,
    pub user_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SlskdDownloadResponse {
    #[serde(alias = "id")]
    pub download_id: String,
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SlskdCancelRequest {
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
            .await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("slskd search failed {}: {}", status, text);
        }
        let data: serde_json::Value = resp.json().await?;
        let id = data
            .get("id")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow::anyhow!("slskd search response is missing an id"))?
            .to_string();
        let result = self.search_complete(&id).await?;
        Ok(result)
    }

    pub async fn search_complete(&self, id: &str) -> Result<SlskdSearchResponse> {
        const POLL_INTERVAL: Duration = Duration::from_secs(2);
        const MAX_WAIT: Duration = Duration::from_secs(60);

        let deadline = Instant::now() + MAX_WAIT;

        loop {
            sleep(POLL_INTERVAL).await;

            let url = format!("{}/api/v0/searches/{}", self.base_url, id);
            let resp = self
                .client
                .get(&url)
                .query(&[("includeResponses", "true")])
                .headers(self.headers())
                .send()
                .await?;
            if !resp.status().is_success() {
                let status = resp.status();
                let text = resp.text().await.unwrap_or_default();
                anyhow::bail!("slskd search failed {}: {}", status, text);
            }

            let data: serde_json::Value = resp.json().await?;
            if data["isComplete"].as_bool().unwrap_or(false) {
                let mut result: SlskdSearchResponse = serde_json::from_value(data)?;
                result.responses.sort_by(|a, b| {
                    match (a.has_free_upload_slot, b.has_free_upload_slot) {
                        (true, false) => std::cmp::Ordering::Less,
                        (false, true) => std::cmp::Ordering::Greater,
                        _ => b.upload_speed.cmp(&a.upload_speed),
                    }
                });
                return Ok(result);
            }

            if Instant::now() >= deadline {
                anyhow::bail!(
                    "slskd search {} did not complete within {} seconds",
                    id,
                    MAX_WAIT.as_secs()
                );
            }
        }
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
            .await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("slskd download failed {}: {}", status, text);
        }
        let data: serde_json::Value = resp.json().await?;
        let result = serde_json::from_value(data)?;
        Ok(result)
    }

    pub async fn list_downloads(&self) -> Result<Vec<SlskdDownloadInfo>> {
        let url = format!("{}/api/v0/transfers/downloads", self.base_url);
        let resp = self.client.get(&url).headers(self.headers()).send().await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("slskd list downloads failed {}: {}", status, text);
        }
        let users: Vec<SlskdUserTransfer> = resp.json().await?;
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
            .await?;
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
        let resp = self.client.get(&url).headers(self.headers()).send().await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("slskd health check failed {}: {}", status, text);
        }
        let version = resp.text().await?;
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
                record
                    .url
                    .trim_end_matches('/')
                    .rsplit('/')
                    .next()
                    .unwrap_or("")
                    .to_string()
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
        let results: Vec<DlSearchItem> = resp
            .responses
            .into_iter()
            .flat_map(|r| {
                let username = r.username;
                r.files
                    .into_iter()
                    .filter(|f| is_audio_extension(&f.extension))
                    .map(move |f| DlSearchItem {
                        filename: f.filename.clone(),
                        size: f.size,
                        source_type: "slskd".to_string(),
                        username: Some(username.clone()),
                        title: None,
                        thumbnail: None,
                        duration: None,
                        channel: None,
                        bitrate: f.bitrate,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserializes_search_payload_with_omitted_nullable_file_fields() {
        let json = serde_json::json!({
            "id": "0f1e2d3c-4b5a-6978-8796-a5b4c3d2e1f0",
            "searchText": "aphex twin",
            "state": "Completed",
            "isComplete": true,
            "fileCount": 2,
            "responseCount": 1,
            "responses": [{
                "fileCount": 2,
                "hasFreeUploadSlot": true,
                "uploadSpeed": 1000000,
                "username": "peer",
                "files": [
                    {
                        "code": 1,
                        "extension": "mp3",
                        "filename": "Music\\Aphex Twin\\Windowlicker.mp3",
                        "size": 1234567,
                        "isLocked": false
                    },
                    {
                        "bitRate": 320,
                        "code": 1,
                        "extension": "flac",
                        "filename": "Music\\Aphex Twin\\Xtal.flac",
                        "length": 293,
                        "size": 50000000,
                        "isLocked": false
                    }
                ]
            }]
        });

        let parsed: SlskdSearchResponse = serde_json::from_value(json).unwrap();

        assert_eq!(parsed.search_id, "0f1e2d3c-4b5a-6978-8796-a5b4c3d2e1f0");
        assert!(parsed.is_complete);
        assert_eq!(parsed.state, "Completed");
        assert_eq!(parsed.responses.len(), 1);

        let files = &parsed.responses[0].files;
        assert_eq!(files[0].bitrate, None);
        assert_eq!(files[0].length, None);
        assert_eq!(files[1].bitrate, Some(320));
        assert_eq!(files[1].length, Some(293));
    }

    #[test]
    fn filters_non_audio_extensions_in_search() {
        let json = serde_json::json!({
            "id": "test-id",
            "searchText": "test",
            "state": "Completed",
            "isComplete": true,
            "fileCount": 4,
            "responseCount": 1,
            "responses": [{
                "fileCount": 4,
                "hasFreeUploadSlot": true,
                "uploadSpeed": 1000000,
                "username": "peer",
                "files": [
                    { "extension": "mp3", "filename": "song.mp3", "size": 1000, "code": 1, "isLocked": false },
                    { "extension": "flac", "filename": "song.flac", "size": 2000, "code": 1, "isLocked": false },
                    { "extension": "txt", "filename": "readme.txt", "size": 100, "code": 1, "isLocked": false },
                    { "extension": "jpg", "filename": "cover.jpg", "size": 5000, "code": 1, "isLocked": false },
                ]
            }]
        });

        let resp: SlskdSearchResponse = serde_json::from_value(json).unwrap();
        let results: Vec<DlSearchItem> = resp
            .responses
            .into_iter()
            .flat_map(|r| {
                let username = r.username;
                r.files
                    .into_iter()
                    .filter(|f| is_audio_extension(&f.extension))
                    .map(move |f| DlSearchItem {
                        filename: f.filename.clone(),
                        size: f.size,
                        source_type: "slskd".to_string(),
                        username: Some(username.clone()),
                        title: None,
                        thumbnail: None,
                        duration: None,
                        channel: None,
                        bitrate: f.bitrate,
                        extension: Some(f.extension.clone()),
                        webpage_url: None,
                    })
            })
            .collect();

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].extension, Some("mp3".to_string()));
        assert_eq!(results[1].extension, Some("flac".to_string()));
    }
}
