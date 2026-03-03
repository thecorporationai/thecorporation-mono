use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct Config {
    /// Root directory for git repo storage.
    /// Each workspace gets a subdirectory, each entity gets a bare repo within.
    #[serde(default = "default_data_dir")]
    pub data_dir: PathBuf,

    #[serde(default = "default_port")]
    pub port: u16,

    #[serde(default)]
    pub redis_url: Option<String>,

    #[serde(default)]
    pub jwt_private_key_pem: Option<String>,

    #[serde(default)]
    pub jwt_public_key_pem: Option<String>,

    /// PEM-encoded Ed25519 private key for signing git commits.
    /// When absent, commits are unsigned (backward-compatible).
    #[serde(default)]
    pub commit_signing_key: Option<String>,
}

fn default_data_dir() -> PathBuf {
    PathBuf::from("./data/repos")
}

fn default_port() -> u16 {
    8000
}

impl Config {
    pub fn from_env() -> Result<Self, envy::Error> {
        envy::from_env()
    }
}
