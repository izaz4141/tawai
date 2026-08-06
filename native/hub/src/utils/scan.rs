use std::sync::Arc;
use std::sync::atomic::Ordering;

use rinf::{DartSignal, RustSignal};

use crate::signals::{self, library::ScanProgressSignal};
use crate::utils::logger;
use tawai_core::app_context::AppContext;
use tawai_core::db::{account, library_source};

pub async fn handle_scan_library(context: Arc<AppContext>) {
    use signals::library::*;
    let receiver = ScanLibraryRequest::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;
        let id = msg.id.clone();

        if context.scan_running.swap(true, Ordering::SeqCst) {
            ScanLibraryResponse {
                id,
                started: false,
                error: Some("A scan is already in progress".to_string()),
            }
            .send_signal_to_dart();
            continue;
        }
        *context.scan_progress.write().await = None;

        let db = context.db().await;
        let mk = context.master_key.read().await.clone();
        let Ok(Some(user)) = account::get_user_by_id(db.pool(), &msg.user_id, &mk).await else {
            context.scan_running.store(false, Ordering::SeqCst);
            logger::error(&format!("Cant find user with id: {}", msg.user_id));
            ScanLibraryResponse {
                id,
                started: false,
                error: Some(format!("Cant find user with id: {}", msg.user_id)),
            }
            .send_signal_to_dart();
            continue;
        };

        let sources = library_source::list_accessible_sources(db.pool(), &user.id, &user.role)
            .await
            .unwrap_or_default();

        if sources.is_empty() {
            context.scan_running.store(false, Ordering::SeqCst);
            ScanLibraryResponse {
                id,
                started: false,
                error: Some("No library sources configured".to_string()),
            }
            .send_signal_to_dart();
            continue;
        }

        ScanLibraryResponse {
            id,
            started: true,
            error: None,
        }
        .send_signal_to_dart();

        let (tx, mut rx) =
            tokio::sync::watch::channel(tawai_core::signals::library::ScanProgress::default());
        let ctx = context.clone();
        tokio::spawn(async move {
            while rx.changed().await.is_ok() {
                let p = rx.borrow_and_update().clone();
                *ctx.scan_progress.write().await = Some(p);
            }
        });

        let force = msg.force;
        let client = context.client().clone();
        let ctx = context.clone();
        tokio::spawn(async move {
            let core_result =
                tawai_core::audio::scan::run_scan(db.pool(), client, &sources, force, Some(tx))
                    .await;

            ctx.scan_running.store(false, Ordering::SeqCst);
            *ctx.scan_progress.write().await = Some(tawai_core::signals::library::ScanProgress {
                complete: true,
                tracks_found: core_result.tracks_found,
                new_tracks: core_result.new_tracks,
                duplicates: core_result.duplicates,
                deleted: core_result.deleted,
                error: core_result.error.clone(),
                ..Default::default()
            });
        });
    }
}

pub async fn handle_scan_status(context: Arc<AppContext>) {
    use signals::library::*;
    let receiver = ScanStatusRequest::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let id = signal_pack.message.id;
        let running = context.scan_running.load(Ordering::SeqCst);
        let progress = context.scan_progress.read().await.clone();
        ScanStatusResponse {
            id,
            running,
            progress: progress.map(|p| ScanProgressSignal {
                id: String::new(),
                current_file: p.current_file,
                files_scanned: p.files_scanned,
                total_files: p.total_files,
                stage: p.stage,
                complete: p.complete,
                tracks_found: p.tracks_found,
                new_tracks: p.new_tracks,
                duplicates: p.duplicates,
                deleted: p.deleted,
                current_source: p.current_source,
                error: p.error,
            }),
        }
        .send_signal_to_dart();
    }
}

pub async fn handle_scan_source(context: Arc<AppContext>) {
    use signals::library::*;
    let receiver = ScanSourceRequest::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;
        let id = msg.id.clone();

        let db = context.db().await;
        let mk = context.master_key.read().await.clone();
        let Ok(Some(user)) = account::get_user_by_id(db.pool(), &msg.user_id, &mk).await else {
            logger::error(&format!("Cant find user with id: {}", msg.user_id));
            ScanSourceResponse {
                id,
                started: false,
                error: Some("User not found".to_string()),
            }
            .send_signal_to_dart();
            continue;
        };

        // Enforce that the target source is accessible to the requesting user.
        let accessible = library_source::list_accessible_sources(db.pool(), &user.id, &user.role)
            .await
            .unwrap_or_default();
        let Some(source) = accessible.into_iter().find(|s| s.id == msg.source_id) else {
            ScanSourceResponse {
                id,
                started: false,
                error: Some("Library source is not accessible".to_string()),
            }
            .send_signal_to_dart();
            continue;
        };

        if context.scan_running.swap(true, Ordering::SeqCst) {
            ScanSourceResponse {
                id,
                started: false,
                error: Some("A scan is already in progress".to_string()),
            }
            .send_signal_to_dart();
            continue;
        }

        let (tx, mut rx) =
            tokio::sync::watch::channel(tawai_core::signals::library::ScanProgress::default());
        let progress_id = id.clone();
        let ctx = context.clone();
        tokio::spawn(async move {
            while rx.changed().await.is_ok() {
                let p = rx.borrow_and_update().clone();
                *ctx.scan_progress.write().await = Some(p.clone());
                ScanProgressSignal {
                    id: progress_id.clone(),
                    current_file: p.current_file,
                    files_scanned: p.files_scanned,
                    total_files: p.total_files,
                    stage: p.stage,
                    complete: p.complete,
                    tracks_found: p.tracks_found,
                    new_tracks: p.new_tracks,
                    duplicates: p.duplicates,
                    deleted: p.deleted,
                    current_source: p.current_source,
                    error: p.error,
                }
                .send_signal_to_dart();
            }
        });

        let client = context.client().clone();
        let sources = vec![source];
        let ctx = context.clone();
        tokio::spawn(async move {
            let pool = db.pool();
            let result =
                tawai_core::audio::scan::run_scan(&pool, client, &sources, msg.force, Some(tx))
                    .await;
            *ctx.scan_progress.write().await = Some(tawai_core::signals::library::ScanProgress {
                complete: true,
                tracks_found: result.tracks_found,
                new_tracks: result.new_tracks,
                duplicates: result.duplicates,
                deleted: result.deleted,
                error: result.error.clone(),
                ..Default::default()
            });
            ctx.scan_running.store(false, Ordering::SeqCst);
        });

        ScanSourceResponse {
            id,
            started: true,
            error: None,
        }
        .send_signal_to_dart();
    }
}

pub async fn handle_start_periodic_scan(context: Arc<AppContext>) {
    use signals::library::StartPeriodicScanRequest;
    use signals::library::StartPeriodicScanResponse;
    use std::time::Duration;
    use tawai_core::signals::library::ScanProgress;
    let receiver = StartPeriodicScanRequest::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let id = signal_pack.message.id;
        context.spawn_periodic_scan(
            Duration::from_secs(3600 * 3),
            Arc::new(|_p: ScanProgress| {}),
        );
        StartPeriodicScanResponse { id }.send_signal_to_dart();
    }
}
