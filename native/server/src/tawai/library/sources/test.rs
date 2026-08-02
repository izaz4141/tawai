use axum::{Json, response::IntoResponse};
use serde::Deserialize;
use tawai_core::libsources::jellyfin::JellyfinParser;

#[derive(Deserialize)]
pub struct TestJellyfinPayload {
    pub url: String,
}

pub async fn handle_test_jellyfin_source(
    Json(payload): Json<TestJellyfinPayload>,
) -> impl IntoResponse {
    let client = reqwest::Client::new();
    let parser = JellyfinParser::new(client);

    match parser.fetch_libraries(&payload.url).await {
        Ok(libraries) => Json(serde_json::json!({ "libraries": libraries })).into_response(),
        Err(e) => {
            tawai_core::utils::logger::error(&format!("test_jellyfin_source failed: {}", e));
            (
                axum::http::StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
                .into_response()
        }
    }
}
