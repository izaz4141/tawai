use axum::{
    Json,
    extract::{Extension, State},
    response::IntoResponse,
};
use tawai_core::db::{account, database::DatabasePool, history, library};
use tawai_core::signals::playback::{ReportPlaybackRequest, ReportPlaybackResponse};

use crate::server::SharedState;

#[utoipa::path(
    post,
    path = "/api/tawai/discovery/lb/report",
    tags = ["tawai.discovery"],
    security(("ApiKeyAuth" = [])),
    request_body = ReportPlaybackRequest,
    responses(
        (status = 200, description = "Playback recorded", body = ReportPlaybackResponse),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Server error")
    )
)]
pub async fn handle_report_playback(
    State(state): State<SharedState>,
    Extension(user_id): Extension<String>,
    Json(payload): Json<ReportPlaybackRequest>,
) -> impl IntoResponse {
    let db = state.context.db().await;
    let mk = state.context.master_key.read().await.clone();
    let user = match account::get_user_by_id(db.pool(), &user_id, &mk).await {
        Ok(Some(u)) => u,
        _ => {
            return (
                axum::http::StatusCode::UNAUTHORIZED,
                Json(ReportPlaybackResponse {
                    id: payload.id,
                    success: false,
                }),
            )
                .into_response();
        }
    };

    match history::record_playback(db.pool(), &user.id, &payload.track_id, &payload.source).await {
        Ok(hid) => {
            if payload.source != "jellyfin" {
                let mk = state.context.master_key.read().await.clone();
                let token = history::get_listenbrainz_token(db.pool(), &user.id, &mk).await;
                if let Some(lb_token) = token {
                    if let Ok(Some(track)) =
                        library::lookup_track(db.pool(), &payload.track_id).await
                    {
                        let client = state.context.client().clone();
                        let pool = match db.pool() {
                            DatabasePool::Sqlite(p) => DatabasePool::Sqlite(p.clone()),
                            DatabasePool::Postgres(p) => DatabasePool::Postgres(p.clone()),
                        };
                        tokio::spawn(async move {
                            match tawai_core::discovery::listenbrainz::scrobble(
                                &client, &lb_token, &track,
                            )
                            .await
                            {
                                Ok(()) => {
                                    let _ = history::mark_scrobbled(&pool, &hid).await;
                                }
                                Err(e) => {
                                    tawai_core::utils::logger::error(&format!(
                                        "scrobble failed: {}",
                                        e
                                    ));
                                }
                            }
                        });
                    }
                }
            }

            Json(ReportPlaybackResponse {
                id: payload.id,
                success: true,
            })
            .into_response()
        }
        Err(e) => {
            tawai_core::utils::logger::error(&format!("record playback failed: {}", e));
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(ReportPlaybackResponse {
                    id: payload.id,
                    success: false,
                }),
            )
                .into_response()
        }
    }
}
