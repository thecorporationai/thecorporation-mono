//! Entity store — reads and writes entity data to git repos or Valkey.

use std::cell::RefCell;
use std::rc::Rc;

use serde::Serialize;

use crate::domain::equity::cap_table::CapTable;
use crate::domain::formation::{
    document::Document, entity::Entity, filing::Filing, tax_profile::TaxProfile,
};
use crate::domain::governance::{agenda_item::AgendaItem, resolution::Resolution, vote::Vote};
use crate::domain::ids::{
    AgendaItemId, DocumentId, EntityId, MeetingId, ResolutionId, VoteId, WorkspaceId,
};
use crate::git::commit::{FileWrite, commit_files};
use crate::git::error::GitStorageError;
use crate::git::repo::CorpRepo;

use super::RepoLayout;
use super::stored_entity::StoredEntity;

/// Normalized branch info returned by dispatch methods.
pub struct BranchInfoResult {
    pub name: String,
    pub head_oid: String,
}

/// Normalized merge result returned by dispatch methods.
#[derive(Debug)]
pub enum MergeOutcome {
    FastForward { oid: String },
    AlreadyUpToDate,
    ThreeWayMerge { oid: String },
    Squash { oid: String },
}

/// Internal backend discriminant.
enum Backend {
    Git {
        repo: CorpRepo,
    },
    Kv {
        store: Rc<RefCell<corp_store::CorpStore<redis::Connection>>>,
    },
}

/// Operations on a single entity's storage (git repo or Valkey).
pub struct EntityStore<'a> {
    backend: Backend,
    layout: &'a RepoLayout,
}

impl<'a> EntityStore<'a> {
    /// Initialize a new entity repo with the initial entity record.
    pub fn init(
        layout: &'a RepoLayout,
        workspace_id: WorkspaceId,
        entity_id: EntityId,
        entity: &Entity,
        valkey_client: Option<&redis::Client>,
    ) -> Result<Self, GitStorageError> {
        let backend = match valkey_client {
            None => {
                let path = layout.entity_repo_path(workspace_id, entity_id);
                let repo = CorpRepo::init(&path, None)?;
                let files = vec![FileWrite::json("corp.json", entity)?];
                commit_files(&repo, "main", "Initialize entity", &files, None)?;
                Backend::Git { repo }
            }
            Some(client) => {
                let con = client
                    .get_connection()
                    .map_err(|e| GitStorageError::Git(e.to_string()))?;
                let ws = workspace_id.to_string();
                let ent = entity_id.to_string();
                let mut cs = corp_store::CorpStore::new(con, &ws, &ent);
                let vfiles = vec![corp_store::entry::FileWrite::json("corp.json", entity)
                    .map_err(|e| GitStorageError::SerializationError(e.to_string()))?];
                cs.commit_files(
                    "main",
                    "Initialize entity",
                    &vfiles,
                    None,
                    chrono::Utc::now(),
                )?;
                Backend::Kv {
                    store: Rc::new(RefCell::new(cs)),
                }
            }
        };
        Ok(Self { backend, layout })
    }

    /// Open an existing entity store.
    pub fn open(
        layout: &'a RepoLayout,
        workspace_id: WorkspaceId,
        entity_id: EntityId,
        valkey_client: Option<&redis::Client>,
    ) -> Result<Self, GitStorageError> {
        let backend = match valkey_client {
            None => {
                let path = layout.entity_repo_path(workspace_id, entity_id);
                let repo = CorpRepo::open(&path)?;
                Backend::Git { repo }
            }
            Some(client) => {
                let con = client
                    .get_connection()
                    .map_err(|e| GitStorageError::Git(e.to_string()))?;
                let ws = workspace_id.to_string();
                let ent = entity_id.to_string();
                let mut cs = corp_store::CorpStore::new(con, &ws, &ent);
                // Verify the entity exists by resolving its main ref.
                match cs.resolve_ref("main") {
                    Ok(_) => {}
                    Err(corp_store::StoreError::RefNotFound(_)) => {
                        return Err(GitStorageError::RepoNotFound(format!("{ws}/{ent}")));
                    }
                    Err(e) => return Err(GitStorageError::from(e)),
                }
                Backend::Kv {
                    store: Rc::new(RefCell::new(cs)),
                }
            }
        };
        Ok(Self { backend, layout })
    }

    /// Open an existing entity store, reusing a shared KV connection.
    ///
    /// When `shared_con` is `Some`, uses the provided connection instead of
    /// creating a new one. This avoids creating a TCP connection per store
    /// in loops that open many stores sequentially.
    pub fn open_shared(
        layout: &'a RepoLayout,
        workspace_id: WorkspaceId,
        entity_id: EntityId,
        shared_store: Option<Rc<RefCell<corp_store::CorpStore<redis::Connection>>>>,
    ) -> Result<Self, GitStorageError> {
        let backend = match shared_store {
            None => {
                let path = layout.entity_repo_path(workspace_id, entity_id);
                let repo = CorpRepo::open(&path)?;
                Backend::Git { repo }
            }
            Some(store) => {
                let ws = workspace_id.to_string();
                let ent = entity_id.to_string();
                {
                    let mut s = store.borrow_mut();
                    match corp_store::store::resolve_ref(s.con(), &ws, &ent, "main") {
                        Ok(_) => {}
                        Err(corp_store::StoreError::RefNotFound(_)) => {
                            return Err(GitStorageError::RepoNotFound(format!("{ws}/{ent}")));
                        }
                        Err(e) => return Err(GitStorageError::from(e)),
                    }
                }
                Backend::Kv { store }
            }
        };
        Ok(Self { backend, layout })
    }

    /// List entity IDs and return a shared connection for subsequent `open_shared` calls.
    ///
    /// In git mode, lists entities from the filesystem and returns `None` for the connection.
    /// In Valkey mode, creates a single connection, lists entities, and returns the connection
    /// wrapped in `Rc<RefCell<_>>` for reuse.
    /// List entity IDs and return a shared connection for subsequent `open_shared` calls.
    ///
    /// Returns both a `CorpStore`-wrapped connection (for EntityStore) and a raw
    /// connection (for WorkspaceStore, which hasn't been migrated yet).
    pub fn list_and_prepare(
        layout: &RepoLayout,
        workspace_id: WorkspaceId,
        valkey_client: Option<&redis::Client>,
    ) -> Result<(
        Vec<EntityId>,
        Option<Rc<RefCell<corp_store::CorpStore<redis::Connection>>>>,
        Option<Rc<RefCell<redis::Connection>>>,
    ), GitStorageError> {
        match valkey_client {
            None => Ok((layout.list_entity_ids(workspace_id), None, None)),
            Some(client) => {
                // Two connections: one for CorpStore (EntityStore), one raw (WorkspaceStore).
                let mut con1 = client
                    .get_connection()
                    .map_err(|e| GitStorageError::Git(e.to_string()))?;
                let con2 = client
                    .get_connection()
                    .map_err(|e| GitStorageError::Git(e.to_string()))?;
                let ws = workspace_id.to_string();
                let ids = corp_store::store::list_entities(&mut con1, &ws)
                    .map_err(GitStorageError::from)?
                    .into_iter()
                    .filter_map(|s| s.parse().ok())
                    .collect();
                let cs = corp_store::CorpStore::new(con1, &ws, "");
                Ok((
                    ids,
                    Some(Rc::new(RefCell::new(cs))),
                    Some(Rc::new(RefCell::new(con2))),
                ))
            }
        }
    }

    /// Write multiple files atomically.
    pub fn commit(
        &self,
        branch: &str,
        message: &str,
        files: Vec<FileWrite>,
    ) -> Result<(), GitStorageError> {
        self.dispatch_commit(branch, message, &files)
    }

    /// Get the layout reference.
    pub fn layout(&self) -> &RepoLayout {
        self.layout
    }

    // ── Generic StoredEntity methods ─────────────────────────────────

    /// Read a stored entity by ID.
    pub fn read<T: StoredEntity>(&self, branch: &str, id: T::Id) -> Result<T, GitStorageError> {
        self.read_json(branch, &T::storage_path(id))
    }

    /// List all IDs for a stored entity type.
    pub fn list_ids<T: StoredEntity>(&self, branch: &str) -> Result<Vec<T::Id>, GitStorageError> {
        self.list_ids_in_dir(branch, T::storage_dir())
    }

    /// Read all stored entities of a given type from a branch.
    pub fn read_all<T: StoredEntity>(&self, branch: &str) -> Result<Vec<T>, GitStorageError> {
        let ids = self.list_ids::<T>(branch)?;
        let mut items = Vec::with_capacity(ids.len());
        for id in ids {
            items.push(self.read::<T>(branch, id)?);
        }
        Ok(items)
    }

    /// Write a stored entity and commit.
    pub fn write<T: StoredEntity>(
        &self,
        branch: &str,
        id: T::Id,
        value: &T,
        message: &str,
    ) -> Result<(), GitStorageError> {
        let path = T::storage_path(id);
        let files = vec![FileWrite::json(path, value)?];
        self.dispatch_commit(branch, message, &files)
    }

    // ── Singletons & special paths (not ID-based) ────────────────────

    /// Read the entity record (corp.json) from a branch.
    pub fn read_entity(&self, branch: &str) -> Result<Entity, GitStorageError> {
        self.read_json(branch, "corp.json")
    }

    /// Write the entity record.
    pub fn write_entity(
        &self,
        branch: &str,
        entity: &Entity,
        message: &str,
    ) -> Result<(), GitStorageError> {
        let files = vec![FileWrite::json("corp.json", entity)?];
        self.dispatch_commit(branch, message, &files)
    }

    /// Read the cap table record.
    pub fn read_cap_table(&self, branch: &str) -> Result<CapTable, GitStorageError> {
        self.read_json(branch, "cap-table/cap-table.json")
    }

    /// Read filing record.
    pub fn read_filing(&self, branch: &str) -> Result<Filing, GitStorageError> {
        self.read_json(branch, "formation/filing.json")
    }

    /// Read tax profile.
    pub fn read_tax_profile(&self, branch: &str) -> Result<TaxProfile, GitStorageError> {
        self.read_json(branch, "tax/profile.json")
    }

    // ── Documents (special: skips filing.json) ───────────────────────

    /// Read a document.
    pub fn read_document(
        &self,
        branch: &str,
        doc_id: DocumentId,
    ) -> Result<Document, GitStorageError> {
        self.read_json(branch, &format!("formation/{}.json", doc_id))
    }

    /// Write a document.
    pub fn write_document(
        &self,
        branch: &str,
        doc: &Document,
        message: &str,
    ) -> Result<(), GitStorageError> {
        let path = format!("formation/{}.json", doc.document_id());
        let files = vec![FileWrite::json(path, doc)?];
        self.dispatch_commit(branch, message, &files)
    }

    /// List all document IDs in the formation/ directory.
    pub fn list_document_ids(&self, branch: &str) -> Result<Vec<DocumentId>, GitStorageError> {
        let entries = self.list_dir(branch, "formation")?;
        let mut ids = Vec::new();
        for (name, is_dir) in entries {
            if is_dir {
                continue;
            }
            if name == "filing.json" {
                continue;
            }
            if let Some(uuid_str) = name.strip_suffix(".json") {
                if let Ok(id) = uuid_str.parse() {
                    ids.push(id);
                }
            }
        }
        Ok(ids)
    }

    // ── Nested governance types (need meeting_id context) ────────────

    /// Read an agenda item from a meeting.
    pub fn read_agenda_item(
        &self,
        branch: &str,
        meeting_id: MeetingId,
        id: AgendaItemId,
    ) -> Result<AgendaItem, GitStorageError> {
        self.read_json(
            branch,
            &format!("governance/meetings/{}/agenda/{}.json", meeting_id, id),
        )
    }

    /// List all agenda item IDs for a meeting.
    pub fn list_agenda_item_ids(
        &self,
        branch: &str,
        meeting_id: MeetingId,
    ) -> Result<Vec<AgendaItemId>, GitStorageError> {
        self.list_ids_in_dir(
            branch,
            &format!("governance/meetings/{}/agenda", meeting_id),
        )
    }

    /// Read a vote from a meeting.
    pub fn read_vote(
        &self,
        branch: &str,
        meeting_id: MeetingId,
        id: VoteId,
    ) -> Result<Vote, GitStorageError> {
        self.read_json(
            branch,
            &format!("governance/meetings/{}/votes/{}.json", meeting_id, id),
        )
    }

    /// List all vote IDs for a meeting.
    pub fn list_vote_ids(
        &self,
        branch: &str,
        meeting_id: MeetingId,
    ) -> Result<Vec<VoteId>, GitStorageError> {
        self.list_ids_in_dir(branch, &format!("governance/meetings/{}/votes", meeting_id))
    }

    /// Read a resolution from a meeting.
    pub fn read_resolution(
        &self,
        branch: &str,
        meeting_id: MeetingId,
        id: ResolutionId,
    ) -> Result<Resolution, GitStorageError> {
        self.read_json(
            branch,
            &format!("governance/meetings/{}/resolutions/{}.json", meeting_id, id),
        )
    }

    /// List all resolution IDs for a meeting.
    pub fn list_resolution_ids(
        &self,
        branch: &str,
        meeting_id: MeetingId,
    ) -> Result<Vec<ResolutionId>, GitStorageError> {
        self.list_ids_in_dir(
            branch,
            &format!("governance/meetings/{}/resolutions", meeting_id),
        )
    }

    // ── Generic helpers ──────────────────────────────────────────────

    /// Read raw bytes from a path.
    pub fn read_blob(&self, branch: &str, path: &str) -> Result<Vec<u8>, GitStorageError> {
        match &self.backend {
            Backend::Git { repo } => repo.read_blob(branch, path),
            Backend::Kv { store } => Ok(store.borrow_mut().read_blob(branch, path)?),
        }
    }

    /// Read any deserializable JSON from a path.
    pub fn read_json<T: serde::de::DeserializeOwned>(
        &self,
        branch: &str,
        path: &str,
    ) -> Result<T, GitStorageError> {
        match &self.backend {
            Backend::Git { repo } => repo.read_json(branch, path),
            Backend::Kv { store } => Ok(store.borrow_mut().read_json(branch, path)?),
        }
    }

    /// Write any serializable value to a JSON path and commit it.
    pub fn write_json<T: Serialize>(
        &self,
        branch: &str,
        path: &str,
        value: &T,
        message: &str,
    ) -> Result<(), GitStorageError> {
        let files = vec![FileWrite::json(path, value)?];
        self.dispatch_commit(branch, message, &files)
    }

    /// List directory entries at a given branch and path.
    pub fn list_dir(
        &self,
        branch: &str,
        dir_path: &str,
    ) -> Result<Vec<(String, bool)>, GitStorageError> {
        match &self.backend {
            Backend::Git { repo } => repo.list_dir(branch, dir_path),
            Backend::Kv { store } => Ok(store.borrow_mut().list_dir(branch, dir_path)?),
        }
    }

    /// Check if a path exists at a given branch.
    pub fn path_exists(&self, branch: &str, path: &str) -> Result<bool, GitStorageError> {
        match &self.backend {
            Backend::Git { repo } => repo.path_exists(branch, path),
            Backend::Kv { store } => Ok(store.borrow_mut().path_exists(branch, path)?),
        }
    }

    /// List UUID-style IDs from files in a directory.
    pub fn list_ids_in_dir<T: std::str::FromStr>(
        &self,
        branch: &str,
        dir_path: &str,
    ) -> Result<Vec<T>, GitStorageError> {
        let entries = match self.list_dir(branch, dir_path) {
            Ok(entries) => entries,
            Err(GitStorageError::NotFound(_)) => return Ok(Vec::new()),
            Err(e) => return Err(e),
        };
        let mut ids = Vec::new();
        for (name, is_dir) in entries {
            if is_dir {
                if let Ok(id) = name.parse() {
                    ids.push(id);
                }
                continue;
            }
            if let Some(uuid_str) = name.strip_suffix(".json") {
                if let Ok(id) = uuid_str.parse() {
                    ids.push(id);
                }
            }
        }
        Ok(ids)
    }

    // ── Branch management ────────────────────────────────────────────

    pub fn create_branch(
        &self,
        name: &str,
        from_ref: &str,
    ) -> Result<BranchInfoResult, GitStorageError> {
        match &self.backend {
            Backend::Git { repo } => {
                let info = crate::git::branch::create_branch(repo, name, from_ref)?;
                Ok(BranchInfoResult { name: info.name, head_oid: info.head_oid.to_string() })
            }
            Backend::Kv { store } => {
                let info = store.borrow_mut().create_branch(name, from_ref)?;
                Ok(BranchInfoResult { name: info.name, head_oid: info.head_sha1 })
            }
        }
    }

    pub fn list_branches(&self) -> Result<Vec<BranchInfoResult>, GitStorageError> {
        match &self.backend {
            Backend::Git { repo } => {
                let branches = crate::git::branch::list_branches(repo)?;
                Ok(branches.into_iter().map(|b| BranchInfoResult {
                    name: b.name, head_oid: b.head_oid.to_string(),
                }).collect())
            }
            Backend::Kv { store } => {
                let branches = store.borrow_mut().list_branches()?;
                Ok(branches.into_iter().map(|b| BranchInfoResult {
                    name: b.name, head_oid: b.head_sha1,
                }).collect())
            }
        }
    }

    pub fn delete_branch(&self, name: &str) -> Result<(), GitStorageError> {
        match &self.backend {
            Backend::Git { repo } => crate::git::branch::delete_branch(repo, name),
            Backend::Kv { store } => Ok(store.borrow_mut().delete_branch(name)?),
        }
    }

    pub fn merge_branch(
        &self,
        source: &str,
        target: &str,
        squash: bool,
    ) -> Result<MergeOutcome, GitStorageError> {
        match &self.backend {
            Backend::Git { repo } => {
                let result = if squash {
                    crate::git::merge::merge_branch_squash(repo, source, target, None)?
                } else {
                    crate::git::merge::merge_branch(repo, source, target, None)?
                };
                Ok(match result {
                    crate::git::merge::MergeResult::FastForward { new_oid } =>
                        MergeOutcome::FastForward { oid: new_oid.to_string() },
                    crate::git::merge::MergeResult::AlreadyUpToDate =>
                        MergeOutcome::AlreadyUpToDate,
                    crate::git::merge::MergeResult::ThreeWayMerge { new_oid } =>
                        MergeOutcome::ThreeWayMerge { oid: new_oid.to_string() },
                    crate::git::merge::MergeResult::Squash { new_oid } =>
                        MergeOutcome::Squash { oid: new_oid.to_string() },
                })
            }
            Backend::Kv { store } => {
                let result = store.borrow_mut().merge_branch(source, target, squash, None)?;
                Ok(match result {
                    corp_store::merge::MergeResult::FastForward { sha1 } =>
                        MergeOutcome::FastForward { oid: sha1 },
                    corp_store::merge::MergeResult::AlreadyUpToDate =>
                        MergeOutcome::AlreadyUpToDate,
                    corp_store::merge::MergeResult::ThreeWayMerge { sha1 } =>
                        MergeOutcome::ThreeWayMerge { oid: sha1 },
                    corp_store::merge::MergeResult::Squash { sha1 } =>
                        MergeOutcome::Squash { oid: sha1 },
                })
            }
        }
    }

    // ── Internal dispatch ────────────────────────────────────────────

    fn dispatch_commit(
        &self,
        branch: &str,
        message: &str,
        files: &[FileWrite],
    ) -> Result<(), GitStorageError> {
        match &self.backend {
            Backend::Git { repo } => {
                commit_files(repo, branch, message, files, None)?;
                Ok(())
            }
            Backend::Kv { store } => {
                let vfiles: Vec<corp_store::entry::FileWrite> = files
                    .iter()
                    .map(|f| corp_store::entry::FileWrite::new(&f.path, f.content.clone()))
                    .collect();
                store.borrow_mut().commit_files(
                    branch, message, &vfiles, None, chrono::Utc::now(),
                )?;
                Ok(())
            }
        }
    }
}
