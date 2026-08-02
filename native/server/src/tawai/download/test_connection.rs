use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use serde::Deserialize;
use tawai_core::dclient::nadekodon::NadekodonClient;
use tawai_core::dclient::slskd::SlskdClient;

use crate::server::SharedState;

#[derive(Deserialize)]
pub struct TestConnectionBody {
    pub source_type: String,
    pub url: String,
    pub api_key: Option<String>,
}

#[utoipa::path(
    post,
    path = "/api/tawai/download/test-connection",
    tags = ["tawai.download"],
    security(("ApiKeyAuth" = [])),
    request_body = inline(serde_json::Value),
    responses(
        (status = 200, description = "Connection OK"),
        (status = 400, description = "Connection failed"),
    )
)]
pub async fn handle_test_connection(
    State(state): State<SharedState>,
    Json(body): Json<TestConnectionBody>,
) -> impl IntoResponse {
    let result = match body.source_type.as_str() {
        "nadekodon" => {
            let cfg = state.context.cfg().await;
            match NadekodonClient::from_config(&cfg, state.context.client()) {
                Ok(c) => c.test_connection().await,
                Err(e) => {
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(serde_json::json!({ "error": e.to_string() })),
                    )
                        .into_response();
                }
            }
        }
        "slskd" => {
            let key = body.api_key.unwrap_or_default();
            let client = SlskdClient::new(body.url, key, state.context.client().clone());
            client.test_connection().await
        }
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": "unknown source_type" })),
            )
                .into_response();
        }
    };

    match result {
        Ok(version) => Json(serde_json::json!({ "version": version })).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}
