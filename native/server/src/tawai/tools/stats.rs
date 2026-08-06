use axum::{
    Json,
    extract::{Query, State},
    response::IntoResponse,
};

use crate::server::SharedState;
use tawai_core::signals::tools::{GetLibraryStatsRequest, GetLibraryStatsResponse};

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
    match tawai_core::tools::stats::get_library_stats(db.pool(), query.naming_pattern.as_deref())
        .await
    {
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
