use std::sync::Arc;

use rinf::{DartSignal, RustSignal};
use tawai_core::app_context::AppContext;
use tawai_core::audio::tags::AudioTag;
use tawai_core::db::account::get_user_role;
use tawai_core::utils::identify::preview_naming;

use crate::signals::metadata::{FormatNamingPreviewRequest, FormatNamingPreviewResponse};

pub async fn handle_format_naming_preview(context: Arc<AppContext>) {
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

        let db = context.db().await;
        let master_key = context.master_key.read().await.clone();

        // Only the file-path preview resolves a source, so skip the lookup for
        // the naming-format help dialog.
        let role = if msg.file_path.is_some() {
            get_user_role(db.pool(), &msg.user_id).await.ok().flatten()
        } else {
            None
        };

        let result = preview_naming(
            db.pool(),
            &master_key,
            &msg.user_id,
            role.as_deref(),
            msg.track_id.as_deref(),
            msg.target_source_id.as_deref(),
            msg.file_path.as_deref(),
            &msg.pattern,
            &tag,
        )
        .await;

        FormatNamingPreviewResponse { id: msg.id, result }.send_signal_to_dart();
    }
}
