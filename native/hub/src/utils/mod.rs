pub mod account;
pub mod database;
pub mod discovery;
pub mod download;
pub mod identify;
pub mod library;
pub mod library_source;
pub mod logger;
pub mod naming;
pub mod playback;
pub mod scan;
pub mod search;
pub mod security;
pub mod server;
pub mod tagging;
pub mod tagging_editor;
pub mod tools;
pub mod user_settings;
pub mod version;

use tawai_core::app_context::AppContext;
use tawai_core::utils::master_key::KeyBytes;

/// Returns a zeroized-on-drop copy of the master key, logging when it is
/// unavailable so every signal handler can answer or skip consistently.
pub fn master_key_or_log(context: &AppContext) -> Option<KeyBytes> {
    match context.master_key_bytes() {
        Ok(key) => Some(key),
        Err(e) => {
            logger::error(&format!("master key unavailable: {e}"));
            None
        }
    }
}
