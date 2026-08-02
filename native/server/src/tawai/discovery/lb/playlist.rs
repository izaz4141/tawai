use axum::{
    Json,
    extract::{Extension, Query, State},
    response::IntoResponse,
};
use serde::Deserialize;
use serde_json::json;
use tawai_core::discovery::listenbrainz;
use tawai_core::signals::discovery::DiscoveryRecording;
use tawai_core::tools::duplicates;
use utoipa::ToSchema;

use crate::server::SharedState;

use super::util;

#[derive(Deserialize, ToSchema)]
pub struct PlaylistQuery {
    #[serde(rename = "type")]
    pub playlist_type: String,
    #[serde(default)]
    pub index: u32,
}

#[utoipa::path(
    get,
    path = "/api/tawai/discovery/lb/playlist",
    tags = ["tawai.discovery"],
    security(("ApiKeyAuth" = [])),
    params(
        ("type" = String, Query, description = "Playlist type: weekly, daily, weekly-exploration, or year"),
        ("index" = usize, Query, description = "0 = most recent, 1 = previous, etc."),
    ),
    responses(
        (status = 200, description = "Playlist tracks", body = Vec<DiscoveryRecording>),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Server error")
    )
)]
pub async fn handle_get_playlist(
    State(state): State<SharedState>,
    Extension(username): Extension<String>,
    Query(query): Query<PlaylistQuery>,
) -> impl IntoResponse {
    let (token, user_name) = match util::resolve_lb_user(&state, &username).await {
        Ok(ok) => ok,
        Err(err) => return err,
    };

    let filter = match query.playlist_type.as_str() {
        "year" => "discoveries",
        t => t,
    };

    let result = listenbrainz::fetch_createdfor(
        state.context.client(),
        &token,
        &user_name,
        filter,
        query.index,
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
                Json(json!({
                    "recordings": disc,
                    "playlist_title": playlist_title,
                    "playlist_id": playlist_id,
                    "playlist_count": playlist_count,
                })),
            )
                .into_response()
        }
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}
