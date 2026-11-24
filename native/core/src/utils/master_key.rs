use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;

use jsonwebtoken::{Header, TokenData, Validation};
use zeroize::Zeroizing;

use crate::utils::tagging::{dii, oc};

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
        let encoded = fs::read_to_string(path).map_err(|e| MasterKeyError::KeyFileUnreadable {
            path: path.to_path_buf(),
            reason: e.to_string(),
        })?;
        let encoded = encoded.trim();

        if encoded.is_empty() || hex::decode(encoded).is_err() {
            return Err(MasterKeyError::KeyFileInvalid {
                path: path.to_path_buf(),
                reason: "file is not valid hex".to_string(),
            });
        }

        let key = MasterKey::unprotect(encoded, device_id).map_err(|reason| {
            MasterKeyError::KeyFileInvalid {
                path: path.to_path_buf(),
                reason,
            }
        })?;

        return Ok((key, KeySource::File));
    }

    let key = MasterKey::generate();
    let encoded =
        Zeroizing::new(
            key.protect(device_id)
                .map_err(|reason| MasterKeyError::KeyFileWrite {
                    path: path.to_path_buf(),
                    reason,
                })?,
        );

    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent).map_err(|e| MasterKeyError::KeyFileWrite {
            path: path.to_path_buf(),
            reason: e.to_string(),
        })?;
    }

    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    options.mode(0o600);

    let mut file = options
        .open(path)
        .map_err(|e| MasterKeyError::KeyFileWrite {
            path: path.to_path_buf(),
            reason: e.to_string(),
        })?;

    file.write_all(encoded.as_bytes())
        .map_err(|e| MasterKeyError::KeyFileWrite {
            path: path.to_path_buf(),
            reason: e.to_string(),
        })?;
    file.sync_all().map_err(|e| MasterKeyError::KeyFileWrite {
        path: path.to_path_buf(),
        reason: e.to_string(),
    })?;

    Ok((key, KeySource::Created))
}

/// A master key held in protected memory.
///
/// The absence of `Clone`, `Copy`, `Display`, `Serialize` and `Deserialize` is
/// deliberate: the only way to create a key is [`MasterKey::from_hex`], and the
/// only way to use it is through [`MasterKeySlot`] and [`KeyBytes`].
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

    /// Encodes the key for the on-disk format without exposing a printable form.
    fn protect(&self, device_id: &str) -> Result<String, String> {
        let borrowed = self.inner.borrow();
        let hex_key = Zeroizing::new(hex::encode(&*borrowed));
        oc(hex_key.as_str(), device_id)
    }

    fn unprotect(encoded: &str, device_id: &str) -> Result<Self, String> {
        let hex_key = Zeroizing::new(dii(encoded, device_id)?);
        Self::from_hex(hex_key.as_str()).map_err(|e| e.to_string())
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
}

#[cfg(test)]
mod tests {
    use super::*;

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

        let stored: [u8; MASTER_KEY_LEN] = *key.inner.borrow();
        let encoded = fs::read_to_string(&path).expect("read key file");
        assert!(!encoded.contains(&hex::encode(stored)));
        assert_eq!(
            dii(encoded.trim(), "test-device").expect("decode key file"),
            hex::encode(stored)
        );

        let (reloaded, source) = resolve(&path, "test-device").expect("reload key");
        assert_eq!(source, KeySource::File);
        assert_eq!(*reloaded.inner.borrow(), stored);
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
