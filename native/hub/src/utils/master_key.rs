use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use rinf::{DartSignal, RustSignal};
use tawai_core::app_context::AppContext;
use tawai_core::utils::master_key::{self, KeySource, MasterKeyError};
use tawai_core::utils::paths;

use crate::signals::crypt::{GenerateMasterKeyRequest, GenerateMasterKeyResponse};
use crate::signals::server::{SetTawaiHome, SetTawaiHomeResponse};
use crate::utils::logger;

#[derive(Clone)]
struct Bootstrap {
    home: PathBuf,
    device_id: String,
}

static BOOTSTRAP: Mutex<Option<Bootstrap>> = Mutex::new(None);

pub async fn handle_set_tawai_home(context: Arc<AppContext>) {
    let receiver = SetTawaiHome::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;
        let (success, error) = match bootstrap(&context, &msg.path, &msg.device_id) {
            Ok(()) => (true, None),
            Err(e) => {
                logger::error(&format!("failed to initialize tawai home: {e}"));
                (false, Some(e))
            }
        };
        SetTawaiHomeResponse {
            id: msg.id,
            success,
            error,
        }
        .send_signal_to_dart();
    }
}

fn bootstrap(context: &AppContext, path: &str, device_id: &str) -> Result<(), String> {
    paths::set_tawai_home(path)?;
    let home = PathBuf::from(paths::tawai_home());
    install(context, &home, device_id)?;

    let mut stored = BOOTSTRAP.lock().unwrap_or_else(|e| e.into_inner());
    *stored = Some(Bootstrap {
        home,
        device_id: device_id.to_string(),
    });
    Ok(())
}

fn install(context: &AppContext, home: &Path, device_id: &str) -> Result<(), String> {
    if context.has_master_key() {
        let (key, _) = master_key::read_existing(home, device_id).map_err(|e| e.to_string())?;
        if !context.master_key_matches(&key) {
            return Err(MasterKeyError::KeyFileMismatch {
                path: PathBuf::from(paths::master_key_path()),
            }
            .to_string());
        }
        return Ok(());
    }

    let (key, source) = master_key::resolve(home, device_id).map_err(|e| e.to_string())?;
    context.set_master_key(key).map_err(|e| e.to_string())?;

    match source {
        KeySource::Created => logger::info(&format!(
            "created master key at {}",
            paths::master_key_path()
        )),
        KeySource::Environment => {
            logger::debug("loaded master key from TAWAI_SERVER_MASTER_KEY");
        }
        KeySource::File => logger::debug(&format!(
            "loaded master key from {}",
            paths::master_key_path()
        )),
    }

    Ok(())
}

/// Prepares the key only through the bootstrap path. It never generates a new
/// key on demand: rotating here would make every stored API key undecryptable.
pub async fn handle_generate_master_key(context: Arc<AppContext>) {
    let receiver = GenerateMasterKeyRequest::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;
        let result = if context.has_master_key() {
            Ok(())
        } else {
            let bootstrap = BOOTSTRAP.lock().unwrap_or_else(|e| e.into_inner()).clone();
            match bootstrap {
                Some(bootstrap) => install(&context, &bootstrap.home, &bootstrap.device_id),
                None => Err("tawai home is not initialized".to_string()),
            }
        };

        let (success, error) = match result {
            Ok(()) => (true, None),
            Err(e) => {
                logger::error(&format!("failed to prepare master key: {e}"));
                (false, Some(e))
            }
        };
        GenerateMasterKeyResponse {
            id: msg.id,
            success,
            error,
        }
        .send_signal_to_dart();
    }
}
