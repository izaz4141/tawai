use axum::{Json, extract::State, response::IntoResponse};
use serde::Serialize;
use tawai_core::db::library;
use tawai_core::signals::library::ArtistInfo;
use utoipa::ToSchema;

use crate::server::SharedState;

#[derive(Serialize, ToSchema)]
pub struct ArtistsResponse {
    pub artists: Vec<ArtistInfo>,
}

pub async fn handle_list_artists(State(state): State<SharedState>) -> impl IntoResponse {
    let db = state.context.db().await;
    let artists = library::list_artists(db.pool()).await.unwrap_or_default();
    Json(ArtistsResponse { artists }).into_response()
}
