//! Atomic multi-file commit support.
//!
//! Builds a new tree by overlaying file writes onto the existing tree at a
//! ref's HEAD, then creates a single commit. Supports arbitrarily nested
//! paths (e.g. `"cap-table/grants/abc.json"`).

use git2::Oid;
use serde::Serialize;
use std::collections::HashMap;

use super::error::GitStorageError;
use super::repo::CorpRepo;

/// A file to write in a commit. Path is relative to repo root.
pub struct FileWrite {
    pub path: String,
    pub content: Vec<u8>,
}

impl FileWrite {
    /// Create a `FileWrite` from a serializable value (pretty-printed JSON).
    pub fn json<T: Serialize>(path: impl Into<String>, value: &T) -> Result<Self, GitStorageError> {
        let content = serde_json::to_vec_pretty(value).map_err(|e| {
            GitStorageError::SerializationError(format!("failed to serialize: {e}"))
        })?;
        Ok(Self {
            path: path.into(),
            content,
        })
    }

    /// Create a `FileWrite` from raw bytes.
    pub fn raw(path: impl Into<String>, content: Vec<u8>) -> Self {
        Self {
            path: path.into(),
            content,
        }
    }
}

/// Commit one or more files atomically to a ref.
///
/// Builds a new tree by overlaying file writes onto the existing tree at the
/// ref's HEAD. The ref must already exist (created during repo init or via
/// branch creation).
pub fn commit_files(
    repo: &CorpRepo,
    refname: &str,
    message: &str,
    files: &[FileWrite],
) -> Result<Oid, GitStorageError> {
    let git = repo.inner();
    let full_ref = CorpRepo::normalize_ref(refname);

    // Resolve parent commit and existing tree.
    let parent_oid = repo.resolve_ref(refname)?;
    let parent_commit = git.find_commit(parent_oid)?;
    let base_tree = parent_commit.tree()?;

    // Group files by their top-level directory structure.
    // We build a tree of `NestedWrite` entries, then flush bottom-up.
    let new_tree_oid = build_tree_overlay(git, &base_tree, files)?;

    let new_tree = git.find_tree(new_tree_oid)?;
    let sig = CorpRepo::signature()?;

    let commit_oid = git.commit(
        Some(&full_ref),
        &sig,
        &sig,
        message,
        &new_tree,
        &[&parent_commit],
    )?;

    tracing::debug!(
        ref_ = %full_ref,
        oid = %commit_oid,
        files = files.len(),
        "committed files"
    );

    Ok(commit_oid)
}

// ── Internal tree-building machinery ──────────────────────────────────

/// A node in a tree of pending writes. Either a blob or a subtree with children.
enum TreeNode {
    Blob(Vec<u8>),
    Dir(HashMap<String, TreeNode>),
}

impl TreeNode {
    /// Get or create a child directory node.
    fn ensure_dir(&mut self, name: &str) -> Result<&mut TreeNode, GitStorageError> {
        match self {
            TreeNode::Dir(children) => Ok(children
                .entry(name.to_owned())
                .or_insert_with(|| TreeNode::Dir(HashMap::new()))),
            _ => Err(GitStorageError::Git(format!(
                "path conflict: expected directory at '{name}', found file"
            ))),
        }
    }
}

/// Build a new root tree OID by overlaying `files` onto `base_tree`.
fn build_tree_overlay(
    git: &git2::Repository,
    base_tree: &git2::Tree<'_>,
    files: &[FileWrite],
) -> Result<Oid, GitStorageError> {
    // Organize writes into a tree structure.
    let mut root = TreeNode::Dir(HashMap::new());

    for fw in files {
        if fw.path.is_empty() {
            return Err(GitStorageError::Git("file path cannot be empty".into()));
        }
        if fw.path.starts_with('/') || fw.path.contains("..") {
            return Err(GitStorageError::Git(format!(
                "invalid file path: '{}'", fw.path
            )));
        }

        let parts: Vec<&str> = fw.path.split('/').collect();
        let mut cursor = &mut root;
        for (i, part) in parts.iter().enumerate() {
            if i == parts.len() - 1 {
                // Leaf — insert blob.
                match cursor {
                    TreeNode::Dir(children) => {
                        children.insert((*part).to_owned(), TreeNode::Blob(fw.content.clone()));
                    }
                    _ => {
                        return Err(GitStorageError::Git(format!(
                            "path conflict: expected directory, found file at '{}'",
                            fw.path
                        )));
                    }
                }
            } else {
                // Intermediate directory.
                cursor = cursor.ensure_dir(part)?;
            }
        }
    }

    // Now flush the tree structure into git objects, merging with existing trees.
    flush_tree(git, Some(base_tree), &root)
}

/// Recursively write a `TreeNode` into the git object store, using `existing`
/// as the base tree to preserve entries that aren't being overwritten.
fn flush_tree(
    git: &git2::Repository,
    existing: Option<&git2::Tree<'_>>,
    node: &TreeNode,
) -> Result<Oid, GitStorageError> {
    let children = match node {
        TreeNode::Dir(children) => children,
        _ => return Err(GitStorageError::Git("expected directory node at root".into())),
    };

    let mut builder = git.treebuilder(existing)?;

    for (name, child) in children {
        match child {
            TreeNode::Blob(content) => {
                let blob_oid = git.blob(content)?;
                builder.insert(name, blob_oid, 0o100644)?;
            }
            TreeNode::Dir(_) => {
                // Resolve existing subtree, if any, to merge into.
                let existing_subtree = existing
                    .and_then(|t| t.get_name(name))
                    .and_then(|entry| entry.to_object(git).ok())
                    .and_then(|obj| obj.into_tree().ok());

                let subtree_oid =
                    flush_tree(git, existing_subtree.as_ref(), child)?;
                builder.insert(name, subtree_oid, 0o040000)?;
            }
        }
    }

    let oid = builder.write()?;
    Ok(oid)
}
