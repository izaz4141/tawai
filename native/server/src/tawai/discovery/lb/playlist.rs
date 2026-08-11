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
    path = "/api/tawai/discovery/lb/playlist",
    tags = ["tawai.discovery"],
    security(("ApiKeyAuth" = [])),
    params(
        ("rec_type" = String, Query, description = "Playlist type: weekly, daily, weekly-exploration, or year"),
        ("index" = Option<u32>, Query, description = "0 = most recent, 1 = previous, etc."),
    ),
    responses(
        (status = 200, description = "Playlist tracks", body = GetLBRecommendationsResponse),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Server error")
    )
)]
pub async fn handle_get_playlist(
    State(state): State<SharedState>,
    Extension(user_id): Extension<String>,
    Query(query): Query<GetLBRecommendationsRequest>,
) -> impl IntoResponse {
    let (token, user_name) = match util::resolve_lb_user(&state, &user_id).await {
        Ok(ok) => ok,
        Err(err) => return err,
    };

    let filter = match query.rec_type.as_str() {
        "year" => "discoveries",
        t => t,
    };

    let result = listenbrainz::fetch_createdfor(
        state.context.client(),
        &token,
        &user_name,
        filter,
        query.index.unwrap_or(0),
    )
    .await;

    match result {
        Ok(cr) => {
            let playlist_title = cr.playlist_title.clone();
            let playlist_id = cr.playlist_id.clone();
            let playlist_count = cr.playlist_count;
            let mut disc: Vec<DiscoveryRecording> =
                cr.recordings.into_iter().map(Into::into).collect();
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
                    playlist_title: Some(playlist_title),
                    playlist_id: Some(playlist_id),
                    playlist_count: Some(playlist_count),
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
