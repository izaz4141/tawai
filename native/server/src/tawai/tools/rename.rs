use axum::{Json, extract::State, response::IntoResponse};

use crate::server::SharedState;
use tawai_core::signals::tools::{
    BatchRenameRequest, BatchRenameResponse, CheckConventionRequest, CheckConventionResponse,
};

#[utoipa::path(
    post,
    path = "/api/tawai/tools/rename-preview",
    tag = "tawai.tools",
    responses(
        (status = 200, description = "Preview of renamed files", body = BatchRenameResponse)
    )
)]
pub async fn handle_batch_rename_preview(
    State(state): State<SharedState>,
    Json(body): Json<BatchRenameRequest>,
) -> impl IntoResponse {
    let result = if body.source_id.is_some() || body.file_paths.is_empty() {
        let db = state.context.db().await;
        tawai_core::tools::rename::batch_rename_preview_from_db(
            db.pool(),
            body.source_id.as_deref(),
            &body.pattern,
        )
        .await
    } else {
        tawai_core::tools::rename::batch_rename_preview(&body.file_paths, &body.pattern).await
    };
    match result {
        Ok(previews) => Json(BatchRenameResponse {
            previews,
            error: None,
        }),
        Err(e) => Json(BatchRenameResponse {
            previews: vec![],
            error: Some(e.to_string()),
        }),
    }
}

#[utoipa::path(
    post,
    path = "/api/tawai/tools/rename-apply",
    tag = "tawai.tools",
    responses(
        (status = 200, description = "Result of renamed files", body = BatchRenameResponse)
    )
)]
pub async fn handle_batch_rename_apply(
    State(state): State<SharedState>,
    Json(body): Json<BatchRenameRequest>,
) -> impl IntoResponse {
    match tawai_core::tools::rename::batch_rename_apply(&body.file_paths, &body.pattern).await {
        Ok(results) => Json(BatchRenameResponse {
            previews: results,
            error: None,
        }),
        Err(e) => Json(BatchRenameResponse {
            previews: vec![],
            error: Some(e.to_string()),
        }),
    }
}

#[utoipa::path(
    post,
    path = "/api/tawai/tools/check-convention",
    tag = "tawai.tools",
    responses(
        (status = 200, description = "Naming convention violations", body = CheckConventionResponse)
    )
)]
pub async fn handle_check_naming_convention(
    State(state): State<SharedState>,
    Json(query): Json<CheckConventionRequest>,
) -> impl IntoResponse {
    let db = state.context.db().await;
    match tawai_core::tools::rename::check_naming_convention(
        db.pool(),
        query.source_id.as_deref(),
        &query.pattern,
    )
    .await
    {
        Ok(violations) => Json(CheckConventionResponse {
            violations,
            error: None,
        }),
        Err(e) => Json(CheckConventionResponse {
            violations: vec![],
            error: Some(e.to_string()),
        }),
    }
}
