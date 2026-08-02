use axum::{
    Json,
    extract::{Extension, State},
    response::IntoResponse,
};
use serde::Deserialize;
use tawai_core::db::{account, history, library};
use tawai_core::discovery::listenbrainz;
use utoipa::ToSchema;

use crate::server::SharedState;

#[derive(Deserialize, ToSchema)]
pub struct UpdateNowPlayingPayload {
    pub track_id: String,
}

#[utoipa::path(
    post,
    path = "/api/tawai/discovery/lb/now-playing",
    tags = ["tawai.discovery"],
    security(("ApiKeyAuth" = [])),
    request_body = UpdateNowPlayingPayload,
    responses(
        (status = 200, description = "Now playing updated"),
        (status = 500, description = "Server error")
    )
)]
pub async fn handle_update_now_playing(
    State(state): State<SharedState>,
    Extension(username): Extension<String>,
    Json(payload): Json<UpdateNowPlayingPayload>,
) -> impl IntoResponse {
    let db = state.context.db().await;
    let mk = state.context.master_key.read().await.clone();
    let user = match account::get_user_by_username(db.pool(), &username, &mk).await {
        Ok(Some(u)) => u,
        _ => return axum::http::StatusCode::UNAUTHORIZED.into_response(),
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
            axum::http::StatusCode::OK.into_response()
        }
        _ => axum::http::StatusCode::NOT_FOUND.into_response(),
    }
}
