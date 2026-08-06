use axum::{Json, extract::State, response::IntoResponse};

use crate::server::SharedState;
use tawai_core::signals::tools::FindDuplicatesResponse;

#[utoipa::path(
    post,
    path = "/api/tawai/tools/find-duplicates",
    tag = "tawai.tools",
    responses(
        (status = 200, description = "Duplicate groups", body = FindDuplicatesResponse)
    )
)]
pub async fn handle_find_duplicates(
    State(state): State<SharedState>,
    Json(body): Json<tawai_core::signals::tools::FindDuplicatesRequest>,
) -> impl IntoResponse {
    let db = state.context.db().await;
    let options = tawai_core::tools::duplicates::FindDuplicatesOptions {
        check_fingerprint: body.check_fingerprint,
        check_mbid: body.check_mbid,
        check_file_size_duration: body.check_file_size_duration,
        check_title_artist: body.check_title_artist,
        min_confidence: body.min_confidence.unwrap_or(0.0),
        source_id: body.source_id,
    };
    match tawai_core::tools::duplicates::find_duplicates(db.pool(), &options).await {
        Ok(result) => Json(result),
        Err(e) => Json(FindDuplicatesResponse {
            id: String::new(),
            groups: vec![],
            total_duplicates: 0,
            total_groups: 0,
            error: Some(e.to_string()),
        }),
    }
}
