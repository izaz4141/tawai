use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use tawai_core::dclient::DownloadClient;
use tawai_core::signals::download::{DownloadCreateRequest, DownloadCreateResponse};

use crate::server::SharedState;

#[utoipa::path(
    post,
    path = "/api/tawai/download/create",
    tags = ["tawai.download"],
    security(("ApiKeyAuth" = [])),
    request_body = DownloadCreateRequest,
    responses(
        (status = 200, description = "Download created", body = DownloadCreateResponse),
        (status = 400, description = "Create failed"),
    )
)]
pub async fn handle_create(
    State(state): State<SharedState>,
    Json(req): Json<DownloadCreateRequest>,
) -> impl IntoResponse {
    let cfg = state.context.cfg().await;
    let mk = state.context.master_key.read().await.clone();

    let is_recommendation_direct =
        req.source_type.starts_with("recommendation:") || req.source_type == "preview";
    if is_recommendation_direct {
        let db = state.context.db().await;
        let pool = db.pool();
        let client = state.context.client().clone();
        let result = if let Some(parser) =
            tawai_core::libsources::get_parser(&req.source_type, client.clone(), pool)
        {
            parser
                .download(
                    pool,
                    &req.url,
                    &req.dest,
                    &client,
                    &cfg,
                    &req.user_id,
                    req.extra.as_deref(),
                    &mk,
                )
                .await
        } else {
            tawai_core::libsources::recommendation::download(
                pool,
                &req.url,
                &req.dest,
                &client,
                &cfg,
                &req.user_id,
                req.extra.as_deref(),
                &mk,
            )
            .await
        };
        return match result {
            Ok(download_id) => Json(DownloadCreateResponse {
                id: req.id,
                download_id,
                success: true,
                error: None,
            })
            .into_response(),
            Err(e) => (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
                .into_response(),
        };
    }

    let client = match DownloadClient::from_config(&req.source_type, &cfg, state.context.client()) {
        Ok(c) => c,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
                .into_response();
        }
    };

    let extra = req
        .extra
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok());
    match client.create(&req.url, &req.dest, extra).await {
        Ok(download_id) => Json(DownloadCreateResponse {
            id: req.id,
            download_id,
            success: true,
            error: None,
        })
        .into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}
