//! Configuration management for the `corp` CLI.
//!
//! Settings are stored in `~/.corp/config.json` (overridable via
//! `$CORP_CONFIG_DIR`).  Environment variables always take precedence over
//! persisted values.

use anyhow::{Context, bail};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// ── Config struct ─────────────────────────────────────────────────────────────

/// Persisted CLI configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    /// Base URL for the Corp API.
    pub api_url: Option<String>,
    /// API key used for authentication.
    pub api_key: Option<String>,
    /// Active workspace ID.
    pub workspace_id: Option<String>,
    /// Default entity ID used when none is supplied on the command line.
    pub active_entity_id: Option<String>,
}

// ── Default API URL ───────────────────────────────────────────────────────────

const DEFAULT_API_URL: &str = "https://api.thecorporation.com";

// ── Config impl ───────────────────────────────────────────────────────────────

impl Config {
    // ── Persistence ──────────────────────────────────────────────────────────

    /// Load config from `~/.corp/config.json` (or `$CORP_CONFIG_DIR/config.json`).
    ///
    /// If the file does not exist a default (all-`None`) config is returned.
    pub fn load() -> anyhow::Result<Self> {
        let path = Self::path();
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw = std::fs::read_to_string(&path)
            .with_context(|| format!("reading config file {}", path.display()))?;
        let cfg: Self = serde_json::from_str(&raw)
            .with_context(|| format!("parsing config file {}", path.display()))?;
        Ok(cfg)
    }

    /// Persist the config to disk, creating parent directories as needed.
    pub fn save(&self) -> anyhow::Result<()> {
        let path = Self::path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating config directory {}", parent.display()))?;
        }
        let json = serde_json::to_string_pretty(self).context("serializing config")?;
        std::fs::write(&path, json)
            .with_context(|| format!("writing config file {}", path.display()))?;
        Ok(())
    }

    /// Absolute path to the config file.
    ///
    /// Respects `$CORP_CONFIG_DIR`; falls back to `~/.corp`.
    pub fn path() -> PathBuf {
        let dir = if let Ok(d) = std::env::var("CORP_CONFIG_DIR") {
            PathBuf::from(d)
        } else {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".corp")
        };
        dir.join("config.json")
    }

    // ── Key/value accessors ───────────────────────────────────────────────────

    /// Set a config value by key name.
    ///
    /// Recognized keys: `api_url`, `api_key`, `workspace_id`, `active_entity_id`.
    pub fn set(&mut self, key: &str, value: &str) -> anyhow::Result<()> {
        let v = value.to_owned();
        match key {
            "api_url" => self.api_url = Some(v),
            "api_key" => self.api_key = Some(v),
            "workspace_id" => self.workspace_id = Some(v),
            "active_entity_id" => self.active_entity_id = Some(v),
            other => bail!(
                "unknown config key {:?}; valid keys: api_url, api_key, workspace_id, active_entity_id",
                other
            ),
        }
        Ok(())
    }

    /// Get a config value by key name, returning `None` if unset.
    pub fn get(&self, key: &str) -> Option<String> {
        match key {
            "api_url" => self.api_url.clone(),
            "api_key" => self.api_key.clone(),
            "workspace_id" => self.workspace_id.clone(),
            "active_entity_id" => self.active_entity_id.clone(),
            _ => None,
        }
    }

    // ── Effective values (env > config > default) ─────────────────────────────

    /// Effective API URL: `$CORP_API_URL` > persisted value > built-in default.
    pub fn effective_api_url(&self) -> String {
        std::env::var("CORP_API_URL")
            .ok()
            .or_else(|| self.api_url.clone())
            .unwrap_or_else(|| DEFAULT_API_URL.to_owned())
    }

    /// Effective API key: `$CORP_API_KEY` > persisted value.
    pub fn effective_api_key(&self) -> Option<String> {
        std::env::var("CORP_API_KEY")
            .ok()
            .or_else(|| self.api_key.clone())
    }

    // ── Helpers ───────────────────────────────────────────────────────────────

    /// Return an iterator over `(key, display_value)` pairs for all known keys.
    ///
    /// API keys are masked to protect secrets.
    pub fn display_fields(&self) -> Vec<(&'static str, String)> {
        vec![
            (
                "api_url",
                self.api_url.clone().unwrap_or_else(|| "<unset>".into()),
            ),
            (
                "api_key",
                self.api_key
                    .as_deref()
                    .map(mask_secret)
                    .unwrap_or_else(|| "<unset>".into()),
            ),
            (
                "workspace_id",
                self.workspace_id
                    .clone()
                    .unwrap_or_else(|| "<unset>".into()),
            ),
            (
                "active_entity_id",
                self.active_entity_id
                    .clone()
                    .unwrap_or_else(|| "<unset>".into()),
            ),
        ]
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Mask a secret string, showing only the first 8 chars followed by `…`.
fn mask_secret(s: &str) -> String {
    if s.len() <= 8 {
        "***".to_owned()
    } else {
        format!("{}…", &s[..8])
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_and_get_roundtrip() {
        let mut cfg = Config::default();
        cfg.set("api_url", "https://example.com").unwrap();
        assert_eq!(cfg.get("api_url").as_deref(), Some("https://example.com"));
    }

    #[test]
    fn set_unknown_key_errors() {
        let mut cfg = Config::default();
        assert!(cfg.set("nonsense", "value").is_err());
    }

    #[test]
    fn effective_api_url_env_override_and_default() {
        // This test mutates env vars so it must be a single test to avoid races.
        unsafe { std::env::set_var("CORP_API_URL", "http://localhost:9999"); }
        let cfg = Config::default();
        assert_eq!(cfg.effective_api_url(), "http://localhost:9999");

        unsafe { std::env::remove_var("CORP_API_URL"); }
        let cfg2 = Config::default();
        assert_eq!(cfg2.effective_api_url(), DEFAULT_API_URL);
    }

    #[test]
    fn save_and_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        unsafe { std::env::set_var("CORP_CONFIG_DIR", dir.path().to_str().unwrap()); }

        let mut cfg = Config::default();
        cfg.set("workspace_id", "ws-123").unwrap();
        cfg.save().unwrap();

        let loaded = Config::load().unwrap();
        assert_eq!(loaded.workspace_id.as_deref(), Some("ws-123"));

        unsafe { std::env::remove_var("CORP_CONFIG_DIR"); }
    }

    #[test]
    fn load_nonexistent_returns_default() {
        let dir = tempfile::tempdir().unwrap();
        unsafe {
            std::env::set_var(
                "CORP_CONFIG_DIR",
                dir.path().join("nonexistent").to_str().unwrap(),
            );
        }
        let cfg = Config::load().unwrap();
        assert!(cfg.api_url.is_none());
        unsafe { std::env::remove_var("CORP_CONFIG_DIR"); }
    }

    #[test]
    fn mask_secret_short() {
        assert_eq!(mask_secret("abc"), "***");
    }

    #[test]
    fn mask_secret_long() {
        let s = mask_secret("corp_live_abcdef1234567890");
        assert!(s.ends_with('…'));
        assert_eq!(&s[..8], "corp_live_");
    }
}
