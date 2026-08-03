use std::sync::Arc;

use rinf::{DartSignal, RustSignal};

use crate::signals;
use crate::utils::logger;
use tawai_core::app_context::AppContext;
use tawai_core::db::account::get_user_role;
use tawai_core::db::library_source as core_libsrc;
use tawai_core::libsources::jellyfin::JellyfinParser;

pub async fn handle_add_library_source(context: Arc<AppContext>) {
    use signals::library::*;
    let receiver = AddLibrarySourceRequest::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;
        let db = context.db().await;

        let result = core_libsrc::add_source(
            db.pool(),
            &msg.user_id,
            &msg.url,
            &msg.name,
            &msg.source_type,
            "all",
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
        let result = core_libsrc::list_accessible_sources(db.pool(), &msg.user_id, &role).await;

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
        let result = core_libsrc::list_editable_sources(db.pool(), &msg.user_id, &role).await;

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

pub async fn handle_test_jellyfin_source(context: Arc<AppContext>) {
    use signals::discovery::*;
    let receiver = TestJellyfinSourceRequest::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;
        let client = context.client().clone();
        let parser = JellyfinParser::new(client);

        match parser.fetch_libraries(&msg.url).await {
            Ok(libraries) => {
                let infos: Vec<JellyfinLibraryInfo> =
                    libraries.into_iter().map(Into::into).collect();
                TestJellyfinSourceResponse {
                    id: msg.id,
                    libraries: infos,
                    error: None,
                }
                .send_signal_to_dart();
            }
            Err(e) => {
                logger::error(&format!("test_jellyfin_source failed: {}", e));
                TestJellyfinSourceResponse {
                    id: msg.id,
                    libraries: vec![],
                    error: Some(e.to_string()),
                }
                .send_signal_to_dart();
            }
        }
    }
}
