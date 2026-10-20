use axum::{Extension, Json, extract::State, response::IntoResponse};
use tawai_core::audio::tags::AudioTag;
use tawai_core::db::account;
use tawai_core::signals::metadata::{FormatNamingPreviewRequest, FormatNamingPreviewResponse};
use tawai_core::utils::identify::preview_naming;

use crate::server::SharedState;

pub async fn handle_format_naming_preview(
    State(state): State<SharedState>,
    Extension(user_id): Extension<String>,
    Json(body): Json<FormatNamingPreviewRequest>,
) -> impl IntoResponse {
    let tag = AudioTag {
        title: body.title,
        artist: body.artist,
        album_artist: body.album_artist,
        album: body.album,
        release_date: body.release_date,
        track_number: body.track_number,
        disc_number: body.disc_number,
        album_disambiguation: body.album_disambiguation,
        total_discs: body.total_discs,
        ..Default::default()
    };

    let db = state.context.db().await;
    let master_key = state.context.master_key.read().await.clone();
    let cfg = state.context.cfg().await;
    let global_pattern = tawai_core::tools::rename::naming_pattern_from_config(cfg.as_ref());

    // Only the file-path preview resolves a source, so skip the lookup for the
    // naming-format help dialog.
    let role = if body.file_path.is_some() {
        account::get_user_role(db.pool(), &user_id)
            .await
            .ok()
            .flatten()
    } else {
        None
    };

    let result = preview_naming(
        db.pool(),
        &master_key,
        &user_id,
        role.as_deref(),
        body.track_id.as_deref(),
        body.target_source_id.as_deref(),
        body.file_path.as_deref(),
        body.pattern.as_deref(),
        &global_pattern,
        &tag,
    )
    .await;

    Json(FormatNamingPreviewResponse {
        id: body.id,
        result,
    })
}
