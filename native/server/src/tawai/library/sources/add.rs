use axum::{
    Json,
    extract::{Extension, State},
    response::IntoResponse,
};
use serde::Deserialize;
use tawai_core::db::{account, library_source};

use crate::server::SharedState;

#[derive(Deserialize)]
pub struct AddSourcePayload {
    pub url: String,
    pub name: String,
    pub source_type: String,
}

pub async fn handle_add_source(
    State(state): State<SharedState>,
    Extension(username): Extension<String>,
    Json(payload): Json<AddSourcePayload>,
) -> impl IntoResponse {
    let db = state.context.db().await;
    let mk = state.context.master_key.read().await.clone();
    let user = match account::get_user_by_username(db.pool(), &username, &mk).await {
        Ok(Some(u)) => u,
        _ => return axum::http::StatusCode::UNAUTHORIZED.into_response(),
    };

    match library_source::add_source(
        db.pool(),
        &user.id,
        &payload.url,
        &payload.name,
        &payload.source_type,
        "all",
    )
    .await
    {
        Ok(source_id) => Json(serde_json::json!({ "source_id": source_id })).into_response(),
        Err(e) => {
            tawai_core::utils::logger::error(&format!("add source failed: {}", e));
            axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
