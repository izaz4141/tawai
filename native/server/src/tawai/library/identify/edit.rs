use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use tawai_core::{
    audio,
    signals::metadata::{
        ReadFileTagsRequest, ReadFileTagsResponse, WriteFileTagsRequest, WriteFileTagsResponse,
    },
};

use crate::server::SharedState;

#[utoipa::path(
    post,
    path = "/api/tawai/library/identify/tags/read",
    tags = ["tawai.library"],
    security(("ApiKeyAuth" = [])),
    request_body = ReadFileTagsRequest,
    responses(
        (status = 200, description = "File tags read", body = ReadFileTagsResponse),
        (status = 500, description = "Error reading tags")
    )
)]
pub async fn handle_read_file_tags(
    _state: State<SharedState>,
    Json(body): Json<ReadFileTagsRequest>,
) -> impl IntoResponse {
    let path = std::path::PathBuf::from(&body.path);

    match audio::tags::read_audio_tags(&path) {
        Ok((tag, duration_secs, _sample_rate, _bitrate)) => Json(ReadFileTagsResponse {
            id: body.id,
            title: tag.title,
            artist: tag.artist,
            album: tag.album,
            album_artist: tag.album_artist,
            genres: tag.genres,
            track_number: tag.track_number,
            disc_number: tag.disc_number,
            release_date: tag.release_date,
            lyrics: tag.lyrics,
            cover: tag.cover,
            duration_secs,
            error: None,
        })
        .into_response(),
        Err(e) => {
            tawai_core::utils::logger::error(&format!("read_audio_tags failed: {e}"));
            Json(ReadFileTagsResponse {
                id: body.id,
                title: String::new(),
                artist: String::new(),
                album: String::new(),
                album_artist: String::new(),
                genres: vec![],
                track_number: 0,
                disc_number: 0,
                release_date: None,
                lyrics: None,
                cover: None,
                duration_secs: 0.0,
                error: Some(e.to_string()),
            })
            .into_response()
        }
    }
}

#[utoipa::path(
    post,
    path = "/api/tawai/library/identify/tags/write",
    tags = ["tawai.library"],
    security(("ApiKeyAuth" = [])),
    request_body = WriteFileTagsRequest,
    responses(
        (status = 200, description = "File tags written", body = WriteFileTagsResponse),
        (status = 400, description = "Error writing tags")
    )
)]
pub async fn handle_write_file_tags(
    _state: State<SharedState>,
    Json(body): Json<WriteFileTagsRequest>,
) -> impl IntoResponse {
    let path = std::path::PathBuf::from(&body.path);

    let tag = audio::tags::AudioTag {
        title: body.title,
        artist: body.artist,
        album: body.album,
        album_artist: body.album_artist,
        genres: body.genres,
        track_number: body.track_number,
        disc_number: body.disc_number,
        release_date: body.release_date,
        lyrics: body.lyrics,
        cover: body.cover,
        ..Default::default()
    };

    match audio::tags::write_audio_tags(&path, &tag) {
        Ok(()) => Json(WriteFileTagsResponse {
            id: body.id,
            success: true,
            error: None,
        })
        .into_response(),
        Err(e) => {
            tawai_core::utils::logger::error(&format!("write_audio_tags failed: {e}"));
            (
                StatusCode::BAD_REQUEST,
                Json(WriteFileTagsResponse {
                    id: body.id,
                    success: false,
                    error: Some(e.to_string()),
                }),
            )
                .into_response()
        }
    }
}
