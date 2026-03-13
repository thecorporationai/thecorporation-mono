//! Commit log entry types.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A materialized commit entry stored in the Valkey sorted set.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitEntry {
    /// JSON-LD type.
    #[serde(rename = "@type")]
    pub ld_type: String,

    /// JSON-LD identifier: `git:sha/{sha1_hex}`.
    #[serde(rename = "@id")]
    pub ld_id: String,

    /// Git-compatible SHA-1 hex.
    pub sha1: String,

    /// SHA-256 hex (primary internal key).
    pub sha256: String,

    /// Parent commit SHA-1s.
    pub parents: Vec<String>,

    /// Author name.
    pub author_name: String,

    /// Author email.
    pub author_email: String,

    /// Commit message (without actor trailer).
    pub message: String,

    /// Commit timestamp.
    pub timestamp: DateTime<Utc>,

    /// Sequence number in this entity's log.
    pub sequence: u64,

    /// Workspace that made this commit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_id: Option<String>,

    /// Entity this commit targets.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_id: Option<String>,

    /// Authorization scopes.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub scopes: Vec<String>,

    /// Signing key fingerprint.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signed_by: Option<String>,

    /// SHA-1 of the root tree object.
    pub tree_sha1: String,

    /// File-level changes.
    pub changes: Vec<FileChange>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileChange {
    pub path: String,
    pub action: ChangeAction,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blob_sha1: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blob_sha256: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub old_path: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChangeAction {
    Add,
    Modify,
    Delete,
    Rename,
}

/// Actor identity for commit attribution.
#[derive(Debug, Clone)]
pub struct CommitActor {
    pub workspace_id: String,
    pub entity_id: Option<String>,
    pub scopes: Vec<String>,
    pub signed_by: Option<String>,
}

/// A file to write in a commit.
pub struct FileWrite {
    pub path: String,
    pub content: Vec<u8>,
}

impl FileWrite {
    pub fn new(path: impl Into<String>, content: Vec<u8>) -> Self {
        Self {
            path: path.into(),
            content,
        }
    }

    pub fn json<T: serde::Serialize>(
        path: impl Into<String>,
        value: &T,
    ) -> Result<Self, serde_json::Error> {
        let content = serde_json::to_vec_pretty(value)?;
        Ok(Self::new(path, content))
    }
}
