pub mod account;
pub mod auth;
pub mod discovery;
pub mod download;
pub mod identify;
pub mod library;
pub mod playback;
pub mod settings;
pub mod system;
pub mod tools;
pub mod utils;
pub mod version;

#[allow(ambiguous_glob_reexports)]
pub use account::*;
pub use auth::*;
pub use discovery::*;
pub use download::*;
pub use identify::*;
pub use library::*;
pub use playback::*;
pub use settings::*;
pub use system::*;
pub use tools::*;
pub use utils::*;
pub use version::*;

use crate::server::SharedState;
use axum::Router;

pub fn create_tawai_router(state: SharedState) -> Router<SharedState> {
    let account_router = create_account_router(state.clone());
    let auth_router = create_auth_router(state.clone());
    let discovery_router = create_discovery_router(state.clone());
    let download_router = create_download_router(state.clone());
    let identify_router = create_identify_router();
    let library_router = create_library_router(state.clone());
    let system_router = create_system_router(state.clone());
    let utils_router = create_utils_router(state.clone());
    let version_router = create_version_router(state.clone());
    let playback_router = create_playback_router(state.clone());
    let settings_router = create_settings_router(state.clone());
    let tools_router = create_tools_router(state.clone());

    Router::new()
        .nest("/account", account_router)
        .nest("/auth", auth_router)
        .nest("/discovery", discovery_router)
        .nest("/download", download_router)
        .nest("/identify", identify_router)
        .nest("/library", library_router)
        .nest("/system", system_router)
        .nest("/utils", utils_router)
        .nest("/version", version_router)
        .nest("/playback", playback_router)
        .nest("/settings", settings_router)
        .nest("/tools", tools_router)
        .with_state(state)
}
