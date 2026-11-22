use std::fmt;
use std::ops::Deref;
use std::sync::{Mutex, MutexGuard};

/// Length of a master key in bytes.
pub const MASTER_KEY_LEN: usize = 32;

/// Errors returned when a master key cannot be created or read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MasterKeyError {
    InvalidHex,
    InvalidLength,
    NotSet,
    AlreadySet,
}

impl fmt::Display for MasterKeyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidHex => f.write_str("master key is not valid hex"),
            Self::InvalidLength => f.write_str("master key must be 64 hex characters"),
            Self::NotSet => f.write_str("master key is not set"),
            Self::AlreadySet => f.write_str("master key is already set"),
        }
    }
}

impl std::error::Error for MasterKeyError {}

/// A master key held in protected memory.
///
/// The absence of `Clone`, `Copy`, `Display`, `Serialize` and `Deserialize` is
/// deliberate: the only way to create a key is [`MasterKey::from_hex`], and the
/// only way to use it is through [`MasterKeySlot`].
pub struct MasterKey {
    inner: secrets::SecretBox<[u8; MASTER_KEY_LEN]>,
}

impl MasterKey {
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
/// request.
pub struct KeyBytes {
    bytes: zeroize::Zeroizing<[u8; MASTER_KEY_LEN]>,
}

impl KeyBytes {
    fn new(bytes: [u8; MASTER_KEY_LEN]) -> Self {
        Self {
            bytes: zeroize::Zeroizing::new(bytes),
        }
    }

    pub fn as_array(&self) -> &[u8; MASTER_KEY_LEN] {
        &self.bytes
    }
}

impl Deref for KeyBytes {
    type Target = [u8; MASTER_KEY_LEN];

    fn deref(&self) -> &Self::Target {
        &self.bytes
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

    /// Runs a synchronous closure with a borrow of the key.
    pub fn with<R>(&self, f: impl FnOnce(&[u8; MASTER_KEY_LEN]) -> R) -> Result<R, MasterKeyError> {
        let slot = self.lock();
        let key = slot.as_ref().ok_or(MasterKeyError::NotSet)?;
        let borrowed = key.inner.borrow();
        Ok(f(&borrowed))
    }

    /// Returns a zeroized-on-drop copy for use across an `await`.
    pub fn bytes(&self) -> Result<KeyBytes, MasterKeyError> {
        self.with(|key| KeyBytes::new(*key))
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
        assert_eq!(bytes.bytes().expect("key is set").as_array()[0], 0x00);
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
    fn slot_with_and_bytes_agree() {
        let slot = MasterKeySlot::new();
        slot.set(MasterKey::from_hex(HEX).expect("valid key"))
            .expect("slot is empty");

        let borrowed = slot.with(|key| *key).expect("key is set");
        let owned = slot.bytes().expect("key is set");
        assert_eq!(borrowed, *owned.as_array());
    }
}
