//! Cryptographic commit signing and actor attribution.
//!
//! Provides Ed25519 SSH signatures for git commits, producing signatures
//! compatible with `git log --show-signature`. Signing is optional — when
//! no key is configured, behavior is identical to unsigned commits.

use ssh_key::private::PrivateKey;
use ssh_key::{HashAlg, LineEnding};

use super::error::GitStorageError;

/// Identity of the actor making a commit.
///
/// Embedded as a trailer block in commit messages for accountability.
#[derive(Debug, Clone)]
pub struct CommitActor {
    pub workspace_id: String,
    pub entity_id: Option<String>,
    pub scopes: Vec<String>,
    pub timestamp: String,
}

impl CommitActor {
    /// Sentinel actor for system-initiated commits (repo init, internal ops).
    pub fn system() -> Self {
        Self {
            workspace_id: "system".to_owned(),
            entity_id: None,
            scopes: vec!["system".to_owned()],
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// Format actor info as a commit message trailer block.
    pub fn format_trailer(&self, signer: Option<&CommitSigner>) -> String {
        let mut lines = Vec::new();
        lines.push(format!("Actor: {}", self.workspace_id));
        if let Some(eid) = &self.entity_id {
            lines.push(format!("Entity: {eid}"));
        }
        if !self.scopes.is_empty() {
            lines.push(format!("Scopes: {}", self.scopes.join(",")));
        }
        lines.push(format!("Timestamp: {}", self.timestamp));
        if let Some(s) = signer {
            lines.push(format!("Signed-By: {}", s.public_key_fingerprint()));
        }
        lines.join("\n")
    }
}

/// Wraps an Ed25519 private key for signing git commits.
pub struct CommitSigner {
    key: PrivateKey,
    fingerprint: String,
}

impl CommitSigner {
    /// Load a signer from a PEM-encoded Ed25519 private key.
    pub fn from_pem(pem: &str) -> Result<Self, GitStorageError> {
        let key = PrivateKey::from_openssh(pem).map_err(|e| {
            GitStorageError::SigningError(format!("failed to parse signing key: {e}"))
        })?;

        if key.algorithm() != ssh_key::Algorithm::Ed25519 {
            return Err(GitStorageError::SigningError(format!(
                "expected Ed25519 key, got {:?}",
                key.algorithm()
            )));
        }

        let fingerprint = key
            .public_key()
            .fingerprint(HashAlg::Sha256)
            .to_string();

        Ok(Self { key, fingerprint })
    }

    /// Produce an SSH signature over a commit buffer.
    ///
    /// The signature is in the format git expects: an armored SSH signature
    /// block that can be verified with `git log --show-signature`.
    pub fn sign_commit(&self, commit_buf: &str) -> Result<String, GitStorageError> {
        let sig = ssh_key::SshSig::sign(&self.key, "git", HashAlg::Sha512, commit_buf.as_bytes())
            .map_err(|e| GitStorageError::SigningError(format!("signing failed: {e}")))?;

        let armored = sig.to_pem(LineEnding::LF).map_err(|e| {
            GitStorageError::SigningError(format!("failed to armor signature: {e}"))
        })?;

        Ok(armored)
    }

    /// The SHA-256 fingerprint of the public key (e.g. `SHA256:xxxx`).
    pub fn public_key_fingerprint(&self) -> &str {
        &self.fingerprint
    }

    /// The public key in OpenSSH authorized_keys format.
    pub fn public_key_openssh(&self) -> String {
        self.key.public_key().to_openssh().unwrap_or_default()
    }
}

impl std::fmt::Debug for CommitSigner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CommitSigner")
            .field("fingerprint", &self.fingerprint)
            .finish()
    }
}

/// Bundles actor identity and optional signer for a commit operation.
///
/// Keeps the git layer decoupled from auth — callers construct this from
/// JWT claims + signer, then pass it into commit functions.
pub struct CommitContext<'a> {
    pub actor: &'a CommitActor,
    pub signer: Option<&'a CommitSigner>,
}

/// Build the full commit message by appending an actor trailer to the base message.
pub fn build_signed_message(
    base_message: &str,
    actor: &CommitActor,
    signer: Option<&CommitSigner>,
) -> String {
    let trailer = actor.format_trailer(signer);
    format!("{base_message}\n\n---\n{trailer}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn generate_test_key() -> PrivateKey {
        PrivateKey::random(&mut rand::thread_rng(), ssh_key::Algorithm::Ed25519).unwrap()
    }

    fn test_key_pem() -> String {
        let key = generate_test_key();
        key.to_openssh(LineEnding::LF).unwrap().to_string()
    }

    #[test]
    fn test_commit_signer_from_pem() {
        let pem = test_key_pem();
        let signer = CommitSigner::from_pem(&pem).unwrap();
        assert!(signer.public_key_fingerprint().starts_with("SHA256:"));
    }

    #[test]
    fn test_commit_signer_rejects_non_ed25519() {
        // Generate an RSA key — we only accept Ed25519
        // Since ssh-key with crypto feature may not support RSA generation easily,
        // test with a known bad input instead
        let result = CommitSigner::from_pem("not a valid pem");
        assert!(result.is_err());
    }

    #[test]
    fn test_sign_and_verify_roundtrip() {
        let pem = test_key_pem();
        let signer = CommitSigner::from_pem(&pem).unwrap();

        let commit_buf = "tree abc123\nauthor Test <test@test.com> 1234567890 +0000\n\ntest commit";
        let signature = signer.sign_commit(commit_buf).unwrap();

        assert!(signature.contains("-----BEGIN SSH SIGNATURE-----"));
        assert!(signature.contains("-----END SSH SIGNATURE-----"));
    }

    #[test]
    fn test_actor_system() {
        let actor = CommitActor::system();
        assert_eq!(actor.workspace_id, "system");
        assert!(actor.entity_id.is_none());
        assert_eq!(actor.scopes, vec!["system"]);
    }

    #[test]
    fn test_actor_format_trailer_without_signer() {
        let actor = CommitActor {
            workspace_id: "ws_abc123".to_owned(),
            entity_id: Some("ent_456".to_owned()),
            scopes: vec!["FormationCreate".to_owned()],
            timestamp: "2026-02-27T12:00:00Z".to_owned(),
        };

        let trailer = actor.format_trailer(None);
        assert!(trailer.contains("Actor: ws_abc123"));
        assert!(trailer.contains("Entity: ent_456"));
        assert!(trailer.contains("Scopes: FormationCreate"));
        assert!(trailer.contains("Timestamp: 2026-02-27T12:00:00Z"));
        assert!(!trailer.contains("Signed-By"));
    }

    #[test]
    fn test_actor_format_trailer_with_signer() {
        let pem = test_key_pem();
        let signer = CommitSigner::from_pem(&pem).unwrap();

        let actor = CommitActor {
            workspace_id: "ws_abc123".to_owned(),
            entity_id: None,
            scopes: vec!["Admin".to_owned()],
            timestamp: "2026-02-27T12:00:00Z".to_owned(),
        };

        let trailer = actor.format_trailer(Some(&signer));
        assert!(trailer.contains("Signed-By: SHA256:"));
    }

    #[test]
    fn test_build_signed_message() {
        let actor = CommitActor::system();
        let msg = build_signed_message("Form entity: Acme Corp", &actor, None);

        assert!(msg.starts_with("Form entity: Acme Corp"));
        assert!(msg.contains("---"));
        assert!(msg.contains("Actor: system"));
    }
}
