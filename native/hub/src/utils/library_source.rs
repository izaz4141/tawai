use std::sync::Arc;

use rinf::{DartSignal, RustSignal};

use crate::signals;
use crate::utils::logger;
use tawai_core::app_context::AppContext;
use tawai_core::db::account::get_user_role;
use tawai_core::db::library_source as core_libsrc;
use tawai_core::libsources::jellyfin::JellyfinParser;
use tawai_core::libsources::tawai;

pub async fn handle_add_library_source(context: Arc<AppContext>) {
    use signals::library::*;
    let receiver = AddLibrarySourceRequest::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;
        let db = context.db().await;
        let mk = context.master_key.read().await.clone();

        let result = core_libsrc::add_source(
            db.pool(),
            &msg.user_id,
            &msg.urls,
            &msg.name,
            &msg.source_type,
            "all",
            &mk,
        )
        .await;

        match result {
            Ok(source_id) => {
                AddLibrarySourceResponse {
                    id: msg.id,
                    source_id,
                    success: true,
                }
                .send_signal_to_dart();
            }
            Err(e) => {
                logger::error(&format!("add library source failed: {}", e));
                AddLibrarySourceResponse {
                    id: msg.id,
                    source_id: String::new(),
                    success: false,
                }
                .send_signal_to_dart();
            }
        }
    }
}

pub async fn handle_remove_library_source(context: Arc<AppContext>) {
    use signals::library::*;
    let receiver = RemoveLibrarySourceRequest::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;
        let db = context.db().await;

        match core_libsrc::remove_source(db.pool(), &msg.source_id).await {
            Ok(true) => {
                RemoveLibrarySourceResponse {
                    id: msg.id,
                    success: true,
                }
                .send_signal_to_dart();
            }
            Ok(false) => {
                RemoveLibrarySourceResponse {
                    id: msg.id,
                    success: false,
                }
                .send_signal_to_dart();
            }
            Err(e) => {
                logger::error(&format!("remove library source failed: {}", e));
                RemoveLibrarySourceResponse {
                    id: msg.id,
                    success: false,
                }
                .send_signal_to_dart();
            }
        }
    }
}

pub async fn handle_list_library_sources(context: Arc<AppContext>) {
    use signals::library::*;
    let receiver = ListLibrarySourcesRequest::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;
        let db = context.db().await;
        let mk = context.master_key.read().await.clone();
        let role = match get_user_role(db.pool(), &msg.user_id).await {
            Ok(Some(r)) => r,
            _ => {
                ListLibrarySourcesResponse {
                    id: msg.id,
                    sources: vec![],
                }
                .send_signal_to_dart();
                continue;
            }
        };
        let result = core_libsrc::list_accessible_sources(db.pool(), &msg.user_id, &role, &mk).await;

        match result {
            Ok(sources) => {
                ListLibrarySourcesResponse {
                    id: msg.id,
                    sources: sources.into_iter().map(Into::into).collect(),
                }
                .send_signal_to_dart();
            }
            Err(e) => {
                logger::error(&format!("list library sources failed: {}", e));
                ListLibrarySourcesResponse {
                    id: msg.id,
                    sources: vec![],
                }
                .send_signal_to_dart();
            }
        }
    }
}

pub async fn handle_list_editable_sources(context: Arc<AppContext>) {
    use signals::library::*;
    let receiver = ListEditableSourcesRequest::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;
        let db = context.db().await;
        let mk = context.master_key.read().await.clone();
        let role = match get_user_role(db.pool(), &msg.user_id).await {
            Ok(Some(r)) => r,
            _ => {
                ListEditableSourcesResponse {
                    id: msg.id,
                    sources: vec![],
                }
                .send_signal_to_dart();
                continue;
            }
        };
        let result = core_libsrc::list_editable_sources(db.pool(), &msg.user_id, &role, &mk).await;

        match result {
            Ok(sources) => {
                ListEditableSourcesResponse {
                    id: msg.id,
                    sources: sources.into_iter().map(Into::into).collect(),
                }
                .send_signal_to_dart();
            }
            Err(e) => {
                logger::error(&format!("list editable sources failed: {}", e));
                ListEditableSourcesResponse {
                    id: msg.id,
                    sources: vec![],
                }
                .send_signal_to_dart();
            }
        }
    }
}

pub async fn handle_test_source(context: Arc<AppContext>) {
    use signals::discovery::*;
    let receiver = TestSourceRequest::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;
        let client = context.client().clone();
        let resp = match msg.source_type.as_str() {
            "tawai" => {
                let (libraries, results) = tawai::test_remote_urls(&client, &msg.urls).await;
                TestSourceResponse {
                    id: msg.id,
                    libraries: libraries.into_iter().map(Into::into).collect(),
                    results: results.into_iter().map(Into::into).collect(),
                    error: None,
                }
            }
            _ => {
                // Default: treat as jellyfin for backwards compatibility.
                let parser = JellyfinParser::new(client);
                let mut libraries = Vec::new();
                let mut results = Vec::new();
                let mut last_err: Option<String> = None;
                for url in &msg.urls {
                    match parser.fetch_libraries(url).await {
                        Ok(libs) => {
                            if libraries.is_empty() {
                                libraries = libs;
                            }
                            results.push(ServerTestResult {
                                url: url.clone(),
                                reachable: true,
                                track_count: 0,
                                error: None,
                            });
                        }
                        Err(e) => {
                            last_err = Some(e.to_string());
                            results.push(ServerTestResult {
                                url: url.clone(),
                                reachable: false,
                                track_count: 0,
                                error: Some(e.to_string()),
                            });
                        }
                    }
                }
                let error = if libraries.is_empty() {
                    last_err
                } else {
                    None
                };
                TestSourceResponse {
                    id: msg.id,
                    libraries: libraries.into_iter().map(Into::into).collect(),
                    results: results.into_iter().map(Into::into).collect(),
                    error,
                }
            }
        };
        resp.send_signal_to_dart();
    }
}
