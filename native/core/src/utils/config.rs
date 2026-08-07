use anyhow::{Context as _, Result};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::UNIX_EPOCH,
};

use crate::utils::encryption::decrypt;
use crate::utils::logger;

/// Renders a depth-limited tree listing of `dir` for diagnostics.
fn dir_tree(dir: &Path, max_depth: usize) -> String {
    fn walk(dir: &Path, depth: usize, max_depth: usize, out: &mut String) {
        let indent = "  ".repeat(depth);
        match std::fs::read_dir(dir) {
            Ok(entries) => {
                let mut names: Vec<_> = entries.flatten().map(|e| e.file_name()).collect();
                names.sort();
                for name in names {
                    let name = name.to_string_lossy().to_string();
                    out.push_str(&format!("{indent}{name}\n"));
                    if depth < max_depth {
                        let sub = dir.join(&name);
                        if sub.is_dir() {
                            walk(&sub, depth + 1, max_depth, out);
                        }
                    }
                }
            }
            Err(e) => out.push_str(&format!("{indent}<error: {e}>\n")),
        }
    }

    let mut out = format!("{}:\n", dir.display());
    walk(dir, 0, max_depth, &mut out);
    out
}

/// Config keys holding downstream service secrets. Never exposed to clients
/// and stored encrypted on disk.
pub const PRIVATE_CONFIG_KEYS: &[&str] = &["slskd_api_key", "nadekodon_api_key"];

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
    logger::info(&format!(
        "[load_default_config] creating default config at: {}",
        config_path.display()
    ));

    let contents = tokio::fs::read_to_string("./assets/docs/default.json")
        .await
        .with_context(|| {
            let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("<unresolvable>"));
            format!(
                "read default.json (cwd={}, config_path={})\n{}",
                cwd.display(),
                config_path.display(),
                dir_tree(&cwd, 3)
            )
        })?;

    let config: serde_json::Value =
        serde_json::from_str(&contents).with_context(|| "parse default.json")?;

    if let Some(parent) = config_path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .with_context(|| format!("create_dir_all({})", parent.display()))?;
    }

    let json_str =
        serde_json::to_string_pretty(&config).with_context(|| "serialize default config")?;
    tokio::fs::write(&config_path, &json_str)
        .await
        .with_context(|| format!("write({})", config_path.display()))?;

    let config_metadata = tokio::fs::metadata(&config_path)
        .await
        .with_context(|| format!("metadata({})", config_path.display()))?;
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
