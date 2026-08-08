use axum::{
    Json,
    extract::{Multipart, State},
    http::StatusCode,
    response::IntoResponse,
};
use tawai_core::{
    audio,
    signals::metadata::{
        ReadFileTagsBytesRequest, ReadFileTagsRequest, ReadFileTagsResponse,
        WriteFileTagsBytesRequest, WriteFileTagsBytesResponse, WriteFileTagsRequest,
        WriteFileTagsResponse,
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

#[utoipa::path(
    post,
    path = "/api/tawai/library/identify/tags/read-bytes",
    tags = ["tawai.library"],
    security(("ApiKeyAuth" = [])),
    request_body(content = ReadFileTagsBytesRequest, content_type = "multipart/form-data"),
    responses(
        (status = 200, description = "File tags read from bytes", body = ReadFileTagsResponse),
        (status = 400, description = "Error reading tags")
    )
)]
pub async fn handle_read_file_tags_bytes(
    _state: State<SharedState>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let mut filename = String::new();
    let mut bytes: Vec<u8> = Vec::new();

    while let Some(field) = multipart.next_field().await.unwrap_or(None) {
        match field.name() {
            Some("filename") => {
                if let Ok(text) = field.text().await {
                    filename = text;
                }
            }
            Some("file") => {
                if let Ok(data) = field.bytes().await {
                    bytes = data.to_vec();
                }
            }
            _ => {}
        }
    }

    if filename.is_empty() || bytes.is_empty() {
        return Json(ReadFileTagsResponse {
            id: String::new(),
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
            error: Some("Missing filename or file".to_string()),
        })
        .into_response();
    }

    match audio::tags::read_audio_tags_from_bytes(&filename, &bytes) {
        Ok((tag, duration_secs, _sample_rate, _bitrate)) => Json(ReadFileTagsResponse {
            id: String::new(),
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
            tawai_core::utils::logger::error(&format!("read_audio_tags_from_bytes failed: {e}"));
            (
                StatusCode::BAD_REQUEST,
                Json(ReadFileTagsResponse {
                    id: String::new(),
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
                }),
            )
                .into_response()
        }
    }
}

#[utoipa::path(
    post,
    path = "/api/tawai/library/identify/tags/write-bytes",
    tags = ["tawai.library"],
    security(("ApiKeyAuth" = [])),
    request_body(content = WriteFileTagsBytesRequest, content_type = "multipart/form-data"),
    responses(
        (status = 200, description = "File tags written; returns the modified file bytes", content_type = "application/octet-stream"),
        (status = 400, description = "Error writing tags", body = WriteFileTagsBytesResponse)
    )
)]
pub async fn handle_write_file_tags_bytes(
    _state: State<SharedState>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let mut id = String::new();
    let mut filename = String::new();
    let mut bytes: Vec<u8> = Vec::new();
    let mut title = String::new();
    let mut artist = String::new();
    let mut album = String::new();
    let mut album_artist = String::new();
    let mut genres: Vec<String> = Vec::new();
    let mut track_number = 0i32;
    let mut disc_number = 0i32;
    let mut release_date: Option<String> = None;
    let mut lyrics: Option<String> = None;
    let mut cover: Option<Vec<u8>> = None;

    while let Some(field) = multipart.next_field().await.unwrap_or(None) {
        match field.name().unwrap_or("") {
            "id" => {
                if let Ok(text) = field.text().await {
                    id = text;
                }
            }
            "filename" => {
                if let Ok(text) = field.text().await {
                    filename = text;
                }
            }
            "file" => {
                if let Ok(data) = field.bytes().await {
                    bytes = data.to_vec();
                }
            }
            "title" => {
                if let Ok(text) = field.text().await {
                    title = text;
                }
            }
            "artist" => {
                if let Ok(text) = field.text().await {
                    artist = text;
                }
            }
            "album" => {
                if let Ok(text) = field.text().await {
                    album = text;
                }
            }
            "album_artist" => {
                if let Ok(text) = field.text().await {
                    album_artist = text;
                }
            }
            "genres" => {
                if let Ok(text) = field.text().await {
                    genres.push(text);
                }
            }
            "track_number" => {
                if let Ok(text) = field.text().await {
                    track_number = text.parse().unwrap_or(0);
                }
            }
            "disc_number" => {
                if let Ok(text) = field.text().await {
                    disc_number = text.parse().unwrap_or(0);
                }
            }
            "release_date" => {
                if let Ok(text) = field.text().await {
                    release_date = if text.is_empty() { None } else { Some(text) };
                }
            }
            "lyrics" => {
                if let Ok(text) = field.text().await {
                    lyrics = if text.is_empty() { None } else { Some(text) };
                }
            }
            "cover" => {
                if let Ok(data) = field.bytes().await {
                    cover = Some(data.to_vec());
                }
            }
            _ => {}
        }
    }

    if filename.is_empty() || bytes.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(WriteFileTagsBytesResponse {
                id,
                success: false,
                bytes: vec![],
                error: Some("Missing filename or file".to_string()),
            }),
        )
            .into_response();
    }

    let tag = audio::tags::AudioTag {
        title,
        artist,
        album,
        album_artist,
        genres,
        track_number,
        disc_number,
        release_date,
        lyrics,
        cover,
        ..Default::default()
    };

    match audio::tags::write_audio_tags_to_bytes(&filename, &bytes, &tag) {
        Ok(new_bytes) => (StatusCode::OK, new_bytes).into_response(),
        Err(e) => {
            tawai_core::utils::logger::error(&format!("write_audio_tags_to_bytes failed: {e}"));
            (
                StatusCode::BAD_REQUEST,
                Json(WriteFileTagsBytesResponse {
                    id,
                    success: false,
                    bytes: vec![],
                    error: Some(e.to_string()),
                }),
            )
                .into_response()
        }
    }
}
