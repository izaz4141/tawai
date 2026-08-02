use std::sync::Arc;

use rinf::{DartSignal, RustSignal};
use tawai_core::app_context::AppContext;
use tawai_core::audio::tags::AudioTag;
use tawai_core::tools::rename::format_naming_pattern;

use crate::signals::metadata::{FormatNamingPreviewRequest, FormatNamingPreviewResponse};

pub async fn handle_format_naming_preview(_context: Arc<AppContext>) {
    let receiver = FormatNamingPreviewRequest::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;

        let tag = AudioTag {
            title: msg.title,
            artist: msg.artist,
            album_artist: msg.album_artist,
            album: msg.album,
            release_date: msg.release_date,
            track_number: msg.track_number,
            disc_number: msg.disc_number,
            album_disambiguation: msg.album_disambiguation,
            total_discs: msg.total_discs,
            ..Default::default()
        };

        let result = format_naming_pattern(&msg.pattern, &tag);

        FormatNamingPreviewResponse { id: msg.id, result }.send_signal_to_dart();
    }
}
