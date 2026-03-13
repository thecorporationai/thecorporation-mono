//! Core log engine: materialize, append, replay.
//!
//! Each bare git repo gets a `.corplog/` directory containing:
//! - `log.jsonld`  — NDJSON-LD stream (line 0 = context, lines 1..N = entries)
//!
//! The engine walks git history, diffs each commit against its parent,
//! and appends structured entries. On read, entries are indexed in memory
//! for O(1) SHA lookup and fast sequential replay.

use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use chrono::DateTime;
use git2::{DiffOptions, Oid, Repository, Sort};

use crate::context::LdContext;
use crate::entry::*;
use crate::error::LogEngineError;
use crate::index::{ShaIndex, IndexEntry};

const LOG_DIR: &str = ".corplog";
const LOG_FILE: &str = "log.jsonld";
const CHECKPOINT_INTERVAL: u64 = 100;

/// Per-repo log engine handle.
pub struct LogEngine {
    repo: Repository,
    log_path: PathBuf,
    index: ShaIndex,
}

impl LogEngine {
    /// Open (or create) the log engine for a bare git repo.
    pub fn open(repo_path: &Path) -> Result<Self, LogEngineError> {
        let repo = Repository::open_bare(repo_path)?;
        let log_dir = repo_path.join(LOG_DIR);
        fs::create_dir_all(&log_dir)?;
        let log_path = log_dir.join(LOG_FILE);

        let mut engine = Self {
            repo,
            log_path,
            index: ShaIndex::new(),
        };

        if engine.log_path.exists() {
            engine.rebuild_index()?;
        } else {
            engine.write_context_header()?;
        }

        Ok(engine)
    }

    /// The path to the NDJSON-LD log file.
    pub fn log_path(&self) -> &Path {
        &self.log_path
    }

    /// Number of indexed entries.
    pub fn entry_count(&self) -> usize {
        self.index.len()
    }

    /// SHA of the most recently materialized commit.
    pub fn tip(&self) -> Option<&str> {
        self.index.tip()
    }

    // ── Materialization ──────────────────────────────────────────────

    /// Materialize git history from a ref into the log.
    ///
    /// Walks commits in topological order and appends entries for any
    /// commits not yet in the index. Returns the number of new entries.
    pub fn materialize(&mut self, refname: &str) -> Result<u64, LogEngineError> {
        let full_ref = normalize_ref(refname);
        let head_oid = self
            .repo
            .find_reference(&full_ref)
            .map_err(|e| LogEngineError::Git(format!("ref {full_ref}: {e}")))?
            .target()
            .ok_or_else(|| LogEngineError::Git(format!("ref {full_ref} has no target")))?;

        // Already up to date?
        let head_hex = head_oid.to_string();
        if self.index.get(&head_hex).is_some() {
            return Ok(0);
        }

        // Collect commits in reverse chronological order, then reverse.
        let commits = self.collect_new_commits(head_oid)?;
        if commits.is_empty() {
            return Ok(0);
        }

        let mut file = OpenOptions::new()
            .append(true)
            .open(&self.log_path)?;
        let mut writer = BufWriter::new(&mut file);
        let mut count = 0u64;

        for oid in &commits {
            let commit = self.repo.find_commit(*oid)?;
            let entry = self.build_commit_entry(&commit)?;
            let seq = entry.sequence;
            let sha = entry.sha.clone();

            let offset = writer.stream_position()?;
            let log_entry = LogEntry::Commit(entry);
            serde_json::to_writer(&mut writer, &log_entry)?;
            writer.write_all(b"\n")?;

            self.index.insert(sha.clone(), offset, seq);
            count += 1;

            // Periodic checkpoint.
            if seq > 0 && seq % CHECKPOINT_INTERVAL == 0 {
                let checkpoint = self.build_checkpoint(&commit, seq)?;
                let ckpt_offset = writer.stream_position()?;
                let ckpt_entry = LogEntry::Checkpoint(checkpoint);
                serde_json::to_writer(&mut writer, &ckpt_entry)?;
                writer.write_all(b"\n")?;
                self.index
                    .insert(format!("ckpt:{sha}"), ckpt_offset, seq);
            }
        }

        writer.flush()?;

        tracing::info!(
            ref_ = refname,
            new_entries = count,
            total = self.index.len(),
            "materialized git history"
        );

        Ok(count)
    }

    /// Append a single commit entry (for use right after `commit_files`).
    ///
    /// More efficient than full materialization when you just made a commit.
    pub fn append_commit(&mut self, oid: Oid) -> Result<u64, LogEngineError> {
        let commit = self.repo.find_commit(oid)?;
        let entry = self.build_commit_entry(&commit)?;
        let seq = entry.sequence;
        let sha = entry.sha.clone();

        let mut file = OpenOptions::new()
            .append(true)
            .open(&self.log_path)?;
        let mut writer = BufWriter::new(&mut file);

        let offset = writer.stream_position()?;
        let log_entry = LogEntry::Commit(entry);
        serde_json::to_writer(&mut writer, &log_entry)?;
        writer.write_all(b"\n")?;

        self.index.insert(sha.clone(), offset, seq);

        if seq > 0 && seq % CHECKPOINT_INTERVAL == 0 {
            let checkpoint = self.build_checkpoint(&commit, seq)?;
            let ckpt_offset = writer.stream_position()?;
            let ckpt_entry = LogEntry::Checkpoint(checkpoint);
            serde_json::to_writer(&mut writer, &ckpt_entry)?;
            writer.write_all(b"\n")?;
            self.index
                .insert(format!("ckpt:{sha}"), ckpt_offset, seq);
        }

        writer.flush()?;
        Ok(seq)
    }

    // ── Replay ───────────────────────────────────────────────────────

    /// Replay the log up to `target_sha` and return the file tree state.
    ///
    /// Uses the nearest checkpoint as a starting point for efficiency.
    /// Returns a map of path -> blob SHA.
    pub fn replay_to(&self, target_sha: &str) -> Result<BTreeMap<String, String>, LogEngineError> {
        let target_entry = self
            .index
            .get(target_sha)
            .ok_or_else(|| LogEngineError::CommitNotFound(target_sha.to_owned()))?;
        let target_seq = target_entry.sequence;

        let file = File::open(&self.log_path)?;
        let reader = BufReader::new(file);

        // Find nearest checkpoint before target.
        let (mut tree, start_seq) =
            if let Some((_key, ckpt_entry)) = self.index.latest_checkpoint_before(target_seq) {
                let ckpt = self.read_entry_at(ckpt_entry)?;
                match ckpt {
                    LogEntry::Checkpoint(c) => {
                        let tree: BTreeMap<String, String> = c
                            .tree
                            .into_iter()
                            .map(|f| (f.path, f.blob_sha))
                            .collect();
                        (tree, c.sequence + 1)
                    }
                    _ => (BTreeMap::new(), 1),
                }
            } else {
                (BTreeMap::new(), 1)
            };

        // Replay commits from start_seq..=target_seq.
        for line_result in reader.lines() {
            let line = line_result?;
            if line.starts_with('{') {
                if let Ok(entry) = serde_json::from_str::<LogEntry>(&line) {
                    let seq = entry.sequence();
                    if seq < start_seq {
                        continue;
                    }
                    if seq > target_seq {
                        break;
                    }
                    if let LogEntry::Commit(c) = entry {
                        apply_changes(&mut tree, &c.changes);
                    }
                }
            }
        }

        Ok(tree)
    }

    /// Replay the full log and return the latest file tree state.
    pub fn replay_latest(&self) -> Result<BTreeMap<String, String>, LogEngineError> {
        match self.index.tip() {
            Some(sha) => self.replay_to(sha),
            None => Ok(BTreeMap::new()),
        }
    }

    /// Read all commit entries in sequence order.
    pub fn read_all_commits(&self) -> Result<Vec<CommitEntry>, LogEngineError> {
        let file = File::open(&self.log_path)?;
        let reader = BufReader::new(file);
        let mut commits = Vec::new();

        for line_result in reader.lines() {
            let line = line_result?;
            if line.starts_with('{') {
                if let Ok(LogEntry::Commit(c)) = serde_json::from_str::<LogEntry>(&line) {
                    commits.push(c);
                }
            }
        }

        commits.sort_by_key(|c| c.sequence);
        Ok(commits)
    }

    /// Read a single entry by SHA.
    pub fn read_commit(&self, sha: &str) -> Result<CommitEntry, LogEngineError> {
        let entry = self
            .index
            .get(sha)
            .ok_or_else(|| LogEngineError::CommitNotFound(sha.to_owned()))?;
        match self.read_entry_at(entry)? {
            LogEntry::Commit(c) => Ok(c),
            _ => Err(LogEngineError::CommitNotFound(sha.to_owned())),
        }
    }

    // ── Audit queries ────────────────────────────────────────────────

    /// Return all commits that touched a given file path.
    pub fn history_of(&self, path: &str) -> Result<Vec<CommitEntry>, LogEngineError> {
        let all = self.read_all_commits()?;
        Ok(all
            .into_iter()
            .filter(|c| c.changes.iter().any(|ch| ch.path == path))
            .collect())
    }

    /// Return all commits by a given workspace.
    pub fn commits_by_workspace(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<CommitEntry>, LogEngineError> {
        let all = self.read_all_commits()?;
        Ok(all
            .into_iter()
            .filter(|c| c.workspace_id.as_deref() == Some(workspace_id))
            .collect())
    }

    // ── Internal ─────────────────────────────────────────────────────

    fn write_context_header(&self) -> Result<(), LogEngineError> {
        let mut file = File::create(&self.log_path)?;
        let ctx = LdContext::default();
        serde_json::to_writer(&mut file, &ctx)?;
        file.write_all(b"\n")?;
        Ok(())
    }

    fn rebuild_index(&mut self) -> Result<(), LogEngineError> {
        let file = File::open(&self.log_path)?;
        let reader = BufReader::new(file);
        let mut offset = 0u64;

        for line_result in reader.lines() {
            let line = line_result?;
            let line_len = line.len() as u64 + 1; // +1 for newline

            if line.starts_with('{') {
                if let Ok(entry) = serde_json::from_str::<LogEntry>(&line) {
                    match &entry {
                        LogEntry::Commit(c) => {
                            self.index.insert(c.sha.clone(), offset, c.sequence);
                        }
                        LogEntry::Checkpoint(c) => {
                            self.index.insert(
                                format!("ckpt:{}", c.at_sha),
                                offset,
                                c.sequence,
                            );
                        }
                    }
                }
            }

            offset += line_len;
        }

        tracing::debug!(
            entries = self.index.len(),
            tip = ?self.index.tip(),
            "rebuilt log index"
        );

        Ok(())
    }

    /// Collect commit OIDs not yet in the index, oldest first.
    fn collect_new_commits(&self, head: Oid) -> Result<Vec<Oid>, LogEngineError> {
        let mut revwalk = self.repo.revwalk()?;
        revwalk.push(head)?;
        revwalk.set_sorting(Sort::TOPOLOGICAL | Sort::REVERSE)?;

        let mut new_commits = Vec::new();
        for maybe_oid in revwalk {
            let oid = maybe_oid?;
            if self.index.get(&oid.to_string()).is_none() {
                new_commits.push(oid);
            }
        }

        Ok(new_commits)
    }

    fn build_commit_entry(
        &self,
        commit: &git2::Commit<'_>,
    ) -> Result<CommitEntry, LogEngineError> {
        let sha = commit.id().to_string();
        let parents: Vec<String> = (0..commit.parent_count())
            .map(|i| commit.parent_id(i).unwrap().to_string())
            .collect();

        let author = commit.author();
        let message_raw = commit.message().unwrap_or("");

        // Parse actor trailer from message.
        let (message, trailer) = parse_trailer(message_raw);

        let time = commit.time();
        let timestamp = DateTime::from_timestamp(time.seconds(), 0).unwrap_or_default();

        let changes = self.diff_commit(commit)?;

        let seq = self.index.max_sequence() + 1;

        Ok(CommitEntry {
            ld_id: format!("git:sha/{sha}"),
            sha,
            parents,
            author: PersonIdent {
                name: author.name().unwrap_or("").to_owned(),
                email: author.email().unwrap_or("").to_owned(),
            },
            message: message.to_owned(),
            timestamp,
            sequence: seq,
            workspace_id: trailer.workspace_id,
            entity_id: trailer.entity_id,
            scopes: trailer.scopes,
            signed_by: trailer.signed_by,
            changes,
        })
    }

    fn diff_commit(&self, commit: &git2::Commit<'_>) -> Result<Vec<FileChange>, LogEngineError> {
        let new_tree = commit.tree()?;

        let old_tree = if commit.parent_count() > 0 {
            Some(commit.parent(0)?.tree()?)
        } else {
            None
        };

        let mut opts = DiffOptions::new();
        opts.include_untracked(false);

        let diff = self.repo.diff_tree_to_tree(
            old_tree.as_ref(),
            Some(&new_tree),
            Some(&mut opts),
        )?;

        let mut changes = Vec::new();
        diff.foreach(
            &mut |delta, _| {
                let action = match delta.status() {
                    git2::Delta::Added => ChangeAction::Add,
                    git2::Delta::Deleted => ChangeAction::Delete,
                    git2::Delta::Modified => ChangeAction::Modify,
                    git2::Delta::Renamed => ChangeAction::Rename,
                    _ => return true,
                };

                let new_file = delta.new_file();
                let path = new_file
                    .path()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default();

                let blob_sha = if action != ChangeAction::Delete {
                    Some(new_file.id().to_string())
                } else {
                    None
                };

                let old_path = if action == ChangeAction::Rename {
                    delta
                        .old_file()
                        .path()
                        .map(|p| p.to_string_lossy().to_string())
                } else {
                    None
                };

                changes.push(FileChange {
                    path,
                    action,
                    blob_sha,
                    old_path,
                });
                true
            },
            None,
            None,
            None,
        )?;

        Ok(changes)
    }

    fn build_checkpoint(
        &self,
        commit: &git2::Commit<'_>,
        sequence: u64,
    ) -> Result<CheckpointEntry, LogEngineError> {
        let sha = commit.id().to_string();
        let tree = commit.tree()?;
        let mut files = Vec::new();

        tree.walk(git2::TreeWalkMode::PreOrder, |dir, entry| {
            if entry.kind() == Some(git2::ObjectType::Blob) {
                let path = if dir.is_empty() {
                    entry.name().unwrap_or("").to_owned()
                } else {
                    format!("{dir}{}", entry.name().unwrap_or(""))
                };
                files.push(TreeFile {
                    path,
                    blob_sha: entry.id().to_string(),
                });
            }
            git2::TreeWalkResult::Ok
        })?;

        let time = commit.time();
        let timestamp = DateTime::from_timestamp(time.seconds(), 0).unwrap_or_default();

        Ok(CheckpointEntry {
            ld_id: format!("git:checkpoint/{sha}"),
            at_sha: sha,
            sequence,
            timestamp,
            tree: files,
        })
    }

    fn read_entry_at(&self, idx_entry: &IndexEntry) -> Result<LogEntry, LogEngineError> {
        let mut file = File::open(&self.log_path)?;
        file.seek(SeekFrom::Start(idx_entry.offset))?;
        let mut reader = BufReader::new(file);
        let mut line = String::new();
        reader.read_line(&mut line)?;
        serde_json::from_str(&line).map_err(|e| {
            LogEngineError::Corrupt(format!("bad entry at offset {}: {e}", idx_entry.offset))
        })
    }
}

// ── Helpers ──────────────────────────────────────────────────────────

fn normalize_ref(refname: &str) -> String {
    if refname.starts_with("refs/") {
        refname.to_owned()
    } else {
        format!("refs/heads/{refname}")
    }
}

fn apply_changes(tree: &mut BTreeMap<String, String>, changes: &[FileChange]) {
    for change in changes {
        match change.action {
            ChangeAction::Add | ChangeAction::Modify => {
                if let Some(ref sha) = change.blob_sha {
                    tree.insert(change.path.clone(), sha.clone());
                }
            }
            ChangeAction::Delete => {
                tree.remove(&change.path);
            }
            ChangeAction::Rename => {
                if let Some(ref old) = change.old_path {
                    tree.remove(old);
                }
                if let Some(ref sha) = change.blob_sha {
                    tree.insert(change.path.clone(), sha.clone());
                }
            }
        }
    }
}

/// Parsed actor trailer from a commit message.
struct ActorTrailer {
    workspace_id: Option<String>,
    entity_id: Option<String>,
    scopes: Vec<String>,
    signed_by: Option<String>,
}

/// Split a commit message into (body, trailer).
fn parse_trailer(message: &str) -> (&str, ActorTrailer) {
    let mut trailer = ActorTrailer {
        workspace_id: None,
        entity_id: None,
        scopes: Vec::new(),
        signed_by: None,
    };

    let body = if let Some(idx) = message.find("\n\n---\n") {
        let trailer_text = &message[idx + 5..];
        for line in trailer_text.lines() {
            if let Some(val) = line.strip_prefix("Actor: ") {
                trailer.workspace_id = Some(val.trim().to_owned());
            } else if let Some(val) = line.strip_prefix("Entity: ") {
                trailer.entity_id = Some(val.trim().to_owned());
            } else if let Some(val) = line.strip_prefix("Scopes: ") {
                trailer.scopes = val.split(',').map(|s| s.trim().to_owned()).collect();
            } else if let Some(val) = line.strip_prefix("Signed-By: ") {
                trailer.signed_by = Some(val.trim().to_owned());
            }
        }
        &message[..idx]
    } else {
        message.trim_end()
    };

    (body, trailer)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn init_bare_repo(path: &Path) -> Repository {
        let repo = Repository::init_bare(path).unwrap();

        // Create initial empty commit. Scope the tree borrow.
        let sig = git2::Signature::now("test", "test@test.com").unwrap();
        {
            let tree_oid = repo.treebuilder(None).unwrap().write().unwrap();
            let tree = repo.find_tree(tree_oid).unwrap();
            repo.commit(Some("refs/heads/main"), &sig, &sig, "initial commit", &tree, &[])
                .unwrap();
        }

        repo
    }

    fn add_file(repo: &Repository, refname: &str, path: &str, content: &[u8], message: &str) {
        let full_ref = normalize_ref(refname);
        let parent_oid = repo
            .find_reference(&full_ref)
            .unwrap()
            .target()
            .unwrap();
        let parent = repo.find_commit(parent_oid).unwrap();
        let base_tree = parent.tree().unwrap();

        let blob_oid = repo.blob(content).unwrap();
        let mut builder = repo.treebuilder(Some(&base_tree)).unwrap();
        builder.insert(path, blob_oid, 0o100644).unwrap();
        let new_tree_oid = builder.write().unwrap();
        let new_tree = repo.find_tree(new_tree_oid).unwrap();

        let sig = git2::Signature::now("test", "test@test.com").unwrap();
        repo.commit(
            Some(&full_ref),
            &sig,
            &sig,
            message,
            &new_tree,
            &[&parent],
        )
        .unwrap();
    }

    #[test]
    fn materialize_and_replay() {
        let tmp = tempfile::TempDir::new().unwrap();
        let repo_path = tmp.path().join("test.git");
        let repo = init_bare_repo(&repo_path);

        add_file(&repo, "main", "hello.txt", b"world", "add hello");
        add_file(&repo, "main", "data.json", b"{}", "add data");
        drop(repo);

        let mut engine = LogEngine::open(&repo_path).unwrap();
        let count = engine.materialize("main").unwrap();
        assert_eq!(count, 3); // initial + 2 file commits

        // Replay should show both files.
        let tree = engine.replay_latest().unwrap();
        assert!(tree.contains_key("hello.txt"));
        assert!(tree.contains_key("data.json"));
        assert_eq!(tree.len(), 2);
    }

    #[test]
    fn incremental_materialize() {
        let tmp = tempfile::TempDir::new().unwrap();
        let repo_path = tmp.path().join("incr.git");
        let repo = init_bare_repo(&repo_path);
        add_file(&repo, "main", "a.txt", b"a", "add a");

        let mut engine = LogEngine::open(&repo_path).unwrap();
        engine.materialize("main").unwrap();
        assert_eq!(engine.entry_count(), 2);

        // Add more commits after first materialize.
        let repo = Repository::open_bare(&repo_path).unwrap();
        add_file(&repo, "main", "b.txt", b"b", "add b");
        drop(repo);

        let count = engine.materialize("main").unwrap();
        assert_eq!(count, 1);
        assert_eq!(engine.entry_count(), 3);
    }

    #[test]
    fn read_single_commit() {
        let tmp = tempfile::TempDir::new().unwrap();
        let repo_path = tmp.path().join("single.git");
        let repo = init_bare_repo(&repo_path);
        add_file(&repo, "main", "f.txt", b"x", "add f");
        drop(repo);

        let mut engine = LogEngine::open(&repo_path).unwrap();
        engine.materialize("main").unwrap();

        let tip_sha = engine.tip().unwrap().to_owned();
        let entry = engine.read_commit(&tip_sha).unwrap();
        assert_eq!(entry.message, "add f");
    }

    #[test]
    fn file_history() {
        let tmp = tempfile::TempDir::new().unwrap();
        let repo_path = tmp.path().join("hist.git");
        let repo = init_bare_repo(&repo_path);
        add_file(&repo, "main", "doc.md", b"v1", "create doc");
        add_file(&repo, "main", "other.txt", b"x", "unrelated");
        add_file(&repo, "main", "doc.md", b"v2", "update doc");
        drop(repo);

        let mut engine = LogEngine::open(&repo_path).unwrap();
        engine.materialize("main").unwrap();

        let history = engine.history_of("doc.md").unwrap();
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].message, "create doc");
        assert_eq!(history[1].message, "update doc");
    }

    #[test]
    fn parses_actor_trailer() {
        let msg = "form entity\n\n---\nActor: ws_123\nEntity: ent_456\nScopes: Admin,FormationCreate\nTimestamp: 2026-03-12T00:00:00Z\nSigned-By: SHA256:abc";
        let (body, trailer) = parse_trailer(msg);
        assert_eq!(body, "form entity");
        assert_eq!(trailer.workspace_id.as_deref(), Some("ws_123"));
        assert_eq!(trailer.entity_id.as_deref(), Some("ent_456"));
        assert_eq!(trailer.scopes, vec!["Admin", "FormationCreate"]);
        assert_eq!(trailer.signed_by.as_deref(), Some("SHA256:abc"));
    }

    #[test]
    fn replay_to_specific_commit() {
        let tmp = tempfile::TempDir::new().unwrap();
        let repo_path = tmp.path().join("replay.git");
        let repo = init_bare_repo(&repo_path);
        add_file(&repo, "main", "a.txt", b"a", "add a");
        add_file(&repo, "main", "b.txt", b"b", "add b");
        add_file(&repo, "main", "c.txt", b"c", "add c");
        drop(repo);

        let mut engine = LogEngine::open(&repo_path).unwrap();
        engine.materialize("main").unwrap();

        let commits = engine.read_all_commits().unwrap();
        // Replay to "add b" (sequence 3: initial=1, a=2, b=3).
        let b_sha = &commits[2].sha;
        let tree = engine.replay_to(b_sha).unwrap();
        assert!(tree.contains_key("a.txt"));
        assert!(tree.contains_key("b.txt"));
        assert!(!tree.contains_key("c.txt")); // not yet
    }

    #[test]
    fn reopen_preserves_index() {
        let tmp = tempfile::TempDir::new().unwrap();
        let repo_path = tmp.path().join("reopen.git");
        let repo = init_bare_repo(&repo_path);
        add_file(&repo, "main", "x.txt", b"x", "add x");
        drop(repo);

        {
            let mut engine = LogEngine::open(&repo_path).unwrap();
            engine.materialize("main").unwrap();
            assert_eq!(engine.entry_count(), 2);
        }

        // Reopen — index rebuilt from file.
        let engine = LogEngine::open(&repo_path).unwrap();
        assert_eq!(engine.entry_count(), 2);
        let tree = engine.replay_latest().unwrap();
        assert!(tree.contains_key("x.txt"));
    }

    #[test]
    fn log_file_is_valid_ndjsonld() {
        let tmp = tempfile::TempDir::new().unwrap();
        let repo_path = tmp.path().join("format.git");
        let repo = init_bare_repo(&repo_path);
        add_file(&repo, "main", "f.txt", b"content", "add file\n\n---\nActor: ws_demo\nEntity: ent_42\nScopes: Admin\nTimestamp: 2026-03-12T00:00:00Z");
        drop(repo);

        let mut engine = LogEngine::open(&repo_path).unwrap();
        engine.materialize("main").unwrap();

        // Every line should be valid JSON.
        let content = std::fs::read_to_string(engine.log_path()).unwrap();
        for (i, line) in content.lines().enumerate() {
            let parsed: serde_json::Value = serde_json::from_str(line)
                .unwrap_or_else(|e| panic!("line {i} invalid JSON: {e}"));
            if i == 0 {
                // Context line has @context.
                assert!(parsed.get("@context").is_some(), "line 0 missing @context");
            } else {
                // Entry lines have @type.
                assert!(parsed.get("@type").is_some(), "line {i} missing @type");
            }
        }

        // Check actor trailer was parsed into the commit entry.
        let commits = engine.read_all_commits().unwrap();
        let last = commits.last().unwrap();
        assert_eq!(last.workspace_id.as_deref(), Some("ws_demo"));
        assert_eq!(last.entity_id.as_deref(), Some("ent_42"));
        assert_eq!(last.scopes, vec!["Admin"]);
    }

    #[test]
    fn append_commit_after_materialize() {
        let tmp = tempfile::TempDir::new().unwrap();
        let repo_path = tmp.path().join("append.git");
        let repo = init_bare_repo(&repo_path);
        add_file(&repo, "main", "a.txt", b"a", "add a");
        drop(repo);

        let mut engine = LogEngine::open(&repo_path).unwrap();
        engine.materialize("main").unwrap();
        assert_eq!(engine.entry_count(), 2);

        // Make a new commit directly, then append it.
        let repo = Repository::open_bare(&repo_path).unwrap();
        add_file(&repo, "main", "b.txt", b"b", "add b");
        let head = repo
            .find_reference("refs/heads/main")
            .unwrap()
            .target()
            .unwrap();
        drop(repo);

        let seq = engine.append_commit(head).unwrap();
        assert_eq!(seq, 3);
        assert_eq!(engine.entry_count(), 3);

        let tree = engine.replay_latest().unwrap();
        assert!(tree.contains_key("b.txt"));
    }

    #[test]
    fn many_commits_trigger_checkpoint() {
        let tmp = tempfile::TempDir::new().unwrap();
        let repo_path = tmp.path().join("ckpt.git");
        let repo = init_bare_repo(&repo_path);

        // Create 101 commits (1 initial + 100 files) to trigger checkpoint at seq 100.
        for i in 0..100 {
            add_file(
                &repo,
                "main",
                &format!("file_{i:03}.txt"),
                format!("content_{i}").as_bytes(),
                &format!("add file {i}"),
            );
        }
        drop(repo);

        let mut engine = LogEngine::open(&repo_path).unwrap();
        engine.materialize("main").unwrap();

        // Should have 101 commit entries + 1 checkpoint.
        let content = std::fs::read_to_string(engine.log_path()).unwrap();
        let checkpoint_count = content
            .lines()
            .filter(|l| l.contains("\"Checkpoint\""))
            .count();
        assert!(checkpoint_count >= 1, "expected at least 1 checkpoint, got {checkpoint_count}");

        // Replay should still work correctly.
        let tree = engine.replay_latest().unwrap();
        assert_eq!(tree.len(), 100);
    }
}
