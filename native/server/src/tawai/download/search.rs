use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use tawai_core::dclient::DownloadClient;
use tawai_core::signals::download::{
    DlSearchResult, DownloadSearchRequest, DownloadSearchResponse,
};

use crate::server::SharedState;

#[utoipa::path(
    post,
    path = "/api/tawai/download/search",
    tags = ["tawai.download"],
    security(("ApiKeyAuth" = [])),
    request_body = DownloadSearchRequest,
    responses(
        (status = 200, description = "Search results", body = DownloadSearchResponse),
        (status = 400, description = "Search failed"),
    )
)]
pub async fn handle_search(
    State(state): State<SharedState>,
    Json(req): Json<DownloadSearchRequest>,
) -> impl IntoResponse {
    let cfg = state.context.cfg().await;
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

    match client.search(&req.query).await {
        Ok(response) => Json(DownloadSearchResponse {
            id: req.id,
            results: response.results,
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
