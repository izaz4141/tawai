use anyhow::Result;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::UNIX_EPOCH,
};

use crate::utils::encryption::{decrypt, encrypt, valid_encryption_format};

/// Default config JSON, embedded at compile time so it works regardless of
const DEFAULT_CONFIG_JSON: &str = include_str!("../../../../assets/docs/default.json");

/// Config keys holding downstream service secrets. Never exposed to clients
pub const PRIVATE_CONFIG_KEYS: &[&str] = &["slskd_api_key", "nadekodon_api_key"];

/// Per-user setting keys whose value is stored encrypted at rest. The value is
/// encrypted before it is written and decrypted when it is read back, so the
/// plaintext never leaves the process.
pub const SECRET_USER_SETTING_KEYS: &[&str] = &["listenbrainz_token"];

/// Returns true when the per-user setting `key` holds a secret value.
pub fn is_secret_user_setting(key: &str) -> bool {
    SECRET_USER_SETTING_KEYS.contains(&key)
}

/// Prepares a per-user setting value for storage.
///
/// Secret values are encrypted with the master key; every other value is
/// stored as-is. An already encrypted value is left untouched, so writing the
/// same value twice does not encrypt it twice. When the master key cannot be
/// used the plaintext is stored, so the setting stays readable.
pub fn encode_user_setting(key: &str, value: &str, master_key: &str) -> String {
    if !is_secret_user_setting(key) || value.is_empty() {
        return value.to_string();
    }
    if valid_encryption_format(value) {
        return value.to_string();
    }
    encrypt(value, master_key).unwrap_or_else(|_| value.to_string())
}

/// Restores a per-user setting value read from storage.
///
/// Secret values are decrypted with the master key. Values stored before
/// encryption was introduced are still plaintext and pass through unchanged,
/// as do values that cannot be decrypted.
pub fn decode_user_setting(key: &str, value: &str, master_key: &str) -> String {
    if !is_secret_user_setting(key) {
        return value.to_string();
    }
    if !valid_encryption_format(value) {
        return value.to_string();
    }
    decrypt(value, master_key).unwrap_or_else(|_| value.to_string())
}

/// Returns true if `host` refers to the local machine.
pub fn is_local_host(host: &str) -> bool {
    let h = host.trim();
    h.eq_ignore_ascii_case("127.0.0.1") || h.eq_ignore_ascii_case("localhost")
}

/// Returns a deep copy of `value` with every key listed in `keys` removed.
pub fn strip_keys(value: &serde_json::Value, keys: &[&str]) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => {
            let mut out = serde_json::Map::with_capacity(map.len());
            for (k, v) in map {
                if keys.contains(&k.as_str()) {
                    continue;
                }
                out.insert(k.clone(), strip_keys(v, keys));
            }
            serde_json::Value::Object(out)
        }
        serde_json::Value::Array(items) => {
            serde_json::Value::Array(items.iter().map(|i| strip_keys(i, keys)).collect())
        }
        other => other.clone(),
    }
}

pub struct AppConfig {
    pub value: serde_json::Value,
    pub path: PathBuf,
    pub mtime: u64,
}

pub fn dash_cache_dir(cfg: &AppConfig) -> String {
    let config_dir = cfg
        .path
        .parent()
        .unwrap_or_else(|| Path::new("/home/tawai/config"));
    config_dir
        .join("..")
        .join("cache")
        .join("dash")
        .to_string_lossy()
        .to_string()
}

pub async fn load_config(path: String, master_key: String) -> Result<Arc<AppConfig>> {
    let config_path = PathBuf::from(path.clone());
    if !config_path.exists() || !config_path.is_file() {
        anyhow::bail!("Config file is not valid: {}", path);
    }

    let contents = tokio::fs::read_to_string(&config_path).await?;
    let config_raw: serde_json::Value = serde_json::from_str(&contents)?;
    let mut config_val = config_raw.clone();

    let api_keys = vec!["slskd_api_key", "nadekodon_api_key"];
    for key in api_keys {
        let eapi = normalize_secret(&config_raw[key].to_string().as_str()).to_string();
        if let Ok(dapi) = decrypt(eapi.as_str(), master_key.as_str()) {
            config_val[key] = serde_json::Value::String(dapi);
        }
    }

    if let Some(accounts) = config_val
        .get_mut("accounts")
        .and_then(|v| v.as_array_mut())
    {
        for acc in accounts.iter_mut() {
            if let Some(eapi) = acc.get("api_key").and_then(|v| v.as_str()) {
                if let Ok(dapi) = decrypt(eapi, master_key.as_str()) {
                    acc["api_key"] = serde_json::Value::String(dapi);
                }
            }
        }
    }

    let config_metadata = tokio::fs::metadata(&config_path).await?;
    let config_modified = config_metadata.modified()?;
    let config_mtime = config_modified.duration_since(UNIX_EPOCH)?.as_secs();
    Ok(Arc::new(AppConfig {
        value: config_val,
        path: config_path,
        mtime: config_mtime,
    }))
}

pub async fn load_default_config(path: String) -> Result<Arc<AppConfig>> {
    let config_path = PathBuf::from(&path);
    let config: serde_json::Value = serde_json::from_str(DEFAULT_CONFIG_JSON)?;

    if let Some(parent) = config_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let json_str = serde_json::to_string_pretty(&config)?;
    tokio::fs::write(&config_path, json_str).await?;

    let config_metadata = tokio::fs::metadata(&config_path).await?;
    let config_modified = config_metadata.modified()?;
    let config_mtime = config_modified.duration_since(UNIX_EPOCH)?.as_secs();
    Ok(Arc::new(AppConfig {
        value: config,
        path: config_path,
        mtime: config_mtime,
    }))
}

pub fn normalize_secret(s: &str) -> &str {
    let s = s.trim();

    if s.len() >= 2 {
        let b = s.as_bytes();
        if (b[0] == b'"' && b[s.len() - 1] == b'"') || (b[0] == b'\'' && b[s.len() - 1] == b'\'') {
            return &s[1..s.len() - 1];
        }
    }

    s
}
