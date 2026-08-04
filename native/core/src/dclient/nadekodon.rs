use anyhow::{Context, Result};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::time::{Duration, sleep};

use crate::db;
use crate::db::database::DatabasePool;
use crate::signals::download::{DlGlance, DlListResponse, DlSearchResponse, DlSearchResult};
use crate::utils::config::AppConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadGlance {
    pub id: String,
    pub download_type: String,
    pub name: String,
    pub dest: String,
    pub total_size: i64,
    pub downloaded: i64,
    pub uploaded: i64,
    pub dspeed: Option<f64>,
    pub state: String,
    pub category: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ListResponse {
    pub id: String,
    pub downloads: Vec<DownloadGlance>,
    pub total_count: i64,
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreateResponse {
    pub id: String,
    pub download_id: String,
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DetailsResponse {
    pub id: String,
    pub download_id: String,
    pub name: String,
    pub url: String,
    pub dest: String,
    pub total_size: i64,
    pub downloaded: i64,
    pub speed: Option<f64>,
    pub state: String,
    pub part_info: Vec<PartInfo>,
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartInfo {
    pub start: i64,
    pub end: i64,
    pub current: i64,
}

const TAWAI_CATEGORY: &str = "tawai";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NadekodonApiLoginResponse {
    api_key: String,
    access_token: String,
    csrf_token: String,
    expires_in: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NadekodonApiStatusResponse {
    status: String,
    version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NadekodonApiCreateRequest {
    url: String,
    dest: String,
    video_format: Option<String>,
    audio_format: Option<String>,
    is_ytdl: bool,
    cookie: Option<String>,
    user_agent: Option<String>,
    referer: Option<String>,
    category: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NadekodonApiListRequest {
    offset_index: u64,
    before: u64,
    after: u64,
    statuses: Vec<String>,
    categories: Vec<String>,
    search_query: Option<String>,
    sort_by: u32,
    ascending: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NadekodonApiGlance {
    id: String,
    download_type: String,
    name: String,
    dest: String,
    total_size: i64,
    downloaded: i64,
    uploaded: i64,
    dspeed: Option<f64>,
    uspeed: Option<f64>,
    state: String,
    referer: Option<String>,
    category: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NadekodonApiListResponse {
    list: Vec<NadekodonApiGlance>,
    total_count: i64,
    start_index: i64,
    categories: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NadekodonApiDetailResponse {
    id: String,
    name: String,
    url: String,
    dest: String,
    total_size: i64,
    downloaded: i64,
    speed: Option<f64>,
    state: String,
    part_info: Vec<NadekodonApiPartInfo>,
    uploaded: i64,
    upload_speed: Option<f64>,
    peers: Option<u32>,
    ratio: Option<f64>,
    eta: Option<f64>,
    referer: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NadekodonApiPartInfo {
    start: i64,
    end: i64,
    current: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NadekodonApiIdRequest {
    id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NadekodonApiDeleteRequest {
    id: String,
    delete_file: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NadekodonApiCategory {
    name: String,
    save_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NadekodonApiCategoryListResponse {
    categories: Vec<NadekodonApiCategory>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NadekodonApiCategoryUpdateRequest {
    categories: Vec<NadekodonApiCategory>,
}

// ytdl search types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YtdlSearchResult {
    pub id: String,
    pub title: String,
    pub url: String,
    pub thumbnail: Option<String>,
    pub duration: Option<f64>,
    pub channel: Option<String>,
    pub webpage_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YtdlSearchOutput {
    pub results: Vec<YtdlSearchResult>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YtdlFormat {
    pub format_id: String,
    pub ext: String,
    pub filesize: Option<u64>,
    pub url: String,
    pub vcodec: Option<String>,
    pub acodec: Option<String>,
    pub note: String,
    pub abr: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YtdlItem {
    pub name: String,
    pub thumbnail: Option<String>,
    pub videos: Vec<YtdlFormat>,
    pub audios: Vec<YtdlFormat>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YtdlQueryOutput {
    pub items: Vec<YtdlItem>,
    pub error: Option<String>,
}

pub struct NadekodonClient {
    base_url: String,
    api_key: String,
    client: reqwest::Client,
}

impl NadekodonClient {
    pub fn new(base_url: String, api_key: String, client: reqwest::Client) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key,
            client,
        }
    }

    pub fn from_config(cfg: &AppConfig, client: &reqwest::Client) -> Result<Self> {
        let url = cfg.value["nadekodon_url"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("nadekodon_url not configured"))?
            .to_string();
        let key = cfg.value["nadekodon_api_key"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("nadekodon_api_key not configured"))?
            .to_string();
        Ok(NadekodonClient::new(url, key, client.clone()))
    }

    fn headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        let header_name = HeaderName::from_static("x-api-key");
        if let Ok(value) = HeaderValue::from_str(&self.api_key) {
            headers.insert(header_name, value);
        }
        headers
    }

    pub async fn test_connection(&self) -> Result<String> {
        let url = format!("{}/api/nadeko/system/status", self.base_url);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to connect to nadekodon — is the server running?")?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("nadekodon health check failed {}: {}", status, text);
        }
        let data: NadekodonApiStatusResponse = resp
            .json()
            .await
            .context("Failed to parse nadekodon status response")?;
        Ok(data.version)
    }

    pub async fn login(&self, username: &str, password: &str) -> Result<String> {
        let url = format!("{}/api/nadeko/auth/login", self.base_url);

        let resp = self
            .client
            .post(&url)
            .basic_auth(username, Some(password))
            .send()
            .await
            .context("Failed to send nadekodon login request")?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("nadekodon login failed {}: {}", status, text);
        }
        let data: NadekodonApiLoginResponse = resp
            .json()
            .await
            .context("Failed to parse nadekodon login response")?;
        Ok(data.api_key)
    }

    pub async fn create_category(&self, name: &str, save_path: &str) -> Result<()> {
        let url = format!("{}/api/nadeko/download/categories", self.base_url);
        let body = NadekodonApiCategoryUpdateRequest {
            categories: vec![NadekodonApiCategory {
                name: name.to_string(),
                save_path: Some(save_path.to_string()),
            }],
        };
        let resp = self
            .client
            .post(&url)
            .headers(self.headers())
            .json(&body)
            .send()
            .await
            .context("Failed to create nadekodon category")?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("nadekodon create category failed {}: {}", status, text);
        }
        Ok(())
    }

    pub async fn list_categories(&self) -> Result<Vec<NadekodonApiCategory>> {
        let url = format!("{}/api/nadeko/download/categories", self.base_url);
        let resp = self
            .client
            .get(&url)
            .headers(self.headers())
            .send()
            .await
            .context("Failed to fetch nadekodon categories")?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("nadekodon list categories failed {}: {}", status, text);
        }
        let data: NadekodonApiCategoryListResponse = resp
            .json()
            .await
            .context("Failed to parse nadekodon categories response")?;
        Ok(data.categories)
    }

    pub async fn ensure_category(&self, name: &str, save_path: &str) -> Result<()> {
        let categories = self.list_categories().await?;
        if !categories.iter().any(|c| c.name == name) {
            self.create_category(name, save_path).await?;
        }
        Ok(())
    }

    pub async fn create_download(
        &self,
        url: &str,
        dest: &str,
        is_ytdl: bool,
        video_format: Option<String>,
        audio_format: Option<String>,
    ) -> Result<String> {
        let api_url = format!("{}/api/nadeko/download/create", self.base_url);
        let body = NadekodonApiCreateRequest {
            url: url.to_string(),
            dest: dest.to_string(),
            video_format,
            audio_format,
            is_ytdl,
            cookie: None,
            user_agent: None,
            referer: None,
            category: Some(TAWAI_CATEGORY.to_string()),
        };
        let resp = self
            .client
            .post(&api_url)
            .headers(self.headers())
            .json(&body)
            .send()
            .await
            .context("Failed to send nadekodon create download request")?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("nadekodon create download failed {}: {}", status, text);
        }
        let id_value: serde_json::Value = resp
            .json()
            .await
            .context("Failed to parse nadekodon create response")?;
        let download_id = id_value["id"].as_str().unwrap_or_default().to_string();
        Ok(download_id)
    }

    pub async fn list_downloads(
        &self,
        offset: u64,
        limit: u64,
        statuses: Vec<String>,
        categories: Vec<String>,
    ) -> Result<ListResponse> {
        let url = format!("{}/api/nadeko/download/list", self.base_url);
        let mut cats = categories;
        if !cats.iter().any(|c| c == TAWAI_CATEGORY) {
            cats.push(TAWAI_CATEGORY.to_string());
        }
        let body = NadekodonApiListRequest {
            offset_index: offset,
            before: limit,
            after: 0,
            statuses,
            categories: cats,
            search_query: None,
            sort_by: 0,
            ascending: false,
        };
        let resp = self
            .client
            .post(&url)
            .headers(self.headers())
            .json(&body)
            .send()
            .await
            .context("Failed to fetch nadekodon downloads")?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("nadekodon list downloads failed {}: {}", status, text);
        }
        let data: NadekodonApiListResponse = resp
            .json()
            .await
            .context("Failed to parse nadekodon list response")?;
        let downloads = data
            .list
            .into_iter()
            .map(|g| DownloadGlance {
                id: g.id,
                download_type: g.download_type,
                name: g.name,
                dest: g.dest,
                total_size: g.total_size,
                downloaded: g.downloaded,
                uploaded: g.uploaded,
                dspeed: g.dspeed,
                state: g.state,
                category: g.category,
            })
            .collect();
        Ok(ListResponse {
            id: String::new(),
            downloads,
            total_count: data.total_count,
            success: true,
            error: None,
        })
    }

    pub async fn get_details(&self, download_id: &str) -> Result<DetailsResponse> {
        let url = format!(
            "{}/api/nadeko/download/details/{}",
            self.base_url, download_id
        );
        let resp = self
            .client
            .get(&url)
            .headers(self.headers())
            .send()
            .await
            .context("Failed to fetch nadekodon download details")?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("nadekodon get details failed {}: {}", status, text);
        }
        let data: NadekodonApiDetailResponse = resp
            .json()
            .await
            .context("Failed to parse nadekodon details response")?;
        Ok(DetailsResponse {
            id: String::new(),
            download_id: data.id,
            name: data.name,
            url: data.url,
            dest: data.dest,
            total_size: data.total_size,
            downloaded: data.downloaded,
            speed: data.speed,
            state: data.state,
            part_info: data
                .part_info
                .into_iter()
                .map(|p| PartInfo {
                    start: p.start,
                    end: p.end,
                    current: p.current,
                })
                .collect(),
            success: true,
            error: None,
        })
    }

    async fn send_id_action(&self, path: &str, download_id: &str) -> Result<()> {
        let url = format!("{}{}", self.base_url, path);
        let body = NadekodonApiIdRequest {
            id: download_id.to_string(),
        };
        let resp = self
            .client
            .post(&url)
            .headers(self.headers())
            .json(&body)
            .send()
            .await
            .with_context(|| format!("Failed to send nadekodon action to {}", path))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("nadekodon {} failed {}: {}", path, status, text);
        }
        Ok(())
    }

    pub async fn pause_download(&self, download_id: &str) -> Result<()> {
        self.send_id_action("/api/nadeko/download/pause", download_id)
            .await
    }

    pub async fn resume_download(&self, download_id: &str) -> Result<()> {
        self.send_id_action("/api/nadeko/download/resume", download_id)
            .await
    }

    pub async fn cancel_download(&self, download_id: &str) -> Result<()> {
        self.send_id_action("/api/nadeko/download/cancel", download_id)
            .await
    }

    pub async fn delete_download(&self, download_id: &str, delete_file: bool) -> Result<()> {
        let url = format!("{}/api/nadeko/download/delete", self.base_url);
        let body = NadekodonApiDeleteRequest {
            id: download_id.to_string(),
            delete_file,
        };
        let resp = self
            .client
            .post(&url)
            .headers(self.headers())
            .json(&body)
            .send()
            .await
            .context("Failed to send nadekodon delete request")?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("nadekodon delete failed {}: {}", status, text);
        }
        Ok(())
    }

    pub async fn download(
        &self,
        url: &str,
        dest: &str,
        is_ytdl: bool,
        video_format: Option<String>,
        audio_format: Option<String>,
        pool: &DatabasePool,
        local_id: &str,
    ) -> Result<CreateResponse> {
        let download_id = self
            .create_download(url, dest, is_ytdl, video_format, audio_format)
            .await?;
        self.poll_download(&download_id, pool, local_id).await
    }

    pub async fn poll_download(
        &self,
        download_id: &str,
        pool: &DatabasePool,
        local_id: &str,
    ) -> Result<CreateResponse> {
        loop {
            sleep(Duration::from_secs(10)).await;

            let downloads = match self
                .list_downloads(0, 100, vec![], vec![TAWAI_CATEGORY.to_string()])
                .await
            {
                Ok(r) => r.downloads,
                Err(e) => {
                    eprintln!("nadekodon poll: list_downloads failed: {}", e);
                    continue;
                }
            };

            let found = downloads.into_iter().find(|d| d.id == download_id);

            let glance = match found {
                Some(d) => d,
                None => continue,
            };

            let tawai_state = map_nadekodon_state(&glance.state);
            let err = if tawai_state == "error" {
                format!("nadekodon: {}", glance.state)
            } else {
                String::new()
            };

            let _ = db::download::update_download_state(
                pool,
                local_id,
                tawai_state,
                &err,
                glance.downloaded,
                glance.total_size,
            )
            .await;

            if is_terminal_nadekodon_state(&glance.state) {
                let success = glance.state == "Completed";
                let error = if success {
                    None
                } else {
                    Some(format!("nadekodon: {}", glance.state))
                };
                return Ok(CreateResponse {
                    id: String::new(),
                    download_id: download_id.to_string(),
                    success,
                    error,
                });
            }
        }
    }

    pub async fn search_ytdl(&self, query: &str) -> Result<YtdlSearchOutput> {
        let url = format!("{}/api/nadeko/utils/search-ytdl", self.base_url);
        let body = serde_json::json!({ "query": query });
        let resp = self
            .client
            .post(&url)
            .headers(self.headers())
            .json(&body)
            .send()
            .await
            .context("Failed to send nadekodon search-ytdl request")?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("nadekodon search-ytdl failed {}: {}", status, text);
        }
        let data: YtdlSearchOutput = resp
            .json()
            .await
            .context("Failed to parse nadekodon search-ytdl response")?;
        Ok(data)
    }

    pub async fn query_ytdl(&self, url: &str) -> Result<YtdlQueryOutput> {
        let api_url = format!("{}/api/nadeko/utils/query-ytdl", self.base_url);
        let body = serde_json::json!({ "url": url });
        let resp = self
            .client
            .post(&api_url)
            .headers(self.headers())
            .json(&body)
            .send()
            .await
            .context("Failed to query nadekodon ytdl")?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("nadekodon query-ytdl failed {}: {}", status, text);
        }
        let data: YtdlQueryOutput = resp
            .json()
            .await
            .context("Failed to parse nadekodon query-ytdl response")?;
        Ok(data)
    }

    pub async fn sync_downloads(&self, pool: &DatabasePool) -> Result<u32> {
        let resp = self
            .list_downloads(0, 200, vec![], vec![TAWAI_CATEGORY.to_string()])
            .await?;
        let mut synced = 0;
        for glance in resp.downloads {
            if glance.category.as_deref() != Some("tawai") {
                continue;
            }
            match db::download::get_download_by_source(pool, "nadekodon", &glance.id).await? {
                Some(record) => {
                    let state = map_nadekodon_state(&glance.state);
                    let err = if state == "error" {
                        format!("nadekodon: {}", glance.state)
                    } else {
                        String::new()
                    };
                    db::download::update_download_state(
                        pool,
                        &record.id,
                        state,
                        &err,
                        glance.downloaded,
                        glance.total_size,
                    )
                    .await?;
                    if is_terminal_nadekodon_state(&glance.state) {
                        if let Some(fname) = glance.name.split('/').last() {
                            if !fname.is_empty() {
                                db::download::update_download_filename(pool, &record.id, fname)
                                    .await?;
                            }
                        }
                    }
                    synced += 1;
                }
                None => {}
            }
        }
        Ok(synced)
    }

    // --- generalized dispatch methods ---

    pub async fn create(&self, url: &str, dest: &str, extra: Option<Value>) -> Result<String> {
        let is_ytdl = extra
            .as_ref()
            .and_then(|e| e.get("is_ytdl").and_then(|v| v.as_bool()))
            .unwrap_or(true);
        let video_format = extra
            .as_ref()
            .and_then(|e| e.get("video_format").and_then(|v| v.as_str()))
            .map(String::from);
        let audio_format = extra
            .as_ref()
            .and_then(|e| e.get("audio_format").and_then(|v| v.as_str()))
            .map(String::from);
        self.create_download(url, dest, is_ytdl, video_format, audio_format)
            .await
    }

    pub async fn list(
        &self,
        offset: u64,
        limit: u64,
        statuses: Vec<String>,
    ) -> Result<DlListResponse> {
        let resp = self
            .list_downloads(offset, limit, statuses, vec![TAWAI_CATEGORY.to_string()])
            .await?;
        let glances = resp
            .downloads
            .into_iter()
            .map(|g| DlGlance {
                id: g.id,
                name: g.name,
                total_size: g.total_size,
                downloaded: g.downloaded,
                state: map_nadekodon_state(&g.state).to_string(),
                speed: g.dspeed,
            })
            .collect();
        Ok(DlListResponse {
            downloads: glances,
            total_count: resp.total_count,
        })
    }

    pub async fn pause(&self, id: &str) -> Result<()> {
        self.pause_download(id).await
    }

    pub async fn resume(&self, id: &str) -> Result<()> {
        self.resume_download(id).await
    }

    pub async fn cancel(&self, id: &str) -> Result<()> {
        self.cancel_download(id).await
    }

    pub async fn delete(&self, id: &str, delete_file: bool) -> Result<()> {
        self.delete_download(id, delete_file).await
    }

    pub async fn search(&self, query: &str) -> Result<DlSearchResponse> {
        let resp = self.search_ytdl(query).await?;
        let results = resp
            .results
            .into_iter()
            .map(|r| DlSearchResult {
                filename: r.title.clone(),
                size: 0,
                source_type: "nadekodon".to_string(),
                username: None,
                title: Some(r.title),
                thumbnail: r.thumbnail,
                duration: r.duration,
                channel: r.channel,
                bitrate: None,
                extension: None,
                webpage_url: r.webpage_url,
            })
            .collect();
        Ok(DlSearchResponse { results })
    }

    pub async fn get_info(&self, url: &str) -> Result<Value> {
        let resp = self.query_ytdl(url).await?;
        Ok(serde_json::to_value(resp)?)
    }

    pub async fn sync(&self, pool: &DatabasePool) -> Result<u32> {
        self.sync_downloads(pool).await
    }
}

pub async fn resolve_audio_url(
    cfg: &AppConfig,
    http_client: &reqwest::Client,
    artist: &str,
    title: &str,
) -> Result<Option<String>> {
    let client = NadekodonClient::from_config(cfg, http_client)?;
    let search = client.search_ytdl(&format!("{} {}", artist, title)).await?;
    let first = match search.results.into_iter().next() {
        Some(r) => r,
        None => return Ok(None),
    };
    let info_url = first.webpage_url.as_deref().unwrap_or(&first.url);
    if info_url.is_empty() {
        return Ok(None);
    }
    let info = client.query_ytdl(info_url).await?;
    let best = info
        .items
        .into_iter()
        .flat_map(|i| i.audios)
        .max_by(|a, b| {
            a.abr
                .partial_cmp(&b.abr)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    Ok(best.map(|f| f.url))
}

pub fn map_nadekodon_state(state: &str) -> &'static str {
    match state {
        "Queued" | "Downloading" | "Running" => "downloading",
        "Paused" => "paused",
        "Completed" => "completed",
        "Cancelled" => "cancelled",
        "Errored" => "error",
        _ => "downloading",
    }
}

pub fn is_terminal_nadekodon_state(state: &str) -> bool {
    matches!(state, "Completed" | "Cancelled" | "Errored")
}
