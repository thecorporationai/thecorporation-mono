//! Pure-Rust git backend using the `gix` library (v0.72).
//!
//! All functions in this module are **synchronous** and are intended to be
//! called from within `tokio::task::spawn_blocking` closures so that they
//! do not block the async runtime.
//!
//! The repository format is a **bare** git repo.  Each logical "branch" maps
//! to a real git branch ref.  Files are stored as blobs in a flat or nested
//! tree, committed with the provided commit message and an artificial
//! committer identity.
//!
//! # Error mapping
//! Every `gix` error is mapped to [`StorageError::GitError`].
//!
//! # API notes (gix 0.72)
//! - `repo.write_blob(bytes)` — writes raw bytes as a Blob object.
//! - `repo.write_object(&obj)` — writes any `impl WriteTo` object (Tree,
//!   Commit, etc.).  The `Id` it returns is bound to `&self`; call `.detach()`
//!   to obtain an owned `ObjectId`.
//! - Use `gix::objs::Commit` (owned) for commit construction to avoid
//!   lifetime issues with `CommitRef<'a>`.

use std::collections::BTreeMap;
use std::path::Path;

use gix::ObjectId;
use gix::bstr::BString;
use gix::objs::tree::EntryKind;
use gix::refs::transaction::{Change, LogChange, PreviousValue, RefEdit};
use smallvec::SmallVec;

use crate::error::StorageError;

// ── Internal helpers ──────────────────────────────────────────────────────────

type Result<T> = std::result::Result<T, StorageError>;

fn git_err(e: impl std::fmt::Display) -> StorageError {
    StorageError::GitError(e.to_string())
}

fn branch_ref(branch: &str) -> String {
    if branch.starts_with("refs/") {
        branch.to_owned()
    } else {
        format!("refs/heads/{}", branch)
    }
}

fn open_repo(path: &Path) -> Result<gix::Repository> {
    gix::open(path).map_err(git_err)
}

// ── In-memory tree representation ────────────────────────────────────────────

/// An in-memory copy of a git tree used for read-modify-write operations.
enum TreeNode {
    Blob(ObjectId),
    Dir(BTreeMap<String, TreeNode>),
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Initialise a new bare repository at `path`.  Idempotent.
pub fn init_bare_repo(path: &Path) -> Result<()> {
    if path.join("HEAD").exists() {
        return Ok(());
    }
    gix::init_bare(path).map_err(git_err)?;
    Ok(())
}

/// Read the raw bytes of `file_path` from `branch`.
pub fn read_file(repo_path: &Path, branch: &str, file_path: &str) -> Result<Vec<u8>> {
    let repo = open_repo(repo_path)?;
    let ref_name = branch_ref(branch);

    let reference = repo
        .find_reference(&ref_name)
        .map_err(|_| StorageError::NotFound(format!("branch '{}'", branch)))?;

    let commit_id = reference.into_fully_peeled_id().map_err(git_err)?.detach();

    let commit_obj = repo.find_object(commit_id).map_err(git_err)?;
    let commit = commit_obj.into_commit();
    let tree_id = commit.tree_id().map_err(git_err)?.detach();

    let parts: Vec<&str> = file_path.split('/').filter(|s| !s.is_empty()).collect();
    if parts.is_empty() {
        return Err(StorageError::InvalidData("empty file path".into()));
    }

    find_blob(&repo, tree_id, &parts)
}

fn find_blob(repo: &gix::Repository, tree_id: ObjectId, parts: &[&str]) -> Result<Vec<u8>> {
    let tree_obj = repo.find_object(tree_id).map_err(git_err)?;
    let tree = tree_obj.into_tree();
    let decoded = tree.decode().map_err(git_err)?;
    let name = parts[0];

    for entry in &decoded.entries {
        let entry_name = String::from_utf8_lossy(entry.filename);
        if entry_name != name {
            continue;
        }

        if parts.len() == 1 {
            match entry.mode.kind() {
                EntryKind::Blob | EntryKind::BlobExecutable => {}
                _ => {
                    return Err(StorageError::InvalidData(format!(
                        "'{}' is not a file",
                        name
                    )));
                }
            }
            return Ok(repo.find_object(entry.oid).map_err(git_err)?.data.to_vec());
        }

        if entry.mode.kind() == EntryKind::Tree {
            return find_blob(repo, entry.oid.into(), &parts[1..]);
        }
    }

    Err(StorageError::NotFound(format!(
        "path '{}' not found",
        parts.join("/")
    )))
}

/// Write one or more files to `branch` as a single atomic commit.
///
/// Creates the branch (orphan commit) if it does not yet exist.
pub fn write_files(
    repo_path: &Path,
    branch: &str,
    files: &[(String, Vec<u8>)],
    message: &str,
) -> Result<()> {
    let repo = open_repo(repo_path)?;
    let ref_name = branch_ref(branch);

    // Load the existing tree, or start empty.
    let mut tree_root: BTreeMap<String, TreeNode> = BTreeMap::new();
    let mut parent_ids: SmallVec<[ObjectId; 1]> = SmallVec::new();

    if let Ok(reference) = repo.find_reference(&ref_name) {
        let commit_id = reference.into_fully_peeled_id().map_err(git_err)?.detach();
        parent_ids.push(commit_id);
        let commit_obj = repo.find_object(commit_id).map_err(git_err)?;
        let commit = commit_obj.into_commit();
        let tree_id = commit.tree_id().map_err(git_err)?.detach();
        load_tree(&repo, tree_id, &mut tree_root)?;
    }

    // Write blobs and insert into the in-memory tree.
    for (path, data) in files {
        let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        if parts.is_empty() {
            return Err(StorageError::InvalidData("empty file path".into()));
        }
        let blob_id = repo.write_blob(data.as_slice()).map_err(git_err)?.detach();
        insert_node(&mut tree_root, &parts, TreeNode::Blob(blob_id));
    }

    let tree_id = write_tree(&repo, &tree_root)?;
    let prev_id = parent_ids.first().copied();
    let commit_id = write_commit(&repo, tree_id, parent_ids, message)?;
    update_ref(&repo, &ref_name, commit_id, prev_id, message)?;
    Ok(())
}

// ── Tree helpers ──────────────────────────────────────────────────────────────

fn load_tree(
    repo: &gix::Repository,
    tree_id: ObjectId,
    map: &mut BTreeMap<String, TreeNode>,
) -> Result<()> {
    let tree_obj = repo.find_object(tree_id).map_err(git_err)?;
    let tree = tree_obj.into_tree();
    let decoded = tree.decode().map_err(git_err)?;

    for entry in &decoded.entries {
        let name = String::from_utf8_lossy(entry.filename).into_owned();
        match entry.mode.kind() {
            EntryKind::Blob | EntryKind::BlobExecutable => {
                map.insert(name, TreeNode::Blob(entry.oid.into()));
            }
            EntryKind::Tree => {
                let mut sub_map = BTreeMap::new();
                load_tree(repo, entry.oid.into(), &mut sub_map)?;
                map.insert(name, TreeNode::Dir(sub_map));
            }
            _ => {}
        }
    }
    Ok(())
}

fn insert_node(map: &mut BTreeMap<String, TreeNode>, parts: &[&str], node: TreeNode) {
    if parts.len() == 1 {
        map.insert(parts[0].to_owned(), node);
    } else {
        let sub = map
            .entry(parts[0].to_owned())
            .or_insert_with(|| TreeNode::Dir(BTreeMap::new()));
        if let TreeNode::Dir(sub_map) = sub {
            insert_node(sub_map, &parts[1..], node);
        } else {
            let mut new_map = BTreeMap::new();
            insert_node(&mut new_map, &parts[1..], node);
            *sub = TreeNode::Dir(new_map);
        }
    }
}

fn remove_node(map: &mut BTreeMap<String, TreeNode>, parts: &[&str]) -> bool {
    if parts.len() == 1 {
        map.remove(parts[0]).is_some()
    } else if let Some(TreeNode::Dir(sub_map)) = map.get_mut(parts[0]) {
        remove_node(sub_map, &parts[1..])
    } else {
        false
    }
}

fn write_tree(repo: &gix::Repository, map: &BTreeMap<String, TreeNode>) -> Result<ObjectId> {
    let mut entries: Vec<gix::objs::tree::Entry> = Vec::new();

    for (name, node) in map {
        match node {
            TreeNode::Blob(oid) => {
                entries.push(gix::objs::tree::Entry {
                    mode: gix::objs::tree::EntryMode::from(EntryKind::Blob),
                    filename: name.as_str().into(),
                    oid: (*oid),
                });
            }
            TreeNode::Dir(sub_map) => {
                let sub_oid = write_tree(repo, sub_map)?;
                entries.push(gix::objs::tree::Entry {
                    mode: gix::objs::tree::EntryMode::from(EntryKind::Tree),
                    filename: name.as_str().into(),
                    oid: sub_oid,
                });
            }
        }
    }

    // Git tree sort order differs from plain lexicographic order: directory
    // entries are compared as if their name has a trailing `/`.  A plain
    // `BTreeMap` sorts by string bytes, so e.g. `"foo"` (dir) would appear
    // before `"foo.json"` (file) because `'f','o','o',NUL < 'f','o','o','.'`
    // — the opposite of what git expects.  Sort with the git comparator so
    // `gix` does not panic on serialization.
    entries.sort_by(|a, b| {
        let a_name: &[u8] = a.filename.as_ref();
        let b_name: &[u8] = b.filename.as_ref();
        let a_is_dir = a.mode.kind() == EntryKind::Tree;
        let b_is_dir = b.mode.kind() == EntryKind::Tree;
        git_entry_cmp(a_name, a_is_dir, b_name, b_is_dir)
    });

    let tree_obj = gix::objs::Tree { entries };
    repo.write_object(&tree_obj)
        .map_err(git_err)
        .map(|id| id.detach())
}

/// Compare two git tree entry names using git's sort order.
///
/// Git sorts tree entries as if directories have a trailing `/`.  For
/// example, `"foo"` (tree) sorts after `"foo.json"` (blob) because
/// `"foo/"` > `"foo.json"` byte-by-byte at the `.` vs `/` comparison
/// (`.` = 0x2E, `/` = 0x2F).
fn git_entry_cmp(a: &[u8], a_is_dir: bool, b: &[u8], b_is_dir: bool) -> std::cmp::Ordering {
    // Build virtual byte sequences that append '/' for directories.
    let a_key: Vec<u8> = if a_is_dir {
        a.iter().copied().chain(std::iter::once(b'/')).collect()
    } else {
        a.to_vec()
    };
    let b_key: Vec<u8> = if b_is_dir {
        b.iter().copied().chain(std::iter::once(b'/')).collect()
    } else {
        b.to_vec()
    };
    a_key.cmp(&b_key)
}

fn write_commit(
    repo: &gix::Repository,
    tree_id: ObjectId,
    parent_ids: SmallVec<[ObjectId; 1]>,
    message: &str,
) -> Result<ObjectId> {
    let time = gix::date::Time::now_local_or_utc();
    let actor = gix::actor::Signature {
        name: BString::from("Corp Storage"),
        email: BString::from("storage@corp.internal"),
        time,
    };

    let commit = gix::objs::Commit {
        tree: tree_id,
        parents: parent_ids,
        author: actor.clone(),
        committer: actor,
        encoding: None,
        message: BString::from(message),
        extra_headers: vec![],
    };

    repo.write_object(&commit)
        .map_err(git_err)
        .map(|id| id.detach())
}

fn update_ref(
    repo: &gix::Repository,
    ref_name: &str,
    new_commit: ObjectId,
    previous: Option<ObjectId>,
    message: &str,
) -> Result<()> {
    let expected = match previous {
        Some(prev) => PreviousValue::MustExistAndMatch(gix::refs::Target::Object(prev)),
        None => PreviousValue::MustNotExist,
    };

    let edit = RefEdit {
        change: Change::Update {
            log: LogChange {
                mode: gix::refs::transaction::RefLog::AndReference,
                force_create_reflog: false,
                message: message.into(),
            },
            expected,
            new: gix::refs::Target::Object(new_commit),
        },
        name: ref_name.try_into().map_err(git_err)?,
        deref: false,
    };

    repo.edit_references(std::iter::once(edit)).map_err(|e| {
        let msg = e.to_string();
        // Detect CAS failure: gix reports "existing object id" mismatch
        // when PreviousValue::MustExistAndMatch fails. Surface as a
        // retryable 409 instead of an opaque 500.
        if msg.contains("existing object")
            || msg.contains("lock")
            || msg.contains("did not match")
        {
            StorageError::ConcurrencyConflict(format!(
                "concurrent write conflict on ref {}: {}",
                ref_name, msg
            ))
        } else {
            git_err(e)
        }
    })?;
    Ok(())
}

// ── Directory listing ─────────────────────────────────────────────────────────

/// List the direct children of `dir_path` on `branch`.  Returns empty on miss.
pub fn list_directory(repo_path: &Path, branch: &str, dir_path: &str) -> Result<Vec<String>> {
    let repo = open_repo(repo_path)?;
    let ref_name = branch_ref(branch);

    let reference = match repo.find_reference(&ref_name) {
        Ok(r) => r,
        Err(_) => return Ok(vec![]),
    };

    let commit_id = reference.into_fully_peeled_id().map_err(git_err)?.detach();
    let commit_obj = repo.find_object(commit_id).map_err(git_err)?;
    let commit = commit_obj.into_commit();
    let tree_id = commit.tree_id().map_err(git_err)?.detach();

    let parts: Vec<&str> = dir_path.split('/').filter(|s| !s.is_empty()).collect();
    let target_tree_id = if parts.is_empty() {
        tree_id
    } else {
        match navigate_to_subtree_id(&repo, tree_id, &parts) {
            Ok(id) => id,
            Err(StorageError::NotFound(_)) => return Ok(vec![]),
            Err(e) => return Err(e),
        }
    };

    let tree_obj = repo.find_object(target_tree_id).map_err(git_err)?;
    let tree = tree_obj.into_tree();
    let decoded = tree.decode().map_err(git_err)?;
    Ok(decoded
        .entries
        .iter()
        .map(|e| String::from_utf8_lossy(e.filename).into_owned())
        .collect())
}

fn navigate_to_subtree_id(
    repo: &gix::Repository,
    mut tree_id: ObjectId,
    parts: &[&str],
) -> Result<ObjectId> {
    for part in parts {
        let tree_obj = repo.find_object(tree_id).map_err(git_err)?;
        let tree = tree_obj.into_tree();
        let decoded = tree.decode().map_err(git_err)?;

        let entry = decoded
            .entries
            .iter()
            .find(|e| String::from_utf8_lossy(e.filename) == *part)
            .ok_or_else(|| StorageError::NotFound(format!("directory component '{}'", part)))?;

        if entry.mode.kind() != EntryKind::Tree {
            return Err(StorageError::InvalidData(format!(
                "'{}' is not a directory",
                part
            )));
        }

        tree_id = entry.oid.into();
    }
    Ok(tree_id)
}

// ── Deletion ──────────────────────────────────────────────────────────────────

/// Delete `file_path` from `branch`.
pub fn delete_file(repo_path: &Path, branch: &str, file_path: &str, message: &str) -> Result<()> {
    let repo = open_repo(repo_path)?;
    let ref_name = branch_ref(branch);

    let reference = repo
        .find_reference(&ref_name)
        .map_err(|_| StorageError::NotFound(format!("branch '{}'", branch)))?;

    let commit_id = reference.into_fully_peeled_id().map_err(git_err)?.detach();
    let commit_obj = repo.find_object(commit_id).map_err(git_err)?;
    let commit = commit_obj.into_commit();
    let tree_id = commit.tree_id().map_err(git_err)?.detach();

    let mut tree_root: BTreeMap<String, TreeNode> = BTreeMap::new();
    load_tree(&repo, tree_id, &mut tree_root)?;

    let parts: Vec<&str> = file_path.split('/').filter(|s| !s.is_empty()).collect();
    if !remove_node(&mut tree_root, &parts) {
        return Err(StorageError::NotFound(format!("file '{}'", file_path)));
    }

    let new_tree_id = write_tree(&repo, &tree_root)?;
    let mut parents: SmallVec<[ObjectId; 1]> = SmallVec::new();
    parents.push(commit_id);
    let new_commit_id = write_commit(&repo, new_tree_id, parents, message)?;
    update_ref(&repo, &ref_name, new_commit_id, Some(commit_id), message)?;
    Ok(())
}

// ── Existence check ───────────────────────────────────────────────────────────

/// Return `true` if `file_path` exists on `branch`.
pub fn file_exists(repo_path: &Path, branch: &str, file_path: &str) -> Result<bool> {
    match read_file(repo_path, branch, file_path) {
        Ok(_) => Ok(true),
        Err(StorageError::NotFound(_)) => Ok(false),
        Err(e) => Err(e),
    }
}
