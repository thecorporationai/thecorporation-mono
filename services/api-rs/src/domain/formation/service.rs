//! Formation service — orchestrates entity creation and formation workflow.

use crate::domain::formation::{
    content::*,
    document::Document,
    entity::Entity,
    filing::Filing,
    tax_profile::TaxProfile,
    types::*,
};
use crate::domain::ids::*;
use crate::git::commit::FileWrite;

use super::error::FormationError;

/// Result of creating a new entity through the formation workflow.
pub struct FormationResult {
    pub entity: Entity,
    pub document_ids: Vec<DocumentId>,
    pub filing: Filing,
    pub tax_profile: TaxProfile,
}

/// Create a new entity — initializes the git repo, generates documents,
/// creates filing and tax profile records, and advances to documents_generated.
#[allow(clippy::too_many_arguments)]
pub fn create_entity(
    layout: &crate::store::RepoLayout,
    workspace_id: WorkspaceId,
    legal_name: String,
    entity_type: EntityType,
    jurisdiction: String,
    registered_agent_name: Option<String>,
    registered_agent_address: Option<String>,
    members: &[MemberInput],
    authorized_shares: Option<i64>,
    par_value: Option<&str>,
) -> Result<FormationResult, FormationError> {
    // Validate members
    if members.is_empty() {
        return Err(FormationError::Validation(
            "at least one member is required".into(),
        ));
    }

    let entity_id = EntityId::new();

    // Generate formation documents first (borrows only).
    let doc_specs = generate_formation_documents(
        entity_type,
        &legal_name,
        &jurisdiction,
        registered_agent_name.as_deref().unwrap_or(""),
        registered_agent_address.as_deref().unwrap_or(""),
        members,
        authorized_shares,
        par_value,
    );

    // Create filing record (borrows jurisdiction).
    let filing_type = match entity_type {
        EntityType::Llc => FilingType::CertificateOfFormation,
        EntityType::Corporation => FilingType::CertificateOfIncorporation,
    };
    let filing = Filing::new(FilingId::new(), entity_id, filing_type, jurisdiction.clone());

    // Create tax profile.
    let classification = TaxProfile::classify(entity_type, members.len());
    let tax_profile = TaxProfile::new(TaxProfileId::new(), entity_id, classification);

    // Create entity record (consumes owned Strings — no clones needed after this point).
    let mut entity = Entity::new(
        entity_id,
        workspace_id,
        legal_name,
        entity_type,
        jurisdiction,
        registered_agent_name,
        registered_agent_address,
    )?;

    // Create Document records
    let mut documents = Vec::new();
    for (doc_type, title, governance_tag, content) in doc_specs {
        let doc = Document::new(
            DocumentId::new(),
            entity_id,
            workspace_id,
            doc_type,
            title,
            content,
            governance_tag,
            None, // no supersedes
        );
        documents.push(doc);
    }

    let document_ids: Vec<DocumentId> = documents.iter().map(|d| d.document_id()).collect();

    // Build all files for the initial commit
    let mut files = Vec::new();
    files.push(
        FileWrite::json("corp.json", &entity)
            .map_err(|e| FormationError::Storage(e.to_string()))?,
    );
    files.push(
        FileWrite::json("formation/filing.json", &filing)
            .map_err(|e| FormationError::Storage(e.to_string()))?,
    );
    files.push(
        FileWrite::json("tax/profile.json", &tax_profile)
            .map_err(|e| FormationError::Storage(e.to_string()))?,
    );
    for doc in &documents {
        let path = format!("formation/{}.json", doc.document_id());
        files.push(
            FileWrite::json(path, doc).map_err(|e| FormationError::Storage(e.to_string()))?,
        );
    }

    // Initialize the entity repo with all files in one atomic commit
    let repo_path = layout.entity_repo_path(workspace_id, entity_id);
    let repo = crate::git::repo::CorpRepo::init(&repo_path, None)
        .map_err(|e| FormationError::Storage(format!("failed to init repo: {e}")))?;
    crate::git::commit::commit_files(
        &repo,
        "main",
        &format!("Form entity: {}", entity.legal_name()),
        &files,
        None,
    )
    .map_err(|e| FormationError::Storage(format!("failed to commit: {e}")))?;

    // Advance status to documents_generated
    entity.advance_status(FormationStatus::DocumentsGenerated)?;

    // Write the updated entity to reflect the new status
    let entity_file = FileWrite::json("corp.json", &entity)
        .map_err(|e| FormationError::Storage(e.to_string()))?;
    crate::git::commit::commit_files(
        &repo,
        "main",
        "Advance to documents_generated",
        &[entity_file],
        None,
    )
    .map_err(|e| FormationError::Storage(format!("failed to commit status: {e}")))?;

    Ok(FormationResult {
        entity,
        document_ids,
        filing,
        tax_profile,
    })
}

/// The next action the user should take based on formation status.
pub fn next_formation_action(status: FormationStatus) -> Option<&'static str> {
    match status {
        FormationStatus::Pending => Some("create_entity"),
        FormationStatus::DocumentsGenerated => Some("sign_documents"),
        FormationStatus::DocumentsSigned => Some("submit_state_filing"),
        FormationStatus::FilingSubmitted => Some("confirm_state_filing"),
        FormationStatus::Filed => Some("apply_for_ein"),
        FormationStatus::EinApplied => Some("confirm_ein"),
        FormationStatus::Active | FormationStatus::Rejected | FormationStatus::Dissolved => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::RepoLayout;
    use tempfile::TempDir;

    fn alice() -> MemberInput {
        MemberInput {
            name: "Alice Smith".to_string(),
            investor_type: InvestorType::NaturalPerson,
            email: Some("alice@example.com".to_string()),
            agent_id: None,
            entity_id: None,
            ownership_pct: Some(60.0),
            membership_units: Some(600),
            share_count: Some(6_000_000),
            share_class: Some("COMMON".to_string()),
            role: Some(MemberRole::Manager),
        }
    }

    fn bob() -> MemberInput {
        MemberInput {
            name: "Bob Jones".to_string(),
            investor_type: InvestorType::NaturalPerson,
            email: Some("bob@example.com".to_string()),
            agent_id: None,
            entity_id: None,
            ownership_pct: Some(40.0),
            membership_units: Some(400),
            share_count: Some(4_000_000),
            share_class: Some("COMMON".to_string()),
            role: Some(MemberRole::Member),
        }
    }

    #[test]
    fn create_entity_llc_full_workflow() {
        let tmp = TempDir::new().unwrap();
        let layout = RepoLayout::new(tmp.path().to_path_buf());
        let workspace_id = WorkspaceId::new();
        let members = vec![alice(), bob()];

        let result = create_entity(
            &layout,
            workspace_id,
            "Acme LLC".to_string(),
            EntityType::Llc,
            "Delaware".to_string(),
            Some("Registered Agents Inc.".to_string()),
            Some("123 Main St, Dover, DE 19901".to_string()),
            &members,
            None,
            None,
        )
        .expect("create_entity should succeed");

        // Verify entity fields
        assert_eq!(result.entity.legal_name(), "Acme LLC");
        assert_eq!(result.entity.entity_type(), EntityType::Llc);
        assert_eq!(result.entity.jurisdiction(), "Delaware");
        assert_eq!(
            result.entity.formation_status(),
            FormationStatus::DocumentsGenerated
        );

        // Verify documents were generated (LLC: articles of org + operating agreement)
        assert_eq!(result.document_ids.len(), 2);

        // Verify the entity repo exists on disk and is readable
        let entity_id = result.entity.entity_id();
        let repo_path = layout.entity_repo_path(workspace_id, entity_id);
        assert!(repo_path.exists(), "entity repo should exist on disk");

        // Open the repo and read back corp.json
        let repo = crate::git::repo::CorpRepo::open(&repo_path).unwrap();
        let entity_from_repo: Entity = repo.read_json("main", "corp.json").unwrap();
        assert_eq!(entity_from_repo.legal_name(), "Acme LLC");
        assert_eq!(
            entity_from_repo.formation_status(),
            FormationStatus::DocumentsGenerated
        );

        // Verify filing record exists
        let filing_from_repo: Filing = repo.read_json("main", "formation/filing.json").unwrap();
        assert_eq!(filing_from_repo.entity_id(), entity_id);
        assert_eq!(filing_from_repo.jurisdiction(), "Delaware");

        // Verify tax profile exists
        let tax_from_repo: TaxProfile = repo.read_json("main", "tax/profile.json").unwrap();
        assert_eq!(tax_from_repo.entity_id(), entity_id);

        // Verify documents exist in formation/ directory
        for doc_id in &result.document_ids {
            let path = format!("formation/{}.json", doc_id);
            assert!(
                repo.path_exists("main", &path).unwrap(),
                "document {} should exist in repo",
                doc_id
            );
        }
    }

    #[test]
    fn create_entity_corporation() {
        let tmp = TempDir::new().unwrap();
        let layout = RepoLayout::new(tmp.path().to_path_buf());
        let workspace_id = WorkspaceId::new();

        let mut alice = alice();
        alice.role = Some(MemberRole::Director);

        let result = create_entity(
            &layout,
            workspace_id,
            "Acme Corp".to_string(),
            EntityType::Corporation,
            "Delaware".to_string(),
            Some("RA Inc.".to_string()),
            Some("123 Main St".to_string()),
            &[alice],
            Some(10_000_000),
            Some("0.0001"),
        )
        .expect("create_entity should succeed for corporation");

        assert_eq!(result.entity.entity_type(), EntityType::Corporation);
        // Corporation: articles of incorporation + bylaws
        assert_eq!(result.document_ids.len(), 2);
        assert_eq!(
            result.entity.formation_status(),
            FormationStatus::DocumentsGenerated
        );
    }

    #[test]
    fn create_entity_rejects_empty_members() {
        let tmp = TempDir::new().unwrap();
        let layout = RepoLayout::new(tmp.path().to_path_buf());
        let workspace_id = WorkspaceId::new();

        let result = create_entity(
            &layout,
            workspace_id,
            "Empty LLC".to_string(),
            EntityType::Llc,
            "Delaware".to_string(),
            None,
            None,
            &[], // no members
            None,
            None,
        );

        assert!(result.is_err());
    }

    #[test]
    fn create_entity_rejects_empty_name() {
        let tmp = TempDir::new().unwrap();
        let layout = RepoLayout::new(tmp.path().to_path_buf());
        let workspace_id = WorkspaceId::new();

        let result = create_entity(
            &layout,
            workspace_id,
            "".to_string(),
            EntityType::Llc,
            "Delaware".to_string(),
            None,
            None,
            &[alice()],
            None,
            None,
        );

        assert!(result.is_err());
    }

    #[test]
    fn create_entity_store_roundtrip() {
        use crate::store::entity_store::EntityStore;

        let tmp = TempDir::new().unwrap();
        let layout = RepoLayout::new(tmp.path().to_path_buf());
        let workspace_id = WorkspaceId::new();

        let result = create_entity(
            &layout,
            workspace_id,
            "Store Test LLC".to_string(),
            EntityType::Llc,
            "Wyoming".to_string(),
            None,
            None,
            &[alice()],
            None,
            None,
        )
        .unwrap();

        // Open via EntityStore and verify reads work
        let store =
            EntityStore::open(&layout, workspace_id, result.entity.entity_id()).unwrap();

        let entity = store.read_entity("main").unwrap();
        assert_eq!(entity.legal_name(), "Store Test LLC");

        let filing = store.read_filing("main").unwrap();
        assert_eq!(filing.jurisdiction(), "Wyoming");

        let tax = store.read_tax_profile("main").unwrap();
        assert_eq!(tax.entity_id(), result.entity.entity_id());

        let doc_ids = store.list_document_ids("main").unwrap();
        assert_eq!(doc_ids.len(), result.document_ids.len());

        // Read each document
        for doc_id in &doc_ids {
            let doc = store.read_document("main", *doc_id).unwrap();
            assert_eq!(doc.entity_id(), result.entity.entity_id());
        }
    }

    #[test]
    fn next_formation_action_values() {
        assert_eq!(
            next_formation_action(FormationStatus::Pending),
            Some("create_entity")
        );
        assert_eq!(
            next_formation_action(FormationStatus::DocumentsGenerated),
            Some("sign_documents")
        );
        assert_eq!(
            next_formation_action(FormationStatus::DocumentsSigned),
            Some("submit_state_filing")
        );
        assert_eq!(
            next_formation_action(FormationStatus::FilingSubmitted),
            Some("confirm_state_filing")
        );
        assert_eq!(
            next_formation_action(FormationStatus::Filed),
            Some("apply_for_ein")
        );
        assert_eq!(
            next_formation_action(FormationStatus::EinApplied),
            Some("confirm_ein")
        );
        assert_eq!(next_formation_action(FormationStatus::Active), None);
        assert_eq!(next_formation_action(FormationStatus::Rejected), None);
    }
}
