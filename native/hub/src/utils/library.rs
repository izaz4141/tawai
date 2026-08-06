use std::sync::Arc;

use crate::signals;
use crate::utils::logger;
use rinf::{DartSignal, RustSignal};
use tawai_core::app_context::AppContext;
use tawai_core::db::library;

pub async fn handle_list_tracks(context: Arc<AppContext>) {
    let receiver = signals::library::ListTracksRequest::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;
        let db = context.db().await;
        let tracks = library::list_tracks(db.pool(), msg.album_id.as_deref())
            .await
            .unwrap_or_default();
        signals::library::ListTracksResponse {
            id: msg.id,
            tracks: tracks.into_iter().map(Into::into).collect(),
        }
        .send_signal_to_dart();
    }
}

pub async fn handle_list_albums(context: Arc<AppContext>) {
    let receiver = signals::library::ListAlbumsRequest::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;
        let db = context.db().await;
        let albums = library::list_albums(db.pool(), msg.artist_id.as_deref())
            .await
            .unwrap_or_default();
        signals::library::ListAlbumsResponse {
            id: msg.id,
            albums: albums.into_iter().map(Into::into).collect(),
        }
        .send_signal_to_dart();
    }
}

pub async fn handle_list_artists(context: Arc<AppContext>) {
    let receiver = signals::library::ListArtistsRequest::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;
        let db = context.db().await;
        let artists = library::list_artists(db.pool()).await.unwrap_or_default();
        signals::library::ListArtistsResponse {
            id: msg.id,
            artists: artists.into_iter().map(Into::into).collect(),
        }
        .send_signal_to_dart();
    }
}

pub async fn handle_list_playlists(context: Arc<AppContext>) {
    let receiver = signals::library::ListPlaylistsRequest::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;
        let db = context.db().await;
        let playlists = library::list_playlists(db.pool()).await.unwrap_or_default();
        signals::library::ListPlaylistsResponse {
            id: msg.id,
            playlists: playlists.into_iter().map(Into::into).collect(),
        }
        .send_signal_to_dart();
    }
}

pub async fn handle_create_playlist(context: Arc<AppContext>) {
    let receiver = signals::library::CreatePlaylistRequest::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;
        let db = context.db().await;
        let result =
            tawai_core::db::library::create_playlist(db.pool(), &msg.user_id, &msg.name).await;
        match result {
            Ok(playlist_id) => {
                signals::library::CreatePlaylistResponse {
                    id: msg.id,
                    playlist_id,
                }
                .send_signal_to_dart();
            }
            Err(e) => {
                logger::error(&format!("create playlist failed: {}", e));
                signals::library::CreatePlaylistResponse {
                    id: msg.id,
                    playlist_id: String::new(),
                }
                .send_signal_to_dart();
            }
        }
    }
}

pub async fn handle_get_track(context: Arc<AppContext>) {
    let receiver = signals::library::GetTrackRequest::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;
        let db = context.db().await;

        match library::lookup_track(db.pool(), &msg.track_id).await {
            Ok(Some(t)) => {
                signals::library::GetTrackResponse {
                    id: msg.id,
                    track: t.into(),
                    error: None,
                }
                .send_signal_to_dart();
            }
            Ok(None) => {
                signals::library::GetTrackResponse {
                    id: msg.id,
                    track: signals::library::TrackInfo::default(),
                    error: Some("Track not found".to_string()),
                }
                .send_signal_to_dart();
            }
            Err(e) => {
                logger::error(&format!("get track failed: {}", e));
                signals::library::GetTrackResponse {
                    id: msg.id,
                    track: signals::library::TrackInfo::default(),
                    error: Some(e.to_string()),
                }
                .send_signal_to_dart();
            }
        }
    }
}

pub async fn handle_delete_playlist(context: Arc<AppContext>) {
    let receiver = signals::library::DeletePlaylistRequest::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;
        let db = context.db().await;
        let result = tawai_core::db::library::delete_playlist(db.pool(), &msg.playlist_id).await;
        match result {
            Ok(()) => {
                signals::library::DeletePlaylistResponse {
                    id: msg.id,
                    success: true,
                }
                .send_signal_to_dart();
            }
            Err(e) => {
                logger::error(&format!("delete playlist failed: {}", e));
                signals::library::DeletePlaylistResponse {
                    id: msg.id,
                    success: false,
                }
                .send_signal_to_dart();
            }
        }
    }
}

pub async fn handle_get_album_cover(context: Arc<AppContext>) {
    let receiver = signals::library::GetAlbumCoverRequest::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;
        let db = context.db().await;
        match tawai_core::db::library::get_album_cover(db.pool(), &msg.album_id).await {
            Ok(cover) => {
                signals::library::GetAlbumCoverResponse { id: msg.id, cover }.send_signal_to_dart();
            }
            Err(e) => {
                logger::error(&format!("get album cover failed: {}", e));
                signals::library::GetAlbumCoverResponse {
                    id: msg.id,
                    cover: None,
                }
                .send_signal_to_dart();
            }
        }
    }
}

pub async fn handle_get_track_cover(context: Arc<AppContext>) {
    let receiver = signals::library::GetTrackCoverRequest::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;
        let db = context.db().await;
        match tawai_core::db::library::get_track_cover(db.pool(), &msg.track_id).await {
            Ok(cover) => {
                signals::library::GetTrackCoverResponse { id: msg.id, cover }.send_signal_to_dart();
            }
            Err(e) => {
                logger::error(&format!("get track cover failed: {}", e));
                signals::library::GetTrackCoverResponse {
                    id: msg.id,
                    cover: None,
                }
                .send_signal_to_dart();
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Playlist tracks
// ---------------------------------------------------------------------------

pub async fn handle_get_playlist_tracks(context: Arc<AppContext>) {
    let receiver = signals::library::GetPlaylistTracksRequest::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;
        let db = context.db().await;
        match tawai_core::db::library::get_playlist_tracks(db.pool(), &msg.playlist_id).await {
            Ok(tracks) => {
                signals::library::GetPlaylistTracksResponse {
                    id: msg.id,
                    tracks: tracks.into_iter().map(Into::into).collect(),
                }
                .send_signal_to_dart();
            }
            Err(e) => {
                logger::error(&format!("get playlist tracks failed: {}", e));
                signals::library::GetPlaylistTracksResponse {
                    id: msg.id,
                    tracks: vec![],
                }
                .send_signal_to_dart();
            }
        }
    }
}

pub async fn handle_add_track_to_playlist(context: Arc<AppContext>) {
    let receiver = signals::library::AddTrackToPlaylistRequest::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;
        let db = context.db().await;
        match tawai_core::db::library::add_track_to_playlist(
            db.pool(),
            &msg.playlist_id,
            &msg.track_id,
        )
        .await
        {
            Ok(()) => {
                signals::library::AddTrackToPlaylistResponse {
                    id: msg.id,
                    success: true,
                }
                .send_signal_to_dart();
            }
            Err(e) => {
                logger::error(&format!("add track to playlist failed: {}", e));
                signals::library::AddTrackToPlaylistResponse {
                    id: msg.id,
                    success: false,
                }
                .send_signal_to_dart();
            }
        }
    }
}

pub async fn handle_remove_track_from_playlist(context: Arc<AppContext>) {
    let receiver = signals::library::RemoveTrackFromPlaylistRequest::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;
        let db = context.db().await;
        match tawai_core::db::library::remove_track_from_playlist(
            db.pool(),
            &msg.playlist_id,
            &msg.track_id,
        )
        .await
        {
            Ok(()) => {
                signals::library::RemoveTrackFromPlaylistResponse {
                    id: msg.id,
                    success: true,
                }
                .send_signal_to_dart();
            }
            Err(e) => {
                logger::error(&format!("remove track from playlist failed: {}", e));
                signals::library::RemoveTrackFromPlaylistResponse {
                    id: msg.id,
                    success: false,
                }
                .send_signal_to_dart();
            }
        }
    }
}

pub async fn handle_reorder_playlist_tracks(context: Arc<AppContext>) {
    let receiver = signals::library::ReorderPlaylistTracksRequest::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;
        let db = context.db().await;
        match tawai_core::db::library::reorder_playlist_tracks(
            db.pool(),
            &msg.playlist_id,
            &msg.track_ids,
        )
        .await
        {
            Ok(()) => {
                signals::library::ReorderPlaylistTracksResponse {
                    id: msg.id,
                    success: true,
                }
                .send_signal_to_dart();
            }
            Err(e) => {
                logger::error(&format!("reorder playlist tracks failed: {}", e));
                signals::library::ReorderPlaylistTracksResponse {
                    id: msg.id,
                    success: false,
                }
                .send_signal_to_dart();
            }
        }
    }
}
