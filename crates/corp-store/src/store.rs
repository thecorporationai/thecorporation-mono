//! Core Valkey-backed store.
//!
//! All operations take `&mut impl ConnectionLike` so the caller controls
//! connection pooling (sync `Connection`, async via `spawn_blocking`, etc.).

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use redis::{Commands, ConnectionLike, pipe};

use crate::entry::*;
use crate::error::StoreError;
use crate::keys;
use crate::oid::{self, CommitHash, DualOid};

// ── Write operations ─────────────────────────────────────────────────

/// Commit one or more files atomically.
///
/// Returns the commit's dual OID (SHA-1 + SHA-256).
pub fn commit_files(
    con: &mut impl ConnectionLike,
    ws: &str,
    ent: &str,
    branch: &str,
    message: &str,
    files: &[FileWrite],
    actor: Option<&CommitActor>,
    timestamp: DateTime<Utc>,
) -> Result<DualOid, StoreError> {
    validate_paths(files)?;

    // 1. Hash and store blobs.
    let mut blob_oids: Vec<(String, DualOid)> = Vec::with_capacity(files.len());
    {
        let mut blob_pipe = pipe();
        let mut oid_pipe = pipe();

        for fw in files {
            let oid = oid::hash_blob(&fw.content);
            // Store blob content under SHA-256.
            blob_pipe.hset_nx(keys::blob_key(), oid.sha256_hex(), &fw.content);
            // SHA-1 ↔ SHA-256 lookup tables.
            oid_pipe.hset_nx(keys::oid_1to256_key(), oid.sha1_hex(), oid.sha256_hex());
            oid_pipe.hset_nx(keys::oid_256to1_key(), oid.sha256_hex(), oid.sha1_hex());
            // Git object type tag.
            oid_pipe.hset_nx(keys::git_obj_type_key(), oid.sha1_hex(), "blob");
            blob_oids.push((fw.path.clone(), oid));
        }

        blob_pipe.query::<()>(con)?;
        oid_pipe.query::<()>(con)?;
    }

    // 2. Get current tree state.
    let tree_key = keys::tree_key(ws, ent, branch);
    let current_tree: BTreeMap<String, String> = con.hgetall(&tree_key)?;

    // Convert to DualOid tree (look up SHA-256 for existing entries).
    let mut tree: BTreeMap<String, DualOid> = BTreeMap::new();
    for (path, sha1_hex) in &current_tree {
        let sha256_hex: String = con.hget(keys::oid_1to256_key(), sha1_hex)?;
        let sha1_bytes: [u8; 20] = hex::decode(sha1_hex)?
            .try_into()
            .map_err(|_| StoreError::NotFound("bad sha1 in tree".into()))?;
        let sha256_bytes: [u8; 32] = hex::decode(&sha256_hex)?
            .try_into()
            .map_err(|_| StoreError::NotFound("bad sha256 in tree".into()))?;
        tree.insert(
            path.clone(),
            DualOid {
                sha1: sha1_bytes,
                sha256: sha256_bytes,
            },
        );
    }

    // 3. Apply file changes.
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

    // 4. Compute tree hash and store raw tree objects.
    let (root_tree_oid, all_trees) = oid::compute_root_tree(&tree);

    {
        let mut tree_pipe = pipe();
        for (tree_oid, raw_content) in &all_trees {
            tree_pipe.hset_nx(keys::blob_key(), tree_oid.sha256_hex(), raw_content);
            tree_pipe.hset_nx(keys::oid_1to256_key(), tree_oid.sha1_hex(), tree_oid.sha256_hex());
            tree_pipe.hset_nx(keys::oid_256to1_key(), tree_oid.sha256_hex(), tree_oid.sha1_hex());
            tree_pipe.hset_nx(keys::git_obj_type_key(), tree_oid.sha1_hex(), "tree");
        }
        tree_pipe.query::<()>(con)?;
    }

    // 5. Get parent commit.
    let ref_key = keys::ref_key(ws, ent);
    let parent_sha1: Option<String> = con.hget(&ref_key, branch)?;

    // 6. Compute commit hash and store raw commit object.
    let unix_ts = timestamp.timestamp();
    let author_name = actor.map_or("corp-engine", |_| "corp-engine");
    let author_email = actor.map_or("engine@thecorporation.ai", |_| "engine@thecorporation.ai");

    let (commit_oid, commit_raw) = oid::build_commit(&CommitHash {
        tree_sha1_hex: &root_tree_oid.sha1_hex(),
        parent_sha1_hex: parent_sha1.as_deref(),
        author_name,
        author_email,
        author_timestamp: unix_ts,
        message,
    });

    store_raw_git_object(
        con,
        &commit_oid.sha1_hex(),
        &commit_oid.sha256_hex(),
        GitObjectType::Commit,
        &commit_raw,
    )?;

    // 7. Build commit entry.
    let seq: u64 = con.incr(keys::seq_key(ws, ent), 1)?;

    let entry = CommitEntry {
        ld_type: "Commit".to_owned(),
        ld_id: format!("git:sha/{}", commit_oid.sha1_hex()),
        sha1: commit_oid.sha1_hex(),
        sha256: commit_oid.sha256_hex(),
        parents: parent_sha1.into_iter().collect(),
        author_name: author_name.to_owned(),
        author_email: author_email.to_owned(),
        message: message.to_owned(),
        timestamp,
        sequence: seq,
        workspace_id: actor.map(|a| a.workspace_id.clone()),
        entity_id: actor.and_then(|a| a.entity_id.clone()),
        scopes: actor.map_or(Vec::new(), |a| a.scopes.clone()),
        signed_by: actor.and_then(|a| a.signed_by.clone()),
        tree_sha1: root_tree_oid.sha1_hex(),
        changes,
    };

    let entry_json = serde_json::to_string(&entry)?;

    // 8. Atomic pipeline: log + refs + tree + indexes.
    {
        let mut p = pipe();
        p.atomic();

        // Commit log.
        p.zadd(keys::log_key(ws, ent), &entry_json, seq as f64);

        // SHA → sequence index.
        p.hset(keys::sha_index_key(ws, ent), &entry.sha1, seq);

        // Branch ref.
        p.hset(&ref_key, branch, &entry.sha1);

        // OID lookup for commit.
        p.hset(keys::oid_1to256_key(), &entry.sha1, &entry.sha256);
        p.hset(keys::oid_256to1_key(), &entry.sha256, &entry.sha1);

        // Update tree state.
        for fc in &entry.changes {
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

        // File history indexes.
        for fc in &entry.changes {
            p.zadd(
                keys::file_history_key(ws, ent, &fc.path),
                &entry.sha1,
                seq as f64,
            );
        }

        // Actor index.
        if let Some(actor) = actor {
            p.zadd(
                keys::actor_key(&actor.workspace_id),
                format!("{ws}:{ent}:{}", entry.sha1),
                seq as f64,
            );
        }

        // Workspace/entity tracking.
        p.sadd(keys::workspaces_key(), ws);
        p.sadd(keys::entities_key(ws), ent);

        p.query::<()>(con)?;
    }

    tracing::debug!(
        ws = ws,
        ent = ent,
        sha1 = %entry.sha1,
        seq = seq,
        files = files.len(),
        "committed"
    );

    Ok(commit_oid)
}

/// Delete a file in a commit.
pub fn delete_file(
    con: &mut impl ConnectionLike,
    ws: &str,
    ent: &str,
    branch: &str,
    path: &str,
    message: &str,
    actor: Option<&CommitActor>,
    timestamp: DateTime<Utc>,
) -> Result<DualOid, StoreError> {
    // Get current tree, remove the path, recompute, commit.
    let tree_key = keys::tree_key(ws, ent, branch);
    let exists: bool = con.hexists(&tree_key, path)?;
    if !exists {
        return Err(StoreError::NotFound(format!("{path} not in tree")));
    }

    // Build a minimal commit that records the deletion.
    // We need the full tree (minus deleted path) to compute the new tree hash.
    let current_tree: BTreeMap<String, String> = con.hgetall(&tree_key)?;
    let mut tree: BTreeMap<String, DualOid> = BTreeMap::new();
    for (p, sha1_hex) in &current_tree {
        if p == path {
            continue;
        }
        let sha256_hex: String = con.hget(keys::oid_1to256_key(), sha1_hex)?;
        let sha1_bytes: [u8; 20] = hex::decode(sha1_hex)?
            .try_into()
            .map_err(|_| StoreError::NotFound("bad sha1".into()))?;
        let sha256_bytes: [u8; 32] = hex::decode(&sha256_hex)?
            .try_into()
            .map_err(|_| StoreError::NotFound("bad sha256".into()))?;
        tree.insert(p.clone(), DualOid { sha1: sha1_bytes, sha256: sha256_bytes });
    }

    let (root_tree_oid, _) = oid::compute_root_tree(&tree);

    let ref_key = keys::ref_key(ws, ent);
    let parent_sha1: Option<String> = con.hget(&ref_key, branch)?;
    let unix_ts = timestamp.timestamp();

    let commit_oid = oid::hash_commit(&CommitHash {
        tree_sha1_hex: &root_tree_oid.sha1_hex(),
        parent_sha1_hex: parent_sha1.as_deref(),
        author_name: "corp-engine",
        author_email: "engine@thecorporation.ai",
        author_timestamp: unix_ts,
        message,
    });

    let seq: u64 = con.incr(keys::seq_key(ws, ent), 1)?;

    let change = FileChange {
        path: path.to_owned(),
        action: ChangeAction::Delete,
        blob_sha1: None,
        blob_sha256: None,
        old_path: None,
    };

    let entry = CommitEntry {
        ld_type: "Commit".to_owned(),
        ld_id: format!("git:sha/{}", commit_oid.sha1_hex()),
        sha1: commit_oid.sha1_hex(),
        sha256: commit_oid.sha256_hex(),
        parents: parent_sha1.into_iter().collect(),
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
        changes: vec![change],
    };

    let entry_json = serde_json::to_string(&entry)?;

    let mut p = pipe();
    p.atomic();
    p.zadd(keys::log_key(ws, ent), &entry_json, seq as f64);
    p.hset(keys::sha_index_key(ws, ent), &entry.sha1, seq);
    p.hset(&ref_key, branch, &entry.sha1);
    p.hset(keys::oid_1to256_key(), &entry.sha1, &entry.sha256);
    p.hset(keys::oid_256to1_key(), &entry.sha256, &entry.sha1);
    p.hdel(&tree_key, path);
    p.zadd(
        keys::file_history_key(ws, ent, path),
        &entry.sha1,
        seq as f64,
    );
    p.query::<()>(con)?;

    Ok(commit_oid)
}

// ── Raw git object storage ───────────────────────────────────────────

/// Git object type tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitObjectType {
    Blob,
    Tree,
    Commit,
}

impl GitObjectType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Blob => "blob",
            Self::Tree => "tree",
            Self::Commit => "commit",
        }
    }

    pub fn from_str(s: &str) -> Result<Self, StoreError> {
        match s {
            "blob" => Ok(Self::Blob),
            "tree" => Ok(Self::Tree),
            "commit" => Ok(Self::Commit),
            other => Err(StoreError::NotFound(format!("unknown object type: {other}"))),
        }
    }

    /// The git pack object type number.
    pub fn pack_type(self) -> u8 {
        match self {
            Self::Commit => 1,
            Self::Tree => 2,
            Self::Blob => 3,
        }
    }
}

/// Store a raw git object (blob, tree, or commit content without header).
///
/// Content is stored in `corp:blob` under SHA-256. The object type is
/// stored in `corp:git-obj-type` under SHA-1 so upload-pack can
/// reconstruct the correct git object header.
pub fn store_raw_git_object(
    con: &mut impl ConnectionLike,
    sha1_hex: &str,
    sha256_hex: &str,
    obj_type: GitObjectType,
    content: &[u8],
) -> Result<(), StoreError> {
    let mut p = pipe();
    p.hset_nx(keys::blob_key(), sha256_hex, content);
    p.hset_nx(keys::oid_1to256_key(), sha1_hex, sha256_hex);
    p.hset_nx(keys::oid_256to1_key(), sha256_hex, sha1_hex);
    p.hset_nx(keys::git_obj_type_key(), sha1_hex, obj_type.as_str());
    p.query::<()>(con)?;
    Ok(())
}

/// Read a raw git object by SHA-1.
///
/// Returns `(type, content)` where content is the object body without
/// the git header (`{type} {len}\0`).
pub fn read_raw_git_object(
    con: &mut impl ConnectionLike,
    sha1_hex: &str,
) -> Result<(GitObjectType, Vec<u8>), StoreError> {
    let type_str: Option<String> = con.hget(keys::git_obj_type_key(), sha1_hex)?;
    let type_str = type_str
        .ok_or_else(|| StoreError::NotFound(format!("git object type {sha1_hex}")))?;
    let obj_type = GitObjectType::from_str(&type_str)?;
    let content = blob_by_sha1(con, sha1_hex)?;
    Ok((obj_type, content))
}

/// Check if a git object exists by SHA-1.
pub fn git_object_exists(
    con: &mut impl ConnectionLike,
    sha1_hex: &str,
) -> Result<bool, StoreError> {
    Ok(con.hexists(keys::git_obj_type_key(), sha1_hex)?)
}

// ── Read operations ──────────────────────────────────────────────────

/// Read a file's content from the current branch head.
pub fn read_blob(
    con: &mut impl ConnectionLike,
    ws: &str,
    ent: &str,
    branch: &str,
    path: &str,
) -> Result<Vec<u8>, StoreError> {
    let tree_key = keys::tree_key(ws, ent, branch);
    let sha1_hex: Option<String> = con.hget(&tree_key, path)?;
    let sha1_hex = sha1_hex
        .ok_or_else(|| StoreError::NotFound(format!("{path} at {branch}")))?;

    blob_by_sha1(con, &sha1_hex)
}

/// Read a file and deserialize as JSON.
pub fn read_json<T: serde::de::DeserializeOwned>(
    con: &mut impl ConnectionLike,
    ws: &str,
    ent: &str,
    branch: &str,
    path: &str,
) -> Result<T, StoreError> {
    let bytes = read_blob(con, ws, ent, branch, path)?;
    Ok(serde_json::from_slice(&bytes)?)
}

/// List files in a directory at the current branch head.
///
/// Returns `(name, is_dir)` pairs.
pub fn list_dir(
    con: &mut impl ConnectionLike,
    ws: &str,
    ent: &str,
    branch: &str,
    dir_path: &str,
) -> Result<Vec<(String, bool)>, StoreError> {
    let tree_key = keys::tree_key(ws, ent, branch);
    let all: BTreeMap<String, String> = con.hgetall(&tree_key)?;

    let prefix = if dir_path.is_empty() {
        String::new()
    } else {
        format!("{dir_path}/")
    };

    let mut entries = BTreeMap::new();
    for path in all.keys() {
        let relative = if prefix.is_empty() {
            path.as_str()
        } else if let Some(rest) = path.strip_prefix(&prefix) {
            rest
        } else {
            continue;
        };

        if let Some((dir, _)) = relative.split_once('/') {
            entries.entry(dir.to_owned()).or_insert(true); // is_dir = true
        } else {
            entries.entry(relative.to_owned()).or_insert(false); // is_dir = false
        }
    }

    Ok(entries.into_iter().collect())
}

/// Check if a path exists at a branch head.
pub fn path_exists(
    con: &mut impl ConnectionLike,
    ws: &str,
    ent: &str,
    branch: &str,
    path: &str,
) -> Result<bool, StoreError> {
    let tree_key = keys::tree_key(ws, ent, branch);
    Ok(con.hexists(&tree_key, path)?)
}

/// Get the current tree state as path → SHA-1 hex.
pub fn tree_state(
    con: &mut impl ConnectionLike,
    ws: &str,
    ent: &str,
    branch: &str,
) -> Result<BTreeMap<String, String>, StoreError> {
    let tree_key = keys::tree_key(ws, ent, branch);
    Ok(con.hgetall(&tree_key)?)
}

/// Resolve a branch ref to a commit SHA-1.
pub fn resolve_ref(
    con: &mut impl ConnectionLike,
    ws: &str,
    ent: &str,
    branch: &str,
) -> Result<String, StoreError> {
    let ref_key = keys::ref_key(ws, ent);
    let sha1: Option<String> = con.hget(&ref_key, branch)?;
    sha1.ok_or_else(|| StoreError::RefNotFound(format!("{ws}/{ent}@{branch}")))
}

/// Fetch raw blob bytes by SHA-1, using the lookup table.
pub fn blob_by_sha1(
    con: &mut impl ConnectionLike,
    sha1_hex: &str,
) -> Result<Vec<u8>, StoreError> {
    let sha256_hex: Option<String> = con.hget(keys::oid_1to256_key(), sha1_hex)?;
    let sha256_hex =
        sha256_hex.ok_or_else(|| StoreError::NotFound(format!("oid {sha1_hex}")))?;
    blob_by_sha256(con, &sha256_hex)
}

/// Fetch raw blob bytes by SHA-256 (primary key).
pub fn blob_by_sha256(
    con: &mut impl ConnectionLike,
    sha256_hex: &str,
) -> Result<Vec<u8>, StoreError> {
    let bytes: Option<Vec<u8>> = con.hget(keys::blob_key(), sha256_hex)?;
    bytes.ok_or_else(|| StoreError::NotFound(format!("blob {sha256_hex}")))
}

/// Translate SHA-1 → SHA-256.
pub fn sha1_to_sha256(
    con: &mut impl ConnectionLike,
    sha1_hex: &str,
) -> Result<String, StoreError> {
    let sha256: Option<String> = con.hget(keys::oid_1to256_key(), sha1_hex)?;
    sha256.ok_or_else(|| StoreError::NotFound(format!("oid mapping {sha1_hex}")))
}

/// Translate SHA-256 → SHA-1.
pub fn sha256_to_sha1(
    con: &mut impl ConnectionLike,
    sha256_hex: &str,
) -> Result<String, StoreError> {
    let sha1: Option<String> = con.hget(keys::oid_256to1_key(), sha256_hex)?;
    sha1.ok_or_else(|| StoreError::NotFound(format!("oid mapping {sha256_hex}")))
}

// ── Log / history queries ────────────────────────────────────────────

/// Recent commits (newest first).
pub fn recent_commits(
    con: &mut impl ConnectionLike,
    ws: &str,
    ent: &str,
    limit: usize,
) -> Result<Vec<CommitEntry>, StoreError> {
    let log_key = keys::log_key(ws, ent);
    let entries: Vec<String> = con.zrevrange(&log_key, 0, (limit - 1) as isize)?;
    entries
        .iter()
        .map(|s| serde_json::from_str(s).map_err(StoreError::from))
        .collect()
}

/// All commits (oldest first).
pub fn all_commits(
    con: &mut impl ConnectionLike,
    ws: &str,
    ent: &str,
) -> Result<Vec<CommitEntry>, StoreError> {
    let log_key = keys::log_key(ws, ent);
    let entries: Vec<String> = con.zrange(&log_key, 0, -1)?;
    entries
        .iter()
        .map(|s| serde_json::from_str(s).map_err(StoreError::from))
        .collect()
}

/// Look up a single commit by SHA-1.
pub fn get_commit(
    con: &mut impl ConnectionLike,
    ws: &str,
    ent: &str,
    sha1_hex: &str,
) -> Result<CommitEntry, StoreError> {
    let sha_key = keys::sha_index_key(ws, ent);
    let seq: Option<f64> = con.hget(&sha_key, sha1_hex)?;
    let seq = seq.ok_or_else(|| StoreError::NotFound(format!("commit {sha1_hex}")))?;

    let log_key = keys::log_key(ws, ent);
    let entries: Vec<String> = con.zrangebyscore(&log_key, seq, seq)?;
    let json = entries
        .first()
        .ok_or_else(|| StoreError::NotFound(format!("commit {sha1_hex} at seq {seq}")))?;
    Ok(serde_json::from_str(json)?)
}

/// Commits that touched a specific file path.
pub fn file_history(
    con: &mut impl ConnectionLike,
    ws: &str,
    ent: &str,
    path: &str,
) -> Result<Vec<CommitEntry>, StoreError> {
    let fh_key = keys::file_history_key(ws, ent, path);
    let shas: Vec<String> = con.zrange(&fh_key, 0, -1)?;

    let mut commits = Vec::with_capacity(shas.len());
    for sha1 in &shas {
        commits.push(get_commit(con, ws, ent, sha1)?);
    }
    Ok(commits)
}

/// List all workspace IDs.
pub fn list_workspaces(
    con: &mut impl ConnectionLike,
) -> Result<Vec<String>, StoreError> {
    Ok(con.smembers(keys::workspaces_key())?)
}

/// List all entity IDs in a workspace.
pub fn list_entities(
    con: &mut impl ConnectionLike,
    ws: &str,
) -> Result<Vec<String>, StoreError> {
    Ok(con.smembers(keys::entities_key(ws))?)
}

// ── Helpers ──────────────────────────────────────────────────────────

fn validate_paths(files: &[FileWrite]) -> Result<(), StoreError> {
    for fw in files {
        if fw.path.is_empty() {
            return Err(StoreError::InvalidPath("empty path".into()));
        }
        if fw.path.starts_with('/') || fw.path.contains("..") {
            return Err(StoreError::InvalidPath(fw.path.clone()));
        }
    }
    Ok(())
}
