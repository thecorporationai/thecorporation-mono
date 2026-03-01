//! API key generation, storage model, and verification.

use argon2::password_hash::SaltString;
use argon2::password_hash::rand_core::OsRng;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use chrono::{DateTime, Utc};
use rand::Rng;
use serde::{Deserialize, Serialize};

use super::error::AuthError;
use super::scopes::ScopeSet;
use crate::domain::ids::{ApiKeyId, ContactId, EntityId, WorkspaceId};

/// Prefix for all API keys.
const API_KEY_PREFIX: &str = "sk_";

/// Length of the random portion of the key (in bytes, hex-encoded).
const KEY_RANDOM_BYTES: usize = 32;

/// A stored API key record. The raw key is never persisted — only the hash.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyRecord {
    key_id: ApiKeyId,
    workspace_id: WorkspaceId,
    name: String,
    key_hash: String,
    scopes: ScopeSet,
    /// When set, this key is scoped to a specific contact. `None` = workspace-wide key.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    contact_id: Option<ContactId>,
    /// When set, restricts the key to specific entities. `None` = all entities.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    entity_ids: Option<Vec<EntityId>>,
    created_at: DateTime<Utc>,
    expires_at: Option<DateTime<Utc>>,
    revoked_at: Option<DateTime<Utc>>,
}

impl ApiKeyRecord {
    /// Returns `true` if the key is not revoked and not expired.
    pub fn is_valid(&self) -> bool {
        if self.revoked_at.is_some() {
            return false;
        }
        if let Some(expires) = self.expires_at {
            if Utc::now() > expires {
                return false;
            }
        }
        true
    }

    /// Revoke this key, setting the revoked timestamp to now.
    pub fn revoke(&mut self) {
        self.revoked_at = Some(Utc::now());
    }

    // ── Accessors ────────────────────────────────────────────────────────

    pub fn key_id(&self) -> ApiKeyId {
        self.key_id
    }

    pub fn workspace_id(&self) -> WorkspaceId {
        self.workspace_id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn key_hash(&self) -> &str {
        &self.key_hash
    }

    pub fn scopes(&self) -> &ScopeSet {
        &self.scopes
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    pub fn expires_at(&self) -> Option<DateTime<Utc>> {
        self.expires_at
    }

    pub fn revoked_at(&self) -> Option<DateTime<Utc>> {
        self.revoked_at
    }

    pub fn contact_id(&self) -> Option<ContactId> {
        self.contact_id
    }

    pub fn entity_ids(&self) -> Option<&[EntityId]> {
        self.entity_ids.as_deref()
    }
}

/// Generate a new API key, returning the raw key string and the storable record.
///
/// The raw key has the format `sk_<64 hex chars>`. The record contains the
/// argon2 hash — never store the raw key.
pub fn generate_api_key(
    workspace_id: WorkspaceId,
    name: String,
    scopes: ScopeSet,
    expires_at: Option<DateTime<Utc>>,
    contact_id: Option<ContactId>,
    entity_ids: Option<Vec<EntityId>>,
) -> Result<(String, ApiKeyRecord), AuthError> {
    let mut rng = rand::thread_rng();
    let random_bytes: Vec<u8> = (0..KEY_RANDOM_BYTES).map(|_| rng.r#gen()).collect();
    let raw_key = format!("{API_KEY_PREFIX}{}", hex::encode(&random_bytes));

    let key_hash = hash_key(&raw_key)?;

    let record = ApiKeyRecord {
        key_id: ApiKeyId::new(),
        workspace_id,
        name,
        key_hash,
        scopes,
        contact_id,
        entity_ids,
        created_at: Utc::now(),
        expires_at,
        revoked_at: None,
    };

    Ok((raw_key, record))
}

/// Verify a raw API key against a stored argon2 hash.
pub fn verify_api_key(raw_key: &str, hash: &str) -> Result<bool, AuthError> {
    let parsed = PasswordHash::new(hash)
        .map_err(|e| AuthError::InvalidToken(format!("corrupt key hash: {e}")))?;
    Ok(Argon2::default()
        .verify_password(raw_key.as_bytes(), &parsed)
        .is_ok())
}

/// Hash a raw key with argon2 for storage.
fn hash_key(raw_key: &str) -> Result<String, AuthError> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2
        .hash_password(raw_key.as_bytes(), &salt)
        .map_err(|e| AuthError::InvalidToken(format!("hashing failed: {e}")))?;
    Ok(hash.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::auth::scopes::Scope;

    #[test]
    fn generated_key_has_sk_prefix() {
        let ws = WorkspaceId::new();
        let scopes = ScopeSet::from_vec(vec![Scope::All]);
        let (raw, _record) = generate_api_key(ws, "test-key".into(), scopes, None, None, None)
            .expect("generate key");
        assert!(raw.starts_with("sk_"));
    }

    #[test]
    fn generated_key_has_correct_length() {
        let ws = WorkspaceId::new();
        let scopes = ScopeSet::from_vec(vec![Scope::All]);
        let (raw, _record) = generate_api_key(ws, "test-key".into(), scopes, None, None, None)
            .expect("generate key");
        // "sk_" (3) + 64 hex chars = 67
        assert_eq!(raw.len(), 3 + KEY_RANDOM_BYTES * 2);
    }

    #[test]
    fn verify_correct_key() {
        let ws = WorkspaceId::new();
        let scopes = ScopeSet::from_vec(vec![Scope::FormationCreate]);
        let (raw, record) = generate_api_key(ws, "test-key".into(), scopes, None, None, None)
            .expect("generate key");
        let ok = verify_api_key(&raw, record.key_hash()).expect("verify key");
        assert!(ok);
    }

    #[test]
    fn verify_wrong_key_fails() {
        let ws = WorkspaceId::new();
        let scopes = ScopeSet::from_vec(vec![Scope::FormationCreate]);
        let (_raw, record) = generate_api_key(ws, "test-key".into(), scopes, None, None, None)
            .expect("generate key");
        let ok = verify_api_key("sk_wrong", record.key_hash()).expect("verify key");
        assert!(!ok);
    }

    #[test]
    fn new_key_is_valid() {
        let ws = WorkspaceId::new();
        let scopes = ScopeSet::from_vec(vec![Scope::All]);
        let (_raw, record) = generate_api_key(ws, "test-key".into(), scopes, None, None, None)
            .expect("generate key");
        assert!(record.is_valid());
    }

    #[test]
    fn revoked_key_is_not_valid() {
        let ws = WorkspaceId::new();
        let scopes = ScopeSet::from_vec(vec![Scope::All]);
        let (_raw, mut record) = generate_api_key(ws, "test-key".into(), scopes, None, None, None)
            .expect("generate key");
        record.revoke();
        assert!(!record.is_valid());
    }

    #[test]
    fn expired_key_is_not_valid() {
        let ws = WorkspaceId::new();
        let scopes = ScopeSet::from_vec(vec![Scope::All]);
        let past = Utc::now() - chrono::Duration::hours(1);
        let (_raw, record) =
            generate_api_key(ws, "test-key".into(), scopes, Some(past), None, None)
                .expect("generate key");
        assert!(!record.is_valid());
    }

    #[test]
    fn record_accessors() {
        let ws = WorkspaceId::new();
        let scopes = ScopeSet::from_vec(vec![Scope::Admin]);
        let (_raw, record) =
            generate_api_key(ws, "my-key".into(), scopes, None, None, None).expect("generate key");
        assert_eq!(record.workspace_id(), ws);
        assert_eq!(record.name(), "my-key");
        assert!(record.scopes().has(Scope::Admin));
        assert!(record.expires_at().is_none());
        assert!(record.revoked_at().is_none());
    }
}
