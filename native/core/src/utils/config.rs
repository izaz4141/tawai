use anyhow::Result;
use serde_json::{Map, Value};
use std::{
    path::{Path, PathBuf},
    sync::{Arc, LazyLock},
    time::UNIX_EPOCH,
};

use crate::utils::encryption::{decrypt, encrypt, valid_encryption_format};

/// The settings schema, embedded at compile time. This file is the single
/// source of truth for the shape, defaults, scope and capabilities of the
/// global settings document; `config.json` is always a concrete instance of it.
const SETTINGS_SCHEMA_JSON: &str = include_str!("../../../../assets/docs/settings_schema.json");

/// Version reported for a config document that carries no `schema_version`.
pub fn current_schema_version() -> u64 {
    SCHEMA.version
}

#[derive(Debug, Clone)]
pub struct SettingSpec {
    pub kind: String,
    pub values: Vec<String>,
    pub default: Value,
    pub scope: String,
    pub admin: bool,
    pub secret: bool,
    pub web_local: bool,
    pub min: Option<f64>,
    pub max: Option<f64>,
}

impl SettingSpec {
    pub fn is_device(&self) -> bool {
        self.scope == "device"
    }

    pub fn is_global(&self) -> bool {
        self.scope == "global"
    }
}

#[derive(Debug)]
pub struct Schema {
    pub version: u64,
    pub entries: Vec<(String, SettingSpec)>,
}

impl Schema {
    pub fn spec(&self, path: &str) -> Option<&SettingSpec> {
        self.entries.iter().find(|(p, _)| p == path).map(|(_, s)| s)
    }

    fn any(&self, path: &str, predicate: impl Fn(&SettingSpec) -> bool) -> bool {
        self.entries
            .iter()
            .any(|(p, s)| matches_prefix(path, p) && predicate(s))
    }
}

/// Returns true when `path` is `prefix` itself or sits underneath it.
fn matches_prefix(path: &str, prefix: &str) -> bool {
    if path == prefix {
        return true;
    }
    path.len() > prefix.len() && path.starts_with(prefix) && path.as_bytes()[prefix.len()] == b'.'
}

fn is_item_path(path: &str) -> bool {
    path.contains("[]")
}

#[derive(serde::Deserialize)]
struct RawSchema {
    version: u64,
    settings: Map<String, Value>,
}

#[derive(serde::Deserialize)]
struct RawSpec {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    values: Vec<String>,
    #[serde(default)]
    default: Value,
    #[serde(default)]
    scope: String,
    #[serde(default)]
    admin: bool,
    #[serde(default)]
    secret: bool,
    #[serde(default)]
    web_local: bool,
    #[serde(default)]
    min: Option<f64>,
    #[serde(default)]
    max: Option<f64>,
}

pub static SCHEMA: LazyLock<Schema> = LazyLock::new(|| {
    let raw: RawSchema = serde_json::from_str(SETTINGS_SCHEMA_JSON)
        .expect("assets/docs/settings_schema.json is not valid JSON");
    let mut entries: Vec<(String, SettingSpec)> = raw
        .settings
        .into_iter()
        .map(|(path, value)| {
            let spec: RawSpec = serde_json::from_value(value)
                .unwrap_or_else(|e| panic!("invalid settings schema entry `{path}`: {e}"));
            (
                path,
                SettingSpec {
                    kind: spec.kind,
                    values: spec.values,
                    default: spec.default,
                    scope: spec.scope,
                    admin: spec.admin,
                    secret: spec.secret,
                    web_local: spec.web_local,
                    min: spec.min,
                    max: spec.max,
                },
            )
        })
        .collect();
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    Schema {
        version: raw.version,
        entries,
    }
});

/// Per-user setting keys whose value is stored encrypted at rest. The value is
/// encrypted before it is written and decrypted when it is read back, so the
/// plaintext never leaves the process.
///
/// These keys are unrelated to the settings schema above: per-user settings are
/// flat key/value rows in the database, not part of the config document.
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

/// The nested default document, folded from the flat manifest.
pub static DEFAULT_SETTINGS: LazyLock<Value> = LazyLock::new(|| {
    let mut root = Value::Object(Map::new());
    set_path(&mut root, "schema_version", Value::from(SCHEMA.version));
    for (path, spec) in &SCHEMA.entries {
        if is_item_path(path) {
            continue;
        }
        set_path(&mut root, path, spec.default.clone());
    }
    root
});

/// A fresh copy of the default document.
pub fn default_settings() -> Value {
    DEFAULT_SETTINGS.clone()
}

pub fn path_is_device(path: &str) -> bool {
    SCHEMA.any(path, |s| s.is_device())
}

pub fn path_is_secret(path: &str) -> bool {
    SCHEMA.any(path, |s| s.secret)
}

pub fn path_is_admin(path: &str) -> bool {
    SCHEMA.any(path, |s| s.admin)
}

pub fn path_is_web_local(path: &str) -> bool {
    SCHEMA.any(path, |s| s.web_local)
}

/// Reads a dotted path out of a JSON document.
pub fn get_path<'a>(root: &'a Value, path: &str) -> Option<&'a Value> {
    let mut current = root;
    for part in path.split('.') {
        current = current.get(part)?;
    }
    Some(current)
}

/// Mutable counterpart to [`get_path`].
pub fn get_path_mut<'a>(root: &'a mut Value, path: &str) -> Option<&'a mut Value> {
    let mut current = root;
    for part in path.split('.') {
        current = current.get_mut(part)?;
    }
    Some(current)
}

/// Removes a dotted path from a JSON document.
pub fn remove_path(root: &mut Value, path: &str) {
    let Some((parent, leaf)) = path.rsplit_once('.') else {
        if let Value::Object(map) = root {
            map.remove(path);
        }
        return;
    };
    if let Some(Value::Object(map)) = get_path_mut(root, parent) {
        map.remove(leaf);
    }
}

/// Writes a dotted path into a JSON document, creating intermediate objects.
pub fn set_path(root: &mut Value, path: &str, value: Value) {
    let mut parts = path.split('.').peekable();
    let mut current = root;
    while let Some(part) = parts.next() {
        if parts.peek().is_none() {
            if !current.is_object() {
                *current = Value::Object(Map::new());
            }
            if let Value::Object(map) = current {
                map.insert(part.to_string(), value);
            }
            return;
        }
        if !current.is_object() {
            *current = Value::Object(Map::new());
        }
        let map = current.as_object_mut().expect("object");
        current = map
            .entry(part.to_string())
            .or_insert_with(|| Value::Object(Map::new()));
    }
}

/// Returns a deep copy of `value` with every path matching `should_strip` removed.
pub fn strip_paths(value: &Value, should_strip: impl Fn(&str) -> bool) -> Value {
    fn walk(value: &Value, prefix: &str, should_strip: &impl Fn(&str) -> bool) -> Value {
        match value {
            Value::Object(map) => {
                let mut out = Map::with_capacity(map.len());
                for (key, child) in map {
                    let path = if prefix.is_empty() {
                        key.clone()
                    } else {
                        format!("{prefix}.{key}")
                    };
                    if should_strip(&path) {
                        continue;
                    }
                    out.insert(key.clone(), walk(child, &path, should_strip));
                }
                Value::Object(out)
            }
            Value::Array(items) => Value::Array(
                items
                    .iter()
                    .map(|i| walk(i, prefix, should_strip))
                    .collect(),
            ),
            other => other.clone(),
        }
    }
    walk(value, "", &should_strip)
}

/// Removes objects that ended up empty, e.g. a parent whose every child was
/// stripped from a response.
pub fn prune_empty(value: Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut out = Map::new();
            for (key, child) in map {
                let child = prune_empty(child);
                if child.as_object().map(|m| m.is_empty()).unwrap_or(false) {
                    continue;
                }
                out.insert(key, child);
            }
            Value::Object(out)
        }
        other => other,
    }
}

/// Merges `incoming` into `current`.
///
/// Objects merge recursively, arrays replace wholesale, and `null` resets the
/// path to `default`. Unknown keys in `current` are preserved.
pub fn merge_into(current: &Value, incoming: &Value, default: &Value) -> Value {
    match (current, incoming) {
        (Value::Object(a), Value::Object(b)) => {
            let mut out = a.clone();
            for (key, value) in b {
                let fallback = default.get(key).unwrap_or(&Value::Null);
                let next = match out.get(key) {
                    Some(existing) => merge_into(existing, value, fallback),
                    None => {
                        if value.is_null() {
                            fallback.clone()
                        } else {
                            value.clone()
                        }
                    }
                };
                out.insert(key.clone(), next);
            }
            Value::Object(out)
        }
        (_, Value::Array(b)) => Value::Array(b.clone()),
        (_, Value::Null) => default.clone(),
        (_, other) => other.clone(),
    }
}

/// Returns true when `value` is usable for `spec`.
pub fn value_matches_spec(spec: &SettingSpec, value: &Value) -> bool {
    let in_range = |n: f64| {
        spec.min.map(|m| n >= m).unwrap_or(true) && spec.max.map(|m| n <= m).unwrap_or(true)
    };
    match spec.kind.as_str() {
        "string" => value.is_string(),
        "bool" => value.is_boolean(),
        "int" => match value.as_i64() {
            Some(n) => in_range(n as f64),
            None => false,
        },
        "number" => match value.as_f64() {
            Some(n) => in_range(n),
            None => false,
        },
        "enum" => value
            .as_str()
            .map(|s| spec.values.iter().any(|v| v == s))
            .unwrap_or(false),
        "color" => value
            .as_str()
            .map(|s| {
                s.len() == 7 && s.starts_with('#') && s[1..].chars().all(|c| c.is_ascii_hexdigit())
            })
            .unwrap_or(false),
        "array" => value.is_array(),
        _ => false,
    }
}

/// Fills every declared path with its default and replaces unusable values.
///
/// Unknown keys are left untouched, and the document's `schema_version` is
/// stamped with the version this build knows.
pub fn apply_schema_defaults(value: &mut Value) {
    for (path, spec) in &SCHEMA.entries {
        if is_item_path(path) {
            continue;
        }
        let usable = get_path(value, path)
            .map(|v| value_matches_spec(spec, v))
            .unwrap_or(false);
        if !usable {
            set_path(value, path, spec.default.clone());
        }
    }
    set_path(value, "schema_version", Value::from(SCHEMA.version));
}

/// Raised when a config document was written by a different schema version, or
/// declares a version that is not an unsigned integer.
#[derive(Debug)]
pub struct SchemaVersionMismatch {
    pub found: Value,
    pub expected: u64,
}

impl std::fmt::Display for SchemaVersionMismatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "config schema version {} does not match this build's {}",
            self.found, self.expected
        )
    }
}

impl std::error::Error for SchemaVersionMismatch {}

/// Returns true if `error` means the document belongs to another schema version.
pub fn is_version_mismatch(error: &anyhow::Error) -> bool {
    error.downcast_ref::<SchemaVersionMismatch>().is_some()
}

pub fn validate_schema_version(value: &Value) -> Result<(), SchemaVersionMismatch> {
    match value.get("schema_version") {
        None => Ok(()),
        Some(declared) if declared.as_u64() == Some(SCHEMA.version) => Ok(()),
        Some(declared) => Err(SchemaVersionMismatch {
            found: declared.clone(),
            expected: SCHEMA.version,
        }),
    }
}

pub async fn load_config(path: String, master_key: String) -> Result<Arc<AppConfig>> {
    let config_path = PathBuf::from(path.clone());
    if !config_path.exists() || !config_path.is_file() {
        anyhow::bail!("Config file is not valid: {}", path);
    }

    let contents = tokio::fs::read_to_string(&config_path).await?;
    let mut config_val: Value = serde_json::from_str(&contents)?;

    validate_schema_version(&config_val).map_err(anyhow::Error::new)?;

    for path in [
        "integrations.slskd.api_key",
        "integrations.nadekodon.api_key",
    ] {
        let stored = get_path(&config_val, path)
            .map(|v| v.as_str().unwrap_or("").to_string())
            .unwrap_or_default();
        if let Ok(plain) = decrypt(normalize_secret(&stored), master_key.as_str()) {
            set_path(&mut config_val, path, Value::String(plain));
        }
    }

    if let Some(accounts) =
        get_path_mut(&mut config_val, "accounts.saved").and_then(|v| v.as_array_mut())
    {
        for account in accounts.iter_mut() {
            if let Some(encrypted) = account.get("api_key").and_then(|v| v.as_str()) {
                if let Ok(plain) = decrypt(encrypted, master_key.as_str()) {
                    account["api_key"] = Value::String(plain);
                }
            }
        }
    }

    apply_schema_defaults(&mut config_val);

    let config_metadata = tokio::fs::metadata(&config_path).await?;
    let config_modified = config_metadata.modified()?;
    let config_mtime = config_modified.duration_since(UNIX_EPOCH)?.as_secs();
    Ok(Arc::new(AppConfig {
        value: config_val,
        path: config_path,
        mtime: config_mtime,
        writable: true,
    }))
}

pub async fn load_default_config(path: String) -> Result<Arc<AppConfig>> {
    let config_path = PathBuf::from(&path);
    let config = default_settings();

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
        writable: true,
    }))
}

/// Returns `path`'s modification time in seconds since the Unix epoch, or
/// `None` when the file is missing or its mtime cannot be read.
pub async fn config_file_mtime(path: &Path) -> Option<u64> {
    let modified = tokio::fs::metadata(path).await.ok()?.modified().ok()?;
    Some(modified.duration_since(UNIX_EPOCH).ok()?.as_secs())
}

/// An in-memory default document that must never be written back over the file
/// it was derived from. `mtime` records the rejected file's modification time
/// so reloads are skipped until that file changes.
pub fn readonly_default_config(path: String, mtime: u64) -> Arc<AppConfig> {
    Arc::new(AppConfig {
        value: default_settings(),
        path: PathBuf::from(path),
        mtime,
        writable: false,
    })
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

/// Returns true if `host` refers to the local machine.
pub fn is_local_host(host: &str) -> bool {
    let h = host.trim();
    h.eq_ignore_ascii_case("127.0.0.1") || h.eq_ignore_ascii_case("localhost")
}

pub struct AppConfig {
    pub value: Value,
    pub path: PathBuf,
    pub mtime: u64,
    /// False when the document could not be read and an in-memory default is
    /// standing in for it. Such a config must never be persisted.
    pub writable: bool,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_is_self_consistent() {
        let mut seen = std::collections::HashSet::new();
        for (path, spec) in &SCHEMA.entries {
            assert!(seen.insert(path.clone()), "duplicate path `{path}`");
            assert!(
                spec.scope == "device" || spec.scope == "global",
                "`{path}` has an unknown scope `{}`",
                spec.scope
            );
            if is_item_path(path) {
                continue;
            }
            assert!(
                value_matches_spec(spec, &spec.default),
                "`{path}` default {:?} does not match its type `{}`",
                spec.default,
                spec.kind
            );
            if spec.kind == "enum" {
                assert!(!spec.values.is_empty(), "`{path}` enum has no values");
                assert!(
                    spec.default
                        .as_str()
                        .map(|d| spec.values.iter().any(|v| v == d))
                        .unwrap_or(false),
                    "`{path}` default is not one of its values"
                );
            }
        }
    }

    #[test]
    fn defaults_fold_to_a_nested_document() {
        let defaults = default_settings();
        assert_eq!(
            defaults.get("schema_version").and_then(|v| v.as_u64()),
            Some(SCHEMA.version)
        );
        assert_eq!(
            get_path(&defaults, "library.organization.naming_pattern").and_then(|v| v.as_str()),
            SCHEMA
                .spec("library.organization.naming_pattern")
                .map(|s| s.default.as_str().unwrap_or(""))
        );
        assert!(
            get_path(&defaults, "accounts.saved")
                .map(|v| v.is_array())
                .unwrap_or(false)
        );
        for (path, spec) in &SCHEMA.entries {
            if is_item_path(path) {
                continue;
            }
            assert_eq!(
                get_path(&defaults, path),
                Some(&spec.default),
                "`{path}` missing from the folded defaults"
            );
        }
    }

    #[test]
    fn stripping_is_path_aware() {
        let mut value = default_settings();
        set_path(&mut value, "accounts.saved", serde_json::json!([]));
        let stripped = strip_paths(&value, path_is_device);
        assert!(get_path(&stripped, "accounts.saved").is_none());
        assert!(get_path(&stripped, "library.organization.naming_pattern").is_some());
        assert!(get_path(&stripped, "integrations.slskd.url").is_some());

        let no_secrets = strip_paths(&value, path_is_secret);
        assert!(get_path(&no_secrets, "integrations.slskd.api_key").is_none());
        assert!(get_path(&no_secrets, "integrations.slskd.url").is_some());
    }

    #[test]
    fn merging_resets_nulls_and_replaces_arrays() {
        let defaults = default_settings();
        let current = default_settings();
        let incoming = serde_json::json!({
            "downloads": { "quality": "lossless" },
            "accounts": { "saved": [] },
            "interface": { "theme": { "mode": null } }
        });
        let merged = merge_into(&current, &incoming, &defaults);
        assert_eq!(
            get_path(&merged, "downloads.quality").and_then(|v| v.as_str()),
            Some("lossless")
        );
        assert_eq!(
            get_path(&merged, "downloads.source").and_then(|v| v.as_str()),
            Some("slskd")
        );
        assert_eq!(
            get_path(&merged, "interface.theme.mode").and_then(|v| v.as_str()),
            Some("system")
        );
    }

    #[test]
    fn absent_schema_version_is_allowed() {
        assert!(validate_schema_version(&serde_json::json!({})).is_ok());
        assert!(validate_schema_version(&serde_json::json!({"server": {"port": 8181}})).is_ok());
    }

    #[test]
    fn present_schema_version_must_be_the_current_unsigned_integer() {
        assert!(
            validate_schema_version(&serde_json::json!({"schema_version": SCHEMA.version})).is_ok()
        );

        let rejected = [
            serde_json::json!({"schema_version": SCHEMA.version + 1}),
            serde_json::json!({"schema_version": "1"}),
            serde_json::json!({"schema_version": 1.5}),
            serde_json::json!({"schema_version": -1}),
            serde_json::json!({"schema_version": true}),
            serde_json::json!({"schema_version": null}),
        ];
        for value in rejected {
            let err = validate_schema_version(&value)
                .expect_err("non-current schema_version should be rejected");
            assert_eq!(err.expected, SCHEMA.version);
            assert!(
                is_version_mismatch(&anyhow::Error::new(err)),
                "invalid schema_version must take the read-only path, not the overwrite path"
            );
        }
    }
}
