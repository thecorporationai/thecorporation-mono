//! Branch merging with JSON-aware conflict resolution.
//!
//! All operations run in-memory against Valkey — no disk I/O, no git.
//! Three-way merge uses JSON field-level conflict resolution for JSON
//! files, with source-wins (LWW) for non-JSON conflicts.

use std::collections::{BTreeMap, HashSet};

use chrono::Utc;
use redis::{Commands, ConnectionLike, pipe};
use serde_json::Value;

use crate::entry::*;
use crate::error::StoreError;
use crate::keys;
use crate::oid::{self, CommitHash, DualOid};

/// Result of a merge operation.
#[derive(Debug)]
pub enum MergeResult {
    FastForward { sha1: String },
    AlreadyUpToDate,
    ThreeWayMerge { sha1: String },
    Squash { sha1: String },
}

/// Merge `source_branch` into `target_branch`.
///
/// Attempts fast-forward first. If branches have diverged, performs
/// a three-way merge with JSON-aware conflict resolution, all
/// in-memory against Valkey.
pub fn merge_branch(
    con: &mut impl ConnectionLike,
    ws: &str,
    ent: &str,
    source_branch: &str,
    target_branch: &str,
    actor: Option<&CommitActor>,
) -> Result<MergeResult, StoreError> {
    let ref_key = keys::ref_key(ws, ent);
    let source_sha: String = con
        .hget(&ref_key, source_branch)
        .ok()
        .flatten()
        .ok_or_else(|| StoreError::RefNotFound(source_branch.to_owned()))?;
    let target_sha: String = con
        .hget(&ref_key, target_branch)
        .ok()
        .flatten()
        .ok_or_else(|| StoreError::RefNotFound(target_branch.to_owned()))?;

    if source_sha == target_sha {
        return Ok(MergeResult::AlreadyUpToDate);
    }

    if is_ancestor(con, ws, ent, &target_sha, &source_sha)? {
        return fast_forward(con, ws, ent, target_branch, source_branch, &source_sha);
    }

    if is_ancestor(con, ws, ent, &source_sha, &target_sha)? {
        return Ok(MergeResult::AlreadyUpToDate);
    }

    merge_in_valkey(con, ws, ent, source_branch, target_branch, actor, false)
}

/// Squash-merge `source_branch` into `target_branch`.
pub fn merge_branch_squash(
    con: &mut impl ConnectionLike,
    ws: &str,
    ent: &str,
    source_branch: &str,
    target_branch: &str,
    actor: Option<&CommitActor>,
) -> Result<MergeResult, StoreError> {
    let ref_key = keys::ref_key(ws, ent);
    let source_sha: String = con
        .hget(&ref_key, source_branch)
        .ok()
        .flatten()
        .ok_or_else(|| StoreError::RefNotFound(source_branch.to_owned()))?;
    let target_sha: String = con
        .hget(&ref_key, target_branch)
        .ok()
        .flatten()
        .ok_or_else(|| StoreError::RefNotFound(target_branch.to_owned()))?;

    if source_sha == target_sha {
        return Ok(MergeResult::AlreadyUpToDate);
    }

    merge_in_valkey(con, ws, ent, source_branch, target_branch, actor, true)
}

// ── Fast-forward ─────────────────────────────────────────────────────

fn fast_forward(
    con: &mut impl ConnectionLike,
    ws: &str,
    ent: &str,
    target_branch: &str,
    source_branch: &str,
    source_sha: &str,
) -> Result<MergeResult, StoreError> {
    let ref_key = keys::ref_key(ws, ent);
    let source_tree_key = keys::tree_key(ws, ent, source_branch);
    let target_tree_key = keys::tree_key(ws, ent, target_branch);

    let tree: BTreeMap<String, String> = con.hgetall(&source_tree_key)?;

    let mut p = pipe();
    p.atomic();
    p.hset(&ref_key, target_branch, source_sha);
    p.del(&target_tree_key);
    for (path, sha1) in &tree {
        p.hset(&target_tree_key, path, sha1);
    }
    p.query::<()>(con)?;

    Ok(MergeResult::FastForward {
        sha1: source_sha.to_owned(),
    })
}

// ── Ancestry check ───────────────────────────────────────────────────

fn is_ancestor(
    con: &mut impl ConnectionLike,
    ws: &str,
    ent: &str,
    ancestor_sha: &str,
    descendant_sha: &str,
) -> Result<bool, StoreError> {
    if ancestor_sha == descendant_sha {
        return Ok(true);
    }

    let mut current = descendant_sha.to_owned();
    let mut visited = HashSet::new();

    loop {
        if visited.contains(&current) {
            return Ok(false);
        }
        visited.insert(current.clone());

        let entry = match crate::store::get_commit(con, ws, ent, &current) {
            Ok(e) => e,
            Err(_) => return Ok(false),
        };

        for parent in &entry.parents {
            if parent == ancestor_sha {
                return Ok(true);
            }
        }

        match entry.parents.first() {
            Some(parent) => current = parent.clone(),
            None => return Ok(false),
        }
    }
}

// ── In-memory merge ──────────────────────────────────────────────────

fn merge_in_valkey(
    con: &mut impl ConnectionLike,
    ws: &str,
    ent: &str,
    source_branch: &str,
    target_branch: &str,
    actor: Option<&CommitActor>,
    squash: bool,
) -> Result<MergeResult, StoreError> {
    let ref_key = keys::ref_key(ws, ent);
    let source_sha: String = con.hget(&ref_key, source_branch)?;
    let target_sha: String = con.hget(&ref_key, target_branch)?;

    // Current tree states (materialized in Valkey).
    let source_tree: BTreeMap<String, String> =
        con.hgetall(keys::tree_key(ws, ent, source_branch))?;
    let target_tree: BTreeMap<String, String> =
        con.hgetall(keys::tree_key(ws, ent, target_branch))?;

    // Find merge base.
    let base_info = find_merge_base(con, ws, ent, &source_sha, &target_sha)?;
    let base_tree = match &base_info {
        Some((_, seq)) => replay_tree_to_seq(con, ws, ent, *seq)?,
        None => BTreeMap::new(),
    };
    let base_sha = base_info.map(|(sha, _)| sha);

    // Three-way merge of trees.
    let merged_tree = three_way_merge_trees(con, &base_tree, &target_tree, &source_tree)?;

    // Build commit message.
    let message = if squash {
        let source_msgs = collect_source_messages(
            con,
            ws,
            ent,
            &source_sha,
            base_sha.as_deref().unwrap_or(""),
        )?;
        let body = source_msgs
            .iter()
            .map(|m| format!("- {}", m.trim()))
            .collect::<Vec<_>>()
            .join("\n");
        format!("squash merge {source_branch} into {target_branch}\n\nSquashed commits:\n{body}")
    } else {
        format!("merge {source_branch} into {target_branch}")
    };

    // Compute changes relative to target.
    let changes = compute_changes(&target_tree, &merged_tree);

    // Compute root tree hash.
    let (root_tree_oid, _) = oid::compute_root_tree(&merged_tree);

    // Build parents list.
    let mut parents = vec![target_sha.clone()];
    if !squash {
        parents.push(source_sha.clone());
    }

    let now = Utc::now();
    let commit_oid = oid::hash_commit(&CommitHash {
        tree_sha1_hex: &root_tree_oid.sha1_hex(),
        parent_sha1_hex: if target_sha.is_empty() {
            None
        } else {
            Some(&target_sha)
        },
        author_name: "corp-engine",
        author_email: "engine@thecorporation.ai",
        author_timestamp: now.timestamp(),
        message: &message,
    });

    let seq: u64 = con.incr(keys::seq_key(ws, ent), 1)?;

    let entry = CommitEntry {
        ld_type: "Commit".to_owned(),
        ld_id: format!("git:sha/{}", commit_oid.sha1_hex()),
        sha1: commit_oid.sha1_hex(),
        sha256: commit_oid.sha256_hex(),
        parents,
        author_name: "corp-engine".to_owned(),
        author_email: "engine@thecorporation.ai".to_owned(),
        message,
        timestamp: now,
        sequence: seq,
        workspace_id: actor.map(|a| a.workspace_id.clone()),
        entity_id: actor.and_then(|a| a.entity_id.clone()),
        scopes: actor.map_or(Vec::new(), |a| a.scopes.clone()),
        signed_by: actor.and_then(|a| a.signed_by.clone()),
        tree_sha1: root_tree_oid.sha1_hex(),
        branch: Some(target_branch.to_owned()),
        changes: changes.clone(),
    };

    let entry_json = serde_json::to_string(&entry)?;

    // Atomic Valkey update.
    let tree_key = keys::tree_key(ws, ent, target_branch);
    {
        let mut p = pipe();
        p.atomic();

        p.zadd(keys::log_key(ws, ent), &entry_json, seq as f64);
        p.hset(keys::sha_index_key(ws, ent), &entry.sha1, seq);
        p.hset(keys::ref_key(ws, ent), target_branch, &entry.sha1);
        p.hset(keys::oid_1to256_key(), &entry.sha1, &entry.sha256);
        p.hset(keys::oid_256to1_key(), &entry.sha256, &entry.sha1);

        // Replace target tree with merged tree.
        p.del(&tree_key);
        for (path, dual) in &merged_tree {
            p.hset(&tree_key, path, dual.sha1_hex());
        }

        // File history.
        for fc in &changes {
            p.zadd(
                keys::file_history_key(ws, ent, &fc.path),
                &entry.sha1,
                seq as f64,
            );
        }

        if let Some(a) = actor {
            p.zadd(
                keys::actor_key(&a.workspace_id),
                format!("{ws}:{ent}:{}", entry.sha1),
                seq as f64,
            );
        }

        p.query::<()>(con)?;
    }

    if squash {
        Ok(MergeResult::Squash { sha1: entry.sha1 })
    } else {
        Ok(MergeResult::ThreeWayMerge { sha1: entry.sha1 })
    }
}

// ── Merge base ───────────────────────────────────────────────────────

fn find_merge_base(
    con: &mut impl ConnectionLike,
    ws: &str,
    ent: &str,
    sha_a: &str,
    sha_b: &str,
) -> Result<Option<(String, u64)>, StoreError> {
    // Collect all ancestors of sha_a.
    let mut ancestors_a: HashSet<String> = HashSet::new();
    let mut current = sha_a.to_owned();
    loop {
        if ancestors_a.contains(&current) {
            break;
        }
        ancestors_a.insert(current.clone());
        match crate::store::get_commit(con, ws, ent, &current) {
            Ok(entry) => match entry.parents.first() {
                Some(parent) => current = parent.clone(),
                None => break,
            },
            Err(_) => break,
        }
    }

    // Walk back from sha_b to find the first common ancestor.
    let mut current = sha_b.to_owned();
    let mut visited = HashSet::new();
    loop {
        if visited.contains(&current) {
            break;
        }
        visited.insert(current.clone());

        if ancestors_a.contains(&current) {
            let entry = crate::store::get_commit(con, ws, ent, &current)?;
            return Ok(Some((current, entry.sequence)));
        }

        match crate::store::get_commit(con, ws, ent, &current) {
            Ok(entry) => match entry.parents.first() {
                Some(parent) => current = parent.clone(),
                None => break,
            },
            Err(_) => break,
        }
    }

    Ok(None)
}

// ── Tree-level three-way merge ───────────────────────────────────────

fn three_way_merge_trees(
    con: &mut impl ConnectionLike,
    base: &BTreeMap<String, String>,
    ours: &BTreeMap<String, String>,
    theirs: &BTreeMap<String, String>,
) -> Result<BTreeMap<String, DualOid>, StoreError> {
    let mut all_paths: BTreeMap<&str, ()> = BTreeMap::new();
    for path in base.keys().chain(ours.keys()).chain(theirs.keys()) {
        all_paths.insert(path, ());
    }

    let mut result: BTreeMap<String, DualOid> = BTreeMap::new();

    for path in all_paths.keys() {
        let b = base.get(*path);
        let o = ours.get(*path);
        let t = theirs.get(*path);

        match (b, o, t) {
            // Both sides have same value — keep it.
            (_, Some(o_sha), Some(t_sha)) if o_sha == t_sha => {
                result.insert(path.to_string(), sha1_to_dual(con, o_sha)?);
            }
            // Both deleted — skip.
            (Some(_), None, None) => {}
            // Ours deleted, theirs unchanged — delete.
            (Some(b_sha), None, Some(t_sha)) if t_sha == b_sha => {}
            // Theirs deleted, ours unchanged — delete.
            (Some(b_sha), Some(o_sha), None) if o_sha == b_sha => {}
            // Ours deleted, theirs changed — keep theirs.
            (Some(_), None, Some(t_sha)) => {
                result.insert(path.to_string(), sha1_to_dual(con, t_sha)?);
            }
            // Theirs deleted, ours changed — keep ours.
            (Some(_), Some(o_sha), None) => {
                result.insert(path.to_string(), sha1_to_dual(con, o_sha)?);
            }
            // Only ours changed from base — keep ours.
            (Some(b_sha), Some(o_sha), Some(_t_sha)) if _t_sha == b_sha => {
                result.insert(path.to_string(), sha1_to_dual(con, o_sha)?);
            }
            // Only theirs changed from base — keep theirs.
            (Some(b_sha), Some(_o_sha), Some(t_sha)) if _o_sha == b_sha => {
                result.insert(path.to_string(), sha1_to_dual(con, t_sha)?);
            }
            // Both changed differently — conflict → JSON merge.
            (_, Some(o_sha), Some(t_sha)) => {
                let resolved =
                    resolve_blob_conflict(con, b.map(|s| s.as_str()), o_sha, t_sha)?;
                result.insert(path.to_string(), resolved);
            }
            // New in ours only.
            (None, Some(o_sha), None) => {
                result.insert(path.to_string(), sha1_to_dual(con, o_sha)?);
            }
            // New in theirs only.
            (None, None, Some(t_sha)) => {
                result.insert(path.to_string(), sha1_to_dual(con, t_sha)?);
            }
            // Not in any tree.
            (None, None, None) => {}
        }
    }

    Ok(result)
}

fn sha1_to_dual(
    con: &mut impl ConnectionLike,
    sha1_hex: &str,
) -> Result<DualOid, StoreError> {
    let sha256_hex: String = con.hget(keys::oid_1to256_key(), sha1_hex)?;
    let sha1: [u8; 20] = hex::decode(sha1_hex)?
        .try_into()
        .map_err(|_| StoreError::NotFound("bad sha1 length".into()))?;
    let sha256: [u8; 32] = hex::decode(&sha256_hex)?
        .try_into()
        .map_err(|_| StoreError::NotFound("bad sha256 length".into()))?;
    Ok(DualOid { sha1, sha256 })
}

fn resolve_blob_conflict(
    con: &mut impl ConnectionLike,
    base_sha1: Option<&str>,
    ours_sha1: &str,
    theirs_sha1: &str,
) -> Result<DualOid, StoreError> {
    let base_content = base_sha1
        .map(|sha1| crate::store::blob_by_sha1(con, sha1))
        .transpose()?;
    let ours_content = crate::store::blob_by_sha1(con, ours_sha1)?;
    let theirs_content = crate::store::blob_by_sha1(con, theirs_sha1)?;

    let merged = resolve_json_conflict(
        base_content.as_deref(),
        Some(&ours_content),
        Some(&theirs_content),
    )?;

    let dual = oid::hash_blob(&merged);

    // Store the merged blob.
    let _: () = redis::cmd("HSETNX")
        .arg(keys::blob_key())
        .arg(dual.sha256_hex())
        .arg(&merged)
        .query(con)?;
    let _: () = redis::cmd("HSETNX")
        .arg(keys::oid_1to256_key())
        .arg(dual.sha1_hex())
        .arg(dual.sha256_hex())
        .query(con)?;
    let _: () = redis::cmd("HSETNX")
        .arg(keys::oid_256to1_key())
        .arg(dual.sha256_hex())
        .arg(dual.sha1_hex())
        .query(con)?;

    Ok(dual)
}

// ── Squash helpers ──────────────────────────────────────────────────

fn collect_source_messages(
    con: &mut impl ConnectionLike,
    ws: &str,
    ent: &str,
    head_sha: &str,
    base_sha: &str,
) -> Result<Vec<String>, StoreError> {
    let mut messages = Vec::new();
    let mut current = head_sha.to_owned();

    while current != base_sha {
        let entry = match crate::store::get_commit(con, ws, ent, &current) {
            Ok(e) => e,
            Err(_) => break,
        };
        messages.push(entry.message);
        match entry.parents.first() {
            Some(parent) => current = parent.clone(),
            None => break,
        }
    }

    messages.reverse();
    Ok(messages)
}

// ── Tree replay ──────────────────────────────────────────────────────

fn replay_tree_to_seq(
    con: &mut impl ConnectionLike,
    ws: &str,
    ent: &str,
    target_seq: u64,
) -> Result<BTreeMap<String, String>, StoreError> {
    let log_key = keys::log_key(ws, ent);
    let entries: Vec<String> = con.zrangebyscore(&log_key, 0f64, target_seq as f64)?;

    let mut tree: BTreeMap<String, String> = BTreeMap::new();
    for json in &entries {
        let entry: CommitEntry = serde_json::from_str(json)?;
        for fc in &entry.changes {
            match fc.action {
                ChangeAction::Add | ChangeAction::Modify => {
                    if let Some(ref sha1) = fc.blob_sha1 {
                        tree.insert(fc.path.clone(), sha1.clone());
                    }
                }
                ChangeAction::Delete => {
                    tree.remove(&fc.path);
                }
                ChangeAction::Rename => {
                    if let Some(ref old) = fc.old_path {
                        tree.remove(old);
                    }
                    if let Some(ref sha1) = fc.blob_sha1 {
                        tree.insert(fc.path.clone(), sha1.clone());
                    }
                }
            }
        }
    }

    Ok(tree)
}

// ── JSON merge ──────────────────────────────────────────────────────

fn resolve_json_conflict(
    ancestor: Option<&[u8]>,
    ours: Option<&[u8]>,
    theirs: Option<&[u8]>,
) -> Result<Vec<u8>, StoreError> {
    match (ours, theirs) {
        (None, None) => {
            return Err(StoreError::Git("both sides deleted".into()));
        }
        (None, Some(theirs_bytes)) => return Ok(theirs_bytes.to_vec()),
        (Some(ours_bytes), None) => return Ok(ours_bytes.to_vec()),
        (Some(_), Some(_)) => {}
    }

    let ours_bytes = ours.unwrap();
    let theirs_bytes = theirs.unwrap();

    let ancestor_val: Option<Value> = ancestor
        .map(|b| {
            serde_json::from_slice(b)
                .map_err(|e| StoreError::Git(format!("ancestor JSON: {e}")))
        })
        .transpose()?;
    let ours_val: Value = serde_json::from_slice(ours_bytes)
        .map_err(|e| StoreError::Git(format!("ours JSON: {e}")))?;
    let theirs_val: Value = serde_json::from_slice(theirs_bytes)
        .map_err(|e| StoreError::Git(format!("theirs JSON: {e}")))?;

    let base = ancestor_val.unwrap_or(Value::Object(serde_json::Map::new()));

    let merged = merge_json_values(&base, &ours_val, &theirs_val);

    serde_json::to_vec_pretty(&merged)
        .map_err(|e| StoreError::Git(format!("serialize: {e}")))
}

fn merge_json_values(base: &Value, ours: &Value, theirs: &Value) -> Value {
    if let (Value::Object(base_map), Value::Object(ours_map), Value::Object(theirs_map)) =
        (base, ours, theirs)
    {
        return merge_json_objects(base_map, ours_map, theirs_map);
    }

    if ours == base && theirs != base {
        theirs.clone()
    } else if theirs == base && ours != base {
        ours.clone()
    } else if ours == theirs {
        ours.clone()
    } else {
        theirs.clone() // source wins (LWW)
    }
}

fn merge_json_objects(
    base: &serde_json::Map<String, Value>,
    ours: &serde_json::Map<String, Value>,
    theirs: &serde_json::Map<String, Value>,
) -> Value {
    let mut all_keys: BTreeMap<&str, ()> = BTreeMap::new();
    for k in base.keys().chain(ours.keys()).chain(theirs.keys()) {
        all_keys.insert(k, ());
    }

    let absent = Value::Null;
    let mut result = serde_json::Map::new();

    for key in all_keys.keys() {
        let b = base.get(*key);
        let o = ours.get(*key);
        let t = theirs.get(*key);

        let in_base = b.is_some();
        let in_ours = o.is_some();
        let in_theirs = t.is_some();

        // Both deleted.
        if in_base && !in_ours && !in_theirs {
            continue;
        }
        // Ours deleted.
        if in_base && !in_ours && in_theirs {
            continue;
        }
        // Theirs deleted.
        if in_base && in_ours && !in_theirs {
            continue;
        }

        // New key added by one side only.
        if !in_base {
            if in_ours && !in_theirs {
                result.insert(key.to_string(), o.unwrap().clone());
                continue;
            }
            if !in_ours && in_theirs {
                result.insert(key.to_string(), t.unwrap().clone());
                continue;
            }
        }

        let bv = b.unwrap_or(&absent);
        let ov = o.unwrap_or(&absent);
        let tv = t.unwrap_or(&absent);
        result.insert(key.to_string(), merge_json_values(bv, ov, tv));
    }

    Value::Object(result)
}

// ── Change computation ──────────────────────────────────────────────

fn compute_changes(
    old_tree: &BTreeMap<String, String>,
    new_tree: &BTreeMap<String, DualOid>,
) -> Vec<FileChange> {
    let mut changes = Vec::new();

    for (path, new_oid) in new_tree {
        let action = match old_tree.get(path) {
            Some(old_sha1) if *old_sha1 == new_oid.sha1_hex() => continue,
            Some(_) => ChangeAction::Modify,
            None => ChangeAction::Add,
        };
        changes.push(FileChange {
            path: path.clone(),
            action,
            blob_sha1: Some(new_oid.sha1_hex()),
            blob_sha256: Some(new_oid.sha256_hex()),
            old_path: None,
        });
    }

    for path in old_tree.keys() {
        if !new_tree.contains_key(path) {
            changes.push(FileChange {
                path: path.clone(),
                action: ChangeAction::Delete,
                blob_sha1: None,
                blob_sha256: None,
                old_path: None,
            });
        }
    }

    changes
}
