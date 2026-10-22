use std::sync::Arc;

use axum::{Extension, Json, extract::State, response::IntoResponse};

use crate::server::SharedState;
use tawai_core::signals::tools::{
    BatchRenameApplyRequest, BatchRenameApplyResponse, BatchRenamePreviewRequest,
    BatchRenamePreviewResponse, CheckNamingConventionRequest, CheckNamingConventionResponse,
};
use tawai_core::tools::rename::resolve_naming_pattern_from_context;

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
    Extension(user_id): Extension<String>,
    Json(body): Json<BatchRenamePreviewRequest>,
) -> impl IntoResponse {
    let db = state.context.db().await;
    // The client pattern is an explicit override; without one the global
    // pattern from the live config is used. This is a dry-run route, so an
    // override never mutates anything.
    let pattern = resolve_naming_pattern_from_context(
        &Arc::downgrade(&state.context),
        body.pattern.as_deref(),
    )
    .await;
    let result = tawai_core::tools::rename::batch_rename_preview(
        db.pool(),
        body.source_id.as_deref(),
        &pattern,
        &user_id,
    )
    .await;
    match result {
        Ok(previews) => Json(BatchRenamePreviewResponse {
            id: body.id,
            previews,
            pattern,
            error: None,
        }),
        Err(e) => Json(BatchRenamePreviewResponse {
            id: body.id,
            previews: vec![],
            pattern,
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
    Extension(user_id): Extension<String>,
    Json(body): Json<BatchRenameApplyRequest>,
) -> impl IntoResponse {
    let db = state.context.db().await;
    // The apply always renames with the global naming pattern: a
    // client-supplied pattern is only an override for the preview, and the
    // request carries no pattern field at all — a client snapshot can never
    // drive a mutation.
    let pattern = resolve_naming_pattern_from_context(&Arc::downgrade(&state.context), None).await;
    match tawai_core::tools::rename::batch_rename_apply(
        db.pool(),
        &body.file_paths,
        &body.track_ids,
        &pattern,
        &user_id,
    )
    .await
    {
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
    Extension(user_id): Extension<String>,
    Json(body): Json<CheckNamingConventionRequest>,
) -> impl IntoResponse {
    let db = state.context.db().await;
    // The client pattern is an explicit override; without one the global
    // pattern from the live config is used. This is a read-only route, so an
    // override never mutates anything.
    let pattern = resolve_naming_pattern_from_context(
        &Arc::downgrade(&state.context),
        body.pattern.as_deref(),
    )
    .await;
    match tawai_core::tools::rename::check_naming_convention(
        db.pool(),
        body.source_id.as_deref(),
        &pattern,
        &user_id,
    )
    .await
    {
        Ok(violations) => Json(CheckNamingConventionResponse {
            id: body.id,
            violations,
            error: None,
        }),
        Err(e) => Json(CheckNamingConventionResponse {
            id: body.id,
            violations: vec![],
            error: Some(e.to_string()),
        }),
    }
}
