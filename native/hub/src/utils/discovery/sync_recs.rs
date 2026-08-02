use std::sync::Arc;

use tawai_core::app_context::AppContext;
use tawai_core::discovery::sync::{SyncRecsParams, sync_recs};

use crate::signals::discovery::{SyncRecsRequest, SyncRecsResponse};

use rinf::{DartSignal, RustSignal};

pub async fn handle_sync_recs(context: Arc<AppContext>) {
    let receiver = SyncRecsRequest::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;
        let db = context.db().await;
        let mk = context.master_key.read().await.clone();

        let result = sync_recs(SyncRecsParams {
            pool: db.pool(),
            client: context.client(),
            master_key: &mk,
            user_id: &msg.user_id,
            included_keys: &msg.included_keys,
        })
        .await;

        SyncRecsResponse {
            id: msg.id,
            success: result.success,
            added_sources: result.added_sources,
            removed_sources: result.removed_sources,
            tracks_added: result.tracks_added,
            tracks_removed: result.tracks_removed,
            error: result.error,
        }
        .send_signal_to_dart();
    }
}
