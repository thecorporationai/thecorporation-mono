//! Git pack protocol integration layer.
//!
//! Functions for processing git push (receive-pack) and serving git
//! fetch/clone (upload-pack) against the Valkey-backed store.

use std::collections::{BTreeMap, HashSet, VecDeque};

use chrono::{DateTime, Utc};
use redis::{Commands, ConnectionLike, pipe};

use crate::entry::*;
use crate::error::StoreError;
use crate::keys;
use crate::oid;
use crate::store::{self, GitObjectType};

// ── Types ────────────────────────────────────────────────────────────

/// A ref update from a git push.
#[derive(Debug, Clone)]
pub struct RefUpdate {
    /// Old SHA-1 (40 hex chars, or "0000000000000000000000000000000000000000" for new ref).
    pub old_sha1: String,
    /// New SHA-1 (40 hex chars, or all-zeros for delete).
    pub new_sha1: String,
    /// Full ref name (e.g., "refs/heads/main").
    pub refname: String,
}

impl RefUpdate {
    /// Extract the short branch name from a full refname.
    pub fn branch_name(&self) -> Option<&str> {
        self.refname.strip_prefix("refs/heads/")
    }

    pub fn is_create(&self) -> bool {
        self.old_sha1.chars().all(|c| c == '0')
    }

    pub fn is_delete(&self) -> bool {
        self.new_sha1.chars().all(|c| c == '0')
    }
}

/// A parsed git object from an incoming pack.
#[derive(Debug, Clone)]
pub struct ParsedObject {
    pub sha1_hex: String,
    pub obj_type: GitObjectType,
    /// Raw object content (without `{type} {len}\0` header).
    pub content: Vec<u8>,
}

/// Result of processing a ref update.
#[derive(Debug)]
pub struct RefUpdateResult {
    pub refname: String,
    pub ok: bool,
    pub error: Option<String>,
}

// ── Receive-pack (push) ──────────────────────────────────────────────

/// Process a git push: store objects, update refs, update indexes.
///
/// This is the core function called after the incoming pack has been
/// parsed into individual objects. It:
///
/// 1. Stores all objects (blobs, trees, commits) in Valkey
/// 2. Validates and applies ref updates
/// 3. Updates materialized tree state for each affected branch
/// 4. Inserts commit entries into the log for REST API visibility
pub fn receive_push(
    con: &mut impl ConnectionLike,
    ws: &str,
    ent: &str,
    ref_updates: &[RefUpdate],
    objects: &[ParsedObject],
) -> Result<Vec<RefUpdateResult>, StoreError> {
    // 1. Store all objects.
    for obj in objects {
        let oid = match obj.obj_type {
            GitObjectType::Blob => oid::hash_blob(&obj.content),
            GitObjectType::Tree => oid::hash_tree_raw(&obj.content),
            GitObjectType::Commit => oid::hash_commit_raw(&obj.content),
        };
        store::store_raw_git_object(
            con,
            &oid.sha1_hex(),
            &oid.sha256_hex(),
            obj.obj_type,
            &obj.content,
        )?;
    }

    // 2. Process ref updates.
    let mut results = Vec::new();
    for update in ref_updates {
        let branch = match update.branch_name() {
            Some(b) => b,
            None => {
                results.push(RefUpdateResult {
                    refname: update.refname.clone(),
                    ok: false,
                    error: Some("only refs/heads/* supported".to_owned()),
                });
                continue;
            }
        };

        if update.is_delete() {
            // Delete branch.
            match crate::branch::delete_branch(con, ws, ent, branch) {
                Ok(()) => results.push(RefUpdateResult {
                    refname: update.refname.clone(),
                    ok: true,
                    error: None,
                }),
                Err(e) => results.push(RefUpdateResult {
                    refname: update.refname.clone(),
                    ok: false,
                    error: Some(e.to_string()),
                }),
            }
            continue;
        }

        // Verify old SHA matches (fast-forward check).
        if !update.is_create() {
            let ref_key = keys::ref_key(ws, ent);
            let current: Option<String> = con.hget(&ref_key, branch)?;
            match current {
                Some(ref cur) if cur != &update.old_sha1 => {
                    results.push(RefUpdateResult {
                        refname: update.refname.clone(),
                        ok: false,
                        error: Some(format!(
                            "non-fast-forward: expected {}, got {}",
                            update.old_sha1, cur
                        )),
                    });
                    continue;
                }
                None => {
                    results.push(RefUpdateResult {
                        refname: update.refname.clone(),
                        ok: false,
                        error: Some(format!("ref not found for update: {branch}")),
                    });
                    continue;
                }
                _ => {}
            }
        }

        // 3. Flatten the new tip commit's tree into materialized tree state.
        match materialize_tree_from_commit(con, ws, ent, branch, &update.new_sha1) {
            Ok(()) => {}
            Err(e) => {
                results.push(RefUpdateResult {
                    refname: update.refname.clone(),
                    ok: false,
                    error: Some(format!("tree materialization failed: {e}")),
                });
                continue;
            }
        }

        // 4. Insert commit entries into the log.
        if let Err(e) = insert_commits_into_log(
            con,
            ws,
            ent,
            &update.new_sha1,
            if update.is_create() { None } else { Some(&update.old_sha1) },
        ) {
            tracing::warn!(
                ws = ws, ent = ent, branch = branch,
                "failed to insert commits into log: {e}"
            );
            // Non-fatal — tree state and ref are still consistent.
        }

        results.push(RefUpdateResult {
            refname: update.refname.clone(),
            ok: true,
            error: None,
        });
    }

    Ok(results)
}

/// Walk the commit's tree object and materialize it as a flat path→SHA-1 map.
fn materialize_tree_from_commit(
    con: &mut impl ConnectionLike,
    ws: &str,
    ent: &str,
    branch: &str,
    commit_sha1: &str,
) -> Result<(), StoreError> {
    // Read the commit to get tree SHA-1.
    let (obj_type, commit_content) = store::read_raw_git_object(con, commit_sha1)?;
    if obj_type != GitObjectType::Commit {
        return Err(StoreError::Git(format!(
            "expected commit, got {:?} for {commit_sha1}",
            obj_type
        )));
    }

    let commit_text = std::str::from_utf8(&commit_content)
        .map_err(|e| StoreError::Git(format!("invalid commit UTF-8: {e}")))?;

    let tree_sha1 = parse_tree_from_commit(commit_text)?;

    // Recursively flatten the tree into path→blob_sha1.
    let mut flat_tree: BTreeMap<String, String> = BTreeMap::new();
    flatten_tree(con, &tree_sha1, "", &mut flat_tree)?;

    // Replace the materialized tree state atomically.
    let tree_key = keys::tree_key(ws, ent, branch);
    let ref_key = keys::ref_key(ws, ent);

    let mut p = pipe();
    p.atomic();
    // Delete old tree state.
    p.del(&tree_key);
    // Write new tree state.
    for (path, sha1) in &flat_tree {
        p.hset(&tree_key, path, sha1);
    }
    // Update ref.
    p.hset(&ref_key, branch, commit_sha1);
    // Ensure workspace/entity tracking.
    p.sadd(keys::workspaces_key(), ws);
    p.sadd(keys::entities_key(ws), ent);
    p.query::<()>(con)?;

    Ok(())
}

/// Parse the tree SHA-1 from a raw git commit body.
fn parse_tree_from_commit(commit_text: &str) -> Result<String, StoreError> {
    for line in commit_text.lines() {
        if let Some(tree_hex) = line.strip_prefix("tree ") {
            return Ok(tree_hex.trim().to_owned());
        }
        if line.is_empty() {
            break; // End of headers.
        }
    }
    Err(StoreError::Git("commit missing tree header".to_owned()))
}

/// Parse parent SHA-1s from a raw git commit body.
fn parse_parents_from_commit(commit_text: &str) -> Vec<String> {
    let mut parents = Vec::new();
    for line in commit_text.lines() {
        if let Some(parent_hex) = line.strip_prefix("parent ") {
            parents.push(parent_hex.trim().to_owned());
        }
        if line.is_empty() {
            break;
        }
    }
    parents
}

/// Recursively flatten a git tree object into path→blob_sha1 entries.
fn flatten_tree(
    con: &mut impl ConnectionLike,
    tree_sha1: &str,
    prefix: &str,
    out: &mut BTreeMap<String, String>,
) -> Result<(), StoreError> {
    let (_obj_type, raw) = store::read_raw_git_object(con, tree_sha1)?;

    // Parse git tree format: {mode} {name}\0{20-byte-sha1} repeated.
    let mut pos = 0;
    while pos < raw.len() {
        // Find the space between mode and name.
        let space = raw[pos..]
            .iter()
            .position(|&b| b == b' ')
            .ok_or_else(|| StoreError::Git("malformed tree entry: no space".to_owned()))?;
        let mode = &raw[pos..pos + space];
        pos += space + 1;

        // Find the null between name and SHA-1.
        let null = raw[pos..]
            .iter()
            .position(|&b| b == 0)
            .ok_or_else(|| StoreError::Git("malformed tree entry: no null".to_owned()))?;
        let name = std::str::from_utf8(&raw[pos..pos + null])
            .map_err(|e| StoreError::Git(format!("invalid tree entry name: {e}")))?;
        pos += null + 1;

        // Read 20-byte raw SHA-1.
        if pos + 20 > raw.len() {
            return Err(StoreError::Git("malformed tree entry: truncated SHA-1".to_owned()));
        }
        let sha1_bytes = &raw[pos..pos + 20];
        let sha1_hex = hex::encode(sha1_bytes);
        pos += 20;

        let full_path = if prefix.is_empty() {
            name.to_owned()
        } else {
            format!("{prefix}{name}")
        };

        if mode == b"40000" {
            // Subdirectory — recurse.
            flatten_tree(con, &sha1_hex, &format!("{full_path}/"), out)?;
        } else {
            // File blob.
            out.insert(full_path, sha1_hex);
        }
    }

    Ok(())
}

/// Walk from new_sha1 back to stop_sha1, inserting CommitEntry records.
fn insert_commits_into_log(
    con: &mut impl ConnectionLike,
    ws: &str,
    ent: &str,
    new_sha1: &str,
    stop_sha1: Option<&str>,
) -> Result<(), StoreError> {
    // Collect commits from new_sha1 back to stop_sha1 (exclusive).
    let mut to_process = VecDeque::new();
    let mut visited = HashSet::new();
    to_process.push_back(new_sha1.to_owned());

    let mut commits_in_order = Vec::new();

    while let Some(sha1) = to_process.pop_front() {
        if !visited.insert(sha1.clone()) {
            continue;
        }
        if stop_sha1.is_some_and(|s| s == sha1) {
            continue;
        }
        // Skip if already in the log.
        let sha_key = keys::sha_index_key(ws, ent);
        let existing: Option<f64> = con.hget(&sha_key, &sha1)?;
        if existing.is_some() {
            continue;
        }

        let (obj_type, content) = store::read_raw_git_object(con, &sha1)?;
        if obj_type != GitObjectType::Commit {
            continue;
        }
        let text = String::from_utf8_lossy(&content);
        let parents = parse_parents_from_commit(&text);
        for parent in &parents {
            to_process.push_back(parent.clone());
        }
        commits_in_order.push((sha1, text.into_owned(), parents));
    }

    // Insert in reverse order (oldest first) so sequence numbers are ordered.
    commits_in_order.reverse();

    for (sha1, text, parents) in &commits_in_order {
        let tree_sha1 = parse_tree_from_commit(text)?;
        let (author_name, author_email, timestamp) = parse_author_from_commit(text);

        // Extract message (everything after blank line).
        let message = text
            .split_once("\n\n")
            .map(|(_, msg)| msg.trim_end())
            .unwrap_or("")
            .to_owned();

        let sha256_hex = store::sha1_to_sha256(con, sha1)?;
        let seq: u64 = con.incr(keys::seq_key(ws, ent), 1)?;

        let entry = CommitEntry {
            ld_type: "Commit".to_owned(),
            ld_id: format!("git:sha/{sha1}"),
            sha1: sha1.clone(),
            sha256: sha256_hex,
            parents: parents.clone(),
            author_name: author_name.clone(),
            author_email: author_email.clone(),
            message,
            timestamp,
            sequence: seq,
            workspace_id: Some(ws.to_owned()),
            entity_id: Some(ent.to_owned()),
            scopes: Vec::new(),
            signed_by: None,
            tree_sha1,
            changes: Vec::new(), // We don't compute per-file diffs for pushed commits.
        };

        let entry_json = serde_json::to_string(&entry)?;

        let mut p = pipe();
        p.atomic();
        p.zadd(keys::log_key(ws, ent), &entry_json, seq as f64);
        p.hset(keys::sha_index_key(ws, ent), &entry.sha1, seq);
        p.hset(keys::oid_1to256_key(), &entry.sha1, &entry.sha256);
        p.hset(keys::oid_256to1_key(), &entry.sha256, &entry.sha1);
        p.query::<()>(con)?;
    }

    Ok(())
}

/// Parse author name, email, and timestamp from commit text.
fn parse_author_from_commit(text: &str) -> (String, String, DateTime<Utc>) {
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("author ") {
            // Format: "Name <email> timestamp +0000"
            if let Some((name_email, ts_tz)) = rest.rsplit_once('>') {
                let name_email = format!("{name_email}>");
                let name = name_email
                    .split('<')
                    .next()
                    .unwrap_or("unknown")
                    .trim()
                    .to_owned();
                let email = name_email
                    .split('<')
                    .nth(1)
                    .and_then(|s| s.strip_suffix('>'))
                    .unwrap_or("unknown@unknown")
                    .to_owned();
                let ts_str = ts_tz.trim().split_whitespace().next().unwrap_or("0");
                let ts: i64 = ts_str.parse().unwrap_or(0);
                let timestamp = DateTime::from_timestamp(ts, 0).unwrap_or_default();
                return (name, email, timestamp);
            }
        }
        if line.is_empty() {
            break;
        }
    }
    ("unknown".to_owned(), "unknown@unknown".to_owned(), Utc::now())
}

// ── Upload-pack (fetch/clone) ────────────────────────────────────────

/// Enumerate all git objects needed for a fetch.
///
/// Walks the commit graph from `wants` (SHAs the client wants),
/// stopping at `haves` (SHAs the client already has). Returns all
/// reachable objects (commits, trees, blobs) as `(sha1, type, content)`.
pub fn enumerate_objects_for_fetch(
    con: &mut impl ConnectionLike,
    wants: &[String],
    haves: &[String],
) -> Result<Vec<(String, GitObjectType, Vec<u8>)>, StoreError> {
    // Build set of objects already available to the client.
    let mut have_set = HashSet::new();
    for sha1 in haves {
        collect_reachable_objects(con, sha1, &mut have_set)?;
    }

    // Walk from wanted commits, collecting all objects not in have_set.
    let mut needed = Vec::new();
    let mut visited = HashSet::new();

    let mut queue = VecDeque::new();
    for sha1 in wants {
        queue.push_back(sha1.clone());
    }

    while let Some(sha1) = queue.pop_front() {
        if !visited.insert(sha1.clone()) {
            continue;
        }
        if have_set.contains(&sha1) {
            continue;
        }

        let (obj_type, content) = match store::read_raw_git_object(con, &sha1) {
            Ok(r) => r,
            Err(_) => continue,
        };

        needed.push((sha1.clone(), obj_type, content.clone()));

        match obj_type {
            GitObjectType::Commit => {
                let text = String::from_utf8_lossy(&content);
                // Queue parent commits.
                for parent in parse_parents_from_commit(&text) {
                    queue.push_back(parent);
                }
                // Queue tree.
                if let Ok(tree_sha1) = parse_tree_from_commit(&text) {
                    queue.push_back(tree_sha1);
                }
            }
            GitObjectType::Tree => {
                // Queue all entries (subtrees + blobs).
                for sha1 in parse_tree_entries_sha1(&content) {
                    queue.push_back(sha1);
                }
            }
            GitObjectType::Blob => {
                // Leaf — nothing to queue.
            }
        }
    }

    Ok(needed)
}

/// Collect all SHA-1s reachable from a commit (transitively).
fn collect_reachable_objects(
    con: &mut impl ConnectionLike,
    sha1: &str,
    out: &mut HashSet<String>,
) -> Result<(), StoreError> {
    let mut queue = VecDeque::new();
    queue.push_back(sha1.to_owned());

    while let Some(s) = queue.pop_front() {
        if !out.insert(s.clone()) {
            continue;
        }
        let (obj_type, content) = match store::read_raw_git_object(con, &s) {
            Ok(r) => r,
            Err(_) => continue,
        };
        match obj_type {
            GitObjectType::Commit => {
                let text = String::from_utf8_lossy(&content);
                for parent in parse_parents_from_commit(&text) {
                    queue.push_back(parent);
                }
                if let Ok(tree) = parse_tree_from_commit(&text) {
                    queue.push_back(tree);
                }
            }
            GitObjectType::Tree => {
                for entry_sha1 in parse_tree_entries_sha1(&content) {
                    queue.push_back(entry_sha1);
                }
            }
            GitObjectType::Blob => {}
        }
    }
    Ok(())
}

/// Extract SHA-1 hex strings from raw tree content.
fn parse_tree_entries_sha1(raw: &[u8]) -> Vec<String> {
    let mut entries = Vec::new();
    let mut pos = 0;
    while pos < raw.len() {
        // Skip mode.
        let Some(space) = raw[pos..].iter().position(|&b| b == b' ') else { break };
        pos += space + 1;
        // Skip name.
        let Some(null) = raw[pos..].iter().position(|&b| b == 0) else { break };
        pos += null + 1;
        // Read 20-byte SHA-1.
        if pos + 20 > raw.len() {
            break;
        }
        entries.push(hex::encode(&raw[pos..pos + 20]));
        pos += 20;
    }
    entries
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_tree_from_commit_basic() {
        let commit = "tree abcd1234abcd1234abcd1234abcd1234abcd1234\n\
                       parent 1111111111111111111111111111111111111111\n\
                       author Test <test@test.com> 1000000000 +0000\n\
                       committer Test <test@test.com> 1000000000 +0000\n\
                       \n\
                       Initial commit\n";
        let tree = parse_tree_from_commit(commit).unwrap();
        assert_eq!(tree, "abcd1234abcd1234abcd1234abcd1234abcd1234");
    }

    #[test]
    fn parse_parents_basic() {
        let commit = "tree 0000000000000000000000000000000000000000\n\
                       parent aaaa000000000000000000000000000000000000\n\
                       parent bbbb000000000000000000000000000000000000\n\
                       author A <a@a> 0 +0000\n\
                       committer A <a@a> 0 +0000\n\
                       \n\
                       merge\n";
        let parents = parse_parents_from_commit(commit);
        assert_eq!(parents.len(), 2);
        assert_eq!(parents[0], "aaaa000000000000000000000000000000000000");
        assert_eq!(parents[1], "bbbb000000000000000000000000000000000000");
    }

    #[test]
    fn parse_author_basic() {
        let commit = "tree 0000\nauthor John Doe <john@example.com> 1609459200 +0000\n\ntest\n";
        let (name, email, ts) = parse_author_from_commit(commit);
        assert_eq!(name, "John Doe");
        assert_eq!(email, "john@example.com");
        assert_eq!(ts.timestamp(), 1609459200);
    }

    #[test]
    fn ref_update_helpers() {
        let create = RefUpdate {
            old_sha1: "0000000000000000000000000000000000000000".to_owned(),
            new_sha1: "abcd".to_owned(),
            refname: "refs/heads/feature".to_owned(),
        };
        assert!(create.is_create());
        assert!(!create.is_delete());
        assert_eq!(create.branch_name(), Some("feature"));

        let delete = RefUpdate {
            old_sha1: "abcd".to_owned(),
            new_sha1: "0000000000000000000000000000000000000000".to_owned(),
            refname: "refs/heads/old".to_owned(),
        };
        assert!(delete.is_delete());
        assert!(!delete.is_create());
    }
}
