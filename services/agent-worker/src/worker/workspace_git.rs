//! Git-backed workspace versioning.
//!
//! Every agent workspace gets a separate gitdir under
//! `{workspace_root}/.gitdirs/{agent_id}/` so that git metadata is never
//! mounted into the container.  A `.git` gitlink file in the workdir is
//! recreated before every git operation.

use std::path::{Path, PathBuf};

use git2::{IndexAddOption, Oid, Repository, RepositoryInitOptions, ResetType, Signature};

use crate::error::WorkerError;

/// Paths derived from `(workspace_root, agent_id)`.
struct Paths {
    workdir: PathBuf,
    gitdir: PathBuf,
}

fn paths(workspace_root: &str, agent_id: &str) -> Paths {
    Paths {
        workdir: PathBuf::from(format!("{workspace_root}/{agent_id}")),
        gitdir: PathBuf::from(format!("{workspace_root}/.gitdirs/{agent_id}")),
    }
}

fn git_err(e: git2::Error) -> WorkerError {
    WorkerError::Git(e.message().to_owned())
}

/// Ensure the `.git` gitlink file in the workdir points at `gitdir`.
fn write_gitlink(workdir: &Path, gitdir: &Path) -> Result<(), WorkerError> {
    let link_path = workdir.join(".git");
    let content = format!("gitdir: {}\n", gitdir.display());
    std::fs::write(&link_path, content)?;
    Ok(())
}

fn signature() -> Result<Signature<'static>, WorkerError> {
    Signature::now("agent-worker", "agent-worker@thecorporation.ai").map_err(git_err)
}

/// Open an existing repo or initialise a new one.
///
/// This is a **blocking** function — callers must use `spawn_blocking`.
pub fn init_or_open(workspace_root: &str, agent_id: &str) -> Result<Repository, WorkerError> {
    let p = paths(workspace_root, agent_id);
    std::fs::create_dir_all(&p.workdir)?;
    std::fs::create_dir_all(&p.gitdir)?;

    if p.gitdir.join("HEAD").exists() {
        // Already initialised — reopen and refresh gitlink.
        write_gitlink(&p.workdir, &p.gitdir)?;
        let repo = Repository::open(&p.gitdir).map_err(git_err)?;
        Ok(repo)
    } else {
        // Fresh init with workdir separate from gitdir.
        // `no_dotgit_dir` puts git metadata directly in gitdir (no .git/ subdir).
        let mut opts = RepositoryInitOptions::new();
        opts.bare(false)
            .no_dotgit_dir(true)
            .workdir_path(&p.workdir)
            .initial_head("main");

        let repo = Repository::init_opts(&p.gitdir, &opts).map_err(git_err)?;
        write_gitlink(&p.workdir, &p.gitdir)?;

        // Create an initial empty commit so `HEAD` always resolves.
        {
            let sig = signature()?;
            let tree_oid = repo.index().map_err(git_err)?.write_tree().map_err(git_err)?;
            let tree = repo.find_tree(tree_oid).map_err(git_err)?;
            repo.commit(Some("HEAD"), &sig, &sig, "initial commit", &tree, &[])
                .map_err(git_err)?;
        }

        Ok(repo)
    }
}

/// Stage all changes and commit.
///
/// Returns `None` if the working tree is clean (nothing to commit).
/// This is a **blocking** function — callers must use `spawn_blocking`.
pub fn commit_execution(
    repo: &Repository,
    execution_id: &str,
    status: &str,
    summary: &str,
) -> Result<Option<Oid>, WorkerError> {
    let mut index = repo.index().map_err(git_err)?;

    // Stage modifications + deletions, then new files.
    index
        .update_all(["*"].iter(), None)
        .map_err(git_err)?;
    index
        .add_all(["*"].iter(), IndexAddOption::DEFAULT, None)
        .map_err(git_err)?;
    index.write().map_err(git_err)?;

    let tree_oid = index.write_tree().map_err(git_err)?;

    // Skip commit if tree matches parent.
    if let Ok(head) = repo.head() {
        if let Ok(parent) = head.peel_to_commit() {
            if parent.tree_id() == tree_oid {
                return Ok(None);
            }
        }
    }

    let tree = repo.find_tree(tree_oid).map_err(git_err)?;
    let sig = signature()?;

    let message = format!("execution {execution_id}: {status}\n\n{summary}");

    let oid = if let Ok(head) = repo.head() {
        let parent = head.peel_to_commit().map_err(git_err)?;
        repo.commit(Some("HEAD"), &sig, &sig, &message, &tree, &[&parent])
            .map_err(git_err)?
    } else {
        // No HEAD yet (shouldn't happen after init, but be defensive).
        repo.commit(Some("HEAD"), &sig, &sig, &message, &tree, &[])
            .map_err(git_err)?
    };

    Ok(Some(oid))
}

/// Hard-reset the working tree to HEAD, discarding all changes.
///
/// This is a **blocking** function — callers must use `spawn_blocking`.
pub fn rollback_to_head(repo: &Repository) -> Result<(), WorkerError> {
    let head = repo.head().map_err(git_err)?;
    let commit = head.peel_to_commit().map_err(git_err)?;

    // Hard reset restores both the index and working tree to match the commit.
    let mut checkout = git2::build::CheckoutBuilder::new();
    checkout.force();
    repo.reset(commit.as_object(), ResetType::Hard, Some(&mut checkout))
        .map_err(git_err)?;

    Ok(())
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn setup() -> (tempfile::TempDir, String) {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().to_str().unwrap().to_owned();
        (tmp, root)
    }

    #[test]
    fn init_creates_repo_with_initial_commit() {
        let (_tmp, root) = setup();
        let repo = init_or_open(&root, "agent-1").unwrap();

        // gitdir exists
        let gitdir = PathBuf::from(format!("{root}/.gitdirs/agent-1"));
        assert!(gitdir.join("HEAD").exists());

        // HEAD resolves to a commit on main
        let head = repo.head().unwrap();
        assert_eq!(head.shorthand().unwrap(), "main");
        let commit = head.peel_to_commit().unwrap();
        assert_eq!(commit.message().unwrap(), "initial commit");
    }

    #[test]
    fn init_is_idempotent() {
        let (_tmp, root) = setup();
        let repo1 = init_or_open(&root, "agent-1").unwrap();
        let oid1 = repo1.head().unwrap().target().unwrap();

        let repo2 = init_or_open(&root, "agent-1").unwrap();
        let oid2 = repo2.head().unwrap().target().unwrap();

        assert_eq!(oid1, oid2, "reopening should not create a new commit");
    }

    #[test]
    fn commit_records_changes() {
        let (_tmp, root) = setup();
        let repo = init_or_open(&root, "agent-1").unwrap();

        // Write a file in the workdir
        let workdir = format!("{root}/agent-1");
        fs::write(format!("{workdir}/output.txt"), "hello").unwrap();

        let oid = commit_execution(&repo, "exec-1", "completed", "all good")
            .unwrap()
            .expect("should produce a commit");

        let commit = repo.find_commit(oid).unwrap();
        assert!(commit.message().unwrap().contains("exec-1"));
        assert!(commit.message().unwrap().contains("completed"));

        // The committed tree should contain output.txt
        let tree = commit.tree().unwrap();
        assert!(tree.get_name("output.txt").is_some());
    }

    #[test]
    fn commit_returns_none_when_no_changes() {
        let (_tmp, root) = setup();
        let repo = init_or_open(&root, "agent-1").unwrap();

        let result = commit_execution(&repo, "exec-1", "completed", "nothing").unwrap();
        assert!(result.is_none(), "no-op commit should be skipped");
    }

    #[test]
    fn commit_handles_deletions() {
        let (_tmp, root) = setup();
        let repo = init_or_open(&root, "agent-1").unwrap();

        let workdir = format!("{root}/agent-1");
        fs::write(format!("{workdir}/temp.txt"), "data").unwrap();
        commit_execution(&repo, "exec-1", "completed", "add file").unwrap();

        // Delete the file
        fs::remove_file(format!("{workdir}/temp.txt")).unwrap();
        let oid = commit_execution(&repo, "exec-2", "completed", "remove file")
            .unwrap()
            .expect("deletion should produce a commit");

        let commit = repo.find_commit(oid).unwrap();
        let tree = commit.tree().unwrap();
        assert!(tree.get_name("temp.txt").is_none(), "deleted file should not be in tree");
    }

    #[test]
    fn handles_deleted_gitlink() {
        let (_tmp, root) = setup();
        init_or_open(&root, "agent-1").unwrap();

        // Delete the .git gitlink
        let gitlink = format!("{root}/agent-1/.git");
        fs::remove_file(&gitlink).unwrap();

        // Reopening should recreate it
        let repo = init_or_open(&root, "agent-1").unwrap();
        assert!(repo.head().is_ok());
        assert!(PathBuf::from(&gitlink).exists());
    }

    #[test]
    fn rollback_discards_changes() {
        let (_tmp, root) = setup();
        let repo = init_or_open(&root, "agent-1").unwrap();

        let workdir = format!("{root}/agent-1");
        fs::write(format!("{workdir}/keep.txt"), "original").unwrap();
        commit_execution(&repo, "exec-1", "completed", "baseline").unwrap();

        // Modify the file
        fs::write(format!("{workdir}/keep.txt"), "modified").unwrap();
        // Add a new untracked file
        fs::write(format!("{workdir}/extra.txt"), "new").unwrap();

        rollback_to_head(&repo).unwrap();

        let content = fs::read_to_string(format!("{workdir}/keep.txt")).unwrap();
        assert_eq!(content, "original", "file should be restored");

        // Note: git reset --hard does not remove untracked files, only
        // restores tracked ones. This matches standard git behavior.
    }
}
