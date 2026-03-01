//! Branch management for bare repos.

use git2::Oid;

use super::error::GitStorageError;
use super::repo::CorpRepo;

/// Summary information about a branch.
pub struct BranchInfo {
    pub name: String,
    pub head_oid: Oid,
}

/// Create a new branch pointing at the same commit as `from_ref`.
pub fn create_branch(
    repo: &CorpRepo,
    name: &str,
    from_ref: &str,
) -> Result<BranchInfo, GitStorageError> {
    let full_ref = CorpRepo::normalize_ref(name);

    // Check the branch doesn't already exist.
    if repo.inner().find_reference(&full_ref).is_ok() {
        return Err(GitStorageError::BranchAlreadyExists(name.to_owned()));
    }

    // Resolve the source ref.
    let source_oid = repo.resolve_ref(from_ref)?;

    // Create the new ref.
    repo.inner().reference(
        &full_ref,
        source_oid,
        false,
        &format!("create branch {name}"),
    )?;

    tracing::debug!(branch = %name, from = %from_ref, oid = %source_oid, "branch created");

    Ok(BranchInfo {
        name: name.to_owned(),
        head_oid: source_oid,
    })
}

/// List all branches (refs/heads/*).
pub fn list_branches(repo: &CorpRepo) -> Result<Vec<BranchInfo>, GitStorageError> {
    let mut branches = Vec::new();

    for reference in repo.inner().references_glob("refs/heads/*")? {
        let reference = reference?;
        let name = match reference.shorthand() {
            Some(n) => n.to_owned(),
            None => continue, // skip branches with invalid names
        };
        let head_oid = reference
            .target()
            .ok_or_else(|| GitStorageError::Git("symbolic ref in refs/heads".into()))?;
        branches.push(BranchInfo { name, head_oid });
    }

    Ok(branches)
}

/// Delete a branch. Cannot delete `"main"`.
pub fn delete_branch(repo: &CorpRepo, name: &str) -> Result<(), GitStorageError> {
    if name == "main" {
        return Err(GitStorageError::Git(
            "cannot delete the main branch".to_owned(),
        ));
    }

    let full_ref = CorpRepo::normalize_ref(name);
    let mut reference = repo
        .inner()
        .find_reference(&full_ref)
        .map_err(|_| GitStorageError::BranchNotFound(name.to_owned()))?;

    reference.delete()?;

    tracing::debug!(branch = %name, "branch deleted");

    Ok(())
}
