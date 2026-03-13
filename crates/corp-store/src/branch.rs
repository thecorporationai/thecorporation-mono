//! Branch management for the Valkey store.
//!
//! Branches are just pointers: a ref (commit SHA-1) and a tree state snapshot.
//! Creating/deleting branches is O(1) ref operations + O(n) tree copy where
//! n is the number of files.

use redis::{Commands, ConnectionLike, pipe};

use crate::error::StoreError;
use crate::keys;

/// Summary of a branch.
#[derive(Debug, Clone)]
pub struct BranchInfo {
    pub name: String,
    pub head_sha1: String,
}

/// Create a new branch pointing at the same commit as `from_branch`.
///
/// Copies the ref and the full tree state so reads on the new branch
/// work immediately without replay.
pub fn create_branch(
    con: &mut impl ConnectionLike,
    ws: &str,
    ent: &str,
    name: &str,
    from_branch: &str,
) -> Result<BranchInfo, StoreError> {
    let ref_key = keys::ref_key(ws, ent);

    // Check it doesn't already exist.
    let exists: bool = con.hexists(&ref_key, name)?;
    if exists {
        return Err(StoreError::Git(format!("branch already exists: {name}")));
    }

    // Resolve source branch.
    let source_sha1: Option<String> = con.hget(&ref_key, from_branch)?;
    let source_sha1 =
        source_sha1.ok_or_else(|| StoreError::RefNotFound(format!("{from_branch}")))?;

    // Copy tree state.
    let source_tree_key = keys::tree_key(ws, ent, from_branch);
    let tree: std::collections::BTreeMap<String, String> = con.hgetall(&source_tree_key)?;

    let new_tree_key = keys::tree_key(ws, ent, name);

    let mut p = pipe();
    p.atomic();
    p.hset(&ref_key, name, &source_sha1);
    for (path, sha1) in &tree {
        p.hset(&new_tree_key, path, sha1);
    }
    p.query::<()>(con)?;

    tracing::debug!(ws = ws, ent = ent, branch = name, from = from_branch, "branch created");

    Ok(BranchInfo {
        name: name.to_owned(),
        head_sha1: source_sha1,
    })
}

/// List all branches.
pub fn list_branches(
    con: &mut impl ConnectionLike,
    ws: &str,
    ent: &str,
) -> Result<Vec<BranchInfo>, StoreError> {
    let ref_key = keys::ref_key(ws, ent);
    let refs: std::collections::BTreeMap<String, String> = con.hgetall(&ref_key)?;

    Ok(refs
        .into_iter()
        .map(|(name, sha1)| BranchInfo {
            name,
            head_sha1: sha1,
        })
        .collect())
}

/// Delete a branch. Cannot delete `"main"`.
pub fn delete_branch(
    con: &mut impl ConnectionLike,
    ws: &str,
    ent: &str,
    name: &str,
) -> Result<(), StoreError> {
    if name == "main" {
        return Err(StoreError::Git("cannot delete the main branch".into()));
    }

    let ref_key = keys::ref_key(ws, ent);
    let exists: bool = con.hexists(&ref_key, name)?;
    if !exists {
        return Err(StoreError::RefNotFound(name.to_owned()));
    }

    let tree_key = keys::tree_key(ws, ent, name);

    let mut p = pipe();
    p.atomic();
    p.hdel(&ref_key, name);
    p.del(&tree_key);
    p.query::<()>(con)?;

    tracing::debug!(ws = ws, ent = ent, branch = name, "branch deleted");
    Ok(())
}
