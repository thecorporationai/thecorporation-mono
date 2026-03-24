//! API key generation and verification.
//!
//! Keys use a `corp_` prefix followed by 32 cryptographically random
//! bytes encoded as URL-safe base64.  Hashing uses Argon2id with default
//! parameters; verification is performed through the Argon2 library's own
//! constant-time comparison so there are no timing side channels.

use argon2::Argon2;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use rand::RngCore;
use rand::rngs::OsRng;

use crate::error::AuthError;

const KEY_PREFIX: &str = "corp_";
const KEY_RANDOM_BYTES: usize = 32;

/// Stateless helper for generating and verifying API keys.
///
/// All methods are associated functions (no `self`) — construct a unit value
/// `ApiKeyManager` or call the methods directly on the type.
pub struct ApiKeyManager;

impl ApiKeyManager {
    /// Generate a fresh API key.
    ///
    /// Returns `(raw_key, hash)` where:
    /// - `raw_key` is the value shown to the user **once** (store it nowhere).
    /// - `hash` is the Argon2id PHC string to store in the database.
    pub fn generate() -> (String, String) {
        let mut bytes = [0u8; KEY_RANDOM_BYTES];
        OsRng.fill_bytes(&mut bytes);

        let encoded = base64_url_encode(&bytes);
        let raw_key = format!("{KEY_PREFIX}{encoded}");

        let hash =
            Self::hash(&raw_key).expect("argon2 hash of freshly-generated key should never fail");

        (raw_key, hash)
    }

    /// Hash a raw API key using Argon2id.
    pub fn hash(raw_key: &str) -> Result<String, AuthError> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();

        argon2
            .hash_password(raw_key.as_bytes(), &salt)
            .map(|h| h.to_string())
            .map_err(|e| AuthError::InternalError(format!("argon2 hash: {e}")))
    }

    /// Verify `raw_key` against a stored PHC hash string.
    ///
    /// Returns `Ok(true)` if the key matches, `Ok(false)` if it does not.
    pub fn verify(raw_key: &str, hash: &str) -> Result<bool, AuthError> {
        let parsed = PasswordHash::new(hash)
            .map_err(|e| AuthError::InternalError(format!("argon2 parse hash: {e}")))?;

        match Argon2::default().verify_password(raw_key.as_bytes(), &parsed) {
            Ok(()) => Ok(true),
            Err(argon2::password_hash::Error::Password) => Ok(false),
            Err(e) => Err(AuthError::InternalError(format!("argon2 verify: {e}"))),
        }
    }

    /// Return the known prefix if the key starts with `corp_`, or `None`.
    pub fn parse_key_prefix(raw_key: &str) -> Option<&str> {
        if raw_key.starts_with(KEY_PREFIX) {
            Some(KEY_PREFIX)
        } else {
            None
        }
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn base64_url_encode(bytes: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

    let mut out = String::with_capacity((bytes.len() * 4).div_ceil(3));
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0] as usize;
        let b1 = if chunk.len() > 1 { chunk[1] as usize } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as usize } else { 0 };

        out.push(ALPHABET[b0 >> 2] as char);
        out.push(ALPHABET[((b0 & 0x3) << 4) | (b1 >> 4)] as char);
        if chunk.len() > 1 {
            out.push(ALPHABET[((b1 & 0xf) << 2) | (b2 >> 6)] as char);
        }
        if chunk.len() > 2 {
            out.push(ALPHABET[b2 & 0x3f] as char);
        }
    }
    out
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_key_has_correct_prefix() {
        let (raw, _hash) = ApiKeyManager::generate();
        assert!(raw.starts_with("corp_"), "key = {raw}");
    }

    #[test]
    fn generated_key_has_sufficient_length() {
        let (raw, _hash) = ApiKeyManager::generate();
        // prefix (5) + 32 bytes as base64 ≈ 43 chars → total ≥ 47
        assert!(raw.len() >= 47, "key too short: {raw}");
    }

    #[test]
    fn verify_correct_key() {
        let (raw, hash) = ApiKeyManager::generate();
        assert!(ApiKeyManager::verify(&raw, &hash).unwrap());
    }

    #[test]
    fn verify_wrong_key_returns_false() {
        let (_raw, hash) = ApiKeyManager::generate();
        let wrong = "corp_thisisnottheoriginalkey0000000000000000";
        assert!(!ApiKeyManager::verify(wrong, &hash).unwrap());
    }

    #[test]
    fn hash_then_verify_roundtrip() {
        let key = "corp_test_roundtrip_key_value_here_1234567890";
        let hash = ApiKeyManager::hash(key).unwrap();
        assert!(ApiKeyManager::verify(key, &hash).unwrap());
        assert!(!ApiKeyManager::verify("corp_different", &hash).unwrap());
    }

    #[test]
    fn parse_key_prefix_valid() {
        let (raw, _) = ApiKeyManager::generate();
        assert_eq!(ApiKeyManager::parse_key_prefix(&raw), Some("corp_"));
    }

    #[test]
    fn parse_key_prefix_invalid() {
        assert_eq!(ApiKeyManager::parse_key_prefix("not_a_key"), None);
        assert_eq!(ApiKeyManager::parse_key_prefix("xyz_abc"), None);
    }
}
