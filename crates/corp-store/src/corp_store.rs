//! `CorpStore` — unified store with transparent S3 durability.
//!
//! Wraps a Redis-protocol connection and an optional `DurableBackend`.
//! When a durable backend is present:
//! - **Writes** go to S3 first (durability point), then update KV indexes
//! - **Blob reads** try KV cache first, fall back to S3 on miss
//! - **Index reads** (tree, refs, commits) always go to KV (rebuilt from S3 if stale)
//!
//! When no durable backend is present, everything goes through KV only
//! (same behavior as before).

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use redis::{Commands, ConnectionLike};

use std::sync::Arc;

use crate::durable::DurableBackend;
use crate::entry::*;
use crate::error::StoreError;
use crate::keys;
use crate::oid::DualOid;
use crate::store::{self, GitObjectType};

// ── CorpStore ────────────────────────────────────────────────────────

/// Unified store with transparent durability.
pub struct CorpStore<C: ConnectionLike> {
    con: C,
    ws: String,
    ent: String,
    durable: Option<Arc<dyn DurableBackend + Send + Sync>>,
}

impl<C: ConnectionLike> CorpStore<C> {
    /// Create a KV-only store (no durable backend).
    pub fn new(con: C, ws: impl Into<String>, ent: impl Into<String>) -> Self {
        Self {
            con,
            ws: ws.into(),
            ent: ent.into(),
            durable: None,
        }
    }

    /// Create a store with a durable backend.
    pub fn with_durable(
        con: C,
        ws: impl Into<String>,
        ent: impl Into<String>,
        backend: Arc<dyn DurableBackend + Send + Sync>,
    ) -> Self {
        Self {
            con,
            ws: ws.into(),
            ent: ent.into(),
            durable: Some(backend),
        }
    }

    /// Whether a durable backend is attached.
    pub fn is_durable(&self) -> bool {
        self.durable.is_some()
    }

    /// Borrow the raw connection (for operations not yet on CorpStore).
    pub fn con(&mut self) -> &mut C {
        &mut self.con
    }

    /// Workspace ID.
    pub fn ws(&self) -> &str {
        &self.ws
    }

    /// Entity ID.
    pub fn ent(&self) -> &str {
        &self.ent
    }

    // ── Write operations ─────────────────────────────────────────

    /// Commit one or more files atomically.
    ///
    /// When a durable backend is present, data goes to S3 first (phase 1),
    /// then KV indexes are updated (phase 2). The commit is considered
    /// durable after phase 1 succeeds.
    pub fn commit_files(
        &mut self,
        branch: &str,
        message: &str,
        files: &[FileWrite],
        actor: Option<&CommitActor>,
        timestamp: DateTime<Utc>,
    ) -> Result<DualOid, StoreError> {
        if let Some(ref backend) = self.durable {
            crate::durable::durable_commit_files(
                &mut self.con,
                backend.as_ref(),
                &self.ws,
                &self.ent,
                branch,
                message,
                files,
                actor,
                timestamp,
            )
        } else {
            store::commit_files(
                &mut self.con,
                &self.ws,
                &self.ent,
                branch,
                message,
                files,
                actor,
                timestamp,
            )
        }
    }

    /// Delete a file in a commit.
    ///
    /// When a durable backend is present, the deletion commit is persisted
    /// to S3 first (phase 1), then KV indexes are updated (phase 2).
    pub fn delete_file(
        &mut self,
        branch: &str,
        path: &str,
        message: &str,
        actor: Option<&CommitActor>,
        timestamp: DateTime<Utc>,
    ) -> Result<DualOid, StoreError> {
        if let Some(ref backend) = self.durable {
            crate::durable::durable_delete_file(
                &mut self.con,
                backend.as_ref(),
                &self.ws,
                &self.ent,
                branch,
                path,
                message,
                actor,
                timestamp,
            )
        } else {
            store::delete_file(
                &mut self.con,
                &self.ws,
                &self.ent,
                branch,
                path,
                message,
                actor,
                timestamp,
            )
        }
    }

    // ── Read operations ──────────────────────────────────────────

    /// Read a file's content from the current branch head.
    ///
    /// When durable, tries KV first, falls back to S3 on miss.
    pub fn read_blob(
        &mut self,
        branch: &str,
        path: &str,
    ) -> Result<Vec<u8>, StoreError> {
        // Get the blob's SHA from the tree index (always in KV)
        let tree_key = keys::tree_key(&self.ws, &self.ent, branch);
        let sha1_hex: Option<String> = self.con.hget(&tree_key, path)?;
        let sha1_hex = sha1_hex
            .ok_or_else(|| StoreError::NotFound(format!("{path} at {branch}")))?;

        self.blob_by_sha1(&sha1_hex)
    }

    /// Read a file and deserialize as JSON.
    pub fn read_json<T: serde::de::DeserializeOwned>(
        &mut self,
        branch: &str,
        path: &str,
    ) -> Result<T, StoreError> {
        let bytes = self.read_blob(branch, path)?;
        Ok(serde_json::from_slice(&bytes)?)
    }

    /// Fetch raw blob bytes by SHA-1.
    ///
    /// When durable, tries KV cache first, falls back to S3.
    pub fn blob_by_sha1(&mut self, sha1_hex: &str) -> Result<Vec<u8>, StoreError> {
        let sha256_hex: Option<String> =
            self.con.hget(keys::oid_1to256_key(), sha1_hex)?;
        let sha256_hex = sha256_hex
            .ok_or_else(|| StoreError::NotFound(format!("oid {sha1_hex}")))?;
        self.blob_by_sha256(&sha256_hex)
    }

    /// Fetch raw blob bytes by SHA-256 (primary key).
    ///
    /// When durable, tries KV cache first, falls back to S3.
    pub fn blob_by_sha256(&mut self, sha256_hex: &str) -> Result<Vec<u8>, StoreError> {
        // Try KV first
        let bytes: Option<Vec<u8>> = self.con.hget(keys::blob_key(), sha256_hex)?;
        if let Some(bytes) = bytes {
            return Ok(bytes);
        }

        // KV miss — try durable backend
        if let Some(ref backend) = self.durable {
            let bytes = backend.get_blob(sha256_hex)?;
            // Optionally cache back to KV for future reads
            let _: () = self.con.hset_nx(keys::blob_key(), sha256_hex, &bytes)?;
            return Ok(bytes);
        }

        Err(StoreError::NotFound(format!("blob {sha256_hex}")))
    }

    /// List files in a directory at the current branch head.
    pub fn list_dir(
        &mut self,
        branch: &str,
        dir_path: &str,
    ) -> Result<Vec<(String, bool)>, StoreError> {
        store::list_dir(&mut self.con, &self.ws, &self.ent, branch, dir_path)
    }

    /// Check if a path exists at a branch head.
    pub fn path_exists(
        &mut self,
        branch: &str,
        path: &str,
    ) -> Result<bool, StoreError> {
        store::path_exists(&mut self.con, &self.ws, &self.ent, branch, path)
    }

    /// Get the current tree state as path → SHA-1 hex.
    pub fn tree_state(
        &mut self,
        branch: &str,
    ) -> Result<BTreeMap<String, String>, StoreError> {
        store::tree_state(&mut self.con, &self.ws, &self.ent, branch)
    }

    /// Resolve a branch ref to a commit SHA-1.
    ///
    /// When a durable backend is present and the KV lookup misses (e.g. after
    /// Dragonfly eviction), falls back to reading the ref from S3 and
    /// re-hydrating the KV index.
    pub fn resolve_ref(&mut self, branch: &str) -> Result<String, StoreError> {
        match store::resolve_ref(&mut self.con, &self.ws, &self.ent, branch) {
            Ok(sha1) => Ok(sha1),
            Err(StoreError::RefNotFound(_)) if self.durable.is_some() => {
                let backend = self.durable.as_ref().unwrap();
                let ref_json = match backend.get_ref(&self.ws, &self.ent, branch) {
                    Ok(data) => data,
                    // S3 doesn't have it either — the ref genuinely doesn't exist
                    Err(StoreError::NotFound(_)) | Err(StoreError::RefNotFound(_)) => {
                        return Err(StoreError::RefNotFound(
                            format!("{}/{}@{}", self.ws, self.ent, branch),
                        ));
                    }
                    Err(e) => return Err(e),
                };
                let parsed: serde_json::Value = serde_json::from_slice(&ref_json)
                    .map_err(|e| StoreError::Internal(format!("corrupt S3 ref: {e}")))?;
                let _sha1 = parsed
                    .get("sha1")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| StoreError::Internal("S3 ref missing sha1 field".into()))?
                    .to_owned();
                // Full rebuild: replay all S3 commits to reconstruct KV indexes
                // (tree state, file listings, refs, etc.)
                tracing::info!(ws = %self.ws, ent = %self.ent, branch, "KV cache miss — rebuilding from S3");
                let replayed = self.rebuild_from_durable()?;
                tracing::info!(ws = %self.ws, ent = %self.ent, replayed, "KV indexes rebuilt from S3");
                // After rebuild, the ref should be in KV — resolve again
                store::resolve_ref(&mut self.con, &self.ws, &self.ent, branch)
                    .map_err(|_| StoreError::Internal(
                        format!("ref still missing after S3 rebuild: {}/{}@{}", self.ws, self.ent, branch),
                    ))
            }
            Err(e) => Err(e),
        }
    }

    /// Read a raw git object by SHA-1.
    pub fn read_raw_git_object(
        &mut self,
        sha1_hex: &str,
    ) -> Result<(GitObjectType, Vec<u8>), StoreError> {
        let type_str: Option<String> =
            self.con.hget(keys::git_obj_type_key(), sha1_hex)?;
        let type_str = type_str
            .ok_or_else(|| StoreError::NotFound(format!("git object type {sha1_hex}")))?;
        let obj_type = GitObjectType::from_str(&type_str)?;
        let content = self.blob_by_sha1(sha1_hex)?;
        Ok((obj_type, content))
    }

    /// Check if a git object exists by SHA-1.
    pub fn git_object_exists(&mut self, sha1_hex: &str) -> Result<bool, StoreError> {
        store::git_object_exists(&mut self.con, sha1_hex)
    }

    /// Store a raw git object.
    pub fn store_raw_git_object(
        &mut self,
        sha1_hex: &str,
        sha256_hex: &str,
        obj_type: GitObjectType,
        content: &[u8],
    ) -> Result<(), StoreError> {
        if let Some(ref backend) = self.durable {
            // Store blob content in S3
            if !backend.blob_exists(sha256_hex)? {
                backend.put_blob(sha256_hex, content)?;
            }
        }
        store::store_raw_git_object(&mut self.con, sha1_hex, sha256_hex, obj_type, content)
    }

    // ── Log / history queries ────────────────────────────────────

    /// Recent commits (newest first).
    pub fn recent_commits(&mut self, limit: usize) -> Result<Vec<CommitEntry>, StoreError> {
        store::recent_commits(&mut self.con, &self.ws, &self.ent, limit)
    }

    /// All commits (oldest first).
    pub fn all_commits(&mut self) -> Result<Vec<CommitEntry>, StoreError> {
        store::all_commits(&mut self.con, &self.ws, &self.ent)
    }

    /// Look up a single commit by SHA-1.
    pub fn get_commit(&mut self, sha1_hex: &str) -> Result<CommitEntry, StoreError> {
        store::get_commit(&mut self.con, &self.ws, &self.ent, sha1_hex)
    }

    /// Commits that touched a specific file path.
    pub fn file_history(&mut self, path: &str) -> Result<Vec<CommitEntry>, StoreError> {
        store::file_history(&mut self.con, &self.ws, &self.ent, path)
    }

    /// SHA-1 → SHA-256 translation.
    pub fn sha1_to_sha256(&mut self, sha1_hex: &str) -> Result<String, StoreError> {
        store::sha1_to_sha256(&mut self.con, sha1_hex)
    }

    /// SHA-256 → SHA-1 translation.
    pub fn sha256_to_sha1(&mut self, sha256_hex: &str) -> Result<String, StoreError> {
        store::sha256_to_sha1(&mut self.con, sha256_hex)
    }

    // ── Branch operations ────────────────────────────────────────

    /// Create a new branch.
    pub fn create_branch(
        &mut self,
        name: &str,
        from_branch: &str,
    ) -> Result<crate::branch::BranchInfo, StoreError> {
        crate::branch::create_branch(&mut self.con, &self.ws, &self.ent, name, from_branch)
    }

    /// List all branches.
    pub fn list_branches(&mut self) -> Result<Vec<crate::branch::BranchInfo>, StoreError> {
        crate::branch::list_branches(&mut self.con, &self.ws, &self.ent)
    }

    /// Delete a branch.
    pub fn delete_branch(&mut self, name: &str) -> Result<(), StoreError> {
        crate::branch::delete_branch(&mut self.con, &self.ws, &self.ent, name)
    }

    // ── Merge operations ─────────────────────────────────────────

    /// Merge a branch (fast-forward or squash).
    pub fn merge_branch(
        &mut self,
        source: &str,
        target: &str,
        squash: bool,
        actor: Option<&CommitActor>,
    ) -> Result<crate::merge::MergeResult, StoreError> {
        if squash {
            crate::merge::merge_branch_squash(
                &mut self.con, &self.ws, &self.ent, source, target, actor,
            )
        } else {
            crate::merge::merge_branch(
                &mut self.con, &self.ws, &self.ent, source, target, actor,
            )
        }
    }

    // ── Rebuild from durable backend ─────────────────────────────

    /// Rebuild all KV indexes from the durable S3 backend.
    ///
    /// This is the recovery path: when KV data is lost or stale,
    /// replay all commits from S3 to reconstruct the complete state.
    pub fn rebuild_from_durable(&mut self) -> Result<u64, StoreError> {
        let backend = self.durable.as_ref()
            .ok_or_else(|| StoreError::Config("no durable backend configured".into()))?;
        crate::durable::rebuild_from_backend(&mut self.con, backend.as_ref(), &self.ws, &self.ent)
    }
}
