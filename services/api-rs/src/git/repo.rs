//! Core repository wrapper.
//!
//! `CorpRepo` wraps a bare `git2::Repository` and exposes read operations
//! against any ref (branch). Write operations live in [`super::commit`].

use git2::{ObjectType, Oid, Repository, Signature, TreeBuilder};
use serde::de::DeserializeOwned;
use std::path::{Path, PathBuf};

use super::error::GitStorageError;
use super::signing::{CommitContext, build_signed_message};

/// A corporate git repository. All repos are bare.
pub struct CorpRepo {
    inner: Repository,
    path: PathBuf,
}

impl CorpRepo {
    // ── Construction ──────────────────────────────────────────────────

    /// Initialize a new bare repo at the given path.
    ///
    /// Creates an initial empty commit on `refs/heads/main` so that callers
    /// never have to deal with an unborn HEAD.
    ///
    /// When `ctx` is provided, the initial commit is signed and includes
    /// an actor trailer.
    pub fn init(path: &Path, ctx: Option<&CommitContext<'_>>) -> Result<Self, GitStorageError> {
        if path.exists() {
            return Err(GitStorageError::Git(format!(
                "path already exists: {}",
                path.display()
            )));
        }

        let repo = Repository::init_bare(path)?;

        // Build an empty tree and create initial commit.
        // Scoped so borrows are released before we move `repo` into Self.
        let commit_oid = {
            let empty_tree_oid = {
                let builder: TreeBuilder<'_> = repo.treebuilder(None)?;
                builder.write()?
            };
            let empty_tree = repo.find_tree(empty_tree_oid)?;

            let sig = Self::signature()?;
            let message = match ctx {
                Some(c) => build_signed_message("initial commit", c.actor, c.signer),
                None => "initial commit".to_owned(),
            };

            match ctx.and_then(|c| c.signer) {
                Some(signer) => {
                    let commit_buf =
                        repo.commit_create_buffer(&sig, &sig, &message, &empty_tree, &[])?;
                    let commit_str = std::str::from_utf8(&commit_buf).map_err(|e| {
                        GitStorageError::SigningError(format!("commit buffer not UTF-8: {e}"))
                    })?;
                    let signature = signer.sign_commit(commit_str)?;
                    let signed_oid = repo.commit_signed(commit_str, &signature, Some("gpgsig"))?;
                    repo.reference("refs/heads/main", signed_oid, true, "initial signed commit")?;
                    signed_oid
                }
                None => repo.commit(
                    Some("refs/heads/main"),
                    &sig,
                    &sig,
                    &message,
                    &empty_tree,
                    &[],
                )?,
            }
        };

        // Point HEAD at main so `head()` works.
        repo.set_head("refs/heads/main")?;

        tracing::debug!(
            path = %path.display(),
            oid = %commit_oid,
            "initialized bare repo"
        );

        Ok(Self {
            inner: repo,
            path: path.to_path_buf(),
        })
    }

    /// Open an existing bare repo.
    pub fn open(path: &Path) -> Result<Self, GitStorageError> {
        if !path.exists() {
            return Err(GitStorageError::RepoNotFound(path.display().to_string()));
        }

        let repo = Repository::open_bare(path)?;
        Ok(Self {
            inner: repo,
            path: path.to_path_buf(),
        })
    }

    // ── Read operations ───────────────────────────────────────────────

    /// Read a JSON file from the tree at a given ref and deserialize it.
    pub fn read_json<T: DeserializeOwned>(
        &self,
        refname: &str,
        path: &str,
    ) -> Result<T, GitStorageError> {
        let bytes = self.read_blob(refname, path)?;
        serde_json::from_slice(&bytes).map_err(|e| {
            GitStorageError::SerializationError(format!(
                "failed to deserialize {path} at {refname}: {e}"
            ))
        })
    }

    /// Read raw bytes from a path at a given ref.
    pub fn read_blob(&self, refname: &str, path: &str) -> Result<Vec<u8>, GitStorageError> {
        let tree = self.tree_at_ref(refname)?;
        let entry = tree
            .get_path(Path::new(path))
            .map_err(|_| GitStorageError::NotFound(format!("{path} not found at ref {refname}")))?;

        let object = entry.to_object(&self.inner)?;
        let blob = object.as_blob().ok_or_else(|| {
            GitStorageError::NotFound(format!("{path} is not a file at ref {refname}"))
        })?;

        Ok(blob.content().to_vec())
    }

    /// List entries in a directory at a given ref.
    ///
    /// Returns `(name, is_dir)` pairs. If `dir_path` is empty, lists the root.
    pub fn list_dir(
        &self,
        refname: &str,
        dir_path: &str,
    ) -> Result<Vec<(String, bool)>, GitStorageError> {
        let root_tree = self.tree_at_ref(refname)?;

        let tree = if dir_path.is_empty() {
            root_tree
        } else {
            let entry = root_tree.get_path(Path::new(dir_path)).map_err(|_| {
                GitStorageError::NotFound(format!("{dir_path} not found at ref {refname}"))
            })?;
            let object = entry.to_object(&self.inner)?;
            object.into_tree().map_err(|_| {
                GitStorageError::NotFound(format!("{dir_path} is not a directory at ref {refname}"))
            })?
        };

        let mut entries = Vec::new();
        for entry in tree.iter() {
            let name = match entry.name() {
                Some(n) => n.to_owned(),
                None => continue, // skip non-UTF-8 entries
            };
            let is_dir = entry.kind() == Some(ObjectType::Tree);
            entries.push((name, is_dir));
        }

        Ok(entries)
    }

    /// Check if a path exists at a given ref.
    pub fn path_exists(&self, refname: &str, path: &str) -> Result<bool, GitStorageError> {
        let tree = self.tree_at_ref(refname)?;
        Ok(tree.get_path(Path::new(path)).is_ok())
    }

    // ── Accessors ─────────────────────────────────────────────────────

    /// Get the underlying `git2::Repository`.
    pub fn inner(&self) -> &Repository {
        &self.inner
    }

    /// The on-disk path of this bare repo.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Resolve a ref name to an `Oid`.
    ///
    /// Accepts short names like `"main"` (resolved as `refs/heads/main`)
    /// or full refs like `"refs/heads/feature"`.
    pub fn resolve_ref(&self, refname: &str) -> Result<Oid, GitStorageError> {
        let full_ref = Self::normalize_ref(refname);
        let reference = self
            .inner
            .find_reference(&full_ref)
            .map_err(|_| GitStorageError::BranchNotFound(refname.to_owned()))?;
        let oid = reference
            .target()
            .ok_or_else(|| GitStorageError::BranchNotFound(refname.to_owned()))?;
        Ok(oid)
    }

    /// Walk the commit history of `refname` and return up to `limit` recent
    /// commits as `(oid_hex, message, iso8601_timestamp)` tuples.
    pub fn recent_commits(
        &self,
        refname: &str,
        limit: usize,
    ) -> Result<Vec<(String, String, String)>, GitStorageError> {
        let oid = self.resolve_ref(refname)?;
        let mut revwalk = self.inner.revwalk()?;
        revwalk.push(oid)?;
        revwalk.set_sorting(git2::Sort::TIME)?;

        let mut results = Vec::new();
        for maybe_oid in revwalk {
            if results.len() >= limit {
                break;
            }
            let commit_oid = maybe_oid?;
            let commit = self.inner.find_commit(commit_oid)?;
            let message = commit.message().unwrap_or("").to_owned();
            let time = commit.time();
            let ts = chrono::DateTime::from_timestamp(time.seconds(), 0)
                .unwrap_or_default()
                .to_rfc3339();
            results.push((commit_oid.to_string(), message, ts));
        }
        Ok(results)
    }

    // ── Helpers (pub(crate)) ──────────────────────────────────────────

    /// Standard commit signature used by the engine.
    pub(crate) fn signature() -> Result<Signature<'static>, GitStorageError> {
        Signature::now("corp-engine", "engine@thecorporation.ai").map_err(GitStorageError::from)
    }

    /// Normalize a ref name: if it doesn't start with `refs/`, prepend `refs/heads/`.
    pub(crate) fn normalize_ref(refname: &str) -> String {
        if refname.starts_with("refs/") {
            refname.to_owned()
        } else {
            format!("refs/heads/{refname}")
        }
    }

    // ── Private helpers ───────────────────────────────────────────────

    /// Get the root tree at a given ref.
    fn tree_at_ref(&self, refname: &str) -> Result<git2::Tree<'_>, GitStorageError> {
        let oid = self.resolve_ref(refname)?;
        let commit = self.inner.find_commit(oid)?;
        let tree = commit.tree()?;
        Ok(tree)
    }
}
