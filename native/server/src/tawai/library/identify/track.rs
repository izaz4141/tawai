use std::path::Path;

use axum::{
    Json,
    extract::{Extension, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use tawai_core::{
    db::{account, library},
    signals::identify::{
        ApplyIdentificationRequest, ApplyIdentificationResponse, ListUnidentifiedTracksRequest,
        ListUnidentifiedTracksResponse,
    },
    signals::library::ListDownloadFolderTracksResponse,
};

use crate::server::SharedState;

#[utoipa::path(
    get,
    path = "/api/tawai/library/identify/download-folder",
    tags = ["tawai.library"],
    security(("ApiKeyAuth" = [])),
    responses(
        (status = 200, description = "Tracks read from the download folder", body = ListDownloadFolderTracksResponse),
    )
)]
pub async fn handle_list_download_folder_tracks(
    State(state): State<SharedState>,
) -> impl IntoResponse {
    let cfg = state.context.cfg().await;
    let folder = cfg
        .value
        .get("download_folder")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let tracks = tawai_core::utils::identify::list_download_folder_tracks(Path::new(&folder))
        .unwrap_or_default();
    Json(ListDownloadFolderTracksResponse {
        id: String::new(),
        tracks,
    })
    .into_response()
}

#[utoipa::path(
    get,
    path = "/api/tawai/library/identify/unidentified",
    tags = ["tawai.library"],
    security(("ApiKeyAuth" = [])),
    params(
        ("source_id" = Option<String>, Query, description = "Filter by library source ID"),
    ),
    responses(
        (status = 200, description = "List of tracks without MBID or album MBID", body = ListUnidentifiedTracksResponse),
    )
)]
pub async fn handle_list_unidentified(
    State(state): State<SharedState>,
    Query(query): Query<ListUnidentifiedTracksRequest>,
) -> impl IntoResponse {
    let db = state.context.db().await;
    match library::list_unidentified_tracks(db.pool(), query.source_id.as_deref()).await {
        Ok(tracks) => Json(ListUnidentifiedTracksResponse {
            id: String::new(),
            tracks,
        })
        .into_response(),
        Err(e) => {
            tawai_core::utils::logger::error(&format!("list_unidentified failed: {e}"));
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ListUnidentifiedTracksResponse {
                    id: String::new(),
                    tracks: vec![],
                }),
            )
                .into_response()
        }
    }
}

#[utoipa::path(
    post,
    path = "/api/tawai/library/identify/apply",
    tags = ["tawai.library"],
    security(("ApiKeyAuth" = [])),
    request_body = ApplyIdentificationRequest,
    responses(
        (status = 200, description = "Identification applied", body = ApplyIdentificationResponse),
        (status = 400, description = "Error applying identification", body = ApplyIdentificationResponse),
    )
)]
pub async fn handle_apply_identification(
    State(state): State<SharedState>,
    Extension(user_id): Extension<String>,
    Json(body): Json<ApplyIdentificationRequest>,
) -> impl IntoResponse {
    let db = state.context.db().await;
    let mk = state.context.master_key.read().await.clone();

    let user = match account::get_user_by_id(db.pool(), &user_id, &mk).await {
        Ok(u) => u,
        Err(_) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(ApplyIdentificationResponse {
                    id: body.id,
                    track_id: body.track_id.clone(),
                    success: false,
                    error: Some("User not found".to_string()),
                    new_file_path: None,
                }),
            )
                .into_response();
        }
    };

    let params = tawai_core::utils::identify::ApplyIdentificationParams {
        track_id: body.track_id.clone(),
        file_path: body.file_path.clone(),
        target_source_id: body.target_source_id.clone(),
        title: body.title.clone(),
        artist: body.artist.clone(),
        artist_mbid: body.artist_mbid.clone(),
        album: body.album.clone(),
        album_mbid: body.album_mbid.clone(),
        album_disambiguation: body.album_disambiguation.clone(),
        release_date: body.release_date.clone(),
        track_num: body.track_num,
        disc_num: body.disc_num,
        mbid_recording: body.mbid_recording.clone(),
        lyrics: body.lyrics.clone(),
        cover_bytes: body.cover_bytes.clone(),
        total_discs: body.total_discs,
    };

    match tawai_core::utils::identify::apply_identification(
        db.pool(),
        &user.id,
        &user.role,
        &params,
    )
    .await
    {
        Ok(outcome) => Json(ApplyIdentificationResponse {
            id: body.id,
            track_id: body.track_id,
            success: true,
            error: None,
            new_file_path: outcome.new_file_path,
        })
        .into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(ApplyIdentificationResponse {
                id: body.id,
                track_id: body.track_id,
                success: false,
                error: Some(e.to_string()),
                new_file_path: None,
            }),
        )
            .into_response(),
    }
}
