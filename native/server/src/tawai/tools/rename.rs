use axum::{Json, extract::State, response::IntoResponse};

use crate::server::SharedState;
use tawai_core::signals::tools::{
    BatchRenameApplyRequest, BatchRenameApplyResponse, BatchRenamePreviewRequest,
    BatchRenamePreviewResponse, CheckNamingConventionRequest, CheckNamingConventionResponse,
};

#[utoipa::path(
    post,
    path = "/api/tawai/tools/rename-preview",
    tag = "tawai.tools",
    responses(
        (status = 200, description = "Preview of renamed files", body = BatchRenamePreviewResponse)
    )
)]
pub async fn handle_batch_rename_preview(
    State(state): State<SharedState>,
    Json(body): Json<BatchRenamePreviewRequest>,
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
        Ok(previews) => Json(BatchRenamePreviewResponse {
            id: body.id,
            previews,
            error: None,
        }),
        Err(e) => Json(BatchRenamePreviewResponse {
            id: body.id,
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
        (status = 200, description = "Result of renamed files", body = BatchRenameApplyResponse)
    )
)]
pub async fn handle_batch_rename_apply(
    State(state): State<SharedState>,
    Json(body): Json<BatchRenameApplyRequest>,
) -> impl IntoResponse {
    match tawai_core::tools::rename::batch_rename_apply(&body.file_paths, &body.pattern).await {
        Ok(results) => Json(BatchRenameApplyResponse {
            id: body.id,
            results,
            error: None,
        }),
        Err(e) => Json(BatchRenameApplyResponse {
            id: body.id,
            results: vec![],
            error: Some(e.to_string()),
        }),
    }
}

#[utoipa::path(
    post,
    path = "/api/tawai/tools/check-convention",
    tag = "tawai.tools",
    responses(
        (status = 200, description = "Naming convention violations", body = CheckNamingConventionResponse)
    )
)]
pub async fn handle_check_naming_convention(
    State(state): State<SharedState>,
    Json(query): Json<CheckNamingConventionRequest>,
) -> impl IntoResponse {
    let db = state.context.db().await;
    match tawai_core::tools::rename::check_naming_convention(
        db.pool(),
        query.source_id.as_deref(),
        &query.pattern,
    )
    .await
    {
        Ok(violations) => Json(CheckNamingConventionResponse {
            id: query.id,
            violations,
            error: None,
        }),
        Err(e) => Json(CheckNamingConventionResponse {
            id: query.id,
            violations: vec![],
            error: Some(e.to_string()),
        }),
    }
}
