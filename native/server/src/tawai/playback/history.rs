use axum::{
    Json,
    extract::{Extension, Query, State},
    response::IntoResponse,
};
use tawai_core::db::history;
use tawai_core::signals::playback::{GetHistoryRequest, GetHistoryResponse};

use crate::server::SharedState;

#[utoipa::path(
    get,
    path = "/api/tawai/playback/history",
    tags = ["tawai.playback"],
    security(("ApiKeyAuth" = [])),
    params(
        ("limit" = Option<i32>, Query, description = "Max records to return"),
    ),
    responses(
        (status = 200, description = "Playback history", body = GetHistoryResponse)
    )
)]
pub async fn handle_get_history(
    State(state): State<SharedState>,
    Extension(user_id): Extension<String>,
    Query(query): Query<GetHistoryRequest>,
) -> impl IntoResponse {
    let db = state.context.db().await;
    let limit = query.limit.unwrap_or(50);

    match history::get_recent_history(db.pool(), &user_id, limit).await {
        Ok(records) => Json(GetHistoryResponse {
            id: query.id,
            records,
        })
        .into_response(),
        Err(e) => {
            tawai_core::utils::logger::error(&format!("get history failed: {}", e));
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(GetHistoryResponse {
                    id: query.id,
                    records: vec![],
                }),
            )
                .into_response()
        }
    }
}
