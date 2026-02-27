//! Branch merging.
//!
//! Supports fast-forward merges and three-way merges with JSON-aware
//! conflict resolution. When branches diverge, a standard git three-way
//! merge is attempted first (line-level). If line-level conflicts remain,
//! JSON field-level last-writer-wins (source branch wins) resolves them.

use git2::Oid;
use serde_json::Value;
use std::collections::BTreeMap;

use super::error::GitStorageError;
use super::repo::CorpRepo;

/// Result of a merge operation.
#[derive(Debug)]
pub enum MergeResult {
    /// The target ref was moved forward to the source commit.
    FastForward { new_oid: Oid },
    /// The target already contained all commits from source.
    AlreadyUpToDate,
    /// A three-way merge commit was created.
    ThreeWayMerge { new_oid: Oid },
}

/// Merge `source_branch` into `target_branch`.
///
/// Attempts fast-forward first. If branches have diverged, falls back to
/// three-way merge with JSON-aware conflict resolution.
pub fn merge_branch(
    repo: &CorpRepo,
    source_branch: &str,
    target_branch: &str,
) -> Result<MergeResult, GitStorageError> {
    let source_oid = repo.resolve_ref(source_branch)?;
    let target_oid = repo.resolve_ref(target_branch)?;

    // If they point to the same commit, nothing to do.
    if source_oid == target_oid {
        return Ok(MergeResult::AlreadyUpToDate);
    }

    // Check if target is an ancestor of source (i.e. fast-forward is possible).
    let git = repo.inner();
    let target_is_ancestor = git.graph_descendant_of(source_oid, target_oid)?;

    if !target_is_ancestor {
        // Also check the reverse — maybe source is behind target.
        let source_is_ancestor = git.graph_descendant_of(target_oid, source_oid)?;
        if source_is_ancestor {
            // Target already contains all of source's commits.
            return Ok(MergeResult::AlreadyUpToDate);
        }

        // Branches have diverged — attempt three-way merge.
        return merge_three_way(repo, source_oid, target_oid, target_branch);
    }

    // Fast-forward: update the target ref to point to source's HEAD.
    let target_full = CorpRepo::normalize_ref(target_branch);
    git.reference(
        &target_full,
        source_oid,
        true, // force — overwrite existing ref
        &format!("fast-forward merge {source_branch} into {target_branch}"),
    )?;

    tracing::debug!(
        source = %source_branch,
        target = %target_branch,
        oid = %source_oid,
        "fast-forward merge"
    );

    Ok(MergeResult::FastForward {
        new_oid: source_oid,
    })
}

/// Perform a three-way merge when branches have diverged.
///
/// Uses `git2::merge_commits` for line-level merge, then resolves remaining
/// conflicts using JSON field-level last-writer-wins (source wins).
fn merge_three_way(
    repo: &CorpRepo,
    source_oid: Oid,
    target_oid: Oid,
    target_branch: &str,
) -> Result<MergeResult, GitStorageError> {
    let git = repo.inner();

    let source_commit = git.find_commit(source_oid)?;
    let target_commit = git.find_commit(target_oid)?;

    // Perform three-way merge (target = "ours", source = "theirs").
    let mut index = git.merge_commits(&target_commit, &source_commit, None)?;

    if index.has_conflicts() {
        // Collect conflicts and try to resolve them via JSON field-level merge.
        let conflicts: Vec<_> = index.conflicts()?.collect::<Result<_, _>>()?;
        let mut unresolved = Vec::new();

        for conflict in &conflicts {
            let path = conflict_path(conflict);

            let ancestor_bytes = conflict
                .ancestor
                .as_ref()
                .map(|e| read_blob_bytes(git, e.id))
                .transpose()?;
            let ours_bytes = conflict
                .our
                .as_ref()
                .map(|e| read_blob_bytes(git, e.id))
                .transpose()?;
            let theirs_bytes = conflict
                .their
                .as_ref()
                .map(|e| read_blob_bytes(git, e.id))
                .transpose()?;

            match resolve_json_conflict(
                ancestor_bytes.as_deref(),
                ours_bytes.as_deref(),
                theirs_bytes.as_deref(),
            ) {
                Ok(merged_bytes) => {
                    let blob_oid = git.blob(&merged_bytes)?;
                    // Build an index entry for the resolved file.
                    let mut entry = git2::IndexEntry {
                        ctime: git2::IndexTime::new(0, 0),
                        mtime: git2::IndexTime::new(0, 0),
                        dev: 0,
                        ino: 0,
                        mode: 0o100644,
                        uid: 0,
                        gid: 0,
                        file_size: merged_bytes.len() as u32,
                        id: blob_oid,
                        flags: 0,
                        flags_extended: 0,
                        path: path.clone().into_bytes(),
                    };
                    index.add(&entry)?;
                    // Remove the conflict markers (stages 1, 2, 3).
                    index.conflict_remove(std::path::Path::new(&path))?;
                    // Re-add at stage 0 after conflict removal.
                    entry.flags = 0;
                    index.add(&entry)?;
                }
                Err(_) => {
                    unresolved.push(path);
                }
            }
        }

        if !unresolved.is_empty() {
            return Err(GitStorageError::MergeConflict(format!(
                "unresolvable conflicts in: {}",
                unresolved.join(", ")
            )));
        }
    }

    // Write the merged tree.
    let tree_oid = index.write_tree_to(git)?;
    let merged_tree = git.find_tree(tree_oid)?;

    // Create merge commit with two parents.
    let sig = CorpRepo::signature()?;
    let target_full = CorpRepo::normalize_ref(target_branch);

    let commit_oid = git.commit(
        Some(&target_full),
        &sig,
        &sig,
        &format!("merge into {target_branch}"),
        &merged_tree,
        &[&target_commit, &source_commit],
    )?;

    tracing::debug!(
        target = %target_branch,
        oid = %commit_oid,
        "three-way merge"
    );

    Ok(MergeResult::ThreeWayMerge {
        new_oid: commit_oid,
    })
}

/// Extract the path from a conflict entry (preferring ours, then theirs, then ancestor).
fn conflict_path(conflict: &git2::IndexConflict) -> String {
    conflict
        .our
        .as_ref()
        .or(conflict.their.as_ref())
        .or(conflict.ancestor.as_ref())
        .and_then(|e| std::str::from_utf8(&e.path).ok())
        .unwrap_or("<unknown>")
        .to_owned()
}

/// Read blob content from the ODB by OID.
fn read_blob_bytes(git: &git2::Repository, oid: Oid) -> Result<Vec<u8>, GitStorageError> {
    let blob = git.find_blob(oid)?;
    Ok(blob.content().to_vec())
}

/// Resolve a conflict between three versions of a file using JSON field-level
/// last-writer-wins. Source (theirs) wins when both sides changed the same field.
fn resolve_json_conflict(
    ancestor: Option<&[u8]>,
    ours: Option<&[u8]>,
    theirs: Option<&[u8]>,
) -> Result<Vec<u8>, GitStorageError> {
    // If one side deleted and the other modified, the modification wins.
    match (ours, theirs) {
        (None, None) => {
            // Both deleted — return empty (shouldn't normally happen as a conflict).
            return Err(GitStorageError::MergeConflict(
                "both sides deleted".to_owned(),
            ));
        }
        (None, Some(theirs_bytes)) => {
            // Ours deleted, theirs modified — theirs wins.
            return Ok(theirs_bytes.to_vec());
        }
        (Some(ours_bytes), None) => {
            // Theirs deleted, ours modified — ours wins.
            return Ok(ours_bytes.to_vec());
        }
        (Some(_), Some(_)) => {
            // Both present — need field-level merge below.
        }
    }

    let ours_bytes = ours.unwrap();
    let theirs_bytes = theirs.unwrap();

    // Parse all versions as JSON.
    let ancestor_val: Option<Value> = ancestor
        .map(|b| {
            serde_json::from_slice(b)
                .map_err(|e| GitStorageError::MergeConflict(format!("ancestor not valid JSON: {e}")))
        })
        .transpose()?;
    let ours_val: Value = serde_json::from_slice(ours_bytes)
        .map_err(|e| GitStorageError::MergeConflict(format!("ours not valid JSON: {e}")))?;
    let theirs_val: Value = serde_json::from_slice(theirs_bytes)
        .map_err(|e| GitStorageError::MergeConflict(format!("theirs not valid JSON: {e}")))?;

    let base = ancestor_val.as_ref().cloned().unwrap_or(Value::Object(
        serde_json::Map::new(),
    ));

    let merged = merge_json_values(&base, &ours_val, &theirs_val);

    serde_json::to_vec_pretty(&merged)
        .map_err(|e| GitStorageError::MergeConflict(format!("failed to serialize merged JSON: {e}")))
}

/// Recursively merge JSON values using three-way diff logic.
///
/// - If only one side changed from base, take that side's value.
/// - If both changed to the same value, take either.
/// - If both changed to different values, source (theirs) wins (LWW).
/// - For objects, merge field-by-field.
fn merge_json_values(base: &Value, ours: &Value, theirs: &Value) -> Value {
    // If both sides are objects and base is an object (or null), do field-level merge.
    if let (Value::Object(base_map), Value::Object(ours_map), Value::Object(theirs_map)) =
        (base, ours, theirs)
    {
        return merge_json_objects(base_map, ours_map, theirs_map);
    }

    // Non-object values: simple three-way comparison.
    if ours == base && theirs != base {
        // Only theirs changed.
        theirs.clone()
    } else if theirs == base && ours != base {
        // Only ours changed.
        ours.clone()
    } else if ours == theirs {
        // Both changed to the same value (or neither changed).
        ours.clone()
    } else {
        // Both changed to different values — source (theirs) wins.
        theirs.clone()
    }
}

/// Field-level merge of JSON objects.
fn merge_json_objects(
    base: &serde_json::Map<String, Value>,
    ours: &serde_json::Map<String, Value>,
    theirs: &serde_json::Map<String, Value>,
) -> Value {
    // Collect all keys from all three versions.
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

        // Key deleted by one or both sides.
        if in_base && !in_ours && !in_theirs {
            // Both deleted — exclude.
            continue;
        }
        if in_base && !in_ours && in_theirs {
            // Ours deleted it — deletion wins.
            continue;
        }
        if in_base && in_ours && !in_theirs {
            // Theirs deleted it — deletion wins.
            continue;
        }

        // Key added by one or both sides (not in base).
        if !in_base {
            if in_ours && !in_theirs {
                result.insert(key.to_string(), o.unwrap().clone());
                continue;
            }
            if !in_ours && in_theirs {
                result.insert(key.to_string(), t.unwrap().clone());
                continue;
            }
            // Both added — merge the values (theirs wins on conflict).
        }

        // Both present — recursively merge.
        let bv = b.unwrap_or(&absent);
        let ov = o.unwrap_or(&absent);
        let tv = t.unwrap_or(&absent);
        result.insert(key.to_string(), merge_json_values(bv, ov, tv));
    }

    Value::Object(result)
}

// ── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::commit::{commit_files, FileWrite};
    use tempfile::TempDir;

    /// Helper: init a repo and return (repo, tmp_dir).
    fn setup_repo() -> (CorpRepo, TempDir) {
        let tmp = TempDir::new().unwrap();
        let repo = CorpRepo::init(tmp.path().join("test.git").as_path()).unwrap();
        (repo, tmp)
    }

    #[test]
    fn test_fast_forward_still_works() {
        let (repo, _tmp) = setup_repo();

        // Create a branch from main.
        crate::git::branch::create_branch(&repo, "feature", "main").unwrap();

        // Commit to feature branch.
        commit_files(
            &repo,
            "feature",
            "add data",
            &[FileWrite::raw("data.json", b"{\"a\": 1}".to_vec())],
        )
        .unwrap();

        // Merge feature into main — should fast-forward.
        let result = merge_branch(&repo, "feature", "main").unwrap();
        assert!(matches!(result, MergeResult::FastForward { .. }));
    }

    #[test]
    fn test_three_way_merge_no_conflicts() {
        let (repo, _tmp) = setup_repo();

        // Commit base file on main.
        commit_files(
            &repo,
            "main",
            "base",
            &[FileWrite::raw(
                "corp.json",
                serde_json::to_vec_pretty(&serde_json::json!({
                    "legal_name": "Acme Inc",
                    "jurisdiction": "Delaware",
                    "status": "active"
                }))
                .unwrap(),
            )],
        )
        .unwrap();

        // Create branch from main.
        crate::git::branch::create_branch(&repo, "feature", "main").unwrap();

        // Change legal_name on main.
        commit_files(
            &repo,
            "main",
            "update name",
            &[FileWrite::raw(
                "corp.json",
                serde_json::to_vec_pretty(&serde_json::json!({
                    "legal_name": "Acme Corp",
                    "jurisdiction": "Delaware",
                    "status": "active"
                }))
                .unwrap(),
            )],
        )
        .unwrap();

        // Change jurisdiction on feature.
        commit_files(
            &repo,
            "feature",
            "update jurisdiction",
            &[FileWrite::raw(
                "corp.json",
                serde_json::to_vec_pretty(&serde_json::json!({
                    "legal_name": "Acme Inc",
                    "jurisdiction": "California",
                    "status": "active"
                }))
                .unwrap(),
            )],
        )
        .unwrap();

        // Merge feature into main.
        let result = merge_branch(&repo, "feature", "main").unwrap();
        assert!(matches!(result, MergeResult::ThreeWayMerge { .. }));

        // Read merged result.
        let merged: Value = repo.read_json("main", "corp.json").unwrap();
        assert_eq!(merged["legal_name"], "Acme Corp"); // from main
        assert_eq!(merged["jurisdiction"], "California"); // from feature
        assert_eq!(merged["status"], "active"); // unchanged
    }

    #[test]
    fn test_three_way_merge_different_files() {
        let (repo, _tmp) = setup_repo();

        // Create branch from main (both start at empty initial commit).
        crate::git::branch::create_branch(&repo, "feature", "main").unwrap();

        // Add a.json on main.
        commit_files(
            &repo,
            "main",
            "add a",
            &[FileWrite::raw("a.json", b"{\"x\": 1}".to_vec())],
        )
        .unwrap();

        // Add b.json on feature.
        commit_files(
            &repo,
            "feature",
            "add b",
            &[FileWrite::raw("b.json", b"{\"y\": 2}".to_vec())],
        )
        .unwrap();

        let result = merge_branch(&repo, "feature", "main").unwrap();
        assert!(matches!(result, MergeResult::ThreeWayMerge { .. }));

        // Both files should exist on main.
        assert!(repo.path_exists("main", "a.json").unwrap());
        assert!(repo.path_exists("main", "b.json").unwrap());
    }

    #[test]
    fn test_three_way_merge_same_field_lww() {
        let (repo, _tmp) = setup_repo();

        // Base state.
        commit_files(
            &repo,
            "main",
            "base",
            &[FileWrite::raw(
                "corp.json",
                serde_json::to_vec_pretty(&serde_json::json!({
                    "status": "draft"
                }))
                .unwrap(),
            )],
        )
        .unwrap();

        crate::git::branch::create_branch(&repo, "feature", "main").unwrap();

        // Main changes status to "active".
        commit_files(
            &repo,
            "main",
            "activate",
            &[FileWrite::raw(
                "corp.json",
                serde_json::to_vec_pretty(&serde_json::json!({
                    "status": "active"
                }))
                .unwrap(),
            )],
        )
        .unwrap();

        // Feature changes status to "suspended".
        commit_files(
            &repo,
            "feature",
            "suspend",
            &[FileWrite::raw(
                "corp.json",
                serde_json::to_vec_pretty(&serde_json::json!({
                    "status": "suspended"
                }))
                .unwrap(),
            )],
        )
        .unwrap();

        // Merge feature into main — source (feature/theirs) wins.
        let result = merge_branch(&repo, "feature", "main").unwrap();
        assert!(matches!(result, MergeResult::ThreeWayMerge { .. }));

        let merged: Value = repo.read_json("main", "corp.json").unwrap();
        assert_eq!(merged["status"], "suspended"); // source (feature) wins
    }

    #[test]
    fn test_three_way_merge_non_json_conflict() {
        let (repo, _tmp) = setup_repo();

        // Base state with non-JSON file.
        commit_files(
            &repo,
            "main",
            "base",
            &[FileWrite::raw("readme.txt", b"hello world".to_vec())],
        )
        .unwrap();

        crate::git::branch::create_branch(&repo, "feature", "main").unwrap();

        // Both sides change the same line.
        commit_files(
            &repo,
            "main",
            "update main",
            &[FileWrite::raw("readme.txt", b"hello from main".to_vec())],
        )
        .unwrap();

        commit_files(
            &repo,
            "feature",
            "update feature",
            &[FileWrite::raw("readme.txt", b"hello from feature".to_vec())],
        )
        .unwrap();

        let result = merge_branch(&repo, "feature", "main");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, GitStorageError::MergeConflict(_)),
            "expected MergeConflict, got: {err:?}"
        );
    }

    #[test]
    fn test_three_way_merge_key_additions() {
        let (repo, _tmp) = setup_repo();

        // Base state.
        commit_files(
            &repo,
            "main",
            "base",
            &[FileWrite::raw(
                "data.json",
                serde_json::to_vec_pretty(&serde_json::json!({
                    "name": "test"
                }))
                .unwrap(),
            )],
        )
        .unwrap();

        crate::git::branch::create_branch(&repo, "feature", "main").unwrap();

        // Main adds field "foo".
        commit_files(
            &repo,
            "main",
            "add foo",
            &[FileWrite::raw(
                "data.json",
                serde_json::to_vec_pretty(&serde_json::json!({
                    "name": "test",
                    "foo": "from_main"
                }))
                .unwrap(),
            )],
        )
        .unwrap();

        // Feature adds field "bar".
        commit_files(
            &repo,
            "feature",
            "add bar",
            &[FileWrite::raw(
                "data.json",
                serde_json::to_vec_pretty(&serde_json::json!({
                    "name": "test",
                    "bar": "from_feature"
                }))
                .unwrap(),
            )],
        )
        .unwrap();

        let result = merge_branch(&repo, "feature", "main").unwrap();
        assert!(matches!(result, MergeResult::ThreeWayMerge { .. }));

        let merged: Value = repo.read_json("main", "data.json").unwrap();
        assert_eq!(merged["name"], "test");
        assert_eq!(merged["foo"], "from_main");
        assert_eq!(merged["bar"], "from_feature");
    }

    #[test]
    fn test_three_way_merge_one_side_deletes_field() {
        let (repo, _tmp) = setup_repo();

        // Base with two fields.
        commit_files(
            &repo,
            "main",
            "base",
            &[FileWrite::raw(
                "data.json",
                serde_json::to_vec_pretty(&serde_json::json!({
                    "keep": "yes",
                    "remove": "this"
                }))
                .unwrap(),
            )],
        )
        .unwrap();

        crate::git::branch::create_branch(&repo, "feature", "main").unwrap();

        // Main leaves both fields, changes "keep".
        commit_files(
            &repo,
            "main",
            "update keep",
            &[FileWrite::raw(
                "data.json",
                serde_json::to_vec_pretty(&serde_json::json!({
                    "keep": "updated",
                    "remove": "this"
                }))
                .unwrap(),
            )],
        )
        .unwrap();

        // Feature deletes "remove" field.
        commit_files(
            &repo,
            "feature",
            "delete remove",
            &[FileWrite::raw(
                "data.json",
                serde_json::to_vec_pretty(&serde_json::json!({
                    "keep": "yes"
                }))
                .unwrap(),
            )],
        )
        .unwrap();

        let result = merge_branch(&repo, "feature", "main").unwrap();
        assert!(matches!(result, MergeResult::ThreeWayMerge { .. }));

        let merged: Value = repo.read_json("main", "data.json").unwrap();
        assert_eq!(merged["keep"], "updated"); // from main (ours)
        assert!(merged.get("remove").is_none()); // deleted by feature
    }

    // ── Unit tests for resolve_json_conflict ─────────────────────────────

    #[test]
    fn test_resolve_json_conflict_ours_deleted() {
        let theirs = b"{\"a\": 1}";
        let result = resolve_json_conflict(Some(b"{\"a\": 0}"), None, Some(theirs)).unwrap();
        let val: Value = serde_json::from_slice(&result).unwrap();
        assert_eq!(val["a"], 1);
    }

    #[test]
    fn test_resolve_json_conflict_theirs_deleted() {
        let ours = b"{\"a\": 1}";
        let result = resolve_json_conflict(Some(b"{\"a\": 0}"), Some(ours), None).unwrap();
        let val: Value = serde_json::from_slice(&result).unwrap();
        assert_eq!(val["a"], 1);
    }
}
