use axum::{Json, extract::State};
use tawai_core::discovery::sync::{SyncRecsParams, sync_recs};
use tawai_core::signals::discovery::{SyncRecsRequest, SyncRecsResponse};

use crate::server::SharedState;

pub async fn handle_sync_recs(
    State(state): State<SharedState>,
    Json(payload): Json<SyncRecsRequest>,
) -> Json<SyncRecsResponse> {
    let ctx = state.context.clone();
    let db = ctx.db().await;
    let mk = ctx.master_key.read().await.clone();

    let result = sync_recs(SyncRecsParams {
        pool: db.pool(),
        client: ctx.client(),
        master_key: &mk,
        user_id: &payload.user_id,
        included_keys: &payload.included_keys,
    })
    .await;

    Json(SyncRecsResponse {
        id: payload.id,
        success: result.success,
        added_sources: result.added_sources,
        removed_sources: result.removed_sources,
        tracks_added: result.tracks_added,
        tracks_removed: result.tracks_removed,
        error: result.error,
    })
}
