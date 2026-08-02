use std::sync::Arc;

use rinf::{DartSignal, RustSignal};
use tawai_core::app_context::AppContext;
use tawai_core::db;
use tawai_core::dclient::DownloadClient;

use crate::signals;
use crate::utils::logger;

pub async fn handle_list_downloads(context: Arc<AppContext>) {
    let receiver = signals::download::ListDownloadsRequest::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;
        let db = context.db().await;
        match db::download::list_downloads(db.pool(), &msg.user_id, msg.source.as_deref()).await {
            Ok(records) => {
                let rinf_records: Vec<signals::download::DownloadRecord> = records
                    .into_iter()
                    .map(|r| signals::download::DownloadRecord {
                        id: r.id,
                        source: r.source,
                        source_id: r.source_id,
                        url: r.url,
                        dest_path: r.dest_path,
                        filename: r.filename,
                        total_size: r.total_size,
                        downloaded: r.downloaded,
                        state: r.state,
                        error: r.error,
                        added_at: r.added_at,
                        updated_at: r.updated_at,
                    })
                    .collect();
                signals::download::ListDownloadsResponse {
                    id: msg.id,
                    downloads: rinf_records,
                }
                .send_signal_to_dart();
            }
            Err(e) => {
                logger::error(&format!("list_downloads failed: {}", e));
                signals::download::ListDownloadsResponse {
                    id: msg.id,
                    downloads: vec![],
                }
                .send_signal_to_dart();
            }
        }
    }
}

async fn client_from_type(
    source_type: &str,
    context: &AppContext,
) -> Result<DownloadClient, String> {
    let cfg = context.cfg().await;
    DownloadClient::from_config(source_type, &cfg, context.client()).map_err(|e| e.to_string())
}

pub async fn handle_download_create(context: Arc<AppContext>) {
    let receiver = signals::download::DownloadCreateRequest::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;
        let req_id = msg.id.clone();
        let source_type = msg.source_type.clone();
        let url = msg.url.clone();
        let dest = msg.dest.clone();
        let user_id = msg.user_id.clone();
        let extra = msg
            .extra
            .as_deref()
            .and_then(|s| serde_json::from_str(s).ok());

        let client = match client_from_type(&source_type, &context).await {
            Ok(c) => c,
            Err(e) => {
                signals::download::DownloadCreateResponse {
                    id: req_id,
                    download_id: String::new(),
                    success: false,
                    error: Some(e),
                }
                .send_signal_to_dart();
                continue;
            }
        };

        match client.create(&url, &dest, extra).await {
            Ok(download_id) => {
                let db = context.db().await;
                let _ = db::download::insert_download(
                    db.pool(),
                    &user_id,
                    &source_type,
                    &download_id,
                    &url,
                    &dest,
                    "",
                )
                .await;
                signals::download::DownloadCreateResponse {
                    id: req_id,
                    download_id,
                    success: true,
                    error: None,
                }
                .send_signal_to_dart();
            }
            Err(e) => {
                logger::error(&format!("{source_type} create failed: {e}"));
                signals::download::DownloadCreateResponse {
                    id: req_id,
                    download_id: String::new(),
                    success: false,
                    error: Some(e.to_string()),
                }
                .send_signal_to_dart();
            }
        }
    }
}

pub async fn handle_download_pause(context: Arc<AppContext>) {
    let receiver = signals::download::DownloadPauseRequest::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;
        let client = match client_from_type(&msg.source_type, &context).await {
            Ok(c) => c,
            Err(e) => {
                signals::download::DownloadPauseResponse {
                    id: msg.id,
                    success: false,
                    error: Some(e),
                }
                .send_signal_to_dart();
                continue;
            }
        };
        match client.pause(&msg.download_id).await {
            Ok(()) => {
                signals::download::DownloadPauseResponse {
                    id: msg.id,
                    success: true,
                    error: None,
                }
                .send_signal_to_dart();
            }
            Err(e) => {
                logger::error(&format!("{} pause failed: {}", msg.source_type, e));
                signals::download::DownloadPauseResponse {
                    id: msg.id,
                    success: false,
                    error: Some(e.to_string()),
                }
                .send_signal_to_dart();
            }
        }
    }
}

pub async fn handle_download_resume(context: Arc<AppContext>) {
    let receiver = signals::download::DownloadResumeRequest::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;
        let client = match client_from_type(&msg.source_type, &context).await {
            Ok(c) => c,
            Err(e) => {
                signals::download::DownloadResumeResponse {
                    id: msg.id,
                    success: false,
                    error: Some(e),
                }
                .send_signal_to_dart();
                continue;
            }
        };
        match client.resume(&msg.download_id).await {
            Ok(()) => {
                signals::download::DownloadResumeResponse {
                    id: msg.id,
                    success: true,
                    error: None,
                }
                .send_signal_to_dart();
            }
            Err(e) => {
                logger::error(&format!("{} resume failed: {}", msg.source_type, e));
                signals::download::DownloadResumeResponse {
                    id: msg.id,
                    success: false,
                    error: Some(e.to_string()),
                }
                .send_signal_to_dart();
            }
        }
    }
}

pub async fn handle_download_cancel(context: Arc<AppContext>) {
    let receiver = signals::download::DownloadCancelRequest::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;
        let client = match client_from_type(&msg.source_type, &context).await {
            Ok(c) => c,
            Err(e) => {
                signals::download::DownloadCancelResponse {
                    id: msg.id,
                    success: false,
                    error: Some(e),
                }
                .send_signal_to_dart();
                continue;
            }
        };
        let pool = context.db().await;
        match client
            .cancel(&msg.download_id, None, Some(pool.pool()))
            .await
        {
            Ok(()) => {
                signals::download::DownloadCancelResponse {
                    id: msg.id,
                    success: true,
                    error: None,
                }
                .send_signal_to_dart();
            }
            Err(e) => {
                logger::error(&format!("{} cancel failed: {}", msg.source_type, e));
                signals::download::DownloadCancelResponse {
                    id: msg.id,
                    success: false,
                    error: Some(e.to_string()),
                }
                .send_signal_to_dart();
            }
        }
    }
}

pub async fn handle_download_delete(context: Arc<AppContext>) {
    let receiver = signals::download::DownloadDeleteRequest::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;
        let client = match client_from_type(&msg.source_type, &context).await {
            Ok(c) => c,
            Err(e) => {
                signals::download::DownloadDeleteResponse {
                    id: msg.id,
                    success: false,
                    error: Some(e),
                }
                .send_signal_to_dart();
                continue;
            }
        };
        match client.delete(&msg.download_id, msg.delete_file).await {
            Ok(()) => {
                signals::download::DownloadDeleteResponse {
                    id: msg.id,
                    success: true,
                    error: None,
                }
                .send_signal_to_dart();
            }
            Err(e) => {
                logger::error(&format!("{} delete failed: {}", msg.source_type, e));
                signals::download::DownloadDeleteResponse {
                    id: msg.id,
                    success: false,
                    error: Some(e.to_string()),
                }
                .send_signal_to_dart();
            }
        }
    }
}

pub async fn handle_download_client_list(context: Arc<AppContext>) {
    let receiver = signals::download::DownloadClientListRequest::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;
        let client = match client_from_type(&msg.source_type, &context).await {
            Ok(c) => c,
            Err(e) => {
                signals::download::DownloadClientListResponse {
                    id: msg.id,
                    downloads: vec![],
                    total_count: 0,
                    success: false,
                    error: Some(e),
                }
                .send_signal_to_dart();
                continue;
            }
        };
        match client
            .list(
                msg.offset.unwrap_or(0),
                msg.limit.unwrap_or(50),
                msg.statuses.unwrap_or_default(),
            )
            .await
        {
            Ok(response) => {
                let items: Vec<signals::download::DlGlance> = response
                    .downloads
                    .into_iter()
                    .map(|d| signals::download::DlGlance {
                        id: d.id,
                        name: d.name,
                        total_size: d.total_size,
                        downloaded: d.downloaded,
                        state: d.state,
                        speed: d.speed,
                    })
                    .collect();
                signals::download::DownloadClientListResponse {
                    id: msg.id,
                    downloads: items,
                    total_count: response.total_count,
                    success: true,
                    error: None,
                }
                .send_signal_to_dart();
            }
            Err(e) => {
                logger::error(&format!("{} list failed: {}", msg.source_type, e));
                signals::download::DownloadClientListResponse {
                    id: msg.id,
                    downloads: vec![],
                    total_count: 0,
                    success: false,
                    error: Some(e.to_string()),
                }
                .send_signal_to_dart();
            }
        }
    }
}

pub async fn handle_download_sync(context: Arc<AppContext>) {
    let receiver = signals::download::DownloadSyncRequest::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;
        let client = match client_from_type(&msg.source_type, &context).await {
            Ok(c) => c,
            Err(e) => {
                signals::download::DownloadSyncResponse {
                    id: msg.id,
                    synced: 0,
                    success: false,
                    error: Some(e),
                }
                .send_signal_to_dart();
                continue;
            }
        };
        let db = context.db().await;
        match client.sync(db.pool()).await {
            Ok(synced) => {
                signals::download::DownloadSyncResponse {
                    id: msg.id,
                    synced,
                    success: true,
                    error: None,
                }
                .send_signal_to_dart();
            }
            Err(e) => {
                logger::error(&format!("{} sync failed: {}", msg.source_type, e));
                signals::download::DownloadSyncResponse {
                    id: msg.id,
                    synced: 0,
                    success: false,
                    error: Some(e.to_string()),
                }
                .send_signal_to_dart();
            }
        }
    }
}

pub async fn handle_download_poll(context: Arc<AppContext>) {
    let receiver = signals::download::DownloadsPollRequest::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;
        let cfg = context.cfg().await;
        let db = context.db().await;
        let pool = db.pool();

        for source_type in &["slskd", "nadekodon"] {
            if let Ok(client) = DownloadClient::from_config(source_type, &cfg, context.client()) {
                if let Err(e) = client.sync(pool).await {
                    logger::error(&format!("{source_type} poll sync failed: {e}"));
                }
            }
        }

        match db::download::list_downloads(pool, &msg.user_id, None).await {
            Ok(records) => {
                let rinf_records: Vec<signals::download::DownloadRecord> = records
                    .into_iter()
                    .map(|r| signals::download::DownloadRecord {
                        id: r.id,
                        source: r.source,
                        source_id: r.source_id,
                        url: r.url,
                        dest_path: r.dest_path,
                        filename: r.filename,
                        total_size: r.total_size,
                        downloaded: r.downloaded,
                        state: r.state,
                        error: r.error,
                        added_at: r.added_at,
                        updated_at: r.updated_at,
                    })
                    .collect();
                signals::download::DownloadsPollResponse {
                    id: msg.id,
                    downloads: rinf_records,
                }
                .send_signal_to_dart();
            }
            Err(e) => {
                logger::error(&format!("poll list_downloads failed: {e}"));
                signals::download::DownloadsPollResponse {
                    id: msg.id,
                    downloads: vec![],
                }
                .send_signal_to_dart();
            }
        }
    }
}

pub async fn handle_download_search(context: Arc<AppContext>) {
    let receiver = signals::download::DownloadSearchRequest::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;
        let client = match client_from_type(&msg.source_type, &context).await {
            Ok(c) => c,
            Err(e) => {
                signals::download::DownloadSearchResponse {
                    id: msg.id,
                    results: vec![],
                    success: false,
                    error: Some(e),
                }
                .send_signal_to_dart();
                continue;
            }
        };
        match client.search(&msg.query).await {
            Ok(response) => {
                let items: Vec<signals::download::DlSearchItem> = response
                    .results
                    .into_iter()
                    .map(|r| signals::download::DlSearchItem {
                        filename: r.filename,
                        size: r.size,
                        source_type: r.source_type,
                        username: r.username,
                        title: r.title,
                        thumbnail: r.thumbnail,
                        duration: r.duration,
                        channel: r.channel,
                        bitrate: r.bitrate,
                        extension: r.extension,
                        webpage_url: r.webpage_url,
                    })
                    .collect();
                signals::download::DownloadSearchResponse {
                    id: msg.id,
                    results: items,
                    success: true,
                    error: None,
                }
                .send_signal_to_dart();
            }
            Err(e) => {
                logger::error(&format!("{} search failed: {}", msg.source_type, e));
                signals::download::DownloadSearchResponse {
                    id: msg.id,
                    results: vec![],
                    success: false,
                    error: Some(e.to_string()),
                }
                .send_signal_to_dart();
            }
        }
    }
}

pub async fn handle_download_test_connection(context: Arc<AppContext>) {
    let receiver = signals::download::DownloadTestConnectionRequest::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;
        let client = match client_from_type(&msg.source_type, &context).await {
            Ok(c) => c,
            Err(e) => {
                signals::download::DownloadTestConnectionResponse {
                    id: msg.id,
                    success: false,
                    version: None,
                    error: Some(e),
                }
                .send_signal_to_dart();
                continue;
            }
        };
        match client.test_connection().await {
            Ok(version) => {
                signals::download::DownloadTestConnectionResponse {
                    id: msg.id,
                    success: true,
                    version: Some(version),
                    error: None,
                }
                .send_signal_to_dart();
            }
            Err(e) => {
                logger::error(&format!(
                    "{} test connection failed: {}",
                    msg.source_type, e
                ));
                signals::download::DownloadTestConnectionResponse {
                    id: msg.id,
                    success: false,
                    version: None,
                    error: Some(e.to_string()),
                }
                .send_signal_to_dart();
            }
        }
    }
}

pub async fn handle_download_get_info(context: Arc<AppContext>) {
    let receiver = signals::download::DownloadGetInfoRequest::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;
        let client = match client_from_type(&msg.source_type, &context).await {
            Ok(c) => c,
            Err(e) => {
                signals::download::DownloadGetInfoResponse {
                    id: msg.id,
                    info: String::new(),
                    success: false,
                    error: Some(e),
                }
                .send_signal_to_dart();
                continue;
            }
        };
        match client.get_info(&msg.url).await {
            Ok(info) => {
                signals::download::DownloadGetInfoResponse {
                    id: msg.id,
                    info: info.to_string(),
                    success: true,
                    error: None,
                }
                .send_signal_to_dart();
            }
            Err(e) => {
                logger::error(&format!("{} get_info failed: {}", msg.source_type, e));
                signals::download::DownloadGetInfoResponse {
                    id: msg.id,
                    info: String::new(),
                    success: false,
                    error: Some(e.to_string()),
                }
                .send_signal_to_dart();
            }
        }
    }
}
