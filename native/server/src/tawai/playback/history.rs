use axum::{
    Json,
    extract::{Extension, Query, State},
    response::IntoResponse,
};
use serde::Deserialize;
use serde::Serialize;
use tawai_core::db::{account, history};
use tawai_core::signals::playback::PlaybackRecord;
use utoipa::ToSchema;

use crate::server::SharedState;

#[derive(Deserialize)]
pub struct HistoryQuery {
    pub limit: Option<i32>,
}

#[derive(Serialize, ToSchema)]
pub struct HistoryResponse {
    pub records: Vec<PlaybackRecord>,
}

#[utoipa::path(
    get,
    path = "/api/tawai/playback/history",
    tags = ["tawai.playback"],
    security(("ApiKeyAuth" = [])),
    params(
        ("limit" = Option<i32>, Query, description = "Max records to return"),
    ),
    responses(
        (status = 200, description = "Playback history", body = HistoryResponse)
    )
)]
pub async fn handle_get_history(
    State(state): State<SharedState>,
    Extension(username): Extension<String>,
    Query(query): Query<HistoryQuery>,
) -> impl IntoResponse {
    let db = state.context.db().await;
    let mk = state.context.master_key.read().await.clone();
    let user_id = match account::get_user_by_username(db.pool(), &username, &mk).await {
        Ok(Some(u)) => u.id,
        _ => return axum::http::StatusCode::UNAUTHORIZED.into_response(),
    };
    let limit = query.limit.unwrap_or(50);

    match history::get_recent_history(db.pool(), &user_id, limit).await {
        Ok(records) => Json(HistoryResponse { records }).into_response(),
        Err(e) => {
            tawai_core::utils::logger::error(&format!("get history failed: {}", e));
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(HistoryResponse { records: vec![] }),
            )
                .into_response()
        }
    }
}
