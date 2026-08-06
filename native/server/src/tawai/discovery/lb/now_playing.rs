use axum::{
    Json,
    extract::{Extension, State},
    response::IntoResponse,
};
use tawai_core::db::{account, history, library};
use tawai_core::discovery::listenbrainz;
use tawai_core::signals::playback::{UpdateNowPlayingRequest, UpdateNowPlayingResponse};

use crate::server::SharedState;

#[utoipa::path(
    post,
    path = "/api/tawai/discovery/lb/now-playing",
    tags = ["tawai.discovery"],
    security(("ApiKeyAuth" = [])),
    request_body = UpdateNowPlayingRequest,
    responses(
        (status = 200, description = "Now playing updated", body = UpdateNowPlayingResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Track not found")
    )
)]
pub async fn handle_update_now_playing(
    State(state): State<SharedState>,
    Extension(user_id): Extension<String>,
    Json(payload): Json<UpdateNowPlayingRequest>,
) -> impl IntoResponse {
    let db = state.context.db().await;
    let mk = state.context.master_key.read().await.clone();
    let user = match account::get_user_by_id(db.pool(), &user_id, &mk).await {
        Ok(Some(u)) => u,
        _ => {
            return (
                axum::http::StatusCode::UNAUTHORIZED,
                Json(UpdateNowPlayingResponse {
                    id: payload.id,
                    success: false,
                }),
            )
                .into_response();
        }
    };

    match library::lookup_track(db.pool(), &payload.track_id).await {
        Ok(Some(track)) => {
            let token = history::get_listenbrainz_token(db.pool(), &user.id, &mk).await;
            if let Some(lb_token) = token {
                let client = state.context.client().clone();
                tokio::spawn(async move {
                    if let Err(e) =
                        listenbrainz::update_now_playing(&client, &lb_token, &track).await
                    {
                        tawai_core::utils::logger::error(&format!(
                            "update now playing failed: {}",
                            e
                        ));
                    }
                });
            }
            Json(UpdateNowPlayingResponse {
                id: payload.id,
                success: true,
            })
            .into_response()
        }
        _ => (
            axum::http::StatusCode::NOT_FOUND,
            Json(UpdateNowPlayingResponse {
                id: payload.id,
                success: false,
            }),
        )
            .into_response(),
    }
}
