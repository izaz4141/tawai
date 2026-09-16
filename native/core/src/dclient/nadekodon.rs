use anyhow::Result;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::time::{Duration, sleep};

use crate::db;
use crate::db::database::DatabasePool;
use crate::signals::download::{DlGlance, DlListResponse, DlSearchItem, DlSearchResponse};
use crate::utils::config::AppConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadGlance {
    pub id: String,
    pub download_type: String,
    pub name: String,
    pub dest: String,
    pub total_size: Option<u64>,
    pub downloaded: u64,
    pub uploaded: u64,
    pub dspeed: Option<f64>,
    pub state: String,
    pub category: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ListResponse {
    pub id: String,
    pub downloads: Vec<DownloadGlance>,
    pub total_count: u64,
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
    pub total_size: Option<u64>,
    pub downloaded: u64,
    pub speed: Option<f64>,
    pub state: String,
    pub part_info: Vec<PartInfo>,
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartInfo {
    pub start: u64,
    pub end: u64,
    pub current: u64,
}

const TAWAI_CATEGORY: &str = "tawai";

/// All download states nadekodon can report; an empty statuses list would
/// match nothing on nadekodon's side.
const ALL_NADEKODON_STATES: &[&str] = &[
    "Queued",
    "Running",
    "Paused",
    "Completed",
    "Seeding",
    "StalledDL",
    "StalledUP",
    "Cancelled",
    "Error",
];

fn all_nadekodon_states() -> Vec<String> {
    ALL_NADEKODON_STATES
        .iter()
        .map(|s| s.to_string())
        .collect()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NadekodonApiLoginResponse {
    api_key: String,
    access_token: String,
    csrf_token: String,
    expires_in: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NadekodonApiStatusResponse {
    status: String,
    version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NadekodonApiCreateRequest {
    id: String,
    url: Option<String>,
    dest: String,
    video_format: Option<YtdlFormat>,
    audio_format: Option<YtdlFormat>,
    #[serde(rename = "is_ytdl")]
    is_ytdl: bool,
    cookie: Option<String>,
    user_agent: Option<String>,
    referer: Option<String>,
    category: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NadekodonApiCreateResponse {
    download_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NadekodonApiListRequest {
    id: String,
    #[serde(rename = "offset_index")]
    offset_index: u32,
    before: u32,
    after: u32,
    statuses: Vec<String>,
    tag: Option<i32>,
    categories: Vec<String>,
    search_query: Option<String>,
    sort_by: Option<i32>,
    ascending: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NadekodonApiGlance {
    id: String,
    download_type: String,
    name: String,
    dest: String,
    total_size: Option<u64>,
    downloaded: u64,
    uploaded: u64,
    dspeed: f64,
    uspeed: Option<f64>,
    state: String,
    referer: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NadekodonApiListResponse {
    id: String,
    list: Vec<NadekodonApiGlance>,
    total_count: u64,
    start_index: u64,
    tag: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NadekodonApiDetailResponse {
    id: String,
    name: String,
    url: String,
    dest: String,
    total_size: Option<u64>,
    downloaded: u64,
    speed: f64,
    state: String,
    part_info: Vec<NadekodonApiPartInfo>,
    uploaded: Option<u64>,
    upload_speed: Option<f64>,
    peers: Option<u64>,
    ratio: Option<f64>,
    eta: Option<String>,
    referer: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NadekodonApiPartInfo {
    start: u64,
    end: u64,
    current: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NadekodonApiIdRequest {
    id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NadekodonApiDeleteRequest {
    id: String,
    delete_file: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NadekodonApiCategory {
    name: String,
    save_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NadekodonApiCategoryListResponse {
    id: String,
    categories: Vec<NadekodonApiCategory>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NadekodonApiCategoryUpdateRequest {
    id: String,
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
    pub id: String,
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
    pub id: String,
    pub items: Vec<YtdlItem>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NadekodonApiSearchRequest {
    id: String,
    query: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NadekodonApiQueryRequest {
    id: String,
    url: String,
}

/// Deserialize a response body, including the raw text in the error so a
/// shape mismatch is diagnosable instead of reqwest's opaque "error decoding
/// response body".
fn decode_body<T>(raw: &str) -> Result<T>
where
    T: serde::de::DeserializeOwned,
{
    serde_json::from_str(raw)
        .map_err(|e| anyhow::anyhow!("nadekodon response decode failed: {e}; raw body: {raw}"))
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

    fn correlation_id() -> String {
        uuid::Uuid::new_v4().to_string()
    }

    pub async fn test_connection(&self) -> Result<String> {
        let url = format!("{}/api/nadeko/system/status", self.base_url);
        let resp = self.client.get(&url).send().await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("nadekodon health check failed {}: {}", status, text);
        }
        let raw = resp.text().await.unwrap_or_default();
        let data: NadekodonApiStatusResponse = decode_body(&raw)?;
        Ok(data.version)
    }

    pub async fn login(&self, username: &str, password: &str) -> Result<String> {
        let url = format!("{}/api/nadeko/auth/login", self.base_url);

        let resp = self
            .client
            .post(&url)
            .basic_auth(username, Some(password))
            .send()
            .await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("nadekodon login failed {}: {}", status, text);
        }
        let raw = resp.text().await.unwrap_or_default();
        let data: NadekodonApiLoginResponse = decode_body(&raw)?;
        Ok(data.api_key)
    }

    pub async fn create_category(&self, name: &str, save_path: &str) -> Result<()> {
        let url = format!("{}/api/nadeko/download/categories", self.base_url);
        let body = NadekodonApiCategoryUpdateRequest {
            id: Self::correlation_id(),
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
            .await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("nadekodon create category failed {}: {}", status, text);
        }
        Ok(())
    }

    pub async fn list_categories(&self) -> Result<Vec<NadekodonApiCategory>> {
        let url = format!(
            "{}/api/nadeko/download/categories?id={}",
            self.base_url,
            Self::correlation_id()
        );
        let resp = self.client.get(&url).headers(self.headers()).send().await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("nadekodon list categories failed {}: {}", status, text);
        }
        let raw = resp.text().await.unwrap_or_default();
        let data: NadekodonApiCategoryListResponse = decode_body(&raw)?;
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
        video_format: Option<YtdlFormat>,
        audio_format: Option<YtdlFormat>,
    ) -> Result<String> {
        let api_url = format!("{}/api/nadeko/download/create", self.base_url);
        let body = NadekodonApiCreateRequest {
            id: Self::correlation_id(),
            url: Some(url.to_string()),
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
            .await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("nadekodon create download failed {}: {}", status, text);
        }
        let raw = resp.text().await.unwrap_or_default();
        let data: NadekodonApiCreateResponse = decode_body(&raw)?;
        data.download_ids
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("nadekodon create: response contained no download ids"))
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
            id: Self::correlation_id(),
            offset_index: offset as u32,
            before: 0,
            after: limit.saturating_sub(1) as u32,
            statuses,
            tag: None,
            categories: cats,
            search_query: None,
            sort_by: None,
            ascending: None,
        };
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
            anyhow::bail!("nadekodon list downloads failed {}: {}", status, text);
        }
        let raw = resp.text().await.unwrap_or_default();
        let data: NadekodonApiListResponse = serde_json::from_str(&raw).map_err(|e| {
            anyhow::anyhow!("nadekodon list decode failed: {e}; raw body: {raw}")
        })?;
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
                dspeed: Some(g.dspeed),
                state: g.state,
                category: None,
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
        let resp = self.client.get(&url).headers(self.headers()).send().await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("nadekodon get details failed {}: {}", status, text);
        }
        let raw = resp.text().await.unwrap_or_default();
        let data: NadekodonApiDetailResponse = decode_body(&raw)?;
        Ok(DetailsResponse {
            id: String::new(),
            download_id: data.id,
            name: data.name,
            url: data.url,
            dest: data.dest,
            total_size: data.total_size,
            downloaded: data.downloaded,
            speed: Some(data.speed),
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
            .await?;
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
            .await?;
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
        video_format: Option<YtdlFormat>,
        audio_format: Option<YtdlFormat>,
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
                .list_downloads(
                    0,
                    100,
                    all_nadekodon_states(),
                    vec![TAWAI_CATEGORY.to_string()],
                )
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
                glance.downloaded as i64,
                glance.total_size.unwrap_or(0) as i64,
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
        let body = NadekodonApiSearchRequest {
            id: Self::correlation_id(),
            query: query.to_string(),
        };
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
            anyhow::bail!("nadekodon search-ytdl failed {}: {}", status, text);
        }
        let raw = resp.text().await.unwrap_or_default();
        let data: YtdlSearchOutput = decode_body(&raw)?;
        Ok(data)
    }

    pub async fn query_ytdl(&self, url: &str) -> Result<YtdlQueryOutput> {
        let api_url = format!("{}/api/nadeko/utils/query-ytdl", self.base_url);
        let body = NadekodonApiQueryRequest {
            id: Self::correlation_id(),
            url: url.to_string(),
        };
        let resp = self
            .client
            .post(&api_url)
            .headers(self.headers())
            .json(&body)
            .send()
            .await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("nadekodon query-ytdl failed {}: {}", status, text);
        }
        let raw = resp.text().await.unwrap_or_default();
        let data: YtdlQueryOutput = decode_body(&raw)?;
        Ok(data)
    }

    pub async fn sync_downloads(&self, pool: &DatabasePool) -> Result<u32> {
        let resp = self
            .list_downloads(
                0,
                200,
                all_nadekodon_states(),
                vec![TAWAI_CATEGORY.to_string()],
            )
            .await?;
        let mut synced = 0;
        for glance in resp.downloads {
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
                        glance.downloaded as i64,
                        glance.total_size.unwrap_or(0) as i64,
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
            .and_then(|e| e.get("video_format").and_then(|v| serde_json::from_value(v.clone()).ok()))
            .or_else(|| {
                extra
                    .as_ref()
                    .and_then(|e| e.get("video_format").and_then(|v| v.as_str()))
                    .map(|fmt_id| YtdlFormat {
                        format_id: fmt_id.to_string(),
                        ext: String::new(),
                        filesize: None,
                        url: String::new(),
                        vcodec: None,
                        acodec: None,
                        note: String::new(),
                    })
            });
        let audio_format = extra
            .as_ref()
            .and_then(|e| e.get("audio_format").and_then(|v| serde_json::from_value(v.clone()).ok()))
            .or_else(|| {
                extra
                    .as_ref()
                    .and_then(|e| e.get("audio_format").and_then(|v| v.as_str()))
                    .map(|fmt_id| YtdlFormat {
                        format_id: fmt_id.to_string(),
                        ext: String::new(),
                        filesize: None,
                        url: String::new(),
                        vcodec: None,
                        acodec: None,
                        note: String::new(),
                    })
            });
        self.create_download(url, dest, is_ytdl, video_format, audio_format)
            .await
    }

    pub async fn list(
        &self,
        offset: u64,
        limit: u64,
        statuses: Vec<String>,
    ) -> Result<DlListResponse> {
        let statuses = if statuses.is_empty() {
            all_nadekodon_states()
        } else {
            statuses
        };
        let resp = self
            .list_downloads(offset, limit, statuses, vec![TAWAI_CATEGORY.to_string()])
            .await?;
        let glances = resp
            .downloads
            .into_iter()
            .map(|g| DlGlance {
                id: g.id,
                name: g.name,
                total_size: g.total_size.unwrap_or(0) as i64,
                downloaded: g.downloaded as i64,
                state: map_nadekodon_state(&g.state).to_string(),
                speed: g.dspeed,
            })
            .collect();
        Ok(DlListResponse {
            downloads: glances,
            total_count: resp.total_count as i64,
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
            .map(|r| DlSearchItem {
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

/// Resolve the best audio format for an artist + title search via nadekodon's
/// ytdl query. Shared by streaming and download.
pub async fn resolve_audio_format(
    cfg: &AppConfig,
    http_client: &reqwest::Client,
    artist: &str,
    title: &str,
) -> Result<Option<YtdlFormat>> {
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
    Ok(info
        .items
        .into_iter()
        .flat_map(|i| i.audios)
        .max_by(|a, b| {
            a.filesize
                .partial_cmp(&b.filesize)
                .unwrap_or(std::cmp::Ordering::Equal)
        }))
}

pub async fn resolve_audio_url(
    cfg: &AppConfig,
    http_client: &reqwest::Client,
    artist: &str,
    title: &str,
) -> Result<Option<String>> {
    Ok(resolve_audio_format(cfg, http_client, artist, title)
        .await?
        .map(|f| f.url))
}

pub fn map_nadekodon_state(state: &str) -> &'static str {
    match state {
        "Queued" | "Downloading" | "Running" | "Seeding" | "StalledDL" | "StalledUP" => {
            "downloading"
        }
        "Paused" => "paused",
        "Completed" => "completed",
        "Cancelled" => "cancelled",
        "Error" => "error",
        _ => "downloading",
    }
}

pub fn is_terminal_nadekodon_state(state: &str) -> bool {
    matches!(state, "Completed" | "Cancelled" | "Error")
}
