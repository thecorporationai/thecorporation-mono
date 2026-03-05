//! Formation service — orchestrates entity creation and formation workflow.

use crate::domain::contacts::contact::Contact;
use crate::domain::contacts::types::{ContactCategory, ContactType};
use crate::domain::equity::holder::{Holder, HolderType};
use crate::domain::equity::instrument::{Instrument, InstrumentKind};
use crate::domain::equity::legal_entity::{LegalEntity, LegalEntityRole};
use crate::domain::equity::position::Position;
use crate::domain::formation::{
    content::*, document::Document, entity::Entity, filing::Filing, tax_profile::TaxProfile,
    types::*,
};
use crate::domain::ids::*;
use crate::git::commit::FileWrite;

use super::error::FormationError;

/// Result of creating a new entity through the formation workflow.
#[derive(Debug)]
pub struct FormationResult {
    pub entity: Entity,
    pub document_ids: Vec<DocumentId>,
    pub filing: Filing,
    pub tax_profile: TaxProfile,
}

/// Summary of a holder created during cap table setup.
#[derive(Debug, Clone, serde::Serialize)]
pub struct HolderSummary {
    pub holder_id: HolderId,
    pub name: String,
    pub shares: i64,
    pub ownership_pct: f64,
}

/// Result of cap table setup after formation.
#[derive(Debug)]
pub struct CapTableSetupResult {
    pub legal_entity_id: LegalEntityId,
    pub instrument_id: InstrumentId,
    pub holders: Vec<HolderSummary>,
}

fn member_role_label(role: Option<MemberRole>, entity_type: EntityType) -> String {
    match role {
        Some(MemberRole::Director) => "director".to_owned(),
        Some(MemberRole::Officer) => "officer".to_owned(),
        Some(MemberRole::Manager) => "manager".to_owned(),
        Some(MemberRole::Member) => "member".to_owned(),
        Some(MemberRole::Chair) => "chair".to_owned(),
        None => match entity_type {
            EntityType::Corporation => "incorporator".to_owned(),
            EntityType::Llc => "organizer".to_owned(),
        },
    }
}

/// Create a new entity — initializes the git repo, generates documents,
/// creates filing and tax profile records, and advances to documents_generated.
#[allow(clippy::too_many_arguments)]
pub fn create_entity(
    layout: &crate::store::RepoLayout,
    workspace_id: WorkspaceId,
    legal_name: String,
    entity_type: EntityType,
    jurisdiction: Jurisdiction,
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

    let designated_attestor = members
        .iter()
        .find(|member| member.investor_type == InvestorType::NaturalPerson)
        .ok_or_else(|| {
            FormationError::Validation(
                "at least one natural_person member is required for filing attestation".to_owned(),
            )
        })?;
    let designated_attestor_name = designated_attestor.name.trim().to_owned();
    if designated_attestor_name.is_empty() {
        return Err(FormationError::Validation(
            "designated natural-person attestor name must not be empty".to_owned(),
        ));
    }
    let designated_attestor_email = designated_attestor
        .email
        .as_ref()
        .map(|email| email.trim().to_owned())
        .filter(|email| !email.is_empty());
    let designated_attestor_role = member_role_label(designated_attestor.role, entity_type);

    let filing = Filing::new(
        FilingId::new(),
        entity_id,
        filing_type,
        jurisdiction.clone(),
        designated_attestor_name,
        designated_attestor_email,
        designated_attestor_role,
    );

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
        files.push(FileWrite::json(path, doc).map_err(|e| FormationError::Storage(e.to_string()))?);
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

/// Map a formation MemberRole to a ContactCategory.
fn member_role_to_contact_category(role: Option<MemberRole>) -> ContactCategory {
    match role {
        Some(MemberRole::Director) | Some(MemberRole::Chair) => ContactCategory::Founder,
        Some(MemberRole::Officer) => ContactCategory::Officer,
        Some(MemberRole::Manager) | Some(MemberRole::Member) | None => ContactCategory::Member,
    }
}

/// Map an InvestorType to a ContactType.
fn investor_type_to_contact_type(it: InvestorType) -> ContactType {
    match it {
        InvestorType::NaturalPerson => ContactType::Individual,
        InvestorType::Entity => ContactType::Organization,
        InvestorType::Agent => ContactType::Individual, // shouldn't reach here
    }
}

/// Map an InvestorType to a HolderType.
fn investor_type_to_holder_type(it: InvestorType) -> HolderType {
    match it {
        InvestorType::NaturalPerson => HolderType::Individual,
        InvestorType::Entity => HolderType::Organization,
        InvestorType::Agent => HolderType::Individual, // shouldn't reach here
    }
}

/// Set up the cap table for a newly formed entity.
///
/// Creates contacts, an equity legal entity, an instrument, holders, and
/// initial positions in one atomic git commit.
#[allow(clippy::too_many_arguments)]
pub fn setup_cap_table(
    layout: &crate::store::RepoLayout,
    workspace_id: WorkspaceId,
    entity_id: EntityId,
    entity_type: EntityType,
    legal_name: &str,
    members: &[MemberInput],
    authorized_shares: Option<i64>,
    par_value: Option<&str>,
) -> Result<CapTableSetupResult, FormationError> {
    let non_agent_members: Vec<&MemberInput> = members
        .iter()
        .filter(|m| m.investor_type != InvestorType::Agent)
        .collect();

    if non_agent_members.is_empty() {
        return Err(FormationError::Validation(
            "at least one non-agent member is required for cap table setup".into(),
        ));
    }

    // 1. Create contacts for each non-agent member
    let mut contacts = Vec::new();
    for m in &non_agent_members {
        let contact = Contact::new(
            ContactId::new(),
            entity_id,
            workspace_id,
            investor_type_to_contact_type(m.investor_type),
            m.name.clone(),
            m.email.clone(),
            member_role_to_contact_category(m.role),
        );
        contacts.push(contact);
    }

    // 2. Create equity legal entity linked to the formation entity
    let legal_entity_id = LegalEntityId::new();
    let legal_entity = LegalEntity::new(
        legal_entity_id,
        workspace_id,
        Some(entity_id),
        legal_name.to_owned(),
        LegalEntityRole::Operating,
    );

    // 3. Create instrument based on entity type
    let instrument_id = InstrumentId::new();
    let (kind, symbol, auth_units, price_cents) = match entity_type {
        EntityType::Llc => {
            let total_units: i64 = non_agent_members
                .iter()
                .filter_map(|m| m.membership_units)
                .sum();
            let auth = if total_units > 0 {
                Some(total_units)
            } else {
                Some(10_000) // default LLC units
            };
            (InstrumentKind::MembershipUnit, "UNITS".to_owned(), auth, None)
        }
        EntityType::Corporation => {
            let shares = authorized_shares.unwrap_or(10_000_000);
            let pv = par_value.unwrap_or("0.0001");
            let price = (pv.parse::<f64>().unwrap_or(0.0001) * 10_000.0).round() as i64; // cents with 2 decimals
            (
                InstrumentKind::CommonEquity,
                "COMMON".to_owned(),
                Some(shares),
                Some(price),
            )
        }
    };

    let instrument = Instrument::new(
        instrument_id,
        legal_entity_id,
        symbol,
        kind,
        auth_units,
        price_cents,
        serde_json::Value::Null,
    );

    // 4. Create holders for each non-agent member and compute positions
    let mut holders = Vec::new();
    let mut positions = Vec::new();
    let mut holder_summaries = Vec::new();

    for (i, m) in non_agent_members.iter().enumerate() {
        let contact_id = contacts[i].contact_id();
        let holder_id = HolderId::new();

        let holder = Holder::new(
            holder_id,
            contact_id,
            Some(entity_id),
            m.name.clone(),
            investor_type_to_holder_type(m.investor_type),
            None,
        );
        holders.push(holder);

        // Determine share count for this member
        let shares = match entity_type {
            EntityType::Llc => {
                m.membership_units.unwrap_or_else(|| {
                    // Calculate from ownership_pct if available
                    let pct = m.ownership_pct.unwrap_or(0.0);
                    let total = auth_units.unwrap_or(10_000);
                    ((pct / 100.0) * total as f64).round() as i64
                })
            }
            EntityType::Corporation => {
                m.shares_purchased
                    .or(m.share_count)
                    .unwrap_or_else(|| {
                        let pct = m.ownership_pct.unwrap_or(0.0);
                        let total = auth_units.unwrap_or(10_000_000);
                        // Reserve 20% for future issuances (Cooley standard)
                        ((pct / 100.0) * (total as f64 * 0.8)).round() as i64
                    })
            }
        };

        let principal = match entity_type {
            EntityType::Corporation => {
                shares * price_cents.unwrap_or(1) // par_value * shares in cents
            }
            EntityType::Llc => 0,
        };

        let ownership_pct = m.ownership_pct.unwrap_or_else(|| {
            if let Some(total) = auth_units {
                if total > 0 {
                    (shares as f64 / total as f64) * 100.0
                } else {
                    0.0
                }
            } else {
                0.0
            }
        });

        let position = Position::new(
            PositionId::new(),
            legal_entity_id,
            holder_id,
            instrument_id,
            shares,
            principal,
            Some("formation".to_owned()),
            None,
            None,
        )
        .map_err(|e| FormationError::Validation(format!("position error: {e}")))?;

        positions.push(position);
        holder_summaries.push(HolderSummary {
            holder_id,
            name: m.name.clone(),
            shares,
            ownership_pct,
        });
    }

    // 5. Write all records in a single atomic commit
    let repo_path = layout.entity_repo_path(workspace_id, entity_id);
    let repo = crate::git::repo::CorpRepo::open(&repo_path)
        .map_err(|e| FormationError::Storage(format!("failed to open repo: {e}")))?;

    let mut files = Vec::new();

    // Contacts
    for contact in &contacts {
        let path = format!("contacts/{}.json", contact.contact_id());
        files.push(
            FileWrite::json(path, contact)
                .map_err(|e| FormationError::Storage(e.to_string()))?,
        );
    }

    // Legal entity
    files.push(
        FileWrite::json(
            format!("cap-table/entities/{}.json", legal_entity_id),
            &legal_entity,
        )
        .map_err(|e| FormationError::Storage(e.to_string()))?,
    );

    // Instrument
    files.push(
        FileWrite::json(
            format!("cap-table/instruments/{}.json", instrument_id),
            &instrument,
        )
        .map_err(|e| FormationError::Storage(e.to_string()))?,
    );

    // Holders
    for holder in &holders {
        let path = format!("cap-table/holders/{}.json", holder.holder_id());
        files.push(
            FileWrite::json(path, holder)
                .map_err(|e| FormationError::Storage(e.to_string()))?,
        );
    }

    // Positions
    for position in &positions {
        let path = format!("cap-table/positions/{}.json", position.position_id());
        files.push(
            FileWrite::json(path, position)
                .map_err(|e| FormationError::Storage(e.to_string()))?,
        );
    }

    crate::git::commit::commit_files(
        &repo,
        "main",
        "Initialize cap table with founding members",
        &files,
        None,
    )
    .map_err(|e| FormationError::Storage(format!("failed to commit cap table: {e}")))?;

    Ok(CapTableSetupResult {
        legal_entity_id,
        instrument_id,
        holders: holder_summaries,
    })
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
            address: None,
            officer_title: None,
            shares_purchased: None,
            vesting: None,
            ip_description: None,
            is_incorporator: None,
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
            address: None,
            officer_title: None,
            shares_purchased: None,
            vesting: None,
            ip_description: None,
            is_incorporator: None,
        }
    }

    fn service_agent() -> MemberInput {
        MemberInput {
            name: "Formation Agent".to_string(),
            investor_type: InvestorType::Agent,
            email: Some("agent@example.com".to_string()),
            agent_id: Some(AgentId::new()),
            entity_id: None,
            ownership_pct: None,
            membership_units: None,
            share_count: None,
            share_class: None,
            role: None,
            address: None,
            officer_title: None,
            shares_purchased: None,
            vesting: None,
            ip_description: None,
            is_incorporator: None,
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
            Jurisdiction::new("Delaware").unwrap(),
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
            Jurisdiction::new("Delaware").unwrap(),
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
            Jurisdiction::new("Delaware").unwrap(),
            None,
            None,
            &[], // no members
            None,
            None,
        );

        assert!(result.is_err());
    }

    #[test]
    fn create_entity_requires_natural_person_attestor() {
        let tmp = TempDir::new().unwrap();
        let layout = RepoLayout::new(tmp.path().to_path_buf());
        let workspace_id = WorkspaceId::new();

        let result = create_entity(
            &layout,
            workspace_id,
            "Agent Only Corp".to_string(),
            EntityType::Corporation,
            Jurisdiction::new("Delaware").unwrap(),
            None,
            None,
            &[service_agent()],
            Some(1_000_000),
            Some("0.0001"),
        );

        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("natural_person member")
        );
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
            Jurisdiction::new("Delaware").unwrap(),
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
            Jurisdiction::new("Wyoming").unwrap(),
            None,
            None,
            &[alice()],
            None,
            None,
        )
        .unwrap();

        // Open via EntityStore and verify reads work
        let store = EntityStore::open(&layout, workspace_id, result.entity.entity_id()).unwrap();

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
