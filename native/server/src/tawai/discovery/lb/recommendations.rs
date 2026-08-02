use axum::{
    Json,
    extract::{Extension, Query, State},
    response::IntoResponse,
};
use serde::Deserialize;
use tawai_core::discovery::listenbrainz;
use tawai_core::signals::discovery::DiscoveryRecording;
use tawai_core::tools::duplicates;
use utoipa::ToSchema;

use crate::server::SharedState;

use super::util;

#[derive(Deserialize, ToSchema)]
pub struct RecommendationsQuery {
    #[serde(rename = "type")]
    pub rec_type: String,
    pub count: Option<i32>,
    pub offset: Option<i32>,
}

#[utoipa::path(
    get,
    path = "/api/tawai/discovery/lb/recommendations",
    tags = ["tawai.discovery"],
    security(("ApiKeyAuth" = [])),
    params(
        ("type" = String, Query, description = "Recommendation type: top, raw, or similar"),
        ("count" = Option<i32>, Query, description = "Number of recommendations"),
        ("offset" = Option<i32>, Query, description = "Offset for pagination"),
    ),
    responses(
        (status = 200, description = "Recommendations", body = Vec<DiscoveryRecording>),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Server error")
    )
)]
pub async fn handle_get_recommendations(
    State(state): State<SharedState>,
    Extension(username): Extension<String>,
    Query(query): Query<RecommendationsQuery>,
) -> impl IntoResponse {
    let (token, user_name) = match util::resolve_lb_user(&state, &username).await {
        Ok(ok) => ok,
        Err(err) => return err,
    };

    let result =
        match query.rec_type.as_str() {
            "top" | "raw" | "similar" => {
                listenbrainz::fetch_recommendations(
                    state.context.client(),
                    &token,
                    &user_name,
                    &query.rec_type,
                    query.count,
                    query.offset,
                )
                .await
            }
            other => return (
                axum::http::StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": format!("unknown recommendation type: {other}")})),
            )
                .into_response(),
        };

    match result {
        Ok(recs) => {
            let mut disc: Vec<DiscoveryRecording> = recs.into_iter().map(Into::into).collect();
            let db = state.context.db().await;
            let pool = db.pool();
            for rec in &mut disc {
                rec.is_owned =
                    duplicates::is_recording_owned(pool, Some(&rec.id), &rec.title, &rec.artist)
                        .await
                        .unwrap_or(false);
            }
            (axum::http::StatusCode::OK, Json(disc)).into_response()
        }
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}
