use std::sync::Arc;

use rinf::{DartSignal, RustSignal};

use crate::signals;
use crate::utils::logger;
use tawai_core::app_context::AppContext;
use tawai_core::audio;
use tawai_core::db::library;
use tawai_core::tools;

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

        let track = match library::lookup_track(db.pool(), &msg.track_id).await {
            Ok(Some(t)) => t,
            Ok(None) => {
                logger::error(&format!("Track not found: {}", msg.track_id));
                ApplyIdentificationResponse {
                    id: msg.id,
                    track_id: msg.track_id,
                    success: false,
                    error: Some("Track not found".to_string()),
                }
                .send_signal_to_dart();
                continue;
            }
            Err(e) => {
                logger::error(&format!("lookup_track failed: {e}"));
                ApplyIdentificationResponse {
                    id: msg.id,
                    track_id: msg.track_id,
                    success: false,
                    error: Some(e.to_string()),
                }
                .send_signal_to_dart();
                continue;
            }
        };

        let file_path = std::path::PathBuf::from(&track.file_path);

        let title = if msg.title.is_empty() && !track.title.is_empty() {
            track.title.clone()
        } else {
            msg.title.clone()
        };
        let artist = if msg.artist.is_empty() && !track.artists_string.is_empty() {
            track.artists_string.clone()
        } else {
            msg.artist.clone()
        };
        let album = if msg.album.is_empty() && !track.album_title.is_empty() {
            track.album_title.clone()
        } else {
            msg.album.clone()
        };
        let track_num = msg.track_num.or(track.track_num);
        let disc_num = msg.disc_num.or(track.disc_num);
        let mbid_recording = msg.mbid_recording.or(track.mbid_recording);
        let lyrics = msg.lyrics.or_else(|| track.lyrics.clone());

        let write_tag = audio::tags::AudioTag {
            title: title.clone(),
            artist: artist.clone(),
            album: album.clone(),
            album_artist: artist.clone(),
            album_disambiguation: msg.album_disambiguation.clone(),
            release_date: msg.release_date.clone(),
            track_number: track_num.unwrap_or(0),
            disc_number: disc_num.unwrap_or(1),
            mbid_recording: mbid_recording.clone(),
            mbid_artist: msg.artist_mbid.clone(),
            mbid_release: msg.album_mbid.clone(),
            mbid_release_artist: msg.artist_mbid.clone(),
            lyrics: lyrics.clone(),
            cover: msg.cover_bytes.clone(),
            ..Default::default()
        };

        if let Err(e) = audio::tags::write_audio_tags(&file_path, &write_tag) {
            logger::error(&format!("write_audio_tags failed: {e}"));
            ApplyIdentificationResponse {
                id: msg.id,
                track_id: msg.track_id,
                success: false,
                error: Some(e.to_string()),
            }
            .send_signal_to_dart();
            continue;
        }

        if let Some(cover_bytes) = &msg.cover_bytes {
            if let Err(e) =
                library::update_album_cover(db.pool(), &track.album_id, cover_bytes).await
            {
                logger::error(&format!("update_album_cover failed (non-fatal): {e}"));
            }
        }

        let new_file_path = match {
            let db2 = context.db().await;
            let pattern = tawai_core::db::user_settings::get_setting(
                db2.pool(),
                &tawai_core::db::account::DEFAULT_USERNAME,
                "identify_naming_pattern",
            )
            .await
            .filter(|s| !s.is_empty());

            if let Some(pattern) = pattern {
                tools::rename::rename_audio_file(&file_path, &pattern, &write_tag)
                    .map(|p| Some(p.to_string_lossy().to_string()))
            } else {
                Ok(None)
            }
        } {
            Ok(path) => path,
            Err(e) => {
                logger::error(&format!("rename_audio_file failed: {e}"));
                ApplyIdentificationResponse {
                    id: msg.id,
                    track_id: msg.track_id,
                    success: false,
                    error: Some(e.to_string()),
                }
                .send_signal_to_dart();
                continue;
            }
        };

        let artist_pairs = [(artist.clone(), msg.artist_mbid.clone())];
        let album_artist_pairs = [(artist.clone(), msg.artist_mbid.clone())];

        if let Err(e) = library::update_track(
            db.pool(),
            &msg.track_id,
            &title,
            &artist_pairs,
            &album,
            &album_artist_pairs,
            msg.album_mbid.as_deref(),
            msg.release_date,
            track_num,
            disc_num,
            mbid_recording.as_deref(),
            lyrics.as_deref(),
            msg.cover_bytes.as_deref(),
            new_file_path.as_deref(),
            msg.album_disambiguation.clone(),
            msg.total_discs,
        )
        .await
        {
            logger::error(&format!("update_track failed: {e}"));
            ApplyIdentificationResponse {
                id: msg.id,
                track_id: msg.track_id,
                success: false,
                error: Some(e.to_string()),
            }
            .send_signal_to_dart();
            continue;
        }

        ApplyIdentificationResponse {
            id: msg.id,
            track_id: msg.track_id,
            success: true,
            error: None,
        }
        .send_signal_to_dart();
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
