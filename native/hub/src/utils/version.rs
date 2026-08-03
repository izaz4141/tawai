use rinf::{DartSignal, RustSignal};
use std::sync::Arc;

use tawai_core::app_context::AppContext;
use tawai_core::utils::version::{get_latest_version, get_local_version};

use crate::signals;

pub async fn handle_get_current_version(_context: Arc<AppContext>) {
    let receiver = signals::version::GetCurrentVersionRequest::get_dart_signal_receiver();

    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;

        match get_local_version(&msg.app).await {
            Ok(version) => {
                signals::version::GetCurrentVersionResponse {
                    id: msg.id,
                    version: Some(version),
                    error: None,
                }
                .send_signal_to_dart();
            }
            Err(e) => {
                signals::version::GetCurrentVersionResponse {
                    id: msg.id,
                    version: None,
                    error: Some(e),
                }
                .send_signal_to_dart();
            }
        }
    }
}

pub async fn handle_get_latest_version(context: Arc<AppContext>) {
    let receiver = signals::version::GetLatestVersionRequest::get_dart_signal_receiver();

    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;

        match get_latest_version(
            context.client(),
            &msg.owner,
            &msg.repo,
            msg.nightly,
            msg.atomic,
        )
        .await
        {
            Ok(info) => {
                signals::version::GetLatestVersionResponse {
                    id: msg.id,
                    version: Some(info.version),
                    tag_name: Some(info.tag_name),
                    release_notes: Some(info.release_notes),
                    published_at: Some(info.published_at),
                    error: info.error,
                }
                .send_signal_to_dart();
            }
            Err(e) => {
                signals::version::GetLatestVersionResponse {
                    id: msg.id,
                    version: None,
                    tag_name: None,
                    release_notes: None,
                    published_at: None,
                    error: Some(e),
                }
                .send_signal_to_dart();
            }
        }
    }
}
