use std::sync::Arc;

use axum::{
    Json,
    extract::{Query, State},
    response::IntoResponse,
};

use crate::server::SharedState;
use tawai_core::signals::tools::{GetLibraryStatsRequest, GetLibraryStatsResponse};
use tawai_core::tools::rename::resolve_naming_pattern_from_context;

#[utoipa::path(
    get,
    path = "/api/tawai/tools/stats",
    tag = "tawai.tools",
    responses(
        (status = 200, description = "Library statistics", body = GetLibraryStatsResponse)
    )
)]
pub async fn handle_get_library_stats(
    State(state): State<SharedState>,
    Query(query): Query<GetLibraryStatsRequest>,
) -> impl IntoResponse {
    let db = state.context.db().await;
    // The query parameter is an explicit override; without one the global
    // pattern from the live config is used.
    let pattern = resolve_naming_pattern_from_context(
        &Arc::downgrade(&state.context),
        query.naming_pattern.as_deref(),
    )
    .await;
    match tawai_core::tools::stats::get_library_stats(db.pool(), &pattern).await {
        Ok(stats) => Json(GetLibraryStatsResponse {
            id: String::new(),
            stats: Some(stats),
            error: None,
        }),
        Err(e) => Json(GetLibraryStatsResponse {
            id: String::new(),
            stats: None,
            error: Some(e.to_string()),
        }),
    }
}
