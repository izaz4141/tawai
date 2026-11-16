use axum::{Json, response::IntoResponse};
use tawai_core::signals::version::{CompareVersionsRequest, CompareVersionsResponse};

#[utoipa::path(
    post,
    path = "/api/tawai/version/compare",
    tags = ["tawai.version"],
    security(("ApiKeyAuth" = [])),
    request_body = CompareVersionsRequest,
    responses(
        (status = 200, description = "Comparison result", body = CompareVersionsResponse)
    )
)]
pub async fn handle_compare_versions(
    Json(payload): Json<CompareVersionsRequest>,
) -> impl IntoResponse {
    let latest = tawai_core::utils::version::compare_versions(&payload.versions);
    Json(CompareVersionsResponse {
        id: payload.id,
        latest,
    })
}
