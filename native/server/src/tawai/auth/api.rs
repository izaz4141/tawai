use crate::server::SharedState;
use axum::extract::{Query, State};
use axum::response::IntoResponse;
use axum::{Extension, Json};
use tawai_core::signals::account::{ApiKeyResponse, GenerateApiKeyRequest};

#[utoipa::path(
    get,
    path = "/api/tawai/auth/generate-api",
    tags = ["tawai.auth"],
    security(("ApiKeyAuth" = [])),
    params(("id" = String, Query, description = "Correlation id")),
    responses(
        (status = 200, description = "API key generated successfully", body = ApiKeyResponse)
    )
)]
pub async fn handle_generate_api(
    State(state): State<SharedState>,
    Extension(user_id): Extension<String>,
    Query(query): Query<GenerateApiKeyRequest>,
) -> impl IntoResponse {
    let db = state.context.db().await;
    let mk = state.context.master_key.read().await.clone();
    let new_key = tawai_core::db::account::regenerate_user_api_key(db.pool(), &user_id, &mk)
        .await
        .unwrap_or_default();

    Json(ApiKeyResponse {
        id: query.id,
        api_key: new_key,
    })
    .into_response()
}
