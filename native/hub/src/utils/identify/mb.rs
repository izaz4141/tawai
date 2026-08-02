use std::sync::Arc;

use rinf::{DartSignal, RustSignal};

use crate::signals;
use crate::utils::logger;
use tawai_core::app_context::AppContext;
use tawai_core::audio;
use tawai_core::db::library;
use tawai_core::metadata::musicbrainz;

pub async fn handle_search_musicbrainz(context: Arc<AppContext>) {
    use signals::metadata::*;
    let receiver = EnhancedSearchRequest::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;
        let resp = match musicbrainz::lookup_by_query(context.client(), &msg.query).await {
            Ok(r) => r,
            Err(e) => {
                logger::error(&format!("enhanced search failed: {}", e));
                EnhancedSearchResponse {
                    id: msg.id,
                    recordings: vec![],
                }
                .send_signal_to_dart();
                continue;
            }
        };
        EnhancedSearchResponse {
            id: msg.id,
            recordings: resp.recordings.into_iter().map(Into::into).collect(),
        }
        .send_signal_to_dart();
    }
}

pub async fn handle_get_release_tracks(context: Arc<AppContext>) {
    use signals::metadata::*;
    let receiver = GetReleaseTracksRequest::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;
        match musicbrainz::fetch_release(context.client(), &msg.release_id).await {
            Ok(release) => {
                let resp = GetReleaseTracksResponse {
                    id: msg.id,
                    release_id: release.id,
                    release_title: release.title,
                    release_date: release.date,
                    artist: release.artist,
                    artist_id: release.artist_id,
                    disambiguation: release.disambiguation,
                    tracks: release.tracks.into_iter().map(Into::into).collect(),
                };
                resp.send_signal_to_dart();
            }
            Err(e) => {
                logger::error(&format!("fetch_release failed: {}", e));
                GetReleaseTracksResponse {
                    id: msg.id,
                    release_id: String::new(),
                    release_title: String::new(),
                    release_date: None,
                    artist: String::new(),
                    artist_id: None,
                    disambiguation: None,
                    tracks: vec![],
                }
                .send_signal_to_dart();
            }
        }
    }
}

pub async fn handle_fetch_recording(context: Arc<AppContext>) {
    use signals::metadata::*;
    let receiver = FetchRecordingRequest::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;
        match musicbrainz::fetch_recording(context.client(), &msg.mbid).await {
            Ok(rec) => {
                FetchRecordingResponse {
                    id: msg.id,
                    recording: Some(rec.into()),
                    error: None,
                }
                .send_signal_to_dart();
            }
            Err(e) => {
                logger::error(&format!("fetch_recording failed: {}", e));
                FetchRecordingResponse {
                    id: msg.id,
                    recording: None,
                    error: Some(e.to_string()),
                }
                .send_signal_to_dart();
            }
        }
    }
}

pub async fn handle_identify_single_track(context: Arc<AppContext>) {
    use signals::metadata::*;
    let receiver = IdentifySingleTrackRequest::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;
        let db = context.db().await;
        let mut candidates: Vec<signals::metadata::MatchCandidate> = Vec::new();

        let (file_path, track_title, _track_artist) =
            match library::lookup_track(db.pool(), &msg.track_id).await {
                Ok(Some(t)) => (t.file_path, t.title.clone(), t.artists_string.clone()),
                _ => {
                    IdentifySingleTrackResponse {
                        id: msg.id,
                        track_id: msg.track_id,
                        candidates: vec![],
                    }
                    .send_signal_to_dart();
                    continue;
                }
            };

        if let Ok(Some((fingerprint, duration))) =
            library::lookup_fingerprint_by_id(db.pool(), &msg.track_id).await
        {
            if let Ok(api_key) = std::env::var("TAWAI_ACOUSTID_API_KEY") {
                match musicbrainz::lookup_by_fingerprint(
                    context.client(),
                    &fingerprint,
                    duration,
                    &api_key,
                )
                .await
                {
                    Ok(info) => {
                        if !info.title.is_empty() {
                            let first_release = info.releases.first();
                            let score = if info.acoust_id.is_some() { 0.95 } else { 0.5 };
                            candidates.push(signals::metadata::MatchCandidate {
                                score,
                                title: info.title.clone(),
                                artist: info.artist.clone(),
                                artist_id: info.artist_id.clone(),
                                album: first_release.map(|r| r.title.clone()).unwrap_or_default(),
                                album_id: first_release.map(|r| r.id.clone()),
                                recording_id: Some(info.id.clone()),
                                release_date: first_release.and_then(|r| r.date.clone()),
                                acoust_id: info.acoust_id.clone(),
                                duration_secs: info.duration_secs,
                            });
                        }
                    }
                    Err(e) => {
                        logger::debug(&format!("AcoustID lookup failed: {e}"));
                    }
                }
            }
        }

        let path = std::path::Path::new(&file_path);
        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
            let (parsed_artist, parsed_title, _parsed_track) =
                audio::tags::parse_filename_tags(stem);
            if let Some(title) = parsed_title {
                if title != track_title || candidates.is_empty() {
                    let score = if parsed_artist.is_some() { 0.3 } else { 0.2 };
                    candidates.push(signals::metadata::MatchCandidate {
                        score,
                        title,
                        artist: parsed_artist.unwrap_or_default(),
                        artist_id: None,
                        album: String::new(),
                        album_id: None,
                        recording_id: None,
                        release_date: None,
                        acoust_id: None,
                        duration_secs: None,
                    });
                }
            }
        }

        candidates.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        IdentifySingleTrackResponse {
            id: msg.id,
            track_id: msg.track_id,
            candidates,
        }
        .send_signal_to_dart();
    }
}

pub async fn handle_fingerprint_track(context: Arc<AppContext>) {
    use signals::metadata::*;
    let receiver = FingerprintTrackRequest::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;
        let db = context.db().await;

        let (fingerprint, duration) =
            match library::lookup_fingerprint_by_id(db.pool(), &msg.track_id).await {
                Ok(Some(v)) => v,
                _ => {
                    FingerprintTrackResponse {
                        id: msg.id,
                        track_id: msg.track_id,
                        recording: None,
                    }
                    .send_signal_to_dart();
                    continue;
                }
            };

        let api_key = match std::env::var("TAWAI_ACOUSTID_API_KEY") {
            Ok(k) => k,
            _ => {
                FingerprintTrackResponse {
                    id: msg.id,
                    track_id: msg.track_id,
                    recording: None,
                }
                .send_signal_to_dart();
                continue;
            }
        };

        let recording = match musicbrainz::lookup_by_fingerprint(
            context.client(),
            &fingerprint,
            duration,
            &api_key,
        )
        .await
        {
            Ok(info) => Some(info.into()),
            Err(e) => {
                logger::debug(&format!("AcoustID lookup failed: {e}"));
                None
            }
        };

        FingerprintTrackResponse {
            id: msg.id,
            track_id: msg.track_id,
            recording,
        }
        .send_signal_to_dart();
    }
}
