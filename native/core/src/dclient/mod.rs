pub mod iprev;
pub mod nadekodon;
pub mod slskd;

use anyhow::Result;
use reqwest::Client;
use serde_json::Value;

use crate::db::database::DatabasePool;
use crate::signals::download::{DlListResponse, DlSearchResponse};
use crate::utils::config::AppConfig;

pub enum DownloadClient {
    Nadekodon(nadekodon::NadekodonClient),
    Slskd(slskd::SlskdClient),
}

impl DownloadClient {
    pub fn from_config(source_type: &str, cfg: &AppConfig, client: &Client) -> Result<Self> {
        match source_type {
            "nadekodon" => Ok(DownloadClient::Nadekodon(
                nadekodon::NadekodonClient::from_config(cfg, client)?,
            )),
            "slskd" => {
                let url = cfg.value["slskd_url"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("slskd_url not configured"))?
                    .to_string();
                let key = cfg.value["slskd_api_key"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("slskd_api_key not configured"))?
                    .to_string();
                Ok(DownloadClient::Slskd(slskd::SlskdClient::new(
                    url,
                    key,
                    client.clone(),
                )))
            }
            _ => Err(anyhow::anyhow!(
                "unknown download source type: {}",
                source_type
            )),
        }
    }

    pub async fn create(&self, url: &str, dest: &str, extra: Option<Value>) -> Result<String> {
        match self {
            DownloadClient::Nadekodon(c) => c.create(url, dest, extra).await,
            DownloadClient::Slskd(c) => c.create(url, dest, extra).await,
        }
    }

    pub async fn list(
        &self,
        offset: u64,
        limit: u64,
        statuses: Vec<String>,
    ) -> Result<DlListResponse> {
        match self {
            DownloadClient::Nadekodon(c) => c.list(offset, limit, statuses).await,
            DownloadClient::Slskd(c) => c.list(offset, limit, statuses).await,
        }
    }

    pub async fn pause(&self, id: &str) -> Result<()> {
        match self {
            DownloadClient::Nadekodon(c) => c.pause(id).await,
            DownloadClient::Slskd(_) => Err(anyhow::anyhow!("slskd does not support pause")),
        }
    }

    pub async fn resume(&self, id: &str) -> Result<()> {
        match self {
            DownloadClient::Nadekodon(c) => c.resume(id).await,
            DownloadClient::Slskd(_) => Err(anyhow::anyhow!("slskd does not support resume")),
        }
    }

    pub async fn cancel(&self, id: &str, pool: Option<&DatabasePool>) -> Result<()> {
        match self {
            DownloadClient::Nadekodon(c) => c.cancel(id).await,
            DownloadClient::Slskd(c) => c.cancel(id, pool).await,
        }
    }

    pub async fn delete(&self, id: &str, delete_file: bool) -> Result<()> {
        match self {
            DownloadClient::Nadekodon(c) => c.delete(id, delete_file).await,
            DownloadClient::Slskd(_) => Err(anyhow::anyhow!("slskd does not support delete")),
        }
    }

    pub async fn sync(&self, pool: &DatabasePool) -> Result<u32> {
        match self {
            DownloadClient::Nadekodon(c) => c.sync(pool).await,
            DownloadClient::Slskd(c) => c.sync(pool).await,
        }
    }

    pub async fn search(&self, query: &str) -> Result<DlSearchResponse> {
        match self {
            DownloadClient::Nadekodon(c) => c.search(query).await,
            DownloadClient::Slskd(c) => c.search(query).await,
        }
    }

    pub async fn test_connection(&self) -> Result<String> {
        match self {
            DownloadClient::Nadekodon(c) => c.test_connection().await,
            DownloadClient::Slskd(c) => c.test_connection().await,
        }
    }

    pub async fn get_info(&self, url: &str) -> Result<Value> {
        match self {
            DownloadClient::Nadekodon(c) => c.get_info(url).await,
            DownloadClient::Slskd(_) => Err(anyhow::anyhow!("slskd does not support get_info")),
        }
    }
}
