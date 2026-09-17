use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use tawai_core::libsources::jellyfin::JellyfinParser;
use tawai_core::libsources::tawai;
use tawai_core::signals::discovery::{
    ServerTestResult, TestSourceRequest, TestSourceResponse,
};

use crate::server::SharedState;

#[utoipa::path(
    post,
    path = "/api/tawai/library/sources/test",
    tags = ["tawai.library"],
    security(("ApiKeyAuth" = [])),
    request_body = TestSourceRequest,
    responses(
        (status = 200, description = "Library source connection tested", body = TestSourceResponse),
        (status = 400, description = "Unsupported source type or connection failed")
    )
)]
pub async fn handle_test_source(
    State(_state): State<SharedState>,
    Json(payload): Json<TestSourceRequest>,
) -> impl IntoResponse {
    let client = reqwest::Client::new();

    let (libraries, results, error) = match payload.source_type.as_str() {
        "tawai" => {
            let results = tawai::test_remote_urls(&client, &payload.urls).await;
            (vec![], results, None)
        }
        "jellyfin" => {
            let parser = JellyfinParser::new(client);
            let mut libraries = Vec::new();
            let mut results = Vec::new();
            let mut last_err: Option<String> = None;
            for url in &payload.urls {
                match parser.fetch_libraries(url).await {
                    Ok(libs) => {
                        if libraries.is_empty() {
                            libraries = libs;
                        }
                        results.push(ServerTestResult {
                            url: url.clone(),
                            reachable: true,
                            track_count: 0,
                            error: None,
                        });
                    }
                    Err(e) => {
                        last_err = Some(e.to_string());
                        results.push(ServerTestResult {
                            url: url.clone(),
                            reachable: false,
                            track_count: 0,
                            error: Some(e.to_string()),
                        });
                    }
                }
            }
            if libraries.is_empty() {
                (libraries, results, last_err)
            } else {
                (libraries, results, None)
            }
        }
        other => {
            return (
                StatusCode::BAD_REQUEST,
                Json(TestSourceResponse {
                    id: payload.id,
                    libraries: vec![],
                    results: vec![],
                    error: Some(format!("unsupported source_type: {other}")),
                }),
            )
                .into_response()
        }
    };

    if error.is_some() && libraries.is_empty() {
        (
            StatusCode::BAD_REQUEST,
            Json(TestSourceResponse {
                id: payload.id,
                libraries,
                results,
                error,
            }),
        )
            .into_response()
    } else {
        Json(TestSourceResponse {
            id: payload.id,
            libraries,
            results,
            error,
        })
        .into_response()
    }
}