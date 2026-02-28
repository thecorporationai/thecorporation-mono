//! Entity store — reads and writes entity data to git repos.

use serde::Serialize;

use crate::domain::equity::cap_table::CapTable;
use crate::domain::formation::{
    document::Document, entity::Entity, filing::Filing, tax_profile::TaxProfile,
};
use crate::domain::governance::{
    agenda_item::AgendaItem,
    resolution::Resolution,
    vote::Vote,
};
use crate::domain::ids::{
    AgendaItemId, DocumentId, EntityId, MeetingId, ResolutionId, VoteId, WorkspaceId,
};
use crate::git::commit::{commit_files, FileWrite};
use crate::git::error::GitStorageError;
use crate::git::repo::CorpRepo;

use super::stored_entity::StoredEntity;
use super::RepoLayout;

/// Operations on a single entity's git repo.
pub struct EntityStore<'a> {
    repo: CorpRepo,
    layout: &'a RepoLayout,
}

impl<'a> EntityStore<'a> {
    /// Initialize a new entity repo with the initial entity record.
    pub fn init(
        layout: &'a RepoLayout,
        workspace_id: WorkspaceId,
        entity_id: EntityId,
        entity: &Entity,
    ) -> Result<Self, GitStorageError> {
        let path = layout.entity_repo_path(workspace_id, entity_id);
        let repo = CorpRepo::init(&path, None)?;
        let files = vec![FileWrite::json("corp.json", entity)?];
        commit_files(&repo, "main", "Initialize entity", &files, None)?;
        Ok(Self { repo, layout })
    }

    /// Open an existing entity repo.
    pub fn open(
        layout: &'a RepoLayout,
        workspace_id: WorkspaceId,
        entity_id: EntityId,
    ) -> Result<Self, GitStorageError> {
        let path = layout.entity_repo_path(workspace_id, entity_id);
        let repo = CorpRepo::open(&path)?;
        Ok(Self { repo, layout })
    }

    /// Write multiple files atomically.
    pub fn commit(
        &self,
        branch: &str,
        message: &str,
        files: Vec<FileWrite>,
    ) -> Result<(), GitStorageError> {
        commit_files(&self.repo, branch, message, &files, None)?;
        Ok(())
    }

    /// Get the underlying repo for advanced operations.
    pub fn repo(&self) -> &CorpRepo {
        &self.repo
    }

    /// Get the layout reference.
    pub fn layout(&self) -> &RepoLayout {
        self.layout
    }

    // ── Generic StoredEntity methods ─────────────────────────────────

    /// Read a stored entity by ID.
    pub fn read<T: StoredEntity>(&self, branch: &str, id: T::Id) -> Result<T, GitStorageError> {
        self.repo.read_json(branch, &T::storage_path(id))
    }

    /// List all IDs for a stored entity type.
    pub fn list_ids<T: StoredEntity>(&self, branch: &str) -> Result<Vec<T::Id>, GitStorageError> {
        self.list_ids_in_dir(branch, T::storage_dir())
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
        commit_files(&self.repo, branch, message, &files, None)?;
        Ok(())
    }

    // ── Singletons & special paths (not ID-based) ────────────────────

    /// Read the entity record (corp.json) from a branch.
    pub fn read_entity(&self, branch: &str) -> Result<Entity, GitStorageError> {
        self.repo.read_json(branch, "corp.json")
    }

    /// Write the entity record.
    pub fn write_entity(
        &self,
        branch: &str,
        entity: &Entity,
        message: &str,
    ) -> Result<(), GitStorageError> {
        let files = vec![FileWrite::json("corp.json", entity)?];
        commit_files(&self.repo, branch, message, &files, None)?;
        Ok(())
    }

    /// Read the cap table record.
    pub fn read_cap_table(&self, branch: &str) -> Result<CapTable, GitStorageError> {
        self.repo.read_json(branch, "cap-table/cap-table.json")
    }

    /// Read filing record.
    pub fn read_filing(&self, branch: &str) -> Result<Filing, GitStorageError> {
        self.repo.read_json(branch, "formation/filing.json")
    }

    /// Read tax profile.
    pub fn read_tax_profile(&self, branch: &str) -> Result<TaxProfile, GitStorageError> {
        self.repo.read_json(branch, "tax/profile.json")
    }

    // ── Documents (special: skips filing.json) ───────────────────────

    /// Read a document.
    pub fn read_document(
        &self,
        branch: &str,
        doc_id: DocumentId,
    ) -> Result<Document, GitStorageError> {
        self.repo
            .read_json(branch, &format!("formation/{}.json", doc_id))
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
        commit_files(&self.repo, branch, message, &files, None)?;
        Ok(())
    }

    /// List all document IDs in the formation/ directory.
    pub fn list_document_ids(&self, branch: &str) -> Result<Vec<DocumentId>, GitStorageError> {
        let entries = self.repo.list_dir(branch, "formation")?;
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
        self.repo.read_json(
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
        self.repo.read_json(
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
        self.list_ids_in_dir(
            branch,
            &format!("governance/meetings/{}/votes", meeting_id),
        )
    }

    /// Read a resolution from a meeting.
    pub fn read_resolution(
        &self,
        branch: &str,
        meeting_id: MeetingId,
        id: ResolutionId,
    ) -> Result<Resolution, GitStorageError> {
        self.repo.read_json(
            branch,
            &format!(
                "governance/meetings/{}/resolutions/{}.json",
                meeting_id, id
            ),
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

    /// Read any deserializable JSON from a path.
    pub fn read_json<T: serde::de::DeserializeOwned>(
        &self,
        branch: &str,
        path: &str,
    ) -> Result<T, GitStorageError> {
        self.repo.read_json(branch, path)
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
        commit_files(&self.repo, branch, message, &files, None)?;
        Ok(())
    }

    /// List UUID-style IDs from files in a directory.
    ///
    /// Expects files named `{uuid}.json`. Returns parsed IDs for all
    /// matching entries, silently skipping non-UUID filenames.
    pub fn list_ids_in_dir<T: std::str::FromStr>(
        &self,
        branch: &str,
        dir_path: &str,
    ) -> Result<Vec<T>, GitStorageError> {
        let entries = match self.repo.list_dir(branch, dir_path) {
            Ok(entries) => entries,
            Err(GitStorageError::NotFound(_)) => return Ok(Vec::new()),
            Err(e) => return Err(e),
        };
        let mut ids = Vec::new();
        for (name, is_dir) in entries {
            if is_dir {
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
}
