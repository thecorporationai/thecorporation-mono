//! SSH public key storage and in-memory fingerprint index.
//!
//! Users register SSH public keys via the API. The keys are stored in the
//! workspace git repo at `ssh-keys/{key_id}.json`. An in-memory fingerprint
//! index provides O(1) lookup during SSH authentication.

use std::collections::HashMap;
use std::sync::RwLock;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ssh_key::PublicKey;

use super::scopes::ScopeSet;
use crate::domain::ids::{ContactId, EntityId, SshKeyId, WorkspaceId};
use crate::store::RepoLayout;

/// A stored SSH public key record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshKeyRecord {
    pub key_id: SshKeyId,
    pub workspace_id: WorkspaceId,
    pub name: String,
    /// Full public key in OpenSSH authorized_keys format.
    pub public_key_openssh: String,
    /// SHA-256 fingerprint (e.g., `"SHA256:abc..."`).
    pub fingerprint: String,
    /// Key algorithm (e.g., `"ssh-ed25519"`, `"ssh-rsa"`).
    pub algorithm: String,
    pub scopes: ScopeSet,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entity_ids: Option<Vec<EntityId>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contact_id: Option<ContactId>,
    pub created_at: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revoked_at: Option<DateTime<Utc>>,
}

impl SshKeyRecord {
    /// Returns `true` if the key is not revoked.
    pub fn is_valid(&self) -> bool {
        self.revoked_at.is_none()
    }

    /// Revoke this key.
    pub fn revoke(&mut self) {
        self.revoked_at = Some(Utc::now());
    }
}

/// Parse an OpenSSH public key string, returning the fingerprint and algorithm.
pub fn parse_public_key(openssh_str: &str) -> Result<(String, String), String> {
    let key = PublicKey::from_openssh(openssh_str)
        .map_err(|e| format!("invalid SSH public key: {e}"))?;
    let fingerprint = key.fingerprint(ssh_key::HashAlg::Sha256).to_string();
    let algorithm = key.algorithm().to_string();
    Ok((fingerprint, algorithm))
}

// ── In-memory fingerprint index ───────────────────────────────────────

/// Result of looking up a fingerprint in the index.
#[derive(Debug, Clone)]
pub struct SshKeyLookup {
    pub workspace_id: WorkspaceId,
    pub key_id: SshKeyId,
    pub scopes: ScopeSet,
    pub entity_ids: Option<Vec<EntityId>>,
    pub contact_id: Option<ContactId>,
}

/// Thread-safe in-memory index mapping SHA-256 fingerprints to key metadata.
pub struct SshKeyIndex {
    inner: RwLock<HashMap<String, SshKeyLookup>>,
}

impl SshKeyIndex {
    /// Build the index by scanning all workspace repos.
    pub fn build(layout: &RepoLayout, valkey_client: Option<&redis::Client>) -> Self {
        use crate::store::workspace_store::WorkspaceStore;

        let mut map = HashMap::new();

        let (workspace_ids, shared_con) =
            match WorkspaceStore::list_and_prepare(layout, valkey_client) {
                Ok(pair) => pair,
                Err(e) => {
                    tracing::warn!("failed to list workspaces for SSH key index: {e}");
                    return Self {
                        inner: RwLock::new(map),
                    };
                }
            };

        for ws_id in workspace_ids {
            let ws_store =
                match WorkspaceStore::open_shared(layout, ws_id, shared_con.clone()) {
                    Ok(s) => s,
                    Err(_) => continue,
                };

            let key_ids: Vec<SshKeyId> = match ws_store.list_ids_in_dir_pub("ssh-keys") {
                Ok(ids) => ids,
                Err(_) => continue,
            };

            for key_id in key_ids {
                let record: SshKeyRecord = match ws_store.read_json(&format!("ssh-keys/{key_id}.json")) {
                    Ok(r) => r,
                    Err(_) => continue,
                };
                if !record.is_valid() {
                    continue;
                }
                map.insert(
                    record.fingerprint.clone(),
                    SshKeyLookup {
                        workspace_id: record.workspace_id,
                        key_id: record.key_id,
                        scopes: record.scopes.clone(),
                        entity_ids: record.entity_ids.clone(),
                        contact_id: record.contact_id,
                    },
                );
            }
        }

        tracing::info!(count = map.len(), "SSH key index built");
        Self {
            inner: RwLock::new(map),
        }
    }

    /// Create an empty index (for testing or when SSH is not configured).
    pub fn empty() -> Self {
        Self {
            inner: RwLock::new(HashMap::new()),
        }
    }

    /// Look up a key by SHA-256 fingerprint.
    pub fn lookup(&self, fingerprint: &str) -> Option<SshKeyLookup> {
        self.inner.read().ok()?.get(fingerprint).cloned()
    }

    /// Insert a key into the index.
    pub fn insert(&self, fingerprint: String, entry: SshKeyLookup) {
        if let Ok(mut map) = self.inner.write() {
            map.insert(fingerprint, entry);
        }
    }

    /// Remove a key from the index by fingerprint.
    pub fn remove(&self, fingerprint: &str) {
        if let Ok(mut map) = self.inner.write() {
            map.remove(fingerprint);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::auth::scopes::Scope;

    #[test]
    fn ssh_key_record_serde_roundtrip() {
        let record = SshKeyRecord {
            key_id: SshKeyId::new(),
            workspace_id: WorkspaceId::new(),
            name: "test-key".to_owned(),
            public_key_openssh: "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAITest test@host".to_owned(),
            fingerprint: "SHA256:abc123".to_owned(),
            algorithm: "ssh-ed25519".to_owned(),
            scopes: ScopeSet::from_vec(vec![Scope::GitRead, Scope::GitWrite]),
            entity_ids: None,
            contact_id: None,
            created_at: Utc::now(),
            revoked_at: None,
        };
        let json = serde_json::to_string(&record).expect("serialize");
        let parsed: SshKeyRecord = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed.key_id, record.key_id);
        assert_eq!(parsed.fingerprint, record.fingerprint);
        assert!(parsed.is_valid());
    }

    #[test]
    fn revoked_key_is_not_valid() {
        let mut record = SshKeyRecord {
            key_id: SshKeyId::new(),
            workspace_id: WorkspaceId::new(),
            name: "test".to_owned(),
            public_key_openssh: String::new(),
            fingerprint: String::new(),
            algorithm: String::new(),
            scopes: ScopeSet::empty(),
            entity_ids: None,
            contact_id: None,
            created_at: Utc::now(),
            revoked_at: None,
        };
        assert!(record.is_valid());
        record.revoke();
        assert!(!record.is_valid());
    }

    #[test]
    fn index_insert_lookup_remove() {
        let index = SshKeyIndex::empty();
        let fp = "SHA256:test123".to_owned();
        let ws = WorkspaceId::new();
        let kid = SshKeyId::new();

        assert!(index.lookup(&fp).is_none());

        index.insert(
            fp.clone(),
            SshKeyLookup {
                workspace_id: ws,
                key_id: kid,
                scopes: ScopeSet::from_vec(vec![Scope::GitRead]),
                entity_ids: None,
                contact_id: None,
            },
        );

        let entry = index.lookup(&fp).expect("should find key");
        assert_eq!(entry.workspace_id, ws);
        assert_eq!(entry.key_id, kid);

        index.remove(&fp);
        assert!(index.lookup(&fp).is_none());
    }
}
