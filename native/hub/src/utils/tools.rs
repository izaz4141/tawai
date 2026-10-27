use std::path::Path;
use std::sync::Arc;

use rinf::{DartSignal, RustSignal};

use crate::signals;
use crate::utils::logger;
use tawai_core::app_context::AppContext;
use tawai_core::db::library;
use tawai_core::tools;

pub async fn handle_get_library_stats(context: Arc<AppContext>) {
    use signals::tools::*;
    let receiver = GetLibraryStatsRequest::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;
        let db = context.db().await;

        // The client pattern is an explicit override; without one the global
        // pattern from the live config is used.
        let pattern = tools::rename::resolve_naming_pattern_from_context(
            &Arc::downgrade(&context),
            msg.naming_pattern.as_deref(),
        )
        .await;

        match tools::stats::get_library_stats(db.pool(), &pattern).await {
            Ok(stats) => {
                GetLibraryStatsResponse {
                    id: msg.id,
                    stats: Some(stats.into()),
                    error: None,
                }
                .send_signal_to_dart();
            }
            Err(e) => {
                logger::error(&format!("get_library_stats failed: {}", e));
                GetLibraryStatsResponse {
                    id: msg.id,
                    stats: None,
                    error: Some(e.to_string()),
                }
                .send_signal_to_dart();
            }
        }
    }
}

pub async fn handle_find_missing_metadata(context: Arc<AppContext>) {
    use signals::tools::*;
    let receiver = FindMissingMetadataRequest::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;
        let db = context.db().await;

        let check = tawai_core::signals::tools::MissingMetadataCheck {
            check_title: msg.check_title,
            check_artist: msg.check_artist,
            check_album: msg.check_album,
            check_genre: msg.check_genre,
            check_year: msg.check_year,
            check_track_number: msg.check_track_number,
            check_cover: msg.check_cover,
        };

        match tools::missing::find_missing_metadata(db.pool(), &check).await {
            Ok(tracks) => {
                FindMissingMetadataResponse {
                    id: msg.id,
                    tracks: tracks.into_iter().map(Into::into).collect(),
                    error: None,
                }
                .send_signal_to_dart();
            }
            Err(e) => {
                logger::error(&format!("find_missing_metadata failed: {}", e));
                FindMissingMetadataResponse {
                    id: msg.id,
                    tracks: vec![],
                    error: Some(e.to_string()),
                }
                .send_signal_to_dart();
            }
        }
    }
}

pub async fn handle_batch_rename_preview(context: Arc<AppContext>) {
    use signals::tools::*;
    let receiver = BatchRenamePreviewRequest::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;
        let db = context.db().await;
        // The client pattern is an explicit override; without one the global
        // pattern from the live config is used. This is a dry-run route, so an
        // override never mutates anything.
        let pattern = tools::rename::resolve_naming_pattern_from_context(
            &Arc::downgrade(&context),
            msg.pattern.as_deref(),
        )
        .await;
        let result = tools::rename::batch_rename_preview(
            db.pool(),
            msg.source_id.as_deref(),
            &pattern,
            &msg.user_id,
        )
        .await;
        match result {
            Ok(previews) => {
                BatchRenamePreviewResponse {
                    id: msg.id,
                    previews: previews.into_iter().map(Into::into).collect(),
                    pattern,
                    error: None,
                }
                .send_signal_to_dart();
            }
            Err(e) => {
                logger::error(&format!("batch_rename_preview failed: {}", e));
                BatchRenamePreviewResponse {
                    id: msg.id,
                    previews: vec![],
                    pattern,
                    error: Some(e.to_string()),
                }
                .send_signal_to_dart();
            }
        }
    }
}

pub async fn handle_batch_rename_apply(context: Arc<AppContext>) {
    use signals::tools::*;
    let receiver = BatchRenameApplyRequest::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;
        let db = context.db().await;
        // The apply always renames with the global naming pattern: a
        // client-supplied pattern is only an override for the preview, and the
        // request carries no pattern field at all — a client snapshot can
        // never drive a mutation.
        let pattern =
            tools::rename::resolve_naming_pattern_from_context(&Arc::downgrade(&context), None)
                .await;
        match tools::rename::batch_rename_apply(
            db.pool(),
            &msg.file_paths,
            &msg.track_ids,
            &pattern,
            &msg.user_id,
        )
        .await
        {
            Ok(results) => {
                BatchRenameApplyResponse {
                    id: msg.id,
                    results: results.into_iter().map(Into::into).collect(),
                    error: None,
                }
                .send_signal_to_dart();
            }
            Err(e) => {
                logger::error(&format!("batch_rename_apply failed: {}", e));
                BatchRenameApplyResponse {
                    id: msg.id,
                    results: vec![],
                    error: Some(e.to_string()),
                }
                .send_signal_to_dart();
            }
        }
    }
}

pub async fn handle_check_naming_convention(context: Arc<AppContext>) {
    use signals::tools::*;
    let receiver = CheckNamingConventionRequest::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;
        let db = context.db().await;
        // The client pattern is an explicit override; without one the global
        // pattern from the live config is used. This is a read-only route, so
        // an override never mutates anything.
        let pattern = tools::rename::resolve_naming_pattern_from_context(
            &Arc::downgrade(&context),
            msg.pattern.as_deref(),
        )
        .await;
        match tools::rename::check_naming_convention(
            db.pool(),
            msg.source_id.as_deref(),
            &pattern,
            &msg.user_id,
        )
        .await
        {
            Ok(violations) => {
                CheckNamingConventionResponse {
                    id: msg.id,
                    violations: violations.into_iter().map(Into::into).collect(),
                    error: None,
                }
                .send_signal_to_dart();
            }
            Err(e) => {
                logger::error(&format!("check_naming_convention failed: {}", e));
                CheckNamingConventionResponse {
                    id: msg.id,
                    violations: vec![],
                    error: Some(e.to_string()),
                }
                .send_signal_to_dart();
            }
        }
    }
}

pub async fn handle_write_track_lyrics(context: Arc<AppContext>) {
    use signals::tools::*;
    let receiver = WriteTrackLyricsRequest::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;
        let db = context.db().await;
        match tools::lyrics::write_track_lyrics(db.pool(), &msg.track_id, &msg.lyrics).await {
            Ok(result) => {
                WriteTrackLyricsResponse {
                    id: msg.id,
                    success: result.success,
                    error: result.error,
                }
                .send_signal_to_dart();
            }
            Err(e) => {
                logger::error(&format!("write_track_lyrics failed: {}", e));
                WriteTrackLyricsResponse {
                    id: msg.id,
                    success: false,
                    error: Some(e.to_string()),
                }
                .send_signal_to_dart();
            }
        }
    }
}

pub async fn handle_romajize_lyrics() {
    use signals::tools::*;
    let receiver = RomajizeLyricsRequest::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;
        let req = tawai_core::signals::tools::RomajizeLyricsRequest {
            id: msg.id.clone(),
            lyrics: msg.lyrics,
            synced: msg.synced,
            lang: msg.lang,
        };
        match tawai_core::tools::lyrics::romajize_lyrics(req) {
            Ok(result) => {
                RomajizeLyricsResponse {
                    id: msg.id,
                    romajized: result.romajized,
                    synced: result.synced,
                    error: None,
                }
                .send_signal_to_dart();
            }
            Err(e) => {
                logger::error(&format!("romajize_lyrics failed: {}", e));
                RomajizeLyricsResponse {
                    id: msg.id,
                    romajized: String::new(),
                    synced: false,
                    error: Some(e.to_string()),
                }
                .send_signal_to_dart();
            }
        }
    }
}

pub async fn handle_find_duplicates(context: Arc<AppContext>) {
    use signals::tools::*;
    let receiver = FindDuplicatesRequest::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;
        let db = context.db().await;
        let options = tawai_core::tools::duplicates::FindDuplicatesOptions {
            check_fingerprint: msg.check_fingerprint,
            check_mbid: msg.check_mbid,
            check_file_size_duration: msg.check_file_size_duration,
            check_title_artist: msg.check_title_artist,
            min_confidence: msg.min_confidence.unwrap_or(0.0),
            source_id: msg.source_id.clone(),
        };
        match tawai_core::tools::duplicates::find_duplicates(db.pool(), &options).await {
            Ok(result) => {
                FindDuplicatesResponse {
                    id: msg.id,
                    groups: result.groups.into_iter().map(Into::into).collect(),
                    total_duplicates: result.total_duplicates,
                    total_groups: result.total_groups,
                    error: result.error,
                }
                .send_signal_to_dart();
            }
            Err(e) => {
                logger::error(&format!("find_duplicates failed: {}", e));
                FindDuplicatesResponse {
                    id: msg.id,
                    groups: vec![],
                    total_duplicates: 0,
                    total_groups: 0,
                    error: Some(e.to_string()),
                }
                .send_signal_to_dart();
            }
        }
    }
}
