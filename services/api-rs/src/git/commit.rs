//! Atomic multi-file commit support.
//!
//! Builds a new tree by overlaying file writes onto the existing tree at a
//! ref's HEAD, then creates a single commit. Supports arbitrarily nested
//! paths (e.g. `"cap-table/grants/abc.json"`).

use git2::Oid;
use serde::Serialize;
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::path::PathBuf;

use super::error::GitStorageError;
use super::repo::CorpRepo;
use super::signing::{CommitContext, build_signed_message};

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
///
/// When `ctx` is provided, the commit message is augmented with an actor
/// trailer. If the context also includes a signer, the commit is
/// cryptographically signed with an SSH Ed25519 key.
pub fn commit_files(
    repo: &CorpRepo,
    refname: &str,
    message: &str,
    files: &[FileWrite],
    ctx: Option<&CommitContext<'_>>,
) -> Result<Oid, GitStorageError> {
    let _lock = RepoWriteLock::acquire(repo)?;
    commit_files_unlocked(repo, refname, message, files, ctx)
}

fn commit_files_unlocked(
    repo: &CorpRepo,
    refname: &str,
    message: &str,
    files: &[FileWrite],
    ctx: Option<&CommitContext<'_>>,
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

    // Build final message (with actor trailer when ctx is present).
    let final_message = match ctx {
        Some(c) => build_signed_message(message, c.actor, c.signer),
        None => message.to_owned(),
    };

    let commit_oid = match ctx.and_then(|c| c.signer) {
        Some(signer) => {
            // Create the commit buffer without writing, sign it, then write signed.
            let commit_buf =
                git.commit_create_buffer(&sig, &sig, &final_message, &new_tree, &[&parent_commit])?;
            let commit_str = std::str::from_utf8(&commit_buf).map_err(|e| {
                GitStorageError::SigningError(format!("commit buffer not UTF-8: {e}"))
            })?;
            let signature = signer.sign_commit(commit_str)?;
            let signed_oid = git.commit_signed(commit_str, &signature, Some("gpgsig"))?;
            // Update ref to point to the signed commit.
            git.reference(&full_ref, signed_oid, true, "signed commit")?;
            signed_oid
        }
        None => git.commit(
            Some(&full_ref),
            &sig,
            &sig,
            &final_message,
            &new_tree,
            &[&parent_commit],
        )?,
    };

    tracing::debug!(
        ref_ = %full_ref,
        oid = %commit_oid,
        files = files.len(),
        "committed files"
    );

    Ok(commit_oid)
}

struct RepoWriteLock {
    file: File,
}

impl RepoWriteLock {
    fn acquire(repo: &CorpRepo) -> Result<Self, GitStorageError> {
        let lock_path = repo_lock_path(repo);
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .open(lock_path)?;
        file.lock()?;
        Ok(Self { file })
    }
}

impl Drop for RepoWriteLock {
    fn drop(&mut self) {
        let _ = self.file.unlock();
    }
}

fn repo_lock_path(repo: &CorpRepo) -> PathBuf {
    let name = repo
        .path()
        .file_name()
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_else(|| "repo".to_owned());
    repo.path().with_file_name(format!("{name}.lock"))
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
                "invalid file path: '{}'",
                fw.path
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
        _ => {
            return Err(GitStorageError::Git(
                "expected directory node at root".into(),
            ));
        }
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

                let subtree_oid = flush_tree(git, existing_subtree.as_ref(), child)?;
                builder.insert(name, subtree_oid, 0o040000)?;
            }
        }
    }

    let oid = builder.write()?;
    Ok(oid)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::repo::CorpRepo;
    use std::sync::{Arc, Barrier};
    use tempfile::TempDir;

    #[test]
    fn concurrent_commits_preserve_all_files() {
        let tmp = TempDir::new().unwrap();
        let repo_path = tmp.path().join("concurrent.git");
        let repo = CorpRepo::init(&repo_path, None).unwrap();
        let barrier = Arc::new(Barrier::new(6));

        let mut handles = Vec::new();
        for worker in 0..6 {
            let repo_path = repo.path().to_path_buf();
            let barrier = Arc::clone(&barrier);
            handles.push(std::thread::spawn(move || {
                let repo = CorpRepo::open(&repo_path).unwrap();
                barrier.wait();
                for seq in 0..8 {
                    let path = format!("parallel/{worker}-{seq}.txt");
                    let file = FileWrite::raw(path, format!("{worker}:{seq}").into_bytes());
                    commit_files(&repo, "main", "parallel write", &[file], None).unwrap();
                }
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        let repo = CorpRepo::open(&repo_path).unwrap();
        for worker in 0..6 {
            for seq in 0..8 {
                let path = format!("parallel/{worker}-{seq}.txt");
                let content = repo.read_blob("main", &path).unwrap();
                assert_eq!(
                    String::from_utf8(content).unwrap(),
                    format!("{worker}:{seq}")
                );
            }
        }
    }
}
