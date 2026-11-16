use axum::{Json, extract::Query, response::IntoResponse};
use tawai_core::signals::version::{GetCurrentVersionRequest, GetCurrentVersionResponse};

#[utoipa::path(
    get,
    path = "/api/tawai/version/current",
    tags = ["tawai.version"],
    security(("ApiKeyAuth" = [])),
    params(
        ("app" = String, Query, description = "App identifier"),
    ),
    responses(
        (status = 200, description = "Current version", body = GetCurrentVersionResponse),
    )
)]
pub async fn handle_version_current(
    Query(params): Query<GetCurrentVersionRequest>,
) -> impl IntoResponse {
    let app = &params.app;

    match tawai_core::utils::version::get_local_version(app).await {
        Ok(version) => Json(GetCurrentVersionResponse {
            id: String::new(),
            version: Some(version),
            error: None,
        })
        .into_response(),
        Err(e) => {
            tawai_core::utils::logger::error(&format!("Cant get local {}: {:#?}", &app, &e));
            (
                axum::http::StatusCode::NOT_FOUND,
                Json(GetCurrentVersionResponse {
                    id: String::new(),
                    version: None,
                    error: Some(e),
                }),
            )
                .into_response()
        }
    }
}
