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
        let force = msg.force;

        if context.scan_running.swap(true, Ordering::SeqCst) {
            ScanProgressSignal {
                id,
                stage: "done".into(),
                complete: true,
                error: Some("A scan is already in progress".to_string()),
                ..Default::default()
            }
            .send_signal_to_dart();
            continue;
        }
        let db = context.db().await;
        let mk = context.master_key.read().await.clone();
        let Ok(Some(user)) = account::get_user_by_id(db.pool(), &msg.user_id, &mk).await else {
            logger::error(&format!("Cant find user with id: {}", msg.user_id));
            continue;
        };

        let sources = library_source::list_accessible_sources(db.pool(), &user.id, &user.role)
            .await
            .unwrap_or_default();

        if sources.is_empty() {
            context.scan_running.store(false, Ordering::SeqCst);
            ScanProgressSignal {
                id,
                stage: "done".into(),
                complete: true,
                error: Some("No library sources configured".to_string()),
                ..Default::default()
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

        let core_result = tawai_core::audio::scan::run_scan(
            db.pool(),
            context.client().clone(),
            &sources,
            force,
            Some(tx),
        )
        .await;

        context.scan_running.store(false, Ordering::SeqCst);
        *context.scan_progress.write().await = Some(tawai_core::signals::library::ScanProgress {
            complete: true,
            tracks_found: core_result.tracks_found,
            new_tracks: core_result.new_tracks,
            duplicates: core_result.duplicates,
            deleted: core_result.deleted,
            error: core_result.error.clone(),
            ..Default::default()
        });
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
            Duration::from_secs(300),
            Arc::new(|p: ScanProgress| {
                ScanProgressSignal {
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
                }
                .send_signal_to_dart();
            }),
        );
        StartPeriodicScanResponse { id }.send_signal_to_dart();
    }
}
