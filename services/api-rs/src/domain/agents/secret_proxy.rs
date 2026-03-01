//! Secret proxy configuration (stored in workspace git repo).
//!
//! Git layout:
//! ```text
//! secrets/<proxy_name>/config.json     # SecretProxyConfig
//! secrets/<proxy_name>/secrets.json    # { "KEY_NAME": "<fernet_encrypted>", ... }
//! ```

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Proxy configuration committed to the workspace repo.
///
/// `url` can be:
/// - `"self"` — secrets are stored locally (encrypted in the same git repo)
/// - An external URL — the worker forwards secret resolution there
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretProxyConfig {
    pub name: String,
    pub url: String,
    #[serde(default)]
    pub description: Option<String>,
    pub created_at: String,
}

/// Encrypted secret values stored in git.
///
/// Each value is a Fernet token (base64-encoded, authenticated, encrypted).
/// The server-side `SECRETS_MASTER_KEY` is needed to decrypt.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EncryptedSecrets {
    #[serde(flatten)]
    pub entries: HashMap<String, String>,
}

impl SecretProxyConfig {
    pub fn new(name: String, url: String, description: Option<String>) -> Self {
        Self {
            name,
            url,
            description,
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// Returns `true` if secrets are stored locally in git.
    pub fn is_self(&self) -> bool {
        self.url == "self"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_serde_roundtrip() {
        let config = SecretProxyConfig::new(
            "openrouter".to_owned(),
            "self".to_owned(),
            Some("OpenRouter API keys".to_owned()),
        );
        let json = serde_json::to_string(&config).unwrap();
        let parsed: SecretProxyConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, "openrouter");
        assert!(parsed.is_self());
    }

    #[test]
    fn encrypted_secrets_serde() {
        let mut secrets = EncryptedSecrets::default();
        secrets
            .entries
            .insert("API_KEY".to_owned(), "gAAAAA...".to_owned());
        secrets
            .entries
            .insert("OTHER".to_owned(), "gAAAAB...".to_owned());

        let json = serde_json::to_string(&secrets).unwrap();
        let parsed: EncryptedSecrets = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.entries.len(), 2);
        assert_eq!(parsed.entries["API_KEY"], "gAAAAA...");
    }

    #[test]
    fn external_proxy_url() {
        let config = SecretProxyConfig::new(
            "custom".to_owned(),
            "https://vault.example.com/resolve".to_owned(),
            None,
        );
        assert!(!config.is_self());
    }
}
