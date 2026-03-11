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
use crate::domain::governance::profile::{
    CompanyAddress, DirectorInfo, DocumentOptions, FiscalYearEnd, FounderInfo,
    GOVERNANCE_PROFILE_PATH, GovernanceProfile, OfficerInfo, StockDetails,
    VestingSchedule as GovernanceVestingSchedule,
};
use crate::domain::governance::{
    body::GovernanceBody,
    doc_ast, doc_generator,
    seat::GovernanceSeat,
    types::{
        BodyStatus, BodyType, QuorumThreshold, SeatRole, SeatStatus, VotingMethod, VotingPower,
    },
};
use crate::domain::ids::*;
use crate::git::commit::FileWrite;
use chrono::{DateTime, Utc};
use std::collections::HashSet;

use super::error::FormationError;

fn default_registered_agent(entity: &Entity) -> (String, String) {
    let jurisdiction = entity.jurisdiction().to_string();
    if jurisdiction.contains("WY") {
        (
            "Wyoming Registered Agent LLC".to_owned(),
            "1712 Pioneer Ave, Suite 500, Cheyenne, WY 82001".to_owned(),
        )
    } else if jurisdiction.contains("DE") {
        (
            "Delaware Registered Agent Inc.".to_owned(),
            "8 The Green, Suite A, Dover, DE 19901".to_owned(),
        )
    } else {
        (
            "National Registered Agents Inc.".to_owned(),
            "1999 Bryan St, Suite 900, Dallas, TX 75201".to_owned(),
        )
    }
}

/// Result of creating a new entity through the formation workflow.
#[derive(Debug)]
pub struct FormationResult {
    pub entity: Entity,
    pub document_ids: Vec<DocumentId>,
    pub filing: Filing,
    pub tax_profile: TaxProfile,
}

/// Summary of a holder created during cap table setup.
#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
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

#[derive(Debug, Clone, Default)]
pub struct FormationProfileOverrides {
    pub formation_date: Option<DateTime<Utc>>,
    pub fiscal_year_end: Option<FiscalYearEnd>,
    pub document_options: Option<DocumentOptions>,
    pub company_address: Option<CompanyAddress>,
    pub board_size: Option<u32>,
    pub principal_name: Option<String>,
}

fn member_role_label(role: Option<MemberRole>, entity_type: EntityType) -> String {
    match role {
        Some(MemberRole::Director) => "director".to_owned(),
        Some(MemberRole::Officer) => "officer".to_owned(),
        Some(MemberRole::Manager) => "manager".to_owned(),
        Some(MemberRole::Member) => "member".to_owned(),
        Some(MemberRole::Chair) => "chair".to_owned(),
        None => match entity_type {
            EntityType::CCorp => "incorporator".to_owned(),
            EntityType::Llc => "organizer".to_owned(),
        },
    }
}

fn validate_member_email(email: &str) -> Result<String, FormationError> {
    let normalized = email.trim().to_ascii_lowercase();
    if normalized.is_empty()
        || normalized.chars().any(char::is_whitespace)
        || normalized.contains(',')
        || normalized.matches('@').count() != 1
    {
        return Err(FormationError::Validation(
            "member email must be a valid single address".to_owned(),
        ));
    }
    let (local, domain) = normalized.split_once('@').ok_or_else(|| {
        FormationError::Validation("member email must be a valid single address".to_owned())
    })?;
    if local.is_empty()
        || domain.is_empty()
        || !domain.contains('.')
        || domain.starts_with('.')
        || domain.ends_with('.')
        || domain.contains("..")
    {
        return Err(FormationError::Validation(
            "member email must be a valid single address".to_owned(),
        ));
    }
    Ok(normalized)
}

fn validate_member_inputs(members: &[MemberInput]) -> Result<(), FormationError> {
    let mut seen_emails = HashSet::new();
    let mut specified_ownership_total = 0.0_f64;

    for member in members {
        if member.name.trim().is_empty() {
            return Err(FormationError::Validation(
                "member name cannot be empty".to_owned(),
            ));
        }
        if let Some(email) = member.email.as_deref() {
            let normalized = validate_member_email(email)?;
            if !seen_emails.insert(normalized.clone()) {
                return Err(FormationError::Validation(format!(
                    "duplicate member email: {normalized}"
                )));
            }
        }
        if let Some(ownership_pct) = member.ownership_pct {
            if !ownership_pct.is_finite() || ownership_pct <= 0.0 || ownership_pct > 100.0 {
                return Err(FormationError::Validation(
                    "ownership_pct must be greater than 0 and at most 100".to_owned(),
                ));
            }
            specified_ownership_total += ownership_pct;
        }
    }

    if specified_ownership_total > 100.000_001 {
        return Err(FormationError::Validation(format!(
            "total ownership_pct cannot exceed 100, got {:.2}",
            specified_ownership_total
        )));
    }

    Ok(())
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
    create_entity_with_profile_overrides(
        layout,
        workspace_id,
        legal_name,
        entity_type,
        jurisdiction,
        registered_agent_name,
        registered_agent_address,
        members,
        authorized_shares,
        par_value,
        FormationProfileOverrides::default(),
    )
}

/// Create a new entity with explicit governance profile overrides.
#[allow(clippy::too_many_arguments)]
pub fn create_entity_with_profile_overrides(
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
    profile_overrides: FormationProfileOverrides,
) -> Result<FormationResult, FormationError> {
    // Validate members
    if members.is_empty() {
        return Err(FormationError::Validation(
            "at least one member is required".into(),
        ));
    }
    validate_member_inputs(members)?;

    let entity_id = EntityId::new();

    // Create filing record (borrows jurisdiction).
    let filing_type = match entity_type {
        EntityType::Llc => FilingType::CertificateOfFormation,
        EntityType::CCorp => FilingType::CertificateOfIncorporation,
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
    if entity.registered_agent_name().is_none() {
        let (default_name, default_address) = default_registered_agent(&entity);
        entity.set_registered_agent(Some(default_name), Some(default_address))?;
    }
    if let Some(formation_date) = profile_overrides.formation_date.as_ref().cloned() {
        entity.set_formation_date(formation_date);
    }
    let governance_profile = build_governance_profile(
        &entity,
        members,
        authorized_shares,
        par_value,
        None,
        Some(&profile_overrides),
    );
    governance_profile
        .validate()
        .map_err(FormationError::Validation)?;

    // Create Document records
    let documents =
        generate_ast_formation_documents(&entity, workspace_id, members, &governance_profile)?;

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
    files.push(
        FileWrite::json(GOVERNANCE_PROFILE_PATH, &governance_profile)
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

pub fn retire_incompatible_governance_for_entity(
    store: &crate::store::entity_store::EntityStore<'_>,
    entity: &Entity,
) -> Result<(), FormationError> {
    let body_ids = store
        .list_ids::<GovernanceBody>("main")
        .map_err(|e| FormationError::Storage(format!("failed to list governance bodies: {e}")))?;
    let seat_ids = store
        .list_ids::<GovernanceSeat>("main")
        .map_err(|e| FormationError::Storage(format!("failed to list governance seats: {e}")))?;
    let incompatible_body_type = match entity.entity_type() {
        EntityType::CCorp => BodyType::LlcMemberVote,
        EntityType::Llc => BodyType::BoardOfDirectors,
    };

    let mut files = Vec::new();
    let mut retired_body_ids = HashSet::new();
    for body_id in body_ids {
        let path = format!("governance/bodies/{body_id}.json");
        let mut body = match store.read::<GovernanceBody>("main", body_id) {
            Ok(body) => body,
            Err(_) => continue,
        };
        if body.entity_id() != entity.entity_id()
            || body.body_type() != incompatible_body_type
            || body.status() != BodyStatus::Active
        {
            continue;
        }
        body.deactivate();
        retired_body_ids.insert(body.body_id());
        files.push(FileWrite::json(path, &body).map_err(|e| {
            FormationError::Storage(format!("failed to serialize governance body: {e}"))
        })?);
    }

    for seat_id in seat_ids {
        let path = format!("governance/seats/{seat_id}.json");
        let mut seat = match store.read::<GovernanceSeat>("main", seat_id) {
            Ok(seat) => seat,
            Err(_) => continue,
        };
        if retired_body_ids.contains(&seat.body_id()) && seat.status() == SeatStatus::Active {
            seat.expire();
            files.push(FileWrite::json(path, &seat).map_err(|e| {
                FormationError::Storage(format!("failed to serialize governance seat: {e}"))
            })?);
        }
    }

    if files.is_empty() {
        return Ok(());
    }

    store
        .commit(
            "main",
            &format!(
                "Retire incompatible governance after converting to {}",
                entity.entity_type()
            ),
            files,
        )
        .map_err(|e| {
            FormationError::Storage(format!("failed to commit governance retirement: {e}"))
        })?;

    Ok(())
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

fn format_member_mailing_address(address: &Address) -> String {
    let mut parts = vec![address.street.clone()];
    if let Some(street2) = &address.street2
        && !street2.trim().is_empty()
    {
        parts.push(street2.clone());
    }
    parts.push(address.city.clone());
    parts.push(address.state.clone());
    parts.push(address.zip.clone());
    parts.join(", ")
}

fn to_company_address(address: &Address) -> CompanyAddress {
    CompanyAddress {
        street: match &address.street2 {
            Some(street2) if !street2.trim().is_empty() => {
                format!("{}, {}", address.street, street2)
            }
            _ => address.street.clone(),
        },
        city: address.city.clone(),
        county: None,
        state: address.state.clone(),
        zip: address.zip.clone(),
    }
}

fn officer_title_label(title: OfficerTitle) -> String {
    match title {
        OfficerTitle::Ceo => "Chief Executive Officer".to_owned(),
        OfficerTitle::Cfo => "Chief Financial Officer".to_owned(),
        OfficerTitle::Cto => "Chief Technology Officer".to_owned(),
        OfficerTitle::Coo => "Chief Operating Officer".to_owned(),
        OfficerTitle::Secretary => "Secretary".to_owned(),
        OfficerTitle::Treasurer => "Treasurer".to_owned(),
        OfficerTitle::President => "President".to_owned(),
        OfficerTitle::Vp => "Vice President".to_owned(),
        OfficerTitle::Other => "Officer".to_owned(),
    }
}

fn default_authorized_units(entity_type: EntityType, authorized_shares: Option<i64>) -> i64 {
    match entity_type {
        EntityType::CCorp => authorized_shares.unwrap_or(10_000_000),
        EntityType::Llc => authorized_shares.unwrap_or(10_000),
    }
}

fn derived_member_units(
    entity_type: EntityType,
    member: &MemberInput,
    authorized_shares: Option<i64>,
) -> Option<u64> {
    let units = match entity_type {
        EntityType::Llc => member.membership_units.unwrap_or_else(|| {
            let pct = member.ownership_pct.unwrap_or(0.0);
            let total = default_authorized_units(entity_type, authorized_shares);
            ((pct / 100.0) * total as f64).round() as i64
        }),
        EntityType::CCorp => member
            .shares_purchased
            .or(member.share_count)
            .unwrap_or_else(|| {
                let pct = member.ownership_pct.unwrap_or(0.0);
                let total = default_authorized_units(entity_type, authorized_shares) as f64 * 0.8;
                ((pct / 100.0) * total).round() as i64
            }),
    };
    (units > 0).then_some(units as u64)
}

fn parse_par_value_units(par_value: Option<&str>) -> Option<u64> {
    let raw = par_value.unwrap_or("0.0001").trim();
    let value = raw.parse::<f64>().ok()?;
    (value > 0.0).then_some((value * 10_000.0).round() as u64)
}

fn voting_power_from_units(units: u64, member_name: &str) -> Result<VotingPower, FormationError> {
    let raw = u32::try_from(units).map_err(|_| {
        FormationError::Validation(format!(
            "voting power for member '{member_name}' exceeds the maximum supported value"
        ))
    })?;
    VotingPower::new(raw).map_err(FormationError::Validation)
}

fn llc_member_vote_voting_power(
    member: &MemberInput,
    authorized_shares: Option<i64>,
) -> Result<Option<VotingPower>, FormationError> {
    if matches!(
        member.role,
        Some(MemberRole::Director | MemberRole::Officer | MemberRole::Manager)
    ) {
        return Ok(None);
    }

    let Some(units) = derived_member_units(EntityType::Llc, member, authorized_shares) else {
        return Ok(None);
    };

    voting_power_from_units(units, &member.name).map(Some)
}

fn corporate_governance_body(entity_id: EntityId) -> Result<GovernanceBody, FormationError> {
    GovernanceBody::new(
        GovernanceBodyId::new(),
        entity_id,
        BodyType::BoardOfDirectors,
        "Board of Directors".to_owned(),
        QuorumThreshold::Majority,
        VotingMethod::PerCapita,
    )
    .map_err(|e| FormationError::Validation(format!("governance body error: {e}")))
}

fn llc_governance_body(entity_id: EntityId) -> Result<GovernanceBody, FormationError> {
    GovernanceBody::new(
        GovernanceBodyId::new(),
        entity_id,
        BodyType::LlcMemberVote,
        "LLC Member Vote".to_owned(),
        QuorumThreshold::Majority,
        VotingMethod::PerUnit,
    )
    .map_err(|e| FormationError::Validation(format!("governance body error: {e}")))
}

fn bootstrap_governance_records(
    entity_id: EntityId,
    entity_type: EntityType,
    members: &[&MemberInput],
    contacts: &[Contact],
    authorized_shares: Option<i64>,
) -> Result<(GovernanceBody, Vec<GovernanceSeat>), FormationError> {
    let pairs: Vec<(&MemberInput, &Contact)> =
        members.iter().copied().zip(contacts.iter()).collect();

    match entity_type {
        EntityType::CCorp => {
            let body = corporate_governance_body(entity_id)?;
            let mut seat_members: Vec<(&MemberInput, &Contact)> = pairs
                .iter()
                .copied()
                .filter(|(member, _)| {
                    matches!(member.role, Some(MemberRole::Director | MemberRole::Chair))
                })
                .collect();
            if seat_members.is_empty() {
                let fallback = pairs
                    .iter()
                    .copied()
                    .find(|(member, _)| {
                        member.is_incorporator == Some(true)
                            && member.investor_type == InvestorType::NaturalPerson
                    })
                    .or_else(|| {
                        pairs
                            .iter()
                            .copied()
                            .find(|(member, _)| member.investor_type == InvestorType::NaturalPerson)
                    })
                    .or_else(|| pairs.first().copied())
                    .ok_or_else(|| {
                        FormationError::Validation(
                            "at least one eligible member is required for governance bootstrap"
                                .to_owned(),
                        )
                    })?;
                seat_members.push(fallback);
            }

            let seats = seat_members
                .into_iter()
                .map(|(member, contact)| {
                    GovernanceSeat::new(
                        GovernanceSeatId::new(),
                        body.body_id(),
                        contact.contact_id(),
                        if matches!(member.role, Some(MemberRole::Chair)) {
                            SeatRole::Chair
                        } else {
                            SeatRole::Member
                        },
                        None,
                        None,
                        Some(VotingPower::new(1).expect("1 is valid voting power")),
                    )
                    .map_err(|e| FormationError::Validation(format!("governance seat error: {e}")))
                })
                .collect::<Result<Vec<_>, _>>()?;

            Ok((body, seats))
        }
        EntityType::Llc => {
            let body = llc_governance_body(entity_id)?;
            let seat_members = pairs
                .into_iter()
                .filter_map(|(member, contact)| {
                    match llc_member_vote_voting_power(member, authorized_shares) {
                        Ok(Some(voting_power)) => Some(Ok((member, contact, voting_power))),
                        Ok(None) => None,
                        Err(err) => Some(Err(err)),
                    }
                })
                .collect::<Result<Vec<_>, _>>()?;
            let seats = seat_members
                .into_iter()
                .map(|(member, contact, voting_power)| {
                    GovernanceSeat::new(
                        GovernanceSeatId::new(),
                        body.body_id(),
                        contact.contact_id(),
                        if matches!(member.role, Some(MemberRole::Chair)) {
                            SeatRole::Chair
                        } else {
                            SeatRole::Member
                        },
                        None,
                        None,
                        Some(voting_power),
                    )
                    .map_err(|e| FormationError::Validation(format!("governance seat error: {e}")))
                })
                .collect::<Result<Vec<_>, _>>()?;

            Ok((body, seats))
        }
    }
}

fn shared_company_address(members: &[MemberInput]) -> Option<CompanyAddress> {
    let mut addresses = members
        .iter()
        .filter(|member| member.investor_type != InvestorType::Agent)
        .filter_map(|member| member.address.as_ref())
        .map(to_company_address);
    let first = addresses.next()?;
    if addresses.all(|address| {
        address.street == first.street
            && address.city == first.city
            && address.state == first.state
            && address.zip == first.zip
    }) {
        Some(first)
    } else {
        None
    }
}

fn default_document_options() -> DocumentOptions {
    DocumentOptions {
        dating_format: "blank_line".to_owned(),
        transfer_restrictions: true,
        right_of_first_refusal: true,
        s_corp_election: false,
    }
}

fn signature_requirement_to_json(req: &SignatureRequirement) -> serde_json::Value {
    let mut value = serde_json::json!({
        "role": req.role,
        "signer_name": req.signer_name,
        "required": req.required,
    });
    if let Some(email) = &req.signer_email {
        value["signer_email"] = serde_json::Value::String(email.clone());
    }
    value
}

fn format_par_value_units(units: u64) -> String {
    if units == 0 {
        return "0".to_owned();
    }
    let whole = units / 10_000;
    let fractional = units % 10_000;
    if fractional == 0 {
        whole.to_string()
    } else {
        let rendered = format!("{}.{:04}", whole, fractional);
        rendered.trim_end_matches('0').to_owned()
    }
}

fn build_governance_profile(
    entity: &Entity,
    members: &[MemberInput],
    authorized_shares: Option<i64>,
    par_value: Option<&str>,
    base_profile: Option<GovernanceProfile>,
    overrides: Option<&FormationProfileOverrides>,
) -> GovernanceProfile {
    let non_agent_members: Vec<&MemberInput> = members
        .iter()
        .filter(|member| member.investor_type != InvestorType::Agent)
        .collect();
    let mut profile = base_profile.unwrap_or_else(|| GovernanceProfile::default_for_entity(entity));

    let incorporator = non_agent_members
        .iter()
        .find(|member| member.is_incorporator == Some(true))
        .copied()
        .or_else(|| non_agent_members.first().copied());
    let mut directors: Vec<DirectorInfo> = non_agent_members
        .iter()
        .filter(|member| matches!(member.role, Some(MemberRole::Director | MemberRole::Chair)))
        .filter_map(|member| {
            Some(DirectorInfo {
                name: member.name.clone(),
                address: member.address.as_ref().map(to_company_address),
            })
        })
        .collect();
    if directors.is_empty() && non_agent_members.len() == 1 {
        let founder = non_agent_members[0];
        directors.push(DirectorInfo {
            name: founder.name.clone(),
            address: founder.address.as_ref().map(to_company_address),
        });
    }

    let officers: Vec<OfficerInfo> = non_agent_members
        .iter()
        .filter_map(|member| {
            member.officer_title.map(|title| OfficerInfo {
                name: member.name.clone(),
                title: officer_title_label(title),
            })
        })
        .collect();
    let founders: Vec<FounderInfo> = non_agent_members
        .iter()
        .map(|member| FounderInfo {
            name: member.name.clone(),
            shares: derived_member_units(entity.entity_type(), member, authorized_shares),
            vesting: member
                .vesting
                .as_ref()
                .map(|vesting| GovernanceVestingSchedule {
                    total_months: vesting.total_months as u32,
                    cliff_months: vesting.cliff_months as u32,
                    acceleration_on_termination: vesting
                        .acceleration
                        .as_deref()
                        .is_some_and(|value| value.eq_ignore_ascii_case("single_trigger")),
                }),
            ip_contribution: member.ip_description.clone(),
            email: member.email.clone(),
            address: member.address.as_ref().map(to_company_address),
        })
        .collect();

    let board_size = overrides
        .and_then(|config| config.board_size)
        .or_else(|| (!directors.is_empty()).then_some(directors.len() as u32));
    let principal = non_agent_members
        .iter()
        .find(|member| matches!(member.role, Some(MemberRole::Manager)))
        .copied()
        .or_else(|| (non_agent_members.len() == 1).then(|| non_agent_members[0]));
    let principal_title = principal.and_then(|member| match member.role {
        Some(MemberRole::Manager) => Some("Manager".to_owned()),
        Some(MemberRole::Member) if entity.entity_type() == EntityType::Llc => {
            Some("Managing Member".to_owned())
        }
        _ => member.officer_title.map(officer_title_label),
    });
    let adopted_by = match entity.entity_type() {
        EntityType::CCorp if !directors.is_empty() => "Board of Directors".to_owned(),
        EntityType::CCorp => "Incorporator".to_owned(),
        EntityType::Llc => "Members".to_owned(),
    };
    let effective_date = overrides
        .and_then(|config| config.formation_date.map(|value| value.date_naive()))
        .or_else(|| entity.formation_date().map(|value| value.date_naive()))
        .unwrap_or(profile.effective_date());
    profile.update(
        entity.legal_name().to_owned(),
        entity.jurisdiction().to_string(),
        effective_date,
        adopted_by,
        profile.last_reviewed(),
        profile.next_mandatory_review(),
        entity
            .registered_agent_name()
            .map(ToOwned::to_owned)
            .or_else(|| profile.registered_agent_name().map(ToOwned::to_owned)),
        entity
            .registered_agent_address()
            .map(ToOwned::to_owned)
            .or_else(|| profile.registered_agent_address().map(ToOwned::to_owned)),
        board_size.or(profile.board_size()),
        incorporator
            .map(|member| member.name.clone())
            .or_else(|| profile.incorporator_name().map(ToOwned::to_owned)),
        incorporator
            .and_then(|member| member.address.as_ref())
            .map(format_member_mailing_address)
            .or_else(|| profile.incorporator_address().map(ToOwned::to_owned)),
        overrides
            .and_then(|config| config.principal_name.clone())
            .or_else(|| principal.map(|member| member.name.clone()))
            .or_else(|| profile.principal_name().map(ToOwned::to_owned)),
        principal_title.or_else(|| profile.principal_title().map(ToOwned::to_owned)),
        Some(profile.incomplete_profile()),
    );
    if let Some(address) = overrides
        .and_then(|config| config.company_address.clone())
        .or_else(|| shared_company_address(members))
    {
        profile.set_company_address(address);
    }
    if !founders.is_empty() {
        profile.set_founders(founders);
    }
    if !directors.is_empty() {
        profile.set_directors(directors);
    }
    if !officers.is_empty() {
        profile.set_officers(officers);
    }
    if entity.entity_type() == EntityType::CCorp {
        let existing_stock = profile.stock_details().cloned();
        profile.set_stock_details(StockDetails {
            authorized_shares: authorized_shares
                .map(|value| value as u64)
                .or_else(|| {
                    existing_stock
                        .as_ref()
                        .map(|details| details.authorized_shares)
                })
                .unwrap_or(
                    default_authorized_units(entity.entity_type(), authorized_shares) as u64,
                ),
            par_value_cents: parse_par_value_units(par_value)
                .or_else(|| {
                    existing_stock
                        .as_ref()
                        .map(|details| details.par_value_cents)
                })
                .unwrap_or(1),
            share_class: "Common Stock".to_owned(),
        });
    }
    profile.set_fiscal_year_end(
        overrides
            .and_then(|config| config.fiscal_year_end.clone())
            .or_else(|| profile.fiscal_year_end().cloned())
            .unwrap_or(FiscalYearEnd { month: 12, day: 31 }),
    );
    profile.set_document_options(
        overrides
            .and_then(|config| config.document_options.clone())
            .or_else(|| profile.document_options().cloned())
            .unwrap_or_else(default_document_options),
    );

    profile
}

fn formation_document_bindings(entity_type: EntityType) -> Vec<(&'static str, DocumentType)> {
    match entity_type {
        EntityType::CCorp => vec![
            (
                "certificate_of_incorporation",
                DocumentType::ArticlesOfIncorporation,
            ),
            ("bylaws", DocumentType::Bylaws),
            ("incorporator_action", DocumentType::IncorporatorAction),
            ("initial_board_consent", DocumentType::InitialBoardConsent),
        ],
        EntityType::Llc => vec![
            (
                "articles_of_organization",
                DocumentType::ArticlesOfOrganization,
            ),
            ("operating_agreement", DocumentType::OperatingAgreement),
            (
                "initial_written_consent",
                DocumentType::InitialWrittenConsent,
            ),
        ],
    }
}

fn formation_signature_requirements(
    entity_type: EntityType,
    governance_tag: &str,
    members: &[MemberInput],
) -> Vec<serde_json::Value> {
    let non_agent_members: Vec<&MemberInput> = members
        .iter()
        .filter(|member| member.investor_type != InvestorType::Agent)
        .collect();
    let incorporator = non_agent_members
        .iter()
        .find(|member| member.is_incorporator == Some(true))
        .copied()
        .or_else(|| non_agent_members.first().copied());
    let directors: Vec<&MemberInput> = non_agent_members
        .iter()
        .copied()
        .filter(|member| matches!(member.role, Some(MemberRole::Director | MemberRole::Chair)))
        .collect();
    let director_signers: Vec<&MemberInput> =
        if directors.is_empty() && non_agent_members.len() == 1 {
            vec![non_agent_members[0]]
        } else {
            directors
        };

    match (entity_type, governance_tag) {
        (EntityType::CCorp, "certificate_of_incorporation")
        | (EntityType::CCorp, "incorporator_action") => incorporator
            .map(|member| {
                vec![signature_requirement_to_json(&SignatureRequirement {
                    role: "Incorporator".to_owned(),
                    signer_name: member.name.clone(),
                    signer_email: member.email.clone(),
                    required: true,
                })]
            })
            .unwrap_or_default(),
        (EntityType::CCorp, "bylaws") | (EntityType::CCorp, "initial_board_consent") => {
            director_signers
                .into_iter()
                .map(|member| {
                    signature_requirement_to_json(&SignatureRequirement {
                        role: "Director".to_owned(),
                        signer_name: member.name.clone(),
                        signer_email: member.email.clone(),
                        required: true,
                    })
                })
                .collect()
        }
        (EntityType::Llc, "articles_of_organization") => incorporator
            .map(|member| {
                vec![signature_requirement_to_json(&SignatureRequirement {
                    role: "Organizer".to_owned(),
                    signer_name: member.name.clone(),
                    signer_email: member.email.clone(),
                    required: true,
                })]
            })
            .unwrap_or_default(),
        (EntityType::Llc, "operating_agreement") | (EntityType::Llc, "initial_written_consent") => {
            non_agent_members
                .into_iter()
                .map(|member| {
                    let role = match member.investor_type {
                        InvestorType::Entity => "Officer".to_owned(),
                        _ => match member.role {
                            Some(MemberRole::Manager) => "Manager".to_owned(),
                            _ => "Member".to_owned(),
                        },
                    };
                    signature_requirement_to_json(&SignatureRequirement {
                        role,
                        signer_name: member.name.clone(),
                        signer_email: member.email.clone(),
                        required: true,
                    })
                })
                .collect()
        }
        _ => Vec::new(),
    }
}

fn validate_ast_document(
    entity_type: EntityType,
    governance_tag: &str,
    profile: &GovernanceProfile,
    content: &serde_json::Value,
) -> Result<(), FormationError> {
    let ast = doc_ast::default_doc_ast();
    let doc_def = ast
        .documents
        .iter()
        .find(|doc| doc.id == governance_tag)
        .ok_or_else(|| {
            FormationError::Validation(format!(
                "no AST document definition matches governance_tag '{}'",
                governance_tag
            ))
        })?;
    let rendered = doc_generator::render_document_from_ast_with_context(
        doc_def,
        ast,
        match entity_type {
            EntityType::CCorp => doc_ast::EntityTypeKey::Corporation,
            EntityType::Llc => doc_ast::EntityTypeKey::Llc,
        },
        profile,
        content,
    );
    let warnings = doc_generator::detect_placeholder_warnings_for_text(governance_tag, &rendered);
    if warnings.is_empty() {
        Ok(())
    } else {
        Err(FormationError::Validation(format!(
            "document '{}' is incomplete for production use: {}",
            governance_tag,
            warnings.join("; ")
        )))
    }
}

fn generate_ast_formation_documents(
    entity: &Entity,
    workspace_id: WorkspaceId,
    members: &[MemberInput],
    profile: &GovernanceProfile,
) -> Result<Vec<Document>, FormationError> {
    let ast = doc_ast::default_doc_ast();
    let mut documents = Vec::new();
    for (governance_tag, document_type) in formation_document_bindings(entity.entity_type()) {
        let doc_def = ast
            .documents
            .iter()
            .find(|doc| doc.id == governance_tag)
            .ok_or_else(|| {
                FormationError::Validation(format!(
                    "no AST document definition matches governance_tag '{}'",
                    governance_tag
                ))
            })?;
        let mut content = serde_json::json!({});
        let signature_requirements =
            formation_signature_requirements(entity.entity_type(), governance_tag, members);
        if !signature_requirements.is_empty() {
            content["signature_requirements"] = serde_json::Value::Array(signature_requirements);
        }
        validate_ast_document(entity.entity_type(), governance_tag, profile, &content)?;
        documents.push(Document::new(
            DocumentId::new(),
            entity.entity_id(),
            workspace_id,
            document_type,
            format!("{} — {}", doc_def.title, entity.legal_name()),
            content,
            Some(governance_tag.to_owned()),
            None,
        ));
    }
    Ok(documents)
}

/// Set up the cap table for a newly formed entity.
///
/// Creates contacts, an equity legal entity, an instrument, initial positions,
/// and default governance records in one atomic git commit.
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
        )
        .map_err(FormationError::Validation)?;
        let mut contact = contact;
        if let Some(address) = &m.address {
            contact.set_mailing_address(Some(format_member_mailing_address(address)));
        }
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
            (
                InstrumentKind::MembershipUnit,
                "UNITS".to_owned(),
                auth,
                None,
            )
        }
        EntityType::CCorp => {
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
            EntityType::CCorp => {
                m.shares_purchased.or(m.share_count).unwrap_or_else(|| {
                    let pct = m.ownership_pct.unwrap_or(0.0);
                    let total = auth_units.unwrap_or(10_000_000);
                    // Reserve 20% for future issuances (standard practice)
                    ((pct / 100.0) * (total as f64 * 0.8)).round() as i64
                })
            }
        };

        let principal = match entity_type {
            EntityType::CCorp => {
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

    let (governance_body, governance_seats) = bootstrap_governance_records(
        entity_id,
        entity_type,
        &non_agent_members,
        &contacts,
        authorized_shares,
    )?;

    // 5. Write all records in a single atomic commit
    let repo_path = layout.entity_repo_path(workspace_id, entity_id);
    let repo = crate::git::repo::CorpRepo::open(&repo_path)
        .map_err(|e| FormationError::Storage(format!("failed to open repo: {e}")))?;

    let mut files = Vec::new();

    // Contacts
    for contact in &contacts {
        let path = format!("contacts/{}.json", contact.contact_id());
        files.push(
            FileWrite::json(path, contact).map_err(|e| FormationError::Storage(e.to_string()))?,
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
            FileWrite::json(path, holder).map_err(|e| FormationError::Storage(e.to_string()))?,
        );
    }

    // Positions
    for position in &positions {
        let path = format!("cap-table/positions/{}.json", position.position_id());
        files.push(
            FileWrite::json(path, position).map_err(|e| FormationError::Storage(e.to_string()))?,
        );
    }

    // Governance body
    files.push(
        FileWrite::json(
            format!("governance/bodies/{}.json", governance_body.body_id()),
            &governance_body,
        )
        .map_err(|e| FormationError::Storage(e.to_string()))?,
    );

    // Governance seats
    for seat in &governance_seats {
        let path = format!("governance/seats/{}.json", seat.seat_id());
        files
            .push(FileWrite::json(path, seat).map_err(|e| FormationError::Storage(e.to_string()))?);
    }

    crate::git::commit::commit_files(
        &repo,
        "main",
        "Initialize cap table and governance with founding members",
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

/// Create a pending entity — initializes a git repo with an empty pending members list
/// and a default governance profile.
///
/// Unlike `create_entity`, this does NOT require members upfront and does NOT generate
/// formation documents. The entity stays in `Pending` status until `finalize_formation`
/// is called after founders have been added via `add_pending_member`.
pub fn create_pending_entity(
    layout: &crate::store::RepoLayout,
    workspace_id: WorkspaceId,
    legal_name: String,
    entity_type: EntityType,
    jurisdiction: Jurisdiction,
) -> Result<Entity, FormationError> {
    create_pending_entity_with_profile_overrides(
        layout,
        workspace_id,
        legal_name,
        entity_type,
        jurisdiction,
        None,
        None,
        FormationProfileOverrides::default(),
    )
}

/// Create a pending entity with explicit company-level metadata.
#[allow(clippy::too_many_arguments)]
pub fn create_pending_entity_with_profile_overrides(
    layout: &crate::store::RepoLayout,
    workspace_id: WorkspaceId,
    legal_name: String,
    entity_type: EntityType,
    jurisdiction: Jurisdiction,
    registered_agent_name: Option<String>,
    registered_agent_address: Option<String>,
    profile_overrides: FormationProfileOverrides,
) -> Result<Entity, FormationError> {
    let entity_id = EntityId::new();

    let mut entity = Entity::new(
        entity_id,
        workspace_id,
        legal_name,
        entity_type,
        jurisdiction,
        registered_agent_name,
        registered_agent_address,
    )?;
    if let Some(formation_date) = profile_overrides.formation_date.as_ref().cloned() {
        entity.set_formation_date(formation_date);
    }
    let governance_profile =
        build_governance_profile(&entity, &[], None, None, None, Some(&profile_overrides));
    governance_profile
        .validate()
        .map_err(FormationError::Validation)?;

    // Build files: corp.json + governance profile + empty pending members list
    let mut files = Vec::new();
    files.push(
        FileWrite::json("corp.json", &entity)
            .map_err(|e| FormationError::Storage(e.to_string()))?,
    );
    files.push(
        FileWrite::json(GOVERNANCE_PROFILE_PATH, &governance_profile)
            .map_err(|e| FormationError::Storage(e.to_string()))?,
    );
    let empty_members: Vec<MemberInput> = Vec::new();
    files.push(
        FileWrite::json("formation/pending_members.json", &empty_members)
            .map_err(|e| FormationError::Storage(e.to_string()))?,
    );

    let repo_path = layout.entity_repo_path(workspace_id, entity_id);
    let repo = crate::git::repo::CorpRepo::init(&repo_path, None)
        .map_err(|e| FormationError::Storage(format!("failed to init repo: {e}")))?;
    crate::git::commit::commit_files(
        &repo,
        "main",
        &format!("Create pending entity: {}", entity.legal_name()),
        &files,
        None,
    )
    .map_err(|e| FormationError::Storage(format!("failed to commit: {e}")))?;

    Ok(entity)
}

/// Add a member to a pending entity's formation member list.
///
/// The entity must be in `Pending` status. Returns the full list of pending members
/// after the new member is appended.
pub fn add_pending_member(
    layout: &crate::store::RepoLayout,
    workspace_id: WorkspaceId,
    entity_id: EntityId,
    member: MemberInput,
) -> Result<Vec<MemberInput>, FormationError> {
    let repo_path = layout.entity_repo_path(workspace_id, entity_id);
    let repo = crate::git::repo::CorpRepo::open(&repo_path)
        .map_err(|e| FormationError::Storage(format!("failed to open repo: {e}")))?;

    // Verify entity is still Pending
    let entity: Entity = repo
        .read_json("main", "corp.json")
        .map_err(|e| FormationError::Storage(format!("failed to read entity: {e}")))?;
    if entity.formation_status() != FormationStatus::Pending {
        return Err(FormationError::Validation(format!(
            "entity must be in Pending status to add members, currently {}",
            entity.formation_status()
        )));
    }

    // Read existing pending members
    let mut members: Vec<MemberInput> = repo
        .read_json("main", "formation/pending_members.json")
        .map_err(|e| FormationError::Storage(format!("failed to read pending members: {e}")))?;

    members.push(member);
    validate_member_inputs(&members)?;

    // Commit updated list
    let file = FileWrite::json("formation/pending_members.json", &members)
        .map_err(|e| FormationError::Storage(e.to_string()))?;
    crate::git::commit::commit_files(
        &repo,
        "main",
        &format!("Add pending member: {}", members.last().unwrap().name),
        &[file],
        None,
    )
    .map_err(|e| FormationError::Storage(format!("failed to commit: {e}")))?;

    Ok(members)
}

/// Finalize a pending entity's formation — generates canonical AST-backed
/// formation documents, cap table records, and advances status.
#[allow(clippy::too_many_arguments)]
pub fn finalize_formation(
    layout: &crate::store::RepoLayout,
    workspace_id: WorkspaceId,
    entity_id: EntityId,
    authorized_shares: Option<i64>,
    par_value: Option<&str>,
) -> Result<(FormationResult, CapTableSetupResult), FormationError> {
    finalize_formation_with_profile_overrides(
        layout,
        workspace_id,
        entity_id,
        authorized_shares,
        par_value,
        None,
        None,
        None,
        None,
        FormationProfileOverrides::default(),
    )
}

/// Finalize a pending entity using explicit company-level metadata overrides.
#[allow(clippy::too_many_arguments)]
pub fn finalize_formation_with_profile_overrides(
    layout: &crate::store::RepoLayout,
    workspace_id: WorkspaceId,
    entity_id: EntityId,
    authorized_shares: Option<i64>,
    par_value: Option<&str>,
    registered_agent_name: Option<String>,
    registered_agent_address: Option<String>,
    incorporator_name_override: Option<String>,
    incorporator_address_override: Option<String>,
    profile_overrides: FormationProfileOverrides,
) -> Result<(FormationResult, CapTableSetupResult), FormationError> {
    let repo_path = layout.entity_repo_path(workspace_id, entity_id);
    let repo = crate::git::repo::CorpRepo::open(&repo_path)
        .map_err(|e| FormationError::Storage(format!("failed to open repo: {e}")))?;

    // Read entity and verify Pending status
    let mut entity: Entity = repo
        .read_json("main", "corp.json")
        .map_err(|e| FormationError::Storage(format!("failed to read entity: {e}")))?;
    if entity.formation_status() != FormationStatus::Pending {
        return Err(FormationError::Validation(format!(
            "entity must be in Pending status to finalize, currently {}",
            entity.formation_status()
        )));
    }

    // Read pending members
    let members: Vec<MemberInput> = repo
        .read_json("main", "formation/pending_members.json")
        .map_err(|e| FormationError::Storage(format!("failed to read pending members: {e}")))?;
    if members.is_empty() {
        return Err(FormationError::Validation(
            "at least one member is required to finalize formation".into(),
        ));
    }
    validate_member_inputs(&members)?;

    if registered_agent_name.is_some() || registered_agent_address.is_some() {
        entity.set_registered_agent(
            registered_agent_name.or_else(|| entity.registered_agent_name().map(ToOwned::to_owned)),
            registered_agent_address
                .or_else(|| entity.registered_agent_address().map(ToOwned::to_owned)),
        )?;
    }
    // Provide jurisdiction-based defaults when no registered agent is set.
    if entity.registered_agent_name().is_none() {
        let (default_name, default_address) = default_registered_agent(&entity);
        entity.set_registered_agent(Some(default_name), Some(default_address))?;
    }
    if let Some(formation_date) = profile_overrides.formation_date.as_ref().cloned() {
        entity.set_formation_date(formation_date);
    }

    let existing_profile =
        match repo.read_json::<GovernanceProfile>("main", GOVERNANCE_PROFILE_PATH) {
            Ok(profile) => Some(profile),
            Err(crate::git::error::GitStorageError::NotFound(_)) => None,
            Err(e) => {
                return Err(FormationError::Storage(format!(
                    "failed to read governance profile: {e}"
                )));
            }
        };
    let resolved_authorized_shares = authorized_shares.or_else(|| {
        existing_profile
            .as_ref()
            .and_then(|profile| profile.stock_details())
            .map(|details| details.authorized_shares as i64)
    });
    let resolved_par_value = par_value.map(ToOwned::to_owned).or_else(|| {
        existing_profile
            .as_ref()
            .and_then(|profile| profile.stock_details())
            .map(|details| format_par_value_units(details.par_value_cents))
    });
    let mut governance_profile = build_governance_profile(
        &entity,
        &members,
        resolved_authorized_shares,
        resolved_par_value.as_deref(),
        existing_profile,
        Some(&profile_overrides),
    );
    // Apply explicit incorporator overrides from the finalize request.
    // This allows setting incorporator details at finalize time when
    // the founder was added without an address.
    if incorporator_name_override.is_some() || incorporator_address_override.is_some() {
        governance_profile
            .patch_incorporator(incorporator_name_override, incorporator_address_override);
    }
    governance_profile
        .validate()
        .map_err(FormationError::Validation)?;

    // Create filing record
    let filing_type = match entity.entity_type() {
        EntityType::Llc => FilingType::CertificateOfFormation,
        EntityType::CCorp => FilingType::CertificateOfIncorporation,
    };
    let designated_attestor = members
        .iter()
        .find(|m| m.investor_type == InvestorType::NaturalPerson)
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
    let designated_attestor_role =
        member_role_label(designated_attestor.role, entity.entity_type());

    let filing = Filing::new(
        FilingId::new(),
        entity_id,
        filing_type,
        entity.jurisdiction().clone(),
        designated_attestor_name,
        designated_attestor_email,
        designated_attestor_role,
    );

    // Create tax profile
    let classification = TaxProfile::classify(entity.entity_type(), members.len());
    let tax_profile = TaxProfile::new(TaxProfileId::new(), entity_id, classification);

    // Create Document records
    let documents =
        generate_ast_formation_documents(&entity, workspace_id, &members, &governance_profile)?;
    let document_ids: Vec<DocumentId> = documents.iter().map(|d| d.document_id()).collect();

    // Build files for the formation commit
    let mut files = Vec::new();
    files.push(
        FileWrite::json("formation/filing.json", &filing)
            .map_err(|e| FormationError::Storage(e.to_string()))?,
    );
    files.push(
        FileWrite::json("tax/profile.json", &tax_profile)
            .map_err(|e| FormationError::Storage(e.to_string()))?,
    );
    files.push(
        FileWrite::json(GOVERNANCE_PROFILE_PATH, &governance_profile)
            .map_err(|e| FormationError::Storage(e.to_string()))?,
    );
    for doc in &documents {
        let path = format!("formation/{}.json", doc.document_id());
        files.push(FileWrite::json(path, doc).map_err(|e| FormationError::Storage(e.to_string()))?);
    }

    crate::git::commit::commit_files(
        &repo,
        "main",
        &format!("Generate formation documents for: {}", entity.legal_name()),
        &files,
        None,
    )
    .map_err(|e| FormationError::Storage(format!("failed to commit: {e}")))?;

    // Advance status to DocumentsGenerated
    entity.advance_status(FormationStatus::DocumentsGenerated)?;
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

    let formation_result = FormationResult {
        entity,
        document_ids,
        filing,
        tax_profile,
    };

    // Set up cap table
    let cap_table_result = setup_cap_table(
        layout,
        workspace_id,
        entity_id,
        formation_result.entity.entity_type(),
        formation_result.entity.legal_name(),
        &members,
        resolved_authorized_shares,
        resolved_par_value.as_deref(),
    )?;

    Ok((formation_result, cap_table_result))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::governance::{
        body::GovernanceBody,
        seat::GovernanceSeat,
        types::{BodyStatus, BodyType, QuorumThreshold, SeatStatus, VotingMethod, VotingPower},
    };
    use crate::store::RepoLayout;
    use crate::store::entity_store::EntityStore;
    use tempfile::TempDir;

    fn delaware_address() -> Address {
        Address {
            street: "123 Main St".to_string(),
            street2: None,
            city: "Dover".to_string(),
            state: "DE".to_string(),
            zip: "19901".to_string(),
        }
    }

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

    fn governance_bodies(repo: &crate::git::repo::CorpRepo) -> Vec<GovernanceBody> {
        let entries = repo
            .list_dir("main", "governance/bodies")
            .expect("governance bodies directory should exist");
        entries
            .into_iter()
            .filter(|(_, is_dir)| !*is_dir)
            .map(|(name, _)| {
                repo.read_json("main", &format!("governance/bodies/{name}"))
                    .unwrap()
            })
            .collect()
    }

    fn governance_seats(repo: &crate::git::repo::CorpRepo) -> Vec<GovernanceSeat> {
        let entries = repo
            .list_dir("main", "governance/seats")
            .expect("governance seats directory should exist");
        entries
            .into_iter()
            .filter(|(_, is_dir)| !*is_dir)
            .map(|(name, _)| {
                repo.read_json("main", &format!("governance/seats/{name}"))
                    .unwrap()
            })
            .collect()
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

        // Verify documents were generated (LLC: articles + operating agreement + initial consent)
        assert_eq!(result.document_ids.len(), 3);

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
        alice.is_incorporator = Some(true);
        alice.address = Some(delaware_address());
        alice.officer_title = Some(OfficerTitle::Ceo);

        let result = create_entity(
            &layout,
            workspace_id,
            "Acme Corp".to_string(),
            EntityType::CCorp,
            Jurisdiction::new("Delaware").unwrap(),
            Some("RA Inc.".to_string()),
            Some("123 Main St".to_string()),
            &[alice],
            Some(10_000_000),
            Some("0.0001"),
        )
        .expect("create_entity should succeed for corporation");

        assert_eq!(result.entity.entity_type(), EntityType::CCorp);
        // Corporation: charter + bylaws + incorporator action + initial board consent
        assert_eq!(result.document_ids.len(), 4);
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
            EntityType::CCorp,
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
            Some("Wyoming Registered Agent LLC".to_string()),
            Some("123 Capitol Ave, Cheyenne, WY 82001".to_string()),
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

    #[test]
    fn staged_flow_create_add_finalize() {
        let tmp = TempDir::new().unwrap();
        let layout = RepoLayout::new(tmp.path().to_path_buf());
        let workspace_id = WorkspaceId::new();

        // Step 1: Create pending entity
        let entity = create_pending_entity_with_profile_overrides(
            &layout,
            workspace_id,
            "Staged LLC".to_string(),
            EntityType::Llc,
            Jurisdiction::new("Wyoming").unwrap(),
            Some("Wyoming Registered Agent LLC".to_string()),
            Some("123 Capitol Ave, Cheyenne, WY 82001".to_string()),
            FormationProfileOverrides::default(),
        )
        .expect("create_pending_entity should succeed");

        assert_eq!(entity.legal_name(), "Staged LLC");
        assert_eq!(entity.formation_status(), FormationStatus::Pending);
        let entity_id = entity.entity_id();
        let repo =
            crate::git::repo::CorpRepo::open(&layout.entity_repo_path(workspace_id, entity_id))
                .expect("pending entity repo should exist");
        let profile: GovernanceProfile = repo
            .read_json("main", GOVERNANCE_PROFILE_PATH)
            .expect("pending entity should seed a governance profile");
        assert_eq!(profile.legal_name(), "Staged LLC");

        // Step 2: Add founders
        let members = add_pending_member(&layout, workspace_id, entity_id, alice())
            .expect("add first member should succeed");
        assert_eq!(members.len(), 1);

        let members = add_pending_member(&layout, workspace_id, entity_id, bob())
            .expect("add second member should succeed");
        assert_eq!(members.len(), 2);

        // Step 3: Finalize
        let (formation, cap_table) =
            finalize_formation(&layout, workspace_id, entity_id, None, None)
                .expect("finalize_formation should succeed");

        assert_eq!(
            formation.entity.formation_status(),
            FormationStatus::DocumentsGenerated
        );
        assert_eq!(formation.document_ids.len(), 3);
        assert_eq!(cap_table.holders.len(), 2);
    }

    #[test]
    fn staged_flow_finalize_rejects_no_members() {
        let tmp = TempDir::new().unwrap();
        let layout = RepoLayout::new(tmp.path().to_path_buf());
        let workspace_id = WorkspaceId::new();

        let entity = create_pending_entity(
            &layout,
            workspace_id,
            "Empty Staged LLC".to_string(),
            EntityType::Llc,
            Jurisdiction::new("Delaware").unwrap(),
        )
        .unwrap();

        let result = finalize_formation(&layout, workspace_id, entity.entity_id(), None, None);

        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("at least one member")
        );
    }

    #[test]
    fn staged_flow_add_member_rejects_after_finalize() {
        let tmp = TempDir::new().unwrap();
        let layout = RepoLayout::new(tmp.path().to_path_buf());
        let workspace_id = WorkspaceId::new();

        let entity = create_pending_entity_with_profile_overrides(
            &layout,
            workspace_id,
            "Finalized LLC".to_string(),
            EntityType::Llc,
            Jurisdiction::new("Wyoming").unwrap(),
            Some("Wyoming Registered Agent LLC".to_string()),
            Some("123 Capitol Ave, Cheyenne, WY 82001".to_string()),
            FormationProfileOverrides::default(),
        )
        .unwrap();
        let entity_id = entity.entity_id();

        add_pending_member(&layout, workspace_id, entity_id, alice()).unwrap();
        finalize_formation(&layout, workspace_id, entity_id, None, None).unwrap();

        // Should reject adding members after finalization
        let result = add_pending_member(&layout, workspace_id, entity_id, bob());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Pending"));
    }

    #[test]
    fn staged_flow_finalize_accepts_company_metadata_overrides() {
        let tmp = TempDir::new().unwrap();
        let layout = RepoLayout::new(tmp.path().to_path_buf());
        let workspace_id = WorkspaceId::new();

        let entity = create_pending_entity(
            &layout,
            workspace_id,
            "Override LLC".to_string(),
            EntityType::Llc,
            Jurisdiction::new("Wyoming").unwrap(),
        )
        .unwrap();
        let entity_id = entity.entity_id();

        add_pending_member(&layout, workspace_id, entity_id, alice()).unwrap();

        let (formation, _) = finalize_formation_with_profile_overrides(
            &layout,
            workspace_id,
            entity_id,
            None,
            None,
            Some("Wyoming Registered Agent LLC".to_string()),
            Some("123 Capitol Ave, Cheyenne, WY 82001".to_string()),
            None,
            None,
            FormationProfileOverrides::default(),
        )
        .expect("finalize_formation_with_profile_overrides should succeed");

        assert_eq!(
            formation.entity.registered_agent_name(),
            Some("Wyoming Registered Agent LLC")
        );
    }

    #[test]
    fn staged_llc_preserves_explicit_jurisdiction_and_bootstraps_member_governance() {
        let tmp = TempDir::new().unwrap();
        let layout = RepoLayout::new(tmp.path().to_path_buf());
        let workspace_id = WorkspaceId::new();

        for jurisdiction in ["US-TX", "US-DE"] {
            let entity = create_pending_entity(
                &layout,
                workspace_id,
                format!("Jurisdiction {jurisdiction} LLC"),
                EntityType::Llc,
                Jurisdiction::new(jurisdiction).unwrap(),
            )
            .unwrap();
            let entity_id = entity.entity_id();

            add_pending_member(&layout, workspace_id, entity_id, alice()).unwrap();
            add_pending_member(&layout, workspace_id, entity_id, bob()).unwrap();

            let (formation, _) =
                finalize_formation(&layout, workspace_id, entity_id, None, None).unwrap();
            assert_eq!(formation.entity.jurisdiction().as_str(), jurisdiction);

            let repo =
                crate::git::repo::CorpRepo::open(&layout.entity_repo_path(workspace_id, entity_id))
                    .unwrap();
            let bodies = governance_bodies(&repo);
            assert_eq!(bodies.len(), 1);
            assert_eq!(bodies[0].body_type(), BodyType::LlcMemberVote);
            assert_eq!(bodies[0].quorum_rule(), QuorumThreshold::Majority);
            assert_eq!(bodies[0].voting_method(), VotingMethod::PerUnit);

            let seats = governance_seats(&repo);
            assert_eq!(seats.len(), 1);
            assert_eq!(seats[0].voting_power().raw(), 400);
        }
    }

    #[test]
    fn staged_llc_member_vote_excludes_managers_and_zero_unit_founders() {
        let entity_id = EntityId::new();
        let workspace_id = WorkspaceId::new();

        let mut zero_unit_member = bob();
        zero_unit_member.name = "Charlie Zero".to_string();
        zero_unit_member.email = Some("charlie@example.com".to_string());
        zero_unit_member.membership_units = None;
        zero_unit_member.ownership_pct = Some(0.0);

        let members = vec![alice(), bob(), zero_unit_member];
        let member_refs: Vec<&MemberInput> = members.iter().collect();
        let contacts: Vec<Contact> = members
            .iter()
            .map(|member| {
                Contact::new(
                    ContactId::new(),
                    entity_id,
                    workspace_id,
                    investor_type_to_contact_type(member.investor_type),
                    member.name.clone(),
                    member.email.clone(),
                    member_role_to_contact_category(member.role),
                )
                .expect("test contact should be valid")
            })
            .collect();

        let (_, seats) =
            bootstrap_governance_records(entity_id, EntityType::Llc, &member_refs, &contacts, None)
                .unwrap();
        assert_eq!(seats.len(), 1);
        assert_eq!(seats[0].voting_power().raw(), 400);
    }

    #[test]
    fn staged_corporation_bootstraps_board_and_applies_incorporator_overrides() {
        let tmp = TempDir::new().unwrap();
        let layout = RepoLayout::new(tmp.path().to_path_buf());
        let workspace_id = WorkspaceId::new();

        let entity = create_pending_entity(
            &layout,
            workspace_id,
            "Board Bootstrap Corp".to_string(),
            EntityType::CCorp,
            Jurisdiction::new("US-DE").unwrap(),
        )
        .unwrap();
        let entity_id = entity.entity_id();

        let mut founder = alice();
        founder.role = Some(MemberRole::Officer);
        founder.officer_title = Some(OfficerTitle::Cto);
        founder.is_incorporator = Some(true);
        founder.address = None;
        founder.share_count = Some(6_000_000);
        founder.shares_purchased = Some(6_000_000);
        add_pending_member(&layout, workspace_id, entity_id, founder).unwrap();

        let (formation, _) = finalize_formation_with_profile_overrides(
            &layout,
            workspace_id,
            entity_id,
            Some(10_000_000),
            Some("0.0001"),
            None,
            None,
            Some("Taylor Incorporator".to_string()),
            Some("1 Incorporator Way, Wilmington, DE 19801".to_string()),
            FormationProfileOverrides {
                company_address: Some(CompanyAddress {
                    street: "500 Market St".to_string(),
                    city: "Wilmington".to_string(),
                    county: None,
                    state: "DE".to_string(),
                    zip: "19801".to_string(),
                }),
                ..FormationProfileOverrides::default()
            },
        )
        .unwrap();

        assert_eq!(formation.entity.jurisdiction().as_str(), "US-DE");
        let repo =
            crate::git::repo::CorpRepo::open(&layout.entity_repo_path(workspace_id, entity_id))
                .unwrap();
        let profile: GovernanceProfile = repo
            .read_json("main", GOVERNANCE_PROFILE_PATH)
            .expect("governance profile should exist");
        assert_eq!(profile.incorporator_name(), Some("Taylor Incorporator"));
        assert_eq!(
            profile.incorporator_address(),
            Some("1 Incorporator Way, Wilmington, DE 19801")
        );
        assert_eq!(
            profile
                .officers()
                .first()
                .map(|officer| officer.title.as_str()),
            Some("Chief Technology Officer")
        );

        let bodies = governance_bodies(&repo);
        assert_eq!(bodies.len(), 1);
        assert_eq!(bodies[0].body_type(), BodyType::BoardOfDirectors);
        assert_eq!(bodies[0].quorum_rule(), QuorumThreshold::Majority);
        assert_eq!(bodies[0].voting_method(), VotingMethod::PerCapita);

        let seats = governance_seats(&repo);
        assert_eq!(seats.len(), 1);
        assert_eq!(seats[0].body_id(), bodies[0].body_id());
        assert_eq!(seats[0].voting_power().raw(), 1);
    }

    #[test]
    fn add_pending_member_rejects_duplicate_email_and_overallocated_ownership() {
        let tmp = TempDir::new().unwrap();
        let layout = RepoLayout::new(tmp.path().to_path_buf());
        let workspace_id = WorkspaceId::new();

        let entity = create_pending_entity(
            &layout,
            workspace_id,
            "Validation LLC".to_string(),
            EntityType::Llc,
            Jurisdiction::new("US-WY").unwrap(),
        )
        .unwrap();
        let entity_id = entity.entity_id();

        add_pending_member(&layout, workspace_id, entity_id, alice()).unwrap();

        let mut duplicate_email = bob();
        duplicate_email.email = Some("ALICE@example.com".to_string());
        let duplicate_result =
            add_pending_member(&layout, workspace_id, entity_id, duplicate_email);
        assert!(
            duplicate_result
                .unwrap_err()
                .to_string()
                .contains("duplicate member email")
        );

        let mut overallocated = bob();
        overallocated.email = Some("charlie@example.com".to_string());
        overallocated.ownership_pct = Some(50.0);
        let ownership_result = add_pending_member(&layout, workspace_id, entity_id, overallocated);
        assert!(
            ownership_result
                .unwrap_err()
                .to_string()
                .contains("total ownership_pct cannot exceed 100")
        );
    }

    #[test]
    fn retire_incompatible_governance_deactivates_old_body_and_seats() {
        let tmp = TempDir::new().unwrap();
        let layout = RepoLayout::new(tmp.path().to_path_buf());
        let workspace_id = WorkspaceId::new();

        let entity = create_pending_entity(
            &layout,
            workspace_id,
            "Converted LLC".to_string(),
            EntityType::Llc,
            Jurisdiction::new("US-WY").unwrap(),
        )
        .unwrap();
        let entity_id = entity.entity_id();
        let store = EntityStore::open(&layout, workspace_id, entity_id).unwrap();

        let body = GovernanceBody::new(
            GovernanceBodyId::new(),
            entity_id,
            BodyType::BoardOfDirectors,
            "Legacy Board".to_string(),
            QuorumThreshold::Majority,
            VotingMethod::PerCapita,
        )
        .unwrap();
        let seat = GovernanceSeat::new(
            GovernanceSeatId::new(),
            body.body_id(),
            ContactId::new(),
            SeatRole::Member,
            None,
            None,
            Some(VotingPower::new(1).unwrap()),
        )
        .unwrap();
        store
            .commit(
                "main",
                "Add legacy governance",
                vec![
                    FileWrite::json(format!("governance/bodies/{}.json", body.body_id()), &body)
                        .unwrap(),
                    FileWrite::json(format!("governance/seats/{}.json", seat.seat_id()), &seat)
                        .unwrap(),
                ],
            )
            .unwrap();

        retire_incompatible_governance_for_entity(&store, &entity).unwrap();

        let retired_body = store
            .read::<GovernanceBody>("main", body.body_id())
            .unwrap();
        let retired_seat = store
            .read::<GovernanceSeat>("main", seat.seat_id())
            .unwrap();
        assert_eq!(retired_body.status(), BodyStatus::Inactive);
        assert_eq!(retired_seat.status(), SeatStatus::Expired);
    }
}
