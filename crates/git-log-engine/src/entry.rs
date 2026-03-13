//! Log entry types: commits and checkpoints.
//!
//! Every entry is a self-contained JSON-LD node. Commits record a single
//! git commit with its diff. Checkpoints snapshot the full file tree at
//! a commit so replay can start from the nearest checkpoint instead of
//! genesis.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A single log entry — either a commit or a periodic checkpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "@type")]
pub enum LogEntry {
    Commit(CommitEntry),
    Checkpoint(CheckpointEntry),
}

impl LogEntry {
    pub fn sequence(&self) -> u64 {
        match self {
            LogEntry::Commit(c) => c.sequence,
            LogEntry::Checkpoint(c) => c.sequence,
        }
    }

    pub fn sha(&self) -> &str {
        match self {
            LogEntry::Commit(c) => &c.sha,
            LogEntry::Checkpoint(c) => &c.at_sha,
        }
    }
}

/// A materialized git commit with its diff and actor metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitEntry {
    /// JSON-LD identifier: `git:sha/{hex}`.
    #[serde(rename = "@id")]
    pub ld_id: String,

    /// Commit SHA hex.
    pub sha: String,

    /// Parent commit SHAs (usually 1; 0 for root, 2+ for merges).
    pub parents: Vec<String>,

    /// Author identity.
    pub author: PersonIdent,

    /// Commit message (without trailer).
    pub message: String,

    /// Commit timestamp.
    pub timestamp: DateTime<Utc>,

    /// Monotonically increasing sequence number within this log.
    pub sequence: u64,

    // ── Actor trailer fields (parsed from commit message) ──

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

    /// File-level changes in this commit.
    pub changes: Vec<FileChange>,
}

/// A point-in-time snapshot of the full file tree.
///
/// Written every N commits (configurable) so replay can skip ahead.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckpointEntry {
    #[serde(rename = "@id")]
    pub ld_id: String,

    /// The commit SHA this checkpoint corresponds to.
    pub at_sha: String,

    /// Sequence number of the corresponding commit entry.
    pub sequence: u64,

    /// Timestamp of the checkpoint.
    pub timestamp: DateTime<Utc>,

    /// Complete file tree: path -> blob SHA.
    pub tree: Vec<TreeFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonIdent {
    pub name: String,
    pub email: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileChange {
    pub path: String,
    pub action: ChangeAction,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blob_sha: Option<String>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TreeFile {
    pub path: String,
    pub blob_sha: String,
}
