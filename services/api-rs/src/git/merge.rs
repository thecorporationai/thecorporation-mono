//! Branch merging.
//!
//! Currently supports fast-forward merges only. Three-way merges with
//! conflict resolution can be added later if needed.

use git2::Oid;

use super::error::GitStorageError;
use super::repo::CorpRepo;

/// Result of a merge operation.
pub enum MergeResult {
    /// The target ref was moved forward to the source commit.
    FastForward { new_oid: Oid },
    /// The target already contained all commits from source.
    AlreadyUpToDate,
}

/// Merge `source_branch` into `target_branch`.
///
/// Only fast-forward merges are supported. If the target is not a direct
/// ancestor of the source, returns [`GitStorageError::MergeConflict`].
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

        return Err(GitStorageError::MergeConflict(format!(
            "cannot fast-forward {target_branch} to {source_branch}: branches have diverged"
        )));
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
