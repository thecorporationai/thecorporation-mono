//! Offline data validation — scans all git repos and reports deserialization errors.
//!
//! Since domain types enforce invariants at deserialization (via `TryFrom`, `NonEmpty`,
//! positive-value validators, etc.), simply deserializing every file into its typed
//! struct catches corruption automatically.

use std::fmt;
use std::path::PathBuf;

use crate::domain::agents::agent::Agent;
use crate::domain::agents::secret_proxy::SecretProxyConfig;
use crate::domain::contacts::contact::Contact;
use crate::domain::equity::cap_table::CapTable;
use crate::domain::equity::funding_round::FundingRound;
use crate::domain::equity::fundraising_workflow::FundraisingWorkflow;
use crate::domain::equity::grant::EquityGrant;
use crate::domain::equity::safe_note::SafeNote;
use crate::domain::equity::share_class::ShareClass;
use crate::domain::equity::transfer::ShareTransfer;
use crate::domain::equity::transfer_workflow::TransferWorkflow;
use crate::domain::equity::valuation::Valuation;
use crate::domain::execution::intent::Intent;
use crate::domain::execution::obligation::Obligation;
use crate::domain::execution::receipt::Receipt;
use crate::domain::formation::contract::Contract;
use crate::domain::formation::contractor::ContractorClassification;
use crate::domain::formation::deadline::Deadline;
use crate::domain::formation::filing::Filing;
use crate::domain::formation::tax_filing::TaxFiling;
use crate::domain::formation::tax_profile::TaxProfile;
use crate::domain::governance::body::GovernanceBody;
use crate::domain::governance::meeting::Meeting;
use crate::domain::governance::seat::GovernanceSeat;
use crate::domain::ids::{AgentId, EntityId, MeetingId, WorkspaceId};
use crate::domain::treasury::account::Account;
use crate::domain::treasury::bank_account::BankAccount;
use crate::domain::treasury::distribution::Distribution;
use crate::domain::treasury::invoice::Invoice;
use crate::domain::treasury::journal_entry::JournalEntry;
use crate::domain::treasury::payment::Payment;
use crate::domain::treasury::payroll::PayrollRun;
use crate::domain::treasury::reconciliation::Reconciliation;
use crate::git::error::GitStorageError;
use crate::store::RepoLayout;
use crate::store::entity_store::EntityStore;
use crate::store::stored_entity::StoredEntity;
use crate::store::workspace_store::WorkspaceStore;

// ── Types ────────────────────────────────────────────────────────────

/// A single validation failure.
struct ValidationError {
    workspace_id: WorkspaceId,
    /// Additional context (e.g. entity ID, "workspace").
    context: String,
    /// The file path within the repo that failed.
    path: String,
    /// Human-readable error message.
    message: String,
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "  ERROR  ws={}  ctx={}  path={}  {}",
            self.workspace_id, self.context, self.path, self.message
        )
    }
}

/// Accumulated validation statistics.
struct ValidationStats {
    workspaces: usize,
    entities: usize,
    files_checked: usize,
    errors: Vec<ValidationError>,
}

impl ValidationStats {
    fn new() -> Self {
        Self {
            workspaces: 0,
            entities: 0,
            files_checked: 0,
            errors: Vec::new(),
        }
    }

    fn push_error(
        &mut self,
        workspace_id: WorkspaceId,
        context: impl Into<String>,
        path: impl Into<String>,
        message: impl Into<String>,
    ) {
        self.errors.push(ValidationError {
            workspace_id,
            context: context.into(),
            path: path.into(),
            message: message.into(),
        });
    }
}

// ── Entry point ──────────────────────────────────────────────────────

/// Validate all repos under `data_dir`. Returns process exit code (0 = ok, 1 = errors).
pub fn run(data_dir: PathBuf) -> i32 {
    if !data_dir.exists() {
        eprintln!(
            "error: data directory does not exist: {}",
            data_dir.display()
        );
        return 1;
    }

    let layout = RepoLayout::new(data_dir);
    let workspace_ids = layout.list_workspace_ids();
    let mut stats = ValidationStats::new();

    for ws_id in &workspace_ids {
        validate_workspace(&layout, *ws_id, &mut stats);
    }

    // Print summary
    println!();
    println!(
        "Validated {} workspace(s), {} entity/entities, {} file(s) checked.",
        stats.workspaces, stats.entities, stats.files_checked
    );

    if stats.errors.is_empty() {
        println!("No errors found.");
        0
    } else {
        println!("{} error(s) found:", stats.errors.len());
        for err in &stats.errors {
            eprintln!("{err}");
        }
        1
    }
}

// ── Workspace-level validation ───────────────────────────────────────

fn validate_workspace(layout: &RepoLayout, ws_id: WorkspaceId, stats: &mut ValidationStats) {
    stats.workspaces += 1;

    // Open workspace store
    let ws_store = match WorkspaceStore::open(layout, ws_id) {
        Ok(s) => s,
        Err(e) => {
            stats.push_error(
                ws_id,
                "workspace",
                "_workspace.git",
                format!("cannot open workspace repo: {e}"),
            );
            return;
        }
    };

    // workspace.json
    stats.files_checked += 1;
    if let Err(e) = ws_store.read_workspace() {
        stats.push_error(ws_id, "workspace", "workspace.json", format!("{e}"));
    }

    // API keys
    validate_workspace_api_keys(&ws_store, ws_id, stats);

    // Agents
    validate_workspace_agents(&ws_store, ws_id, stats);

    // Secret proxies
    validate_workspace_secret_proxies(&ws_store, ws_id, stats);

    // Entities in this workspace
    let entity_ids = layout.list_entity_ids(ws_id);
    for eid in &entity_ids {
        validate_entity(layout, ws_id, *eid, stats);
    }
}

fn validate_workspace_api_keys(
    ws_store: &WorkspaceStore<'_>,
    ws_id: WorkspaceId,
    stats: &mut ValidationStats,
) {
    let key_ids = match ws_store.list_api_key_ids() {
        Ok(ids) => ids,
        Err(GitStorageError::NotFound(_)) => return,
        Err(e) => {
            stats.push_error(
                ws_id,
                "workspace",
                "api-keys/",
                format!("cannot list api keys: {e}"),
            );
            return;
        }
    };

    for key_id in key_ids {
        stats.files_checked += 1;
        if let Err(e) = ws_store.read_api_key(key_id) {
            stats.push_error(
                ws_id,
                "workspace",
                format!("api-keys/{key_id}.json"),
                format!("{e}"),
            );
        }
    }
}

fn validate_workspace_agents(
    ws_store: &WorkspaceStore<'_>,
    ws_id: WorkspaceId,
    stats: &mut ValidationStats,
) {
    let agent_ids: Vec<AgentId> = match ws_store.list_ids_in_dir_pub("agents") {
        Ok(ids) => ids,
        Err(GitStorageError::NotFound(_)) => return,
        Err(e) => {
            stats.push_error(
                ws_id,
                "workspace",
                "agents/",
                format!("cannot list agents: {e}"),
            );
            return;
        }
    };

    for agent_id in agent_ids {
        stats.files_checked += 1;
        let path = format!("agents/{agent_id}.json");
        if let Err(e) = ws_store.read_json::<Agent>(&path) {
            stats.push_error(ws_id, "workspace", &path, format!("{e}"));
        }
    }
}

fn validate_workspace_secret_proxies(
    ws_store: &WorkspaceStore<'_>,
    ws_id: WorkspaceId,
    stats: &mut ValidationStats,
) {
    let names = match ws_store.list_names_in_dir("secrets") {
        Ok(n) => n,
        Err(GitStorageError::NotFound(_)) => return,
        Err(e) => {
            stats.push_error(
                ws_id,
                "workspace",
                "secrets/",
                format!("cannot list secret proxies: {e}"),
            );
            return;
        }
    };

    for name in names {
        let config_path = format!("secrets/{name}/config.json");
        stats.files_checked += 1;
        if let Err(e) = ws_store.read_json::<SecretProxyConfig>(&config_path) {
            stats.push_error(ws_id, "workspace", &config_path, format!("{e}"));
        }
    }
}

// ── Entity-level validation ──────────────────────────────────────────

fn validate_entity(
    layout: &RepoLayout,
    ws_id: WorkspaceId,
    entity_id: EntityId,
    stats: &mut ValidationStats,
) {
    stats.entities += 1;
    let ctx = entity_id.to_string();

    let store = match EntityStore::open(layout, ws_id, entity_id) {
        Ok(s) => s,
        Err(e) => {
            stats.push_error(
                ws_id,
                &ctx,
                format!("{entity_id}.git"),
                format!("cannot open entity repo: {e}"),
            );
            return;
        }
    };

    // ── Singletons ──────────────────────────────────────────────────

    // corp.json (required)
    stats.files_checked += 1;
    if let Err(e) = store.read_entity("main") {
        stats.push_error(ws_id, &ctx, "corp.json", format!("{e}"));
    }

    // cap-table/cap-table.json (optional)
    try_singleton::<CapTable>(&store, ws_id, &ctx, "cap-table/cap-table.json", stats);

    // formation/filing.json (optional)
    try_singleton::<Filing>(&store, ws_id, &ctx, "formation/filing.json", stats);

    // tax/profile.json (optional)
    try_singleton::<TaxProfile>(&store, ws_id, &ctx, "tax/profile.json", stats);

    // ── StoredEntity collections ────────────────────────────────────

    validate_stored::<ShareClass>(&store, ws_id, &ctx, stats);
    validate_stored::<EquityGrant>(&store, ws_id, &ctx, stats);
    validate_stored::<SafeNote>(&store, ws_id, &ctx, stats);
    validate_stored::<Valuation>(&store, ws_id, &ctx, stats);
    validate_stored::<ShareTransfer>(&store, ws_id, &ctx, stats);
    validate_stored::<FundingRound>(&store, ws_id, &ctx, stats);
    validate_stored::<TransferWorkflow>(&store, ws_id, &ctx, stats);
    validate_stored::<FundraisingWorkflow>(&store, ws_id, &ctx, stats);

    validate_stored::<GovernanceBody>(&store, ws_id, &ctx, stats);
    validate_stored::<GovernanceSeat>(&store, ws_id, &ctx, stats);
    validate_stored::<Meeting>(&store, ws_id, &ctx, stats);

    validate_stored::<Account>(&store, ws_id, &ctx, stats);
    validate_stored::<JournalEntry>(&store, ws_id, &ctx, stats);
    validate_stored::<Invoice>(&store, ws_id, &ctx, stats);
    validate_stored::<BankAccount>(&store, ws_id, &ctx, stats);
    validate_stored::<Payment>(&store, ws_id, &ctx, stats);
    validate_stored::<PayrollRun>(&store, ws_id, &ctx, stats);
    validate_stored::<Distribution>(&store, ws_id, &ctx, stats);
    validate_stored::<Reconciliation>(&store, ws_id, &ctx, stats);

    validate_stored::<Intent>(&store, ws_id, &ctx, stats);
    validate_stored::<Obligation>(&store, ws_id, &ctx, stats);
    validate_stored::<Receipt>(&store, ws_id, &ctx, stats);

    validate_stored::<Contact>(&store, ws_id, &ctx, stats);

    validate_stored::<Contract>(&store, ws_id, &ctx, stats);
    validate_stored::<TaxFiling>(&store, ws_id, &ctx, stats);
    validate_stored::<Deadline>(&store, ws_id, &ctx, stats);
    validate_stored::<ContractorClassification>(&store, ws_id, &ctx, stats);

    // ── Documents (special: formation/{id}.json, skips filing.json) ─
    validate_documents(&store, ws_id, &ctx, stats);

    // ── Meetings → nested agenda items, votes, resolutions ──────────
    validate_meetings(&store, ws_id, &ctx, stats);
}

/// Try reading an optional singleton — `NotFound` is not an error.
fn try_singleton<T: serde::de::DeserializeOwned>(
    store: &EntityStore<'_>,
    ws_id: WorkspaceId,
    ctx: &str,
    path: &str,
    stats: &mut ValidationStats,
) {
    stats.files_checked += 1;
    match store.read_json::<T>("main", path) {
        Ok(_) => {}
        Err(GitStorageError::NotFound(_)) => {}
        Err(e) => stats.push_error(ws_id, ctx, path, format!("{e}")),
    }
}

/// Validate all instances of a `StoredEntity` collection.
fn validate_stored<T: StoredEntity>(
    store: &EntityStore<'_>,
    ws_id: WorkspaceId,
    ctx: &str,
    stats: &mut ValidationStats,
) {
    let ids = match store.list_ids::<T>("main") {
        Ok(ids) => ids,
        Err(GitStorageError::NotFound(_)) => return,
        Err(e) => {
            stats.push_error(
                ws_id,
                ctx,
                T::storage_dir(),
                format!("cannot list IDs: {e}"),
            );
            return;
        }
    };

    for id in ids {
        stats.files_checked += 1;
        let path = T::storage_path(id);
        if let Err(e) = store.read::<T>("main", id) {
            stats.push_error(ws_id, ctx, &path, format!("{e}"));
        }
    }
}

/// Validate documents (stored under formation/, skipping filing.json).
fn validate_documents(
    store: &EntityStore<'_>,
    ws_id: WorkspaceId,
    ctx: &str,
    stats: &mut ValidationStats,
) {
    let doc_ids = match store.list_document_ids("main") {
        Ok(ids) => ids,
        Err(GitStorageError::NotFound(_)) => return,
        Err(e) => {
            stats.push_error(
                ws_id,
                ctx,
                "formation/",
                format!("cannot list documents: {e}"),
            );
            return;
        }
    };

    for doc_id in doc_ids {
        stats.files_checked += 1;
        let path = format!("formation/{doc_id}.json");
        if let Err(e) = store.read_document("main", doc_id) {
            stats.push_error(ws_id, ctx, &path, format!("{e}"));
        }
    }
}

/// Validate nested meeting sub-types (agenda items, votes, resolutions).
fn validate_meetings(
    store: &EntityStore<'_>,
    ws_id: WorkspaceId,
    ctx: &str,
    stats: &mut ValidationStats,
) {
    let meeting_ids: Vec<MeetingId> = match store.list_ids::<Meeting>("main") {
        Ok(ids) => ids,
        Err(_) => return, // already reported by validate_stored
    };

    for mid in meeting_ids {
        // Agenda items
        if let Ok(agenda_ids) = store.list_agenda_item_ids("main", mid) {
            for aid in agenda_ids {
                stats.files_checked += 1;
                let path = format!("governance/meetings/{mid}/agenda/{aid}.json");
                if let Err(e) = store.read_agenda_item("main", mid, aid) {
                    stats.push_error(ws_id, ctx, &path, format!("{e}"));
                }
            }
        }

        // Votes
        if let Ok(vote_ids) = store.list_vote_ids("main", mid) {
            for vid in vote_ids {
                stats.files_checked += 1;
                let path = format!("governance/meetings/{mid}/votes/{vid}.json");
                if let Err(e) = store.read_vote("main", mid, vid) {
                    stats.push_error(ws_id, ctx, &path, format!("{e}"));
                }
            }
        }

        // Resolutions
        if let Ok(res_ids) = store.list_resolution_ids("main", mid) {
            for rid in res_ids {
                stats.files_checked += 1;
                let path = format!("governance/meetings/{mid}/resolutions/{rid}.json");
                if let Err(e) = store.read_resolution("main", mid, rid) {
                    stats.push_error(ws_id, ctx, &path, format!("{e}"));
                }
            }
        }
    }
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ids::WorkspaceId;
    use crate::git::commit::{FileWrite, commit_files};
    use crate::git::repo::CorpRepo;
    use crate::store::workspace_store::WorkspaceStore;

    #[test]
    fn empty_data_dir_passes() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(run(dir.path().to_path_buf()), 0);
    }

    #[test]
    fn nonexistent_data_dir_fails() {
        let dir = PathBuf::from("/tmp/does-not-exist-validate-test");
        assert_eq!(run(dir), 1);
    }

    #[test]
    fn valid_workspace_and_entity_passes() {
        let dir = tempfile::tempdir().unwrap();
        let layout = RepoLayout::new(dir.path().to_path_buf());
        let ws_id = WorkspaceId::new();
        let entity_id = EntityId::new();

        // Create workspace
        WorkspaceStore::init(&layout, ws_id, "Test Workspace").unwrap();

        // Create entity with a minimal valid corp.json
        let entity = serde_json::json!({
            "entity_id": entity_id.to_string(),
            "workspace_id": ws_id.to_string(),
            "legal_name": "Acme Corp",
            "entity_type": "corporation",
            "jurisdiction": "Delaware",
            "formation_state": "forming",
            "formation_status": "pending",
            "created_at": "2024-01-01T00:00:00Z"
        });
        let path = layout.entity_repo_path(ws_id, entity_id);
        let repo = CorpRepo::init(&path, None).unwrap();
        let files = vec![FileWrite::json("corp.json", &entity).unwrap()];
        commit_files(&repo, "main", "init entity", &files, None).unwrap();

        assert_eq!(run(dir.path().to_path_buf()), 0);
    }

    #[test]
    fn corrupted_corp_json_fails() {
        let dir = tempfile::tempdir().unwrap();
        let layout = RepoLayout::new(dir.path().to_path_buf());
        let ws_id = WorkspaceId::new();
        let entity_id = EntityId::new();

        // Create workspace
        WorkspaceStore::init(&layout, ws_id, "Test Workspace").unwrap();

        // Create entity with invalid JSON in corp.json
        let path = layout.entity_repo_path(ws_id, entity_id);
        let repo = CorpRepo::init(&path, None).unwrap();
        let files = vec![FileWrite::raw("corp.json", b"{ not valid json".to_vec())];
        commit_files(&repo, "main", "init entity", &files, None).unwrap();

        assert_eq!(run(dir.path().to_path_buf()), 1);
    }
}
