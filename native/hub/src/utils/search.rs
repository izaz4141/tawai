use rinf::{DartSignal, RustSignal};
use std::sync::Arc;

use crate::signals::metadata::{
    FetchLyricsRequest, FetchLyricsResponse, SearchLyricsRequest, SearchLyricsResponse,
};
use crate::utils::logger;
use tawai_core::app_context::AppContext;
use tawai_core::metadata::lrclib;

pub async fn handle_fetch_lyrics(context: Arc<AppContext>) {
    let receiver = FetchLyricsRequest::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;
        let album = msg.album.as_deref().unwrap_or("");
        let duration = msg.duration.unwrap_or(0.0);

        match lrclib::get_lyrics(
            context.client(),
            &msg.title,
            &msg.artist,
            album,
            duration,
            msg.prefer_sync,
        )
        .await
        {
            Ok(lyrics_result) => {
                FetchLyricsResponse {
                    id: msg.id,
                    result: Some(lyrics_result.into()),
                    error: None,
                }
                .send_signal_to_dart();
            }
            Err(e) => {
                logger::error(&format!("fetch_lyrics failed: {}", e));
                FetchLyricsResponse {
                    id: msg.id,
                    result: None,
                    error: Some(e.to_string()),
                }
                .send_signal_to_dart();
            }
        }
    }
}

pub async fn handle_search_lyrics(context: Arc<AppContext>) {
    let receiver = SearchLyricsRequest::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;

        match lrclib::search_lyrics(context.client(), &msg.query).await {
            Ok(results) => {
                SearchLyricsResponse {
                    id: msg.id,
                    results: results.into_iter().map(Into::into).collect(),
                    error: None,
                }
                .send_signal_to_dart();
            }
            Err(e) => {
                logger::error(&format!("search_lyrics failed: {}", e));
                SearchLyricsResponse {
                    id: msg.id,
                    results: vec![],
                    error: Some(e.to_string()),
                }
                .send_signal_to_dart();
            }
        }
    }
}
