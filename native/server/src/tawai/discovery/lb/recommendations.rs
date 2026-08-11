use axum::{
    Json,
    extract::{Extension, Query, State},
    response::IntoResponse,
};
use tawai_core::discovery::listenbrainz;
use tawai_core::signals::discovery::{
    DiscoveryRecording, GetLBRecommendationsRequest, GetLBRecommendationsResponse,
};
use tawai_core::tools::duplicates;

use crate::server::SharedState;

use super::util;

#[utoipa::path(
    get,
    path = "/api/tawai/discovery/lb/recommendations",
    tags = ["tawai.discovery"],
    security(("ApiKeyAuth" = [])),
    params(
        ("rec_type" = String, Query, description = "Recommendation type: top, raw, or similar"),
        ("count" = Option<i32>, Query, description = "Number of recommendations"),
        ("offset" = Option<i32>, Query, description = "Offset for pagination"),
        ("index" = Option<u32>, Query, description = "Index for pagination"),
    ),
    responses(
        (status = 200, description = "Recommendations", body = GetLBRecommendationsResponse),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Server error")
    )
)]
pub async fn handle_get_recommendations(
    State(state): State<SharedState>,
    Extension(user_id): Extension<String>,
    Query(query): Query<GetLBRecommendationsRequest>,
) -> impl IntoResponse {
    let (token, user_name) = match util::resolve_lb_user(&state, &user_id).await {
        Ok(ok) => ok,
        Err(err) => return err,
    };

    let result = match query.rec_type.as_str() {
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
        other => {
            return (
                axum::http::StatusCode::BAD_REQUEST,
                Json(GetLBRecommendationsResponse {
                    id: String::new(),
                    recommendations: vec![],
                    playlist_title: None,
                    playlist_id: None,
                    playlist_count: None,
                    error: Some(format!("unknown recommendation type: {other}")),
                }),
            )
                .into_response();
        }
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
            (
                axum::http::StatusCode::OK,
                Json(GetLBRecommendationsResponse {
                    id: String::new(),
                    recommendations: disc,
                    playlist_title: None,
                    playlist_id: None,
                    playlist_count: None,
                    error: None,
                }),
            )
                .into_response()
        }
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(GetLBRecommendationsResponse {
                id: String::new(),
                recommendations: vec![],
                playlist_title: None,
                playlist_id: None,
                playlist_count: None,
                error: Some(e.to_string()),
            }),
        )
            .into_response(),
    }
}
