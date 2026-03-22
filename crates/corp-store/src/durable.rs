//! Two-phase durable commit with S3-compatible backend.
//!
//! Architecture:
//! ```text
//!   S3 (durable)              Valkey (fast)
//!   ─────────────             ────────────
//!   blobs/{sha256}            corp:blob (cache, optional)
//!   commits/{ws}/{ent}/       corp:log, corp:sha, corp:ref
//!     {seq:010}.json          corp:tree, corp:file, corp:actor
//!   refs/{ws}/{ent}/
//!     {branch}.json
//! ```
//!
//! A commit is considered durable once S3 confirms the write.
//! Valkey is a materialized index — rebuildable from S3 at any time.
//!
//! # Write path (2-phase)
//!
//! 1. **Phase 1 — S3 persist**: Write blobs + commit entry to S3.
//!    If this fails, abort. No side effects.
//! 2. **Phase 2 — Valkey index**: Update Valkey indexes, refs, tree state.
//!    If this fails, S3 has the data. Rebuild Valkey from S3 later.
//!
//! Blob writes are idempotent (content-addressed). Commit entries are
//! keyed by sequence, so retries are safe.

use std::collections::BTreeMap;
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use redis::{Commands, ConnectionLike};

use crate::entry::*;
use crate::error::StoreError;
use crate::oid::{self, DualOid};
use crate::keys;

// ── Backend trait ────────────────────────────────────────────────────

/// S3-compatible durable storage backend.
///
/// Implementations must ensure writes are durable before returning `Ok`.
/// All keys are UTF-8 strings, values are raw bytes.
pub trait DurableBackend {
    /// Store a blob by its SHA-256 hex key.
    /// Must be idempotent — storing the same key+content twice is a no-op.
    fn put_blob(&self, sha256_hex: &str, content: &[u8]) -> Result<(), StoreError>;

    /// Check if a blob exists.
    fn blob_exists(&self, sha256_hex: &str) -> Result<bool, StoreError>;

    /// Retrieve a blob by SHA-256 hex key.
    fn get_blob(&self, sha256_hex: &str) -> Result<Vec<u8>, StoreError>;

    /// Store a commit entry as JSON.
    /// Key: `commits/{ws}/{ent}/{seq:010}.json`
    fn put_commit(
        &self,
        ws: &str,
        ent: &str,
        seq: u64,
        entry_json: &[u8],
    ) -> Result<(), StoreError>;

    /// Store a branch ref.
    /// Key: `refs/{ws}/{ent}/{branch}.json`
    fn put_ref(
        &self,
        ws: &str,
        ent: &str,
        branch: &str,
        ref_json: &[u8],
    ) -> Result<(), StoreError>;

    /// List all commit entries for an entity, oldest first.
    /// Returns the raw JSON bytes of each commit entry.
    fn list_commits(&self, ws: &str, ent: &str) -> Result<Vec<Vec<u8>>, StoreError>;

    /// List all blob SHA-256 keys (for GC / inventory).
    fn list_blobs(&self) -> Result<Vec<String>, StoreError>;
}

// ── Durable commit ──────────────────────────────────────────────────

/// Two-phase durable commit.
///
/// Phase 1: Write blobs + commit entry to the durable backend (S3).
/// Phase 2: Update Valkey indexes.
///
/// Returns the commit's dual OID only after S3 confirms.
pub fn durable_commit_files(
    con: &mut impl ConnectionLike,
    backend: &dyn DurableBackend,
    ws: &str,
    ent: &str,
    branch: &str,
    message: &str,
    files: &[FileWrite],
    actor: Option<&CommitActor>,
    timestamp: DateTime<Utc>,
) -> Result<DualOid, StoreError> {
    // ── Phase 1a: Persist blobs to S3 ──

    let mut blob_oids: Vec<(String, DualOid)> = Vec::with_capacity(files.len());
    for fw in files {
        let oid = oid::hash_blob(&fw.content);

        // Skip if blob already exists in S3 (dedup).
        if !backend.blob_exists(&oid.sha256_hex())? {
            backend.put_blob(&oid.sha256_hex(), &fw.content)?;
        }

        blob_oids.push((fw.path.clone(), oid));
    }

    // ── Phase 1b: Compute commit entry ──

    // Get current tree state from Valkey.
    let tree_key = keys::tree_key(ws, ent, branch);
    let current_tree: BTreeMap<String, String> = con.hgetall(&tree_key)?;

    // Build new tree state.
    let mut tree: BTreeMap<String, DualOid> = BTreeMap::new();
    for (path, sha1_hex) in &current_tree {
        let sha256_hex: String = con.hget(keys::oid_1to256_key(), sha1_hex)?;
        let sha1: [u8; 20] = hex::decode(sha1_hex)?
            .try_into()
            .map_err(|_| StoreError::NotFound("bad sha1".into()))?;
        let sha256: [u8; 32] = hex::decode(&sha256_hex)?
            .try_into()
            .map_err(|_| StoreError::NotFound("bad sha256".into()))?;
        tree.insert(path.clone(), DualOid { sha1, sha256 });
    }

    let mut changes = Vec::new();
    for (path, blob_oid) in &blob_oids {
        let action = if tree.contains_key(path) {
            ChangeAction::Modify
        } else {
            ChangeAction::Add
        };
        changes.push(FileChange {
            path: path.clone(),
            action,
            blob_sha1: Some(blob_oid.sha1_hex()),
            blob_sha256: Some(blob_oid.sha256_hex()),
            old_path: None,
        });
        tree.insert(path.clone(), blob_oid.clone());
    }

    let (root_tree_oid, _) = oid::compute_root_tree(&tree);

    let ref_key = keys::ref_key(ws, ent);
    let parent_sha1: Option<String> = con.hget(&ref_key, branch)?;

    let commit_oid = oid::hash_commit(&oid::CommitHash {
        tree_sha1_hex: &root_tree_oid.sha1_hex(),
        parent_sha1_hex: parent_sha1.as_deref(),
        author_name: "corp-engine",
        author_email: "engine@thecorporation.ai",
        author_timestamp: timestamp.timestamp(),
        message,
    });

    let seq: u64 = con.incr(keys::seq_key(ws, ent), 1)?;

    let entry = CommitEntry {
        ld_type: "Commit".to_owned(),
        ld_id: format!("git:sha/{}", commit_oid.sha1_hex()),
        sha1: commit_oid.sha1_hex(),
        sha256: commit_oid.sha256_hex(),
        parents: parent_sha1.clone().into_iter().collect(),
        author_name: "corp-engine".to_owned(),
        author_email: "engine@thecorporation.ai".to_owned(),
        message: message.to_owned(),
        timestamp,
        sequence: seq,
        workspace_id: actor.map(|a| a.workspace_id.clone()),
        entity_id: actor.and_then(|a| a.entity_id.clone()),
        scopes: actor.map_or(Vec::new(), |a| a.scopes.clone()),
        signed_by: actor.and_then(|a| a.signed_by.clone()),
        tree_sha1: root_tree_oid.sha1_hex(),
        changes: changes.clone(),
    };

    let entry_json = serde_json::to_string(&entry)?;

    // ── Phase 1c: Persist commit entry to S3 ──
    // THIS is the durability point. After this returns Ok, the commit is safe.

    backend.put_commit(ws, ent, seq, entry_json.as_bytes())?;

    let ref_json = serde_json::to_string(&serde_json::json!({
        "branch": branch,
        "sha1": commit_oid.sha1_hex(),
        "sha256": commit_oid.sha256_hex(),
        "sequence": seq,
        "timestamp": timestamp.to_rfc3339(),
    }))?;
    backend.put_ref(ws, ent, branch, ref_json.as_bytes())?;

    tracing::info!(
        ws = ws, ent = ent, sha1 = %commit_oid.sha1_hex(),
        seq = seq, "phase 1 complete — durable in S3"
    );

    // ── Phase 2: Update Valkey indexes ──
    // If this fails, data is safe in S3. Rebuild from S3 later.

    if let Err(e) = update_valkey_indexes(
        con, ws, ent, branch, &entry, &entry_json, &blob_oids, &changes, seq,
    ) {
        tracing::warn!(
            ws = ws, ent = ent, seq = seq,
            error = %e,
            "phase 2 failed — Valkey indexes stale, rebuild from S3"
        );
        // Don't propagate — the commit IS durable in S3.
    }

    Ok(commit_oid)
}

fn update_valkey_indexes(
    con: &mut impl ConnectionLike,
    ws: &str,
    ent: &str,
    branch: &str,
    entry: &CommitEntry,
    entry_json: &str,
    blob_oids: &[(String, DualOid)],
    changes: &[FileChange],
    seq: u64,
) -> Result<(), StoreError> {
    let mut p = redis::pipe();
    p.atomic();

    // Blob cache + OID lookup.
    // (We don't cache blob content in Valkey for durable mode — read from S3.
    //  But we DO need the OID lookup tables for tree state operations.)
    for (_, oid) in blob_oids {
        p.hset_nx(keys::oid_1to256_key(), oid.sha1_hex(), oid.sha256_hex());
        p.hset_nx(keys::oid_256to1_key(), oid.sha256_hex(), oid.sha1_hex());
    }

    // Commit log.
    p.zadd(keys::log_key(ws, ent), entry_json, seq as f64);
    p.hset(keys::sha_index_key(ws, ent), &entry.sha1, seq);

    // Ref.
    p.hset(keys::ref_key(ws, ent), branch, &entry.sha1);

    // OID lookup for commit.
    p.hset(keys::oid_1to256_key(), &entry.sha1, &entry.sha256);
    p.hset(keys::oid_256to1_key(), &entry.sha256, &entry.sha1);

    // Tree state.
    let tree_key = keys::tree_key(ws, ent, branch);
    for fc in changes {
        match fc.action {
            ChangeAction::Add | ChangeAction::Modify => {
                if let Some(ref sha1) = fc.blob_sha1 {
                    p.hset(&tree_key, &fc.path, sha1);
                }
            }
            ChangeAction::Delete => {
                p.hdel(&tree_key, &fc.path);
            }
            ChangeAction::Rename => {
                if let Some(ref old) = fc.old_path {
                    p.hdel(&tree_key, old);
                }
                if let Some(ref sha1) = fc.blob_sha1 {
                    p.hset(&tree_key, &fc.path, sha1);
                }
            }
        }
    }

    // File history.
    for fc in changes {
        p.zadd(
            keys::file_history_key(ws, ent, &fc.path),
            &entry.sha1,
            seq as f64,
        );
    }

    // Actor index.
    if let Some(ref ws_id) = entry.workspace_id {
        p.zadd(
            keys::actor_key(ws_id),
            format!("{ws}:{ent}:{}", entry.sha1),
            seq as f64,
        );
    }

    // Entity tracking.
    p.sadd(keys::workspaces_key(), ws);
    p.sadd(keys::entities_key(ws), ent);

    p.query::<()>(con)?;
    Ok(())
}

// ── Rebuild from S3 ──────────────────────────────────────────────────

/// Rebuild Valkey indexes from the durable S3 backend.
///
/// Reads all commit entries for an entity from S3 and replays them
/// into Valkey. Use this after Valkey data loss or when bootstrapping
/// a new Valkey instance.
pub fn rebuild_from_backend(
    con: &mut impl ConnectionLike,
    backend: &dyn DurableBackend,
    ws: &str,
    ent: &str,
) -> Result<u64, StoreError> {
    let commit_jsons = backend.list_commits(ws, ent)?;
    if commit_jsons.is_empty() {
        return Ok(0);
    }

    // Clear existing Valkey state for this entity.
    let ref_key = keys::ref_key(ws, ent);
    let branches: BTreeMap<String, String> = con.hgetall(&ref_key)?;
    let mut clear_pipe = redis::pipe();
    clear_pipe.del(keys::log_key(ws, ent));
    clear_pipe.del(keys::seq_key(ws, ent));
    clear_pipe.del(keys::sha_index_key(ws, ent));
    clear_pipe.del(&ref_key);
    for (branch, _) in &branches {
        clear_pipe.del(keys::tree_key(ws, ent, branch));
    }
    clear_pipe.query::<()>(con)?;

    let mut count = 0u64;
    let mut tree_state: BTreeMap<String, String> = BTreeMap::new();
    let last_branch = String::from("main");

    for json_bytes in &commit_jsons {
        let entry: CommitEntry = serde_json::from_slice(json_bytes)?;

        // Ensure blobs are in Valkey OID tables (fetch from S3 to get content for hashing).
        for fc in &entry.changes {
            if let (Some(sha1), Some(sha256)) = (&fc.blob_sha1, &fc.blob_sha256) {
                let _: () = redis::cmd("HSETNX")
                    .arg(keys::oid_1to256_key())
                    .arg(sha1)
                    .arg(sha256)
                    .query(con)?;
                let _: () = redis::cmd("HSETNX")
                    .arg(keys::oid_256to1_key())
                    .arg(sha256)
                    .arg(sha1)
                    .query(con)?;
            }
        }

        // Apply changes to tree state.
        for fc in &entry.changes {
            match fc.action {
                ChangeAction::Add | ChangeAction::Modify => {
                    if let Some(ref sha1) = fc.blob_sha1 {
                        tree_state.insert(fc.path.clone(), sha1.clone());
                    }
                }
                ChangeAction::Delete => {
                    tree_state.remove(&fc.path);
                }
                ChangeAction::Rename => {
                    if let Some(ref old) = fc.old_path {
                        tree_state.remove(old);
                    }
                    if let Some(ref sha1) = fc.blob_sha1 {
                        tree_state.insert(fc.path.clone(), sha1.clone());
                    }
                }
            }
        }

        let entry_json = serde_json::to_string(&entry)?;
        let seq = entry.sequence;

        let mut p = redis::pipe();
        p.atomic();
        p.zadd(keys::log_key(ws, ent), &entry_json, seq as f64);
        p.hset(keys::sha_index_key(ws, ent), &entry.sha1, seq);
        p.hset(keys::oid_1to256_key(), &entry.sha1, &entry.sha256);
        p.hset(keys::oid_256to1_key(), &entry.sha256, &entry.sha1);
        p.set(keys::seq_key(ws, ent), seq);

        for fc in &entry.changes {
            p.zadd(
                keys::file_history_key(ws, ent, &fc.path),
                &entry.sha1,
                seq as f64,
            );
        }

        if let Some(ref ws_id) = entry.workspace_id {
            p.zadd(
                keys::actor_key(ws_id),
                format!("{ws}:{ent}:{}", entry.sha1),
                seq as f64,
            );
        }

        p.sadd(keys::workspaces_key(), ws);
        p.sadd(keys::entities_key(ws), ent);

        // Update ref to point to this commit.
        p.hset(&ref_key, &last_branch, &entry.sha1);

        p.query::<()>(con)?;
        count += 1;
    }

    // Write final tree state.
    let tree_key = keys::tree_key(ws, ent, &last_branch);
    if !tree_state.is_empty() {
        let mut p = redis::pipe();
        for (path, sha1) in &tree_state {
            p.hset(&tree_key, path, sha1);
        }
        p.query::<()>(con)?;
    }

    tracing::info!(
        ws = ws, ent = ent, count = count,
        "rebuilt Valkey indexes from S3"
    );

    Ok(count)
}

/// Read a blob — tries Valkey cache first, falls back to S3.
pub fn durable_read_blob(
    con: &mut impl ConnectionLike,
    backend: &dyn DurableBackend,
    ws: &str,
    ent: &str,
    branch: &str,
    path: &str,
) -> Result<Vec<u8>, StoreError> {
    // Get blob SHA from tree state (Valkey).
    let tree_key = keys::tree_key(ws, ent, branch);
    let sha1_hex: Option<String> = con.hget(&tree_key, path)?;
    let sha1_hex =
        sha1_hex.ok_or_else(|| StoreError::NotFound(format!("{path} at {branch}")))?;

    // Try Valkey blob cache first.
    let sha256_hex: String = con.hget(keys::oid_1to256_key(), &sha1_hex)?;
    let cached: Option<Vec<u8>> = con.hget(keys::blob_key(), &sha256_hex)?;

    if let Some(content) = cached {
        return Ok(content);
    }

    // Fall back to S3.
    let content = backend.get_blob(&sha256_hex)?;

    // Optionally cache in Valkey for next read.
    let _: () = redis::cmd("HSETNX")
        .arg(keys::blob_key())
        .arg(&sha256_hex)
        .arg(&content)
        .query(con)?;

    Ok(content)
}

// ── Filesystem backend (for testing & single-node) ───────────────────

/// A `DurableBackend` that writes to a local directory.
///
/// Layout mirrors S3:
/// ```text
/// {root}/blobs/{sha256_hex}
/// {root}/commits/{ws}/{ent}/{seq:010}.json
/// {root}/refs/{ws}/{ent}/{branch}.json
/// ```
///
/// Suitable for testing, single-node deployments, or as a template
/// for implementing S3-compatible backends.
pub struct FsBackend {
    root: PathBuf,
}

impl FsBackend {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    fn blob_path(&self, sha256_hex: &str) -> PathBuf {
        self.root.join("blobs").join(sha256_hex)
    }

    fn commit_path(&self, ws: &str, ent: &str, seq: u64) -> PathBuf {
        self.root
            .join("commits")
            .join(ws)
            .join(ent)
            .join(format!("{seq:010}.json"))
    }

    fn ref_path(&self, ws: &str, ent: &str, branch: &str) -> PathBuf {
        self.root
            .join("refs")
            .join(ws)
            .join(ent)
            .join(format!("{branch}.json"))
    }
}

impl DurableBackend for FsBackend {
    fn put_blob(&self, sha256_hex: &str, content: &[u8]) -> Result<(), StoreError> {
        let path = self.blob_path(sha256_hex);
        if path.exists() {
            return Ok(()); // idempotent
        }
        std::fs::create_dir_all(path.parent().unwrap())?;
        std::fs::write(&path, content)?;
        Ok(())
    }

    fn blob_exists(&self, sha256_hex: &str) -> Result<bool, StoreError> {
        Ok(self.blob_path(sha256_hex).exists())
    }

    fn get_blob(&self, sha256_hex: &str) -> Result<Vec<u8>, StoreError> {
        let path = self.blob_path(sha256_hex);
        std::fs::read(&path)
            .map_err(|_| StoreError::NotFound(format!("blob {sha256_hex}")))
    }

    fn put_commit(
        &self,
        ws: &str,
        ent: &str,
        seq: u64,
        entry_json: &[u8],
    ) -> Result<(), StoreError> {
        let path = self.commit_path(ws, ent, seq);
        std::fs::create_dir_all(path.parent().unwrap())?;
        std::fs::write(&path, entry_json)?;
        Ok(())
    }

    fn put_ref(
        &self,
        ws: &str,
        ent: &str,
        branch: &str,
        ref_json: &[u8],
    ) -> Result<(), StoreError> {
        let path = self.ref_path(ws, ent, branch);
        std::fs::create_dir_all(path.parent().unwrap())?;
        std::fs::write(&path, ref_json)?;
        Ok(())
    }

    fn list_commits(&self, ws: &str, ent: &str) -> Result<Vec<Vec<u8>>, StoreError> {
        let dir = self.root.join("commits").join(ws).join(ent);
        if !dir.exists() {
            return Ok(Vec::new());
        }

        let mut entries: Vec<_> = std::fs::read_dir(&dir)?
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .extension()
                    .is_some_and(|ext| ext == "json")
            })
            .collect();

        // Sort by filename (lexicographic = sequence order due to zero-padding).
        entries.sort_by_key(|e| e.file_name());

        entries
            .iter()
            .map(|e| std::fs::read(e.path()).map_err(StoreError::from))
            .collect()
    }

    fn list_blobs(&self) -> Result<Vec<String>, StoreError> {
        let dir = self.root.join("blobs");
        if !dir.exists() {
            return Ok(Vec::new());
        }

        Ok(std::fs::read_dir(&dir)?
            .filter_map(|e| e.ok())
            .filter_map(|e| e.file_name().into_string().ok())
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store;

    #[test]
    fn fs_backend_blob_roundtrip() {
        let tmp = tempfile::TempDir::new().unwrap();
        let backend = FsBackend::new(tmp.path());

        let content = b"hello durable world";
        let oid = oid::hash_blob(content);

        assert!(!backend.blob_exists(&oid.sha256_hex()).unwrap());

        backend.put_blob(&oid.sha256_hex(), content).unwrap();
        assert!(backend.blob_exists(&oid.sha256_hex()).unwrap());

        let read_back = backend.get_blob(&oid.sha256_hex()).unwrap();
        assert_eq!(read_back, content);

        // Idempotent.
        backend.put_blob(&oid.sha256_hex(), content).unwrap();
    }

    #[test]
    fn fs_backend_commit_list_order() {
        let tmp = tempfile::TempDir::new().unwrap();
        let backend = FsBackend::new(tmp.path());

        backend.put_commit("ws", "ent", 1, b"{\"seq\": 1}").unwrap();
        backend.put_commit("ws", "ent", 2, b"{\"seq\": 2}").unwrap();
        backend.put_commit("ws", "ent", 10, b"{\"seq\": 10}").unwrap();

        let commits = backend.list_commits("ws", "ent").unwrap();
        assert_eq!(commits.len(), 3);
        assert_eq!(commits[0], b"{\"seq\": 1}");
        assert_eq!(commits[1], b"{\"seq\": 2}");
        assert_eq!(commits[2], b"{\"seq\": 10}");
    }

    #[test]
    fn durable_commit_and_rebuild() {
        let Some(mut con) = try_redis() else {
            eprintln!("SKIP: no redis");
            return;
        };

        let tmp = tempfile::TempDir::new().unwrap();
        let backend = FsBackend::new(tmp.path());
        let ws = &format!("dtest_{}", std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos());
        let ent = "ent_durable";

        // Durable commit.
        let files = vec![
            FileWrite::new("a.txt", b"aaa".to_vec()),
            FileWrite::new("b.txt", b"bbb".to_vec()),
        ];
        let oid = durable_commit_files(
            &mut con, &backend, ws, ent, "main",
            "add files", &files, None, Utc::now(),
        ).unwrap();

        assert!(!oid.sha1_hex().is_empty());

        // Verify S3 has the data.
        let s3_commits = backend.list_commits(ws, ent).unwrap();
        assert_eq!(s3_commits.len(), 1);
        let s3_blobs = backend.list_blobs().unwrap();
        assert_eq!(s3_blobs.len(), 2);

        // Verify Valkey reads work.
        let content = store::read_blob(&mut con, ws, ent, "main", "a.txt").unwrap();
        assert_eq!(content, b"aaa");

        // Simulate Valkey data loss — flush entity keys.
        let _: () = redis::cmd("DEL")
            .arg(keys::log_key(ws, ent))
            .arg(keys::seq_key(ws, ent))
            .arg(keys::sha_index_key(ws, ent))
            .arg(keys::ref_key(ws, ent))
            .arg(keys::tree_key(ws, ent, "main"))
            .query(&mut con).unwrap();

        // Valkey reads should fail now.
        assert!(store::read_blob(&mut con, ws, ent, "main", "a.txt").is_err());

        // Rebuild from S3.
        let rebuilt = rebuild_from_backend(&mut con, &backend, ws, ent).unwrap();
        assert_eq!(rebuilt, 1);

        // Reads work again via S3 fallback.
        let content = durable_read_blob(&mut con, &backend, ws, ent, "main", "a.txt").unwrap();
        assert_eq!(content, b"aaa");
    }

    fn try_redis() -> Option<redis::Connection> {
        redis::Client::open("redis://127.0.0.1/").ok()?.get_connection().ok()
    }
}
