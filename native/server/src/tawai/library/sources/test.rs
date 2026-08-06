use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use tawai_core::libsources::jellyfin::JellyfinParser;
use tawai_core::signals::discovery::{TestJellyfinSourceRequest, TestJellyfinSourceResponse};

use crate::server::SharedState;

#[utoipa::path(
    post,
    path = "/api/tawai/library/sources/test-jellyfin",
    tags = ["tawai.library"],
    security(("ApiKeyAuth" = [])),
    request_body = TestJellyfinSourceRequest,
    responses(
        (status = 200, description = "Jellyfin connection tested", body = TestJellyfinSourceResponse),
        (status = 400, description = "Connection failed")
    )
)]
pub async fn handle_test_jellyfin_source(
    State(_state): State<SharedState>,
    Json(payload): Json<TestJellyfinSourceRequest>,
) -> impl IntoResponse {
    let client = reqwest::Client::new();
    let parser = JellyfinParser::new(client);

    match parser.fetch_libraries(&payload.url).await {
        Ok(libraries) => Json(TestJellyfinSourceResponse {
            id: payload.id,
            libraries,
            error: None,
        })
        .into_response(),
        Err(e) => {
            tawai_core::utils::logger::error(&format!("test_jellyfin_source failed: {}", e));
            (
                StatusCode::BAD_REQUEST,
                Json(TestJellyfinSourceResponse {
                    id: payload.id,
                    libraries: vec![],
                    error: Some(e.to_string()),
                }),
            )
                .into_response()
        }
    }
}
