mod signals;
mod utils;

use std::sync::Arc;

use rinf::{dart_shutdown, write_interface};
use tawai_core::app_context::AppContext;
use tokio::{spawn, sync::Notify};

write_interface!();

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let shutdown_signal = Arc::new(Notify::new());
    let client = reqwest::Client::new();
    let context = AppContext::new(shutdown_signal, client);

    let db_done_signal = Arc::new(Notify::new());

    // Database manager + server
    spawn(utils::database::init_database_handler(
        context.clone(),
        db_done_signal,
    ));
    spawn(utils::server::start_server_listener(context.clone()));
    spawn(utils::server::handle_init_config(context.clone()));
    spawn(utils::server::handle_save_config(context.clone()));
    spawn(utils::server::handle_get_global_settings(context.clone()));

    // Encryption / API key handlers
    spawn(utils::server::handle_generate_api_key(context.clone()));
    spawn(utils::server::handle_api_key_generation(context.clone()));
    spawn(utils::server::handle_decrypt_request(context.clone()));
    spawn(utils::server::handle_encrypt_request(context.clone()));
    spawn(utils::security::handle_generate_master_key());
    spawn(utils::security::handle_login(context.clone()));

    // Library handlers
    spawn(utils::library::handle_list_tracks(context.clone()));
    spawn(utils::library::handle_list_albums(context.clone()));
    spawn(utils::library::handle_list_artists(context.clone()));
    spawn(utils::library::handle_list_playlists(context.clone()));
    spawn(utils::library::handle_create_playlist(context.clone()));
    spawn(utils::library::handle_delete_playlist(context.clone()));
    spawn(utils::library::handle_get_track(context.clone()));
    spawn(utils::library::handle_get_album_cover(context.clone()));
    spawn(utils::library::handle_get_track_cover(context.clone()));
    spawn(utils::tagging::handle_tagging());

    // Playback handlers
    spawn(utils::playback::handle_play_track(context.clone()));
    spawn(utils::playback::handle_preview_track(context.clone()));
    spawn(utils::playback::handle_report_playback(context.clone()));
    spawn(utils::playback::handle_get_history(context.clone()));
    spawn(utils::playback::handle_update_now_playing(context.clone()));

    // User settings handlers
    spawn(utils::user_settings::handle_set_user_setting(
        context.clone(),
    ));
    spawn(utils::user_settings::handle_get_user_setting(
        context.clone(),
    ));
    spawn(utils::user_settings::handle_get_all_user_settings(
        context.clone(),
    ));

    // Account handlers
    spawn(utils::account::handle_list_users(context.clone()));
    spawn(utils::account::handle_get_user_by_username(context.clone()));
    spawn(utils::account::handle_get_user_by_id(context.clone()));
    spawn(utils::account::handle_update_account(context.clone()));
    spawn(utils::account::handle_create_account(context.clone()));
    spawn(utils::account::handle_delete_account(context.clone()));
    spawn(utils::account::handle_verify_password(context.clone()));

    // Library source handlers
    spawn(utils::library_source::handle_add_library_source(
        context.clone(),
    ));
    spawn(utils::library_source::handle_remove_library_source(
        context.clone(),
    ));
    spawn(utils::library_source::handle_list_library_sources(
        context.clone(),
    ));
    spawn(utils::library_source::handle_list_editable_sources(
        context.clone(),
    ));
    spawn(utils::library_source::handle_test_jellyfin_source(
        context.clone(),
    ));

    // Playlist track handlers
    spawn(utils::library::handle_get_playlist_tracks(context.clone()));
    spawn(utils::library::handle_add_track_to_playlist(
        context.clone(),
    ));
    spawn(utils::library::handle_remove_track_from_playlist(
        context.clone(),
    ));
    spawn(utils::library::handle_reorder_playlist_tracks(
        context.clone(),
    ));

    // Scan handlers
    spawn(utils::scan::handle_scan_library(context.clone()));
    spawn(utils::scan::handle_scan_status(context.clone()));
    spawn(utils::scan::handle_start_periodic_scan(context.clone()));
    spawn(utils::scan::handle_scan_source(context.clone()));

    // Tag editor / Library identify handlers
    spawn(utils::tagging_editor::handle_list_unidentified_tracks(
        context.clone(),
    ));
    spawn(utils::tagging_editor::handle_apply_identification(
        context.clone(),
    ));
    spawn(utils::tagging_editor::handle_list_tracks_by_source(
        context.clone(),
    ));
    spawn(utils::tagging_editor::handle_get_album_mbid(
        context.clone(),
    ));
    spawn(utils::tagging_editor::handle_read_file_tags(
        context.clone(),
    ));
    spawn(utils::tagging_editor::handle_write_file_tags(
        context.clone(),
    ));
    spawn(utils::tagging_editor::handle_read_file_tags_bytes(
        context.clone(),
    ));
    spawn(utils::tagging_editor::handle_write_file_tags_bytes(
        context.clone(),
    ));
    spawn(utils::tagging_editor::handle_list_download_folder_tracks(
        context.clone(),
    ));

    // Identify / MusicBrainz handlers
    spawn(utils::identify::mb::handle_search_musicbrainz(
        context.clone(),
    ));
    spawn(utils::identify::mb::handle_get_release_tracks(
        context.clone(),
    ));
    spawn(utils::identify::mb::handle_fetch_recording(context.clone()));
    spawn(utils::identify::mb::handle_identify_single_track(
        context.clone(),
    ));
    spawn(utils::identify::mb::handle_fingerprint_track(
        context.clone(),
    ));

    // Lyrics handlers
    spawn(utils::search::handle_fetch_lyrics(context.clone()));
    spawn(utils::search::handle_search_lyrics(context.clone()));

    // Discovery / ListenBrainz handlers
    spawn(utils::discovery::lb::handle_get_lb_recommendations(
        context.clone(),
    ));
    spawn(utils::discovery::lb::handle_validate_lb_token(
        context.clone(),
    ));
    spawn(utils::discovery::sync_recs::handle_sync_recs(
        context.clone(),
    ));

    // Naming format preview
    spawn(utils::naming::handle_format_naming_preview(context.clone()));

    // Tools handlers
    spawn(utils::tools::handle_get_library_stats(context.clone()));
    spawn(utils::tools::handle_find_missing_metadata(context.clone()));
    spawn(utils::tools::handle_batch_rename_preview(context.clone()));
    spawn(utils::tools::handle_batch_rename_apply(context.clone()));
    spawn(utils::tools::handle_check_naming_convention(
        context.clone(),
    ));
    spawn(utils::tools::handle_write_track_lyrics(context.clone()));
    spawn(utils::tools::handle_romajize_lyrics());
    spawn(utils::tools::handle_find_duplicates(context.clone()));

    // Download client handlers (generalized via DownloadClient enum)
    spawn(utils::download::handle_list_downloads(context.clone()));
    spawn(utils::download::handle_download_create(context.clone()));
    spawn(utils::download::handle_download_pause(context.clone()));
    spawn(utils::download::handle_download_resume(context.clone()));
    spawn(utils::download::handle_download_cancel(context.clone()));
    spawn(utils::download::handle_download_delete(context.clone()));
    spawn(utils::download::handle_download_client_list(
        context.clone(),
    ));
    spawn(utils::download::handle_download_sync(context.clone()));
    spawn(utils::download::handle_download_poll(context.clone()));
    spawn(utils::download::handle_download_search(context.clone()));
    spawn(utils::download::handle_download_test_connection(
        context.clone(),
    ));
    spawn(utils::download::handle_download_get_info(context.clone()));

    // Version handlers
    spawn(utils::version::handle_get_current_version(context.clone()));
    spawn(utils::version::handle_get_latest_version(context.clone()));
    spawn(utils::version::handle_compare_versions(context.clone()));

    dart_shutdown().await;
}
