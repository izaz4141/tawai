use std::path::Path;
use std::sync::Arc;

use rinf::{DartSignal, RustSignal};

use crate::signals;
use crate::utils::logger;
use tawai_core::app_context::AppContext;
use tawai_core::audio;
use tawai_core::db::library;

pub async fn handle_list_download_folder_tracks(context: Arc<AppContext>) {
    use signals::library::*;
    let receiver = ListDownloadFolderTracksRequest::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;
        let path = Path::new(&msg.path);
        let tracks: Vec<TrackInfo> = tawai_core::utils::identify::list_download_folder_tracks(path)
            .unwrap_or_default()
            .into_iter()
            .map(Into::into)
            .collect();
        ListDownloadFolderTracksResponse { id: msg.id, tracks }.send_signal_to_dart();
    }
}

pub async fn handle_list_unidentified_tracks(context: Arc<AppContext>) {
    use signals::metadata::*;
    let receiver = ListUnidentifiedTracksRequest::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;
        let db = context.db().await;
        match library::list_unidentified_tracks(db.pool(), msg.source_id.as_deref()).await {
            Ok(tracks) => {
                ListUnidentifiedTracksResponse {
                    id: msg.id,
                    tracks: tracks.into_iter().map(Into::into).collect(),
                }
                .send_signal_to_dart();
            }
            Err(e) => {
                logger::error(&format!("list_unidentified_tracks failed: {}", e));
                ListUnidentifiedTracksResponse {
                    id: msg.id,
                    tracks: vec![],
                }
                .send_signal_to_dart();
            }
        }
    }
}

pub async fn handle_apply_identification(context: Arc<AppContext>) {
    use signals::metadata::*;
    let receiver = ApplyIdentificationRequest::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;
        let db = context.db().await;
        let mk = context.master_key.read().await.clone();

        let user = match tawai_core::db::account::get_user_by_id(db.pool(), &msg.user_id, &mk).await
        {
            Ok(Some(u)) => u,
            _ => {
                logger::error(&format!(
                    "apply_identification: user not found: {}",
                    msg.user_id
                ));
                ApplyIdentificationResponse {
                    id: msg.id,
                    track_id: msg.track_id.clone(),
                    success: false,
                    error: Some("User not found".to_string()),
                    new_file_path: None,
                }
                .send_signal_to_dart();
                continue;
            }
        };

        let params = tawai_core::utils::identify::ApplyIdentificationParams {
            track_id: msg.track_id.clone(),
            file_path: msg.file_path.clone(),
            target_source_id: msg.target_source_id.clone(),
            title: msg.title,
            artist: msg.artist,
            artist_mbid: msg.artist_mbid,
            album: msg.album,
            album_mbid: msg.album_mbid,
            album_disambiguation: msg.album_disambiguation,
            release_date: msg.release_date,
            track_num: msg.track_num,
            disc_num: msg.disc_num,
            mbid_recording: msg.mbid_recording,
            lyrics: msg.lyrics,
            cover_bytes: msg.cover_bytes,
            total_discs: msg.total_discs,
        };

        match tawai_core::utils::identify::apply_identification(
            db.pool(),
            &user.id,
            &user.role,
            &params,
        )
        .await
        {
            Ok(outcome) => {
                ApplyIdentificationResponse {
                    id: msg.id,
                    track_id: msg.track_id,
                    success: true,
                    error: None,
                    new_file_path: outcome.new_file_path,
                }
                .send_signal_to_dart();
            }
            Err(e) => {
                logger::error(&format!("apply_identification failed: {e}"));
                ApplyIdentificationResponse {
                    id: msg.id,
                    track_id: msg.track_id,
                    success: false,
                    error: Some(e.to_string()),
                    new_file_path: None,
                }
                .send_signal_to_dart();
            }
        }
    }
}

pub async fn handle_list_tracks_by_source(context: Arc<AppContext>) {
    use signals::library::*;
    let receiver = ListTracksBySourceRequest::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;
        let db = context.db().await;
        match library::list_tracks_by_source(db.pool(), &msg.source_id).await {
            Ok(tracks) => {
                ListTracksBySourceResponse {
                    id: msg.id,
                    tracks: tracks.into_iter().map(Into::into).collect(),
                }
                .send_signal_to_dart();
            }
            Err(e) => {
                logger::error(&format!("list_tracks_by_source failed: {e}"));
                ListTracksBySourceResponse {
                    id: msg.id,
                    tracks: vec![],
                }
                .send_signal_to_dart();
            }
        }
    }
}

pub async fn handle_get_album_mbid(context: Arc<AppContext>) {
    use signals::library::*;
    let receiver = GetAlbumMbidRequest::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;
        let db = context.db().await;
        match library::get_album_mbid(db.pool(), &msg.album_id).await {
            Ok(mbid) => {
                GetAlbumMbidResponse { id: msg.id, mbid }.send_signal_to_dart();
            }
            Err(e) => {
                logger::error(&format!("get_album_mbid failed: {e}"));
                GetAlbumMbidResponse {
                    id: msg.id,
                    mbid: None,
                }
                .send_signal_to_dart();
            }
        }
    }
}

pub async fn handle_read_file_tags(_context: Arc<AppContext>) {
    use signals::metadata::*;
    let receiver = ReadFileTagsRequest::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;
        let path = std::path::PathBuf::from(&msg.path);

        match audio::tags::read_audio_tags(&path) {
            Ok((tag, duration_secs, _sample_rate, _bitrate)) => {
                ReadFileTagsResponse {
                    id: msg.id,
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
                }
                .send_signal_to_dart();
            }
            Err(e) => {
                logger::error(&format!("read_audio_tags failed: {e}"));
                ReadFileTagsResponse {
                    id: msg.id,
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
                }
                .send_signal_to_dart();
            }
        }
    }
}

pub async fn handle_write_file_tags(_context: Arc<AppContext>) {
    use signals::metadata::*;
    let receiver = WriteFileTagsRequest::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;
        let path = std::path::PathBuf::from(&msg.path);

        let tag = audio::tags::AudioTag {
            title: msg.title,
            artist: msg.artist,
            album: msg.album,
            album_artist: msg.album_artist,
            genres: msg.genres,
            track_number: msg.track_number,
            disc_number: msg.disc_number,
            release_date: msg.release_date,
            lyrics: msg.lyrics,
            cover: msg.cover,
            ..Default::default()
        };

        match audio::tags::write_audio_tags(&path, &tag) {
            Ok(()) => {
                WriteFileTagsResponse {
                    id: msg.id,
                    success: true,
                    error: None,
                }
                .send_signal_to_dart();
            }
            Err(e) => {
                logger::error(&format!("write_audio_tags failed: {e}"));
                WriteFileTagsResponse {
                    id: msg.id,
                    success: false,
                    error: Some(e.to_string()),
                }
                .send_signal_to_dart();
            }
        }
    }
}
