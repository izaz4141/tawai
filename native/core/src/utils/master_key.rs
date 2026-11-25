use std::fmt;
use std::fs;
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use jsonwebtoken::{Header, TokenData, Validation};
use zeroize::Zeroizing;

use crate::utils::tagging::{dii_bytes, oc_bytes};

/// Length of a master key in bytes.
pub const MASTER_KEY_LEN: usize = 32;

/// Errors returned when a master key cannot be created or read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MasterKeyError {
    InvalidHex,
    InvalidLength,
    NotSet,
    AlreadySet,
    MissingDeviceId,
    KeyFileUnreadable { path: PathBuf, reason: String },
    KeyFileInvalid { path: PathBuf, reason: String },
    KeyFileWrite { path: PathBuf, reason: String },
    KeyFileMismatch { path: PathBuf },
}

impl fmt::Display for MasterKeyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidHex => f.write_str("master key is not valid hex"),
            Self::InvalidLength => f.write_str("master key must be 64 hex characters"),
            Self::NotSet => f.write_str("master key is not set"),
            Self::AlreadySet => f.write_str("master key is already set"),
            Self::MissingDeviceId => f.write_str("device id is required to protect the master key"),
            Self::KeyFileUnreadable { path, reason } => {
                write!(
                    f,
                    "cannot read master key file {}: {reason}",
                    path.display()
                )
            }
            Self::KeyFileInvalid { path, reason } => {
                write!(f, "master key file {} is invalid: {reason}", path.display())
            }
            Self::KeyFileWrite { path, reason } => {
                write!(
                    f,
                    "cannot write master key file {}: {reason}",
                    path.display()
                )
            }
            Self::KeyFileMismatch { path } => {
                write!(
                    f,
                    "master key file {} does not match the installed master key",
                    path.display()
                )
            }
        }
    }
}

impl std::error::Error for MasterKeyError {}

/// Where the current master key came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeySource {
    File,
    Created,
}

/// Loads the master key from `path` or creates and persists a new one.
///
/// An existing file is never overwritten. Any read, decode or validation
/// failure is returned as an error so the caller cannot silently replace a key
/// that may still decrypt stored data.
pub fn resolve(path: &Path, device_id: &str) -> Result<(MasterKey, KeySource), MasterKeyError> {
    if device_id.trim().is_empty() {
        return Err(MasterKeyError::MissingDeviceId);
    }

    if path.exists() {
        return Ok((read_key(path, device_id)?, KeySource::File));
    }

    let key = MasterKey::generate();
    let protected = key
        .protect(device_id)
        .map_err(|reason| MasterKeyError::KeyFileWrite {
            path: path.to_path_buf(),
            reason,
        })?;
    let mut encoded = Zeroizing::new([0u8; MASTER_KEY_LEN * 2]);
    hex::encode_to_slice(&protected[..], &mut encoded[..]).map_err(|e| {
        MasterKeyError::KeyFileWrite {
            path: path.to_path_buf(),
            reason: e.to_string(),
        }
    })?;

    let parent = path.parent().filter(|p| !p.as_os_str().is_empty());
    if let Some(parent) = parent {
        fs::create_dir_all(parent).map_err(|e| MasterKeyError::KeyFileWrite {
            path: path.to_path_buf(),
            reason: e.to_string(),
        })?;
    }

    let dir = parent.unwrap_or_else(|| Path::new("."));
    let mut builder = tempfile::Builder::new();
    builder.prefix(".master-key-");
    #[cfg(unix)]
    builder.permissions(fs::Permissions::from_mode(0o600));

    let mut temp = builder
        .tempfile_in(dir)
        .map_err(|e| MasterKeyError::KeyFileWrite {
            path: path.to_path_buf(),
            reason: e.to_string(),
        })?;

    temp.write_all(&encoded[..])
        .map_err(|e| MasterKeyError::KeyFileWrite {
            path: path.to_path_buf(),
            reason: e.to_string(),
        })?;
    temp.as_file()
        .sync_all()
        .map_err(|e| MasterKeyError::KeyFileWrite {
            path: path.to_path_buf(),
            reason: e.to_string(),
        })?;

    match temp.persist_noclobber(path) {
        Ok(_) => Ok((key, KeySource::Created)),
        Err(e) if e.error.kind() == ErrorKind::AlreadyExists => {
            Ok((read_key(path, device_id)?, KeySource::File))
        }
        Err(e) => Err(MasterKeyError::KeyFileWrite {
            path: path.to_path_buf(),
            reason: e.error.to_string(),
        }),
    }
}

/// Reads the master key stored at `path` without creating or replacing it.
///
/// Unlike [`resolve`], a missing file is an error so callers can verify that
/// the requested file already holds the installed key.
pub fn read(path: &Path, device_id: &str) -> Result<MasterKey, MasterKeyError> {
    if device_id.trim().is_empty() {
        return Err(MasterKeyError::MissingDeviceId);
    }

    read_key(path, device_id)
}

fn read_key(path: &Path, device_id: &str) -> Result<MasterKey, MasterKeyError> {
    let raw = Zeroizing::new(
        fs::read(path).map_err(|e| MasterKeyError::KeyFileUnreadable {
            path: path.to_path_buf(),
            reason: e.to_string(),
        })?,
    );
    let protected = decode_stored_key(&raw).map_err(|reason| MasterKeyError::KeyFileInvalid {
        path: path.to_path_buf(),
        reason,
    })?;

    MasterKey::unprotect(&protected, device_id).map_err(|reason| MasterKeyError::KeyFileInvalid {
        path: path.to_path_buf(),
        reason,
    })
}

/// Decodes the 64-hex-character tagged-key format used by current and earlier
/// releases. The writer keeps this format, so no conversion is needed.
fn decode_stored_key(raw: &[u8]) -> Result<Zeroizing<[u8; MASTER_KEY_LEN]>, String> {
    let encoded = std::str::from_utf8(raw)
        .map_err(|_| "file is not valid hex".to_string())?
        .trim();
    if encoded.is_empty() {
        return Err("file is not valid hex".to_string());
    }

    let mut protected = Zeroizing::new([0u8; MASTER_KEY_LEN]);
    hex::decode_to_slice(encoded, &mut protected[..])
        .map_err(|_| "file is not valid hex".to_string())?;
    Ok(protected)
}

/// A master key held in protected memory.
///
/// The absence of `Clone`, `Copy`, `Display`, `Serialize` and `Deserialize` is
/// deliberate: keys enter through [`MasterKey::from_hex`] or the persisted-key
/// loader, and the only way to use one is through [`MasterKeySlot`] and
/// [`KeyBytes`].
pub struct MasterKey {
    inner: secrets::SecretBox<[u8; MASTER_KEY_LEN]>,
}

impl MasterKey {
    /// Creates a key inside protected memory.
    pub fn generate() -> Self {
        Self {
            inner: secrets::SecretBox::random(),
        }
    }

    /// Decodes 64 hex characters into protected memory.
    pub fn from_hex(value: &str) -> Result<Self, MasterKeyError> {
        let value = value.trim();
        if value.len() != MASTER_KEY_LEN * 2 {
            return Err(MasterKeyError::InvalidLength);
        }
        let inner = secrets::SecretBox::try_new(|buf: &mut [u8; MASTER_KEY_LEN]| {
            hex::decode_to_slice(value, buf).map_err(|_| MasterKeyError::InvalidHex)
        })?;
        Ok(Self { inner })
    }

    fn matches(&self, other: &MasterKey) -> bool {
        self.inner == other.inner
    }

    /// Encodes the key for the on-disk format without exposing a printable form.
    fn protect(&self, device_id: &str) -> Result<Zeroizing<[u8; MASTER_KEY_LEN]>, String> {
        let borrowed = self.inner.borrow();
        oc_bytes(&*borrowed, device_id)
    }

    fn unprotect(encoded: &[u8; MASTER_KEY_LEN], device_id: &str) -> Result<Self, String> {
        Ok(Self::from_protected(dii_bytes(encoded, device_id)?))
    }

    fn from_protected(mut bytes: Zeroizing<[u8; MASTER_KEY_LEN]>) -> Self {
        Self {
            inner: secrets::SecretBox::from(&mut *bytes),
        }
    }
}

impl fmt::Debug for MasterKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("MasterKey(<redacted>)")
    }
}

/// An owned copy of a master key that is zeroized on drop.
///
/// Async code cannot hold a [`MasterKey`] borrow across an `await`, so this is
/// the narrow exception that lets a handler use the key for the duration of a
/// request. It exposes only the cryptographic operations built on the key.
pub struct KeyBytes {
    bytes: zeroize::Zeroizing<[u8; MASTER_KEY_LEN]>,
}

impl KeyBytes {
    fn new(bytes: [u8; MASTER_KEY_LEN]) -> Self {
        Self {
            bytes: zeroize::Zeroizing::new(bytes),
        }
    }

    /// Encrypts a secret with the master key.
    pub fn encrypt(&self, plaintext: &str) -> anyhow::Result<String> {
        crate::utils::encryption::encrypt(plaintext, &self.bytes)
    }

    /// Decrypts a secret with the master key.
    pub fn decrypt(&self, ciphertext: &str) -> anyhow::Result<String> {
        crate::utils::encryption::decrypt(ciphertext, &self.bytes)
    }

    /// Signs JWT claims with the master key.
    pub fn jwt_encode<C: serde::Serialize>(
        &self,
        header: &Header,
        claims: &C,
    ) -> Result<String, jsonwebtoken::errors::Error> {
        jsonwebtoken::encode(
            header,
            claims,
            &jsonwebtoken::EncodingKey::from_secret(self.bytes.as_slice()),
        )
    }

    /// Verifies and decodes a JWT with the master key.
    pub fn jwt_decode<C: serde::de::DeserializeOwned>(
        &self,
        token: &str,
        validation: &Validation,
    ) -> Result<TokenData<C>, jsonwebtoken::errors::Error> {
        jsonwebtoken::decode(
            token,
            &jsonwebtoken::DecodingKey::from_secret(self.bytes.as_slice()),
            validation,
        )
    }

    #[cfg(test)]
    pub(crate) fn for_test(bytes: [u8; MASTER_KEY_LEN]) -> Self {
        Self::new(bytes)
    }
}

impl fmt::Debug for KeyBytes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("KeyBytes(<redacted>)")
    }
}

/// The single owner of a [`MasterKey`] inside the process.
///
/// Uses a `std::sync::Mutex` because [`secrets::SecretBox`] is `!Sync` and a
/// synchronous lock keeps the guard out of async code. Never hold the guard
/// across an `await`; use [`MasterKeySlot::bytes`] to get an owned copy instead.
pub struct MasterKeySlot {
    inner: Mutex<Option<MasterKey>>,
}

impl Default for MasterKeySlot {
    fn default() -> Self {
        Self::new()
    }
}

impl MasterKeySlot {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(None),
        }
    }

    fn lock(&self) -> MutexGuard<'_, Option<MasterKey>> {
        self.inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Stores the first key seen and rejects every later one.
    pub fn set(&self, key: MasterKey) -> Result<(), MasterKeyError> {
        let mut slot = self.lock();
        if slot.is_some() {
            return Err(MasterKeyError::AlreadySet);
        }
        *slot = Some(key);
        Ok(())
    }

    /// Stores the first key seen; later calls succeed only for an identical key.
    pub fn install(&self, key: MasterKey) -> Result<(), MasterKeyError> {
        let mut slot = self.lock();
        match slot.as_ref() {
            None => {
                *slot = Some(key);
                Ok(())
            }
            Some(existing) if existing.matches(&key) => Ok(()),
            Some(_) => Err(MasterKeyError::AlreadySet),
        }
    }

    /// Returns a zeroized-on-drop copy for use across an `await`.
    pub fn bytes(&self) -> Result<KeyBytes, MasterKeyError> {
        let slot = self.lock();
        let key = slot.as_ref().ok_or(MasterKeyError::NotSet)?;
        let borrowed = key.inner.borrow();
        Ok(KeyBytes::new(*borrowed))
    }

    pub fn is_set(&self) -> bool {
        self.lock().is_some()
    }

    /// Reports whether the stored key equals `key`.
    pub fn matches(&self, key: &MasterKey) -> bool {
        self.lock()
            .as_ref()
            .is_some_and(|existing| existing.matches(key))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::tagging::{dii, oc};

    const HEX: &str = "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f";

    #[test]
    fn from_hex_accepts_64_characters() {
        let key = MasterKey::from_hex(HEX).expect("valid key");
        let bytes = MasterKeySlot::new();
        bytes.set(key).expect("slot is empty");
        assert_eq!(bytes.bytes().expect("key is set").bytes[0], 0x00);
    }

    #[test]
    fn from_hex_trims_whitespace() {
        assert!(MasterKey::from_hex(&format!(" {HEX}\n")).is_ok());
    }

    #[test]
    fn from_hex_rejects_wrong_length() {
        assert_eq!(
            MasterKey::from_hex(&HEX[..62]).expect_err("63 chars"),
            MasterKeyError::InvalidLength
        );
        assert_eq!(
            MasterKey::from_hex(&format!("{HEX}00")).expect_err("65 chars"),
            MasterKeyError::InvalidLength
        );
    }

    #[test]
    fn from_hex_rejects_non_hex() {
        let value = "z".repeat(MASTER_KEY_LEN * 2);
        assert_eq!(
            MasterKey::from_hex(&value).expect_err("not hex"),
            MasterKeyError::InvalidHex
        );
    }

    #[test]
    fn debug_is_redacted() {
        let key = MasterKey::from_hex(HEX).expect("valid key");
        let debug = format!("{key:?}");
        assert!(debug.contains("redacted"));
        assert!(!debug.contains(HEX));
    }

    #[test]
    fn key_bytes_debug_is_redacted() {
        let slot = MasterKeySlot::new();
        slot.set(MasterKey::from_hex(HEX).expect("valid key"))
            .expect("slot is empty");
        let debug = format!("{:?}", slot.bytes().expect("key is set"));
        assert!(debug.contains("redacted"));
        assert!(!debug.contains(HEX));
    }

    #[test]
    fn slot_is_first_wins() {
        let slot = MasterKeySlot::new();
        assert_eq!(slot.bytes().expect_err("empty"), MasterKeyError::NotSet);
        assert!(!slot.is_set());

        slot.set(MasterKey::from_hex(HEX).expect("valid key"))
            .expect("slot is empty");
        assert!(slot.is_set());
        assert_eq!(
            slot.set(MasterKey::from_hex(HEX).expect("valid key"))
                .expect_err("occupied"),
            MasterKeyError::AlreadySet
        );
    }

    #[test]
    fn slot_install_accepts_first_and_identical_keys() {
        let slot = MasterKeySlot::new();
        slot.install(MasterKey::from_hex(HEX).expect("valid key"))
            .expect("slot is empty");
        slot.install(MasterKey::from_hex(HEX).expect("valid key"))
            .expect("identical key");
    }

    #[test]
    fn slot_install_rejects_a_different_key() {
        let slot = MasterKeySlot::new();
        slot.install(MasterKey::from_hex(HEX).expect("valid key"))
            .expect("slot is empty");

        let other = "ff".repeat(MASTER_KEY_LEN);
        assert_eq!(
            slot.install(MasterKey::from_hex(&other).expect("valid key"))
                .expect_err("different key"),
            MasterKeyError::AlreadySet
        );
    }

    #[test]
    fn key_bytes_encrypt_decrypt_roundtrip() {
        let slot = MasterKeySlot::new();
        slot.set(MasterKey::from_hex(HEX).expect("valid key"))
            .expect("slot is empty");

        let key = slot.bytes().expect("key is set");
        let ciphertext = key.encrypt("secret").expect("encrypt");
        assert!(!ciphertext.contains("secret"));
        assert_eq!(key.decrypt(&ciphertext).expect("decrypt"), "secret");
    }

    #[test]
    fn resolve_creates_and_reloads_key_file() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("config/master.key");

        let (key, source) = resolve(&path, "test-device").expect("create key");
        assert_eq!(source, KeySource::Created);

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(&path).expect("metadata").permissions().mode() & 0o777;
            assert_eq!(mode, 0o600);
        }

        let entries: Vec<_> = fs::read_dir(path.parent().expect("parent"))
            .expect("read config dir")
            .map(|entry| entry.expect("entry").file_name())
            .collect();
        assert_eq!(entries, [std::ffi::OsString::from("master.key")]);

        let stored: [u8; MASTER_KEY_LEN] = *key.inner.borrow();
        let stored_hex = hex::encode(stored);
        let encoded = fs::read_to_string(&path).expect("read key file");
        assert!(!encoded.contains(&stored_hex));
        assert_eq!(
            dii(encoded.trim(), "test-device").expect("decode key file"),
            stored_hex
        );

        let legacy = {
            let hex_key = Zeroizing::new(stored_hex);
            oc(hex_key.as_str(), "test-device").expect("encode legacy key")
        };
        assert_eq!(encoded, legacy);

        let (reloaded, source) = resolve(&path, "test-device").expect("reload key");
        assert_eq!(source, KeySource::File);
        assert_eq!(*reloaded.inner.borrow(), stored);
        assert_eq!(fs::read_to_string(&path).expect("read key file"), encoded);
    }

    #[test]
    fn resolve_reads_legacy_hex_key_file() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("master.key");
        let key = MasterKey::from_hex(HEX).expect("valid key");
        let stored: [u8; MASTER_KEY_LEN] = *key.inner.borrow();
        let legacy = {
            let hex_key = Zeroizing::new(hex::encode(stored));
            oc(hex_key.as_str(), "test-device").expect("encode legacy key")
        };
        fs::write(&path, legacy.as_bytes()).expect("seed legacy key file");

        let (reloaded, source) = resolve(&path, "test-device").expect("read legacy key");
        assert_eq!(source, KeySource::File);
        assert_eq!(*reloaded.inner.borrow(), stored);
    }

    #[test]
    fn read_loads_existing_key() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("master.key");
        resolve(&path, "test-device").expect("create key");

        let key = read(&path, "test-device").expect("read key");
        let slot = MasterKeySlot::new();
        slot.install(key).expect("install read key");

        let (resolved, _) = resolve(&path, "test-device").expect("resolve key");
        assert!(slot.matches(&resolved));
    }

    #[test]
    fn read_rejects_missing_file_without_creating_it() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("master.key");

        assert!(matches!(
            read(&path, "test-device"),
            Err(MasterKeyError::KeyFileUnreadable { .. })
        ));
        assert!(!path.exists());
    }

    #[test]
    fn read_requires_device_id() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("master.key");

        assert_eq!(
            read(&path, "  ").expect_err("empty device id"),
            MasterKeyError::MissingDeviceId
        );
        assert!(!path.exists());
    }

    #[test]
    fn slot_matches_reports_equality() {
        let slot = MasterKeySlot::new();
        let key = MasterKey::from_hex(HEX).expect("valid key");
        assert!(!slot.matches(&key));

        slot.install(MasterKey::from_hex(HEX).expect("valid key"))
            .expect("slot is empty");
        assert!(slot.matches(&key));

        let other = MasterKey::from_hex(&"ff".repeat(MASTER_KEY_LEN)).expect("valid key");
        assert!(!slot.matches(&other));
    }

    #[test]
    fn byte_tagging_matches_legacy_string_tagging() {
        let key = MasterKey::from_hex(HEX).expect("valid key");
        let stored: [u8; MASTER_KEY_LEN] = *key.inner.borrow();
        let legacy = {
            let hex_key = Zeroizing::new(hex::encode(stored));
            oc(hex_key.as_str(), "test-device").expect("encode legacy key")
        };
        let protected = oc_bytes(&stored, "test-device").expect("encode protected bytes");
        assert_eq!(hex::encode(&*protected), legacy);

        let decoded = dii_bytes(&protected, "test-device").expect("decode protected bytes");
        assert_eq!(*decoded, stored);
    }

    #[test]
    fn resolve_reuses_concurrently_created_key() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("config/master.key");
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));

        let handles: Vec<_> = (0..2)
            .map(|_| {
                let path = path.clone();
                let barrier = std::sync::Arc::clone(&barrier);
                std::thread::spawn(move || {
                    barrier.wait();
                    let (key, _) = resolve(&path, "test-device").expect("resolve");
                    *key.inner.borrow()
                })
            })
            .collect();

        let keys: Vec<[u8; MASTER_KEY_LEN]> = handles
            .into_iter()
            .map(|handle| handle.join().expect("thread"))
            .collect();

        assert_eq!(keys[0], keys[1]);

        let entries: Vec<_> = fs::read_dir(path.parent().expect("parent"))
            .expect("read config dir")
            .map(|entry| entry.expect("entry").file_name())
            .collect();
        assert_eq!(entries, [std::ffi::OsString::from("master.key")]);
    }

    #[test]
    fn resolve_leaves_invalid_file_untouched() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("master.key");
        fs::write(&path, "not-hex").expect("seed key file");

        assert!(matches!(
            resolve(&path, "test-device"),
            Err(MasterKeyError::KeyFileInvalid { .. })
        ));
        assert_eq!(fs::read_to_string(&path).expect("read key file"), "not-hex");
    }

    #[test]
    fn resolve_requires_device_id() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("master.key");

        assert_eq!(
            resolve(&path, "  ").expect_err("empty device id"),
            MasterKeyError::MissingDeviceId
        );
        assert!(!path.exists());
    }
}
