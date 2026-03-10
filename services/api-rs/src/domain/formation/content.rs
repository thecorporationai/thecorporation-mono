//! Document content generation for formation documents.
//!
//! Each generator produces a `serde_json::Value` representing the full
//! structured content of a legal document, including fields, filing fees,
//! and signature requirements.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::domain::formation::types::{DocumentType, EntityType};
use crate::domain::ids::{AgentId, EntityId};

// ── Input types ──────────────────────────────────────────────────────────

/// A member/founder as provided in the formation request.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct MemberInput {
    pub name: String,
    pub investor_type: InvestorType,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub agent_id: Option<AgentId>,
    #[serde(default)]
    pub entity_id: Option<EntityId>,
    #[serde(default)]
    pub ownership_pct: Option<f64>,
    #[serde(default)]
    pub membership_units: Option<i64>,
    #[serde(default)]
    pub share_count: Option<i64>,
    #[serde(default)]
    pub share_class: Option<String>,
    #[serde(default)]
    pub role: Option<MemberRole>,
    /// Mailing address of the member.
    #[serde(default)]
    pub address: Option<Address>,
    /// Officer title (CEO, CFO, Secretary, etc.) — corporations only.
    #[serde(default)]
    pub officer_title: Option<OfficerTitle>,
    /// Explicit number of shares being purchased at formation.
    #[serde(default)]
    pub shares_purchased: Option<i64>,
    /// Vesting schedule for the founder's shares.
    #[serde(default)]
    pub vesting: Option<VestingSchedule>,
    /// Description of IP being contributed to the company.
    #[serde(default)]
    pub ip_description: Option<String>,
    /// Whether this member is the sole incorporator (corporations only).
    #[serde(default)]
    pub is_incorporator: Option<bool>,
}

/// A mailing address.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Address {
    pub street: String,
    #[serde(default)]
    pub street2: Option<String>,
    pub city: String,
    pub state: String,
    pub zip: String,
}

/// Officer title for a corporate officer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum OfficerTitle {
    Ceo,
    Cfo,
    Cto,
    Coo,
    Secretary,
    Treasurer,
    President,
    Vp,
    Other,
}

/// Vesting schedule for founder shares.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct VestingSchedule {
    /// Total vesting period in months (e.g. 48).
    pub total_months: i32,
    /// Cliff period in months (e.g. 12).
    pub cliff_months: i32,
    /// Acceleration type: "single_trigger", "double_trigger", or none.
    #[serde(default)]
    pub acceleration: Option<String>,
}

/// Classification of a member/investor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum InvestorType {
    NaturalPerson,
    Agent,
    Entity,
}

/// Role a member holds in the entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum MemberRole {
    Director,
    Officer,
    Manager,
    Member,
    Chair,
}

/// A signature requirement embedded in document content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignatureRequirement {
    pub role: String,
    pub signer_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signer_email: Option<String>,
    pub required: bool,
}

// ── Helpers ──────────────────────────────────────────────────────────────

fn sig_req_to_json(req: &SignatureRequirement) -> Value {
    let mut v = json!({
        "role": req.role,
        "signer_name": req.signer_name,
        "required": req.required,
    });
    if let Some(ref email) = req.signer_email {
        v["signer_email"] = json!(email);
    }
    v
}

fn statutory_reference(jurisdiction: &str, entity_type: &str) -> String {
    match (jurisdiction, entity_type) {
        ("Delaware", "limited_liability_company") => {
            "Delaware Limited Liability Company Act, 6 Del. C. § 18-101 et seq.".to_string()
        }
        ("Delaware", "corporation") => {
            "Delaware General Corporation Law, 8 Del. C. § 101 et seq.".to_string()
        }
        (state, "limited_liability_company") => {
            format!("{state} Limited Liability Company Act")
        }
        (state, "corporation") => {
            format!("{state} Business Corporation Act")
        }
        (state, _) => format!("{state} Business Entity Act"),
    }
}

fn filing_office(jurisdiction: &str) -> String {
    match jurisdiction {
        "Delaware" => "Delaware Division of Corporations".to_string(),
        state => format!("{state} Secretary of State"),
    }
}

fn irs_entity_type(entity_type: EntityType) -> &'static str {
    match entity_type {
        EntityType::Llc => "LLC",
        EntityType::CCorp => "Corporation",
    }
}

// ── Content generators ───────────────────────────────────────────────────

/// Generate Articles of Organization content (LLC).
pub fn generate_articles_of_organization(
    entity_name: &str,
    jurisdiction: &str,
    registered_agent_name: &str,
    registered_agent_address: &str,
    organizer_member: &MemberInput,
) -> Value {
    let sig = SignatureRequirement {
        role: "organizer".to_string(),
        signer_name: organizer_member.name.clone(),
        signer_email: organizer_member.email.clone(),
        required: true,
    };

    json!({
        "document_type": "articles_of_organization",
        "jurisdiction": jurisdiction,
        "fields": {
            "legal_name": entity_name,
            "entity_type": "limited_liability_company",
            "state": jurisdiction,
            "filing_office": filing_office(jurisdiction),
            "registered_agent": {
                "name": registered_agent_name,
                "address": registered_agent_address,
            },
            "organizer": {
                "name": organizer_member.name,
                "email": organizer_member.email,
            },
            "purpose": "any lawful activity",
            "effective_date": null,
            "statutory_reference": statutory_reference(jurisdiction, "limited_liability_company"),
        },
        "filing_fee_cents": 9000,
        "signature_requirements": [sig_req_to_json(&sig)],
    })
}

/// Generate Articles of Incorporation content (Corporation).
pub fn generate_articles_of_incorporation(
    entity_name: &str,
    jurisdiction: &str,
    registered_agent_name: &str,
    registered_agent_address: &str,
    incorporator_member: &MemberInput,
    authorized_shares: i64,
    par_value: &str,
) -> Value {
    let sig = SignatureRequirement {
        role: "incorporator".to_string(),
        signer_name: incorporator_member.name.clone(),
        signer_email: incorporator_member.email.clone(),
        required: true,
    };

    json!({
        "document_type": "articles_of_incorporation",
        "jurisdiction": jurisdiction,
        "fields": {
            "legal_name": entity_name,
            "entity_type": "corporation",
            "state": jurisdiction,
            "filing_office": filing_office(jurisdiction),
            "registered_agent": {
                "name": registered_agent_name,
                "address": registered_agent_address,
            },
            "incorporator": {
                "name": incorporator_member.name,
                "email": incorporator_member.email,
            },
            "authorized_shares": authorized_shares,
            "par_value": par_value,
            "share_classes": [
                {
                    "class_code": "COMMON",
                    "stock_type": "common",
                    "authorized_shares": authorized_shares,
                    "par_value": par_value,
                }
            ],
            "purpose": "any lawful activity",
            "effective_date": null,
            "statutory_reference": statutory_reference(jurisdiction, "corporation"),
        },
        "filing_fee_cents": 8900,
        "signature_requirements": [sig_req_to_json(&sig)],
    })
}

/// Generate Operating Agreement content (LLC).
pub fn generate_operating_agreement(
    entity_name: &str,
    jurisdiction: &str,
    members: &[MemberInput],
) -> Value {
    let has_manager = members.iter().any(|m| m.role == Some(MemberRole::Manager));
    let management_structure = if has_manager {
        "manager-managed"
    } else {
        "member-managed"
    };

    let total_units: i64 = members.iter().filter_map(|m| m.membership_units).sum();

    let members_json: Vec<Value> = members
        .iter()
        .map(|m| {
            json!({
                "name": m.name,
                "investor_type": m.investor_type,
                "email": m.email,
                "ownership_pct": m.ownership_pct,
                "membership_units": m.membership_units,
                "role": m.role,
            })
        })
        .collect();

    // Signature requirements:
    // - natural_person members: sign as their role, required
    // - entity members: sign as "officer", required
    // - agent members: NOT required
    let sig_reqs: Vec<Value> = members
        .iter()
        .filter(|m| m.investor_type != InvestorType::Agent)
        .map(|m| {
            let role = match m.investor_type {
                InvestorType::Entity => "officer".to_string(),
                _ => m
                    .role
                    .map(|r| serde_json::to_value(r).unwrap_or(json!("member")))
                    .and_then(|v| v.as_str().map(String::from))
                    .unwrap_or_else(|| "member".to_string()),
            };
            sig_req_to_json(&SignatureRequirement {
                role,
                signer_name: m.name.clone(),
                signer_email: m.email.clone(),
                required: true,
            })
        })
        .collect();

    json!({
        "document_type": "operating_agreement",
        "jurisdiction": jurisdiction,
        "fields": {
            "legal_name": entity_name,
            "entity_type": "limited_liability_company",
            "state": jurisdiction,
            "formation_date": null,
            "management_structure": management_structure,
            "total_units": total_units,
            "members": members_json,
            "distributions": "pro_rata_by_units",
            "dissolution_provisions": "majority_vote_or_judicial_dissolution",
            "governing_law": jurisdiction,
            "statutory_reference": statutory_reference(jurisdiction, "limited_liability_company"),
        },
        "filing_fee_cents": 0,
        "signature_requirements": sig_reqs,
    })
}

/// Generate Bylaws content (Corporation).
pub fn generate_bylaws(
    entity_name: &str,
    jurisdiction: &str,
    members: &[MemberInput],
    authorized_shares: i64,
    par_value: &str,
) -> Value {
    let directors: Vec<Value> = members
        .iter()
        .filter(|m| m.role == Some(MemberRole::Director) || m.role == Some(MemberRole::Chair))
        .map(|m| {
            json!({
                "name": m.name,
                "email": m.email,
                "role": m.role,
            })
        })
        .collect();

    let officers: Vec<Value> = members
        .iter()
        .filter(|m| m.role == Some(MemberRole::Officer))
        .map(|m| {
            json!({
                "name": m.name,
                "email": m.email,
                "role": m.role,
            })
        })
        .collect();

    // Only natural_person directors sign
    let sig_reqs: Vec<Value> = members
        .iter()
        .filter(|m| {
            m.investor_type == InvestorType::NaturalPerson
                && (m.role == Some(MemberRole::Director) || m.role == Some(MemberRole::Chair))
        })
        .map(|m| {
            sig_req_to_json(&SignatureRequirement {
                role: "director".to_string(),
                signer_name: m.name.clone(),
                signer_email: m.email.clone(),
                required: true,
            })
        })
        .collect();

    json!({
        "document_type": "bylaws",
        "jurisdiction": jurisdiction,
        "fields": {
            "legal_name": entity_name,
            "entity_type": "corporation",
            "state": jurisdiction,
            "authorized_shares": authorized_shares,
            "par_value": par_value,
            "fiscal_year_end": "12-31",
            "directors": directors,
            "officers": officers,
            "board_quorum": "majority",
            "shareholder_quorum": "majority",
            "annual_meeting": "as_set_by_board",
            "governing_law": jurisdiction,
            "statutory_reference": statutory_reference(jurisdiction, "corporation"),
        },
        "filing_fee_cents": 0,
        "signature_requirements": sig_reqs,
    })
}

/// Generate SS-4 Application content (EIN application for IRS).
pub fn generate_ss4_application(
    entity_name: &str,
    entity_type: EntityType,
    jurisdiction: &str,
    responsible_party_member: &MemberInput,
) -> Value {
    let sig = SignatureRequirement {
        role: "responsible_party".to_string(),
        signer_name: responsible_party_member.name.clone(),
        signer_email: responsible_party_member.email.clone(),
        required: true,
    };

    json!({
        "document_type": "ss4_application",
        "jurisdiction": jurisdiction,
        "fields": {
            "legal_name": entity_name,
            "trade_name": null,
            "entity_type_irs": irs_entity_type(entity_type),
            "responsible_party": {
                "name": responsible_party_member.name,
                "email": responsible_party_member.email,
            },
            "business_address": {
                "street": "To be provided",
                "city": "To be provided",
                "state": jurisdiction,
                "zip": "To be provided",
            },
            "formation_date": null,
            "formation_jurisdiction": jurisdiction,
            "reason": "started_new_business",
            "principal_activity": "other",
            "expected_employees": 0,
            "first_wage_date": null,
            "closing_month": "December",
        },
        "filing_fee_cents": 0,
        "signature_requirements": [sig_req_to_json(&sig)],
    })
}

// ── Batch generator ──────────────────────────────────────────────────────

/// Generate all formation documents for a new entity.
///
/// Returns `(document_type, title, governance_tag, content)` tuples.
///
/// For LLC: articles of organization + operating agreement.
/// For Corp: articles of incorporation + bylaws.
/// Both also get an SS-4 application (EIN) if there is a responsible party.
#[allow(clippy::too_many_arguments)]
pub fn generate_formation_documents(
    entity_type: EntityType,
    legal_name: &str,
    jurisdiction: &str,
    registered_agent_name: &str,
    registered_agent_address: &str,
    members: &[MemberInput],
    authorized_shares: Option<i64>,
    par_value: Option<&str>,
) -> Vec<(DocumentType, String, Option<String>, Value)> {
    let mut docs = Vec::new();

    // Pick the first natural-person member as the organizer/incorporator/responsible party.
    let primary_member = members
        .iter()
        .find(|m| m.investor_type == InvestorType::NaturalPerson)
        .or_else(|| members.first());

    match entity_type {
        EntityType::Llc => {
            if let Some(organizer) = primary_member {
                let articles = generate_articles_of_organization(
                    legal_name,
                    jurisdiction,
                    registered_agent_name,
                    registered_agent_address,
                    organizer,
                );
                docs.push((
                    DocumentType::ArticlesOfOrganization,
                    format!("Articles of Organization — {legal_name}"),
                    Some("formation".to_string()),
                    articles,
                ));
            }

            let oa = generate_operating_agreement(legal_name, jurisdiction, members);
            docs.push((
                DocumentType::OperatingAgreement,
                format!("Operating Agreement — {legal_name}"),
                Some("operating_agreement".to_string()),
                oa,
            ));
        }
        EntityType::CCorp => {
            let shares = authorized_shares.unwrap_or(10_000_000);
            let pv = par_value.unwrap_or("0.0001");

            if let Some(incorporator) = primary_member {
                let articles = generate_articles_of_incorporation(
                    legal_name,
                    jurisdiction,
                    registered_agent_name,
                    registered_agent_address,
                    incorporator,
                    shares,
                    pv,
                );
                docs.push((
                    DocumentType::ArticlesOfIncorporation,
                    format!("Articles of Incorporation — {legal_name}"),
                    Some("formation".to_string()),
                    articles,
                ));
            }

            let bylaws = generate_bylaws(legal_name, jurisdiction, members, shares, pv);
            docs.push((
                DocumentType::Bylaws,
                format!("Bylaws — {legal_name}"),
                Some("bylaws".to_string()),
                bylaws,
            ));
        }
    }

    docs
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

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

    fn agent_member() -> MemberInput {
        MemberInput {
            name: "Agent-007".to_string(),
            investor_type: InvestorType::Agent,
            email: None,
            agent_id: Some(AgentId::new()),
            entity_id: None,
            ownership_pct: Some(0.0),
            membership_units: Some(0),
            share_count: None,
            share_class: None,
            role: Some(MemberRole::Member),
            address: None,
            officer_title: None,
            shares_purchased: None,
            vesting: None,
            ip_description: None,
            is_incorporator: None,
        }
    }

    fn entity_member() -> MemberInput {
        MemberInput {
            name: "Acme Holdings LLC".to_string(),
            investor_type: InvestorType::Entity,
            email: Some("legal@acme.example.com".to_string()),
            agent_id: None,
            entity_id: Some(EntityId::new()),
            ownership_pct: Some(10.0),
            membership_units: Some(100),
            share_count: None,
            share_class: None,
            role: Some(MemberRole::Member),
            address: None,
            officer_title: None,
            shares_purchased: None,
            vesting: None,
            ip_description: None,
            is_incorporator: None,
        }
    }

    #[test]
    fn articles_of_organization_structure() {
        let content = generate_articles_of_organization(
            "Test LLC",
            "Delaware",
            "Registered Agents Inc.",
            "123 Main St, Dover, DE 19901",
            &alice(),
        );

        assert_eq!(content["document_type"], "articles_of_organization");
        assert_eq!(content["jurisdiction"], "Delaware");
        assert_eq!(content["filing_fee_cents"], 9000);
        assert_eq!(content["fields"]["legal_name"], "Test LLC");
        assert_eq!(
            content["fields"]["entity_type"],
            "limited_liability_company"
        );
        assert_eq!(content["fields"]["purpose"], "any lawful activity");
        assert!(content["fields"]["effective_date"].is_null());
        assert_eq!(
            content["fields"]["registered_agent"]["name"],
            "Registered Agents Inc."
        );
        assert_eq!(content["fields"]["organizer"]["name"], "Alice Smith");

        let sigs = content["signature_requirements"].as_array().unwrap();
        assert_eq!(sigs.len(), 1);
        assert_eq!(sigs[0]["role"], "organizer");
        assert_eq!(sigs[0]["required"], true);
    }

    #[test]
    fn articles_of_incorporation_structure() {
        let content = generate_articles_of_incorporation(
            "Test Corp",
            "Delaware",
            "Registered Agents Inc.",
            "123 Main St, Dover, DE 19901",
            &alice(),
            10_000_000,
            "0.0001",
        );

        assert_eq!(content["document_type"], "articles_of_incorporation");
        assert_eq!(content["filing_fee_cents"], 8900);
        assert_eq!(content["fields"]["authorized_shares"], 10_000_000);
        assert_eq!(content["fields"]["par_value"], "0.0001");

        let classes = content["fields"]["share_classes"].as_array().unwrap();
        assert_eq!(classes.len(), 1);
        assert_eq!(classes[0]["class_code"], "COMMON");
        assert_eq!(classes[0]["stock_type"], "common");

        let sigs = content["signature_requirements"].as_array().unwrap();
        assert_eq!(sigs.len(), 1);
        assert_eq!(sigs[0]["role"], "incorporator");
    }

    #[test]
    fn operating_agreement_manager_managed() {
        let members = vec![alice(), bob()];
        let content = generate_operating_agreement("Test LLC", "Delaware", &members);

        assert_eq!(content["document_type"], "operating_agreement");
        assert_eq!(content["filing_fee_cents"], 0);
        assert_eq!(content["fields"]["management_structure"], "manager-managed");
        assert_eq!(content["fields"]["total_units"], 1000);
        assert_eq!(content["fields"]["distributions"], "pro_rata_by_units");

        let members_arr = content["fields"]["members"].as_array().unwrap();
        assert_eq!(members_arr.len(), 2);

        // Both natural persons should have signature requirements
        let sigs = content["signature_requirements"].as_array().unwrap();
        assert_eq!(sigs.len(), 2);
    }

    #[test]
    fn operating_agreement_member_managed() {
        let mut m = alice();
        m.role = Some(MemberRole::Member);
        let content = generate_operating_agreement("Test LLC", "Delaware", &[m, bob()]);
        assert_eq!(content["fields"]["management_structure"], "member-managed");
    }

    #[test]
    fn operating_agreement_agent_not_required_signer() {
        let members = vec![alice(), agent_member()];
        let content = generate_operating_agreement("Test LLC", "Delaware", &members);

        let sigs = content["signature_requirements"].as_array().unwrap();
        // Agent should be excluded
        assert_eq!(sigs.len(), 1);
        assert_eq!(sigs[0]["signer_name"], "Alice Smith");
    }

    #[test]
    fn operating_agreement_entity_signs_as_officer() {
        let members = vec![alice(), entity_member()];
        let content = generate_operating_agreement("Test LLC", "Delaware", &members);

        let sigs = content["signature_requirements"].as_array().unwrap();
        assert_eq!(sigs.len(), 2);

        let entity_sig = sigs
            .iter()
            .find(|s| s["signer_name"] == "Acme Holdings LLC")
            .unwrap();
        assert_eq!(entity_sig["role"], "officer");
        assert_eq!(entity_sig["required"], true);
    }

    #[test]
    fn bylaws_structure() {
        let mut alice = alice();
        alice.role = Some(MemberRole::Director);
        let mut bob = bob();
        bob.role = Some(MemberRole::Officer);

        let content = generate_bylaws("Test Corp", "Delaware", &[alice, bob], 10_000_000, "0.0001");

        assert_eq!(content["document_type"], "bylaws");
        assert_eq!(content["filing_fee_cents"], 0);
        assert_eq!(content["fields"]["fiscal_year_end"], "12-31");
        assert_eq!(content["fields"]["board_quorum"], "majority");
        assert_eq!(content["fields"]["annual_meeting"], "as_set_by_board");

        let directors = content["fields"]["directors"].as_array().unwrap();
        assert_eq!(directors.len(), 1);
        assert_eq!(directors[0]["name"], "Alice Smith");

        let officers = content["fields"]["officers"].as_array().unwrap();
        assert_eq!(officers.len(), 1);
        assert_eq!(officers[0]["name"], "Bob Jones");

        // Only natural person directors sign
        let sigs = content["signature_requirements"].as_array().unwrap();
        assert_eq!(sigs.len(), 1);
        assert_eq!(sigs[0]["role"], "director");
    }

    #[test]
    fn ss4_application_structure() {
        let content = generate_ss4_application("Test LLC", EntityType::Llc, "Delaware", &alice());

        assert_eq!(content["document_type"], "ss4_application");
        assert_eq!(content["filing_fee_cents"], 0);
        assert_eq!(content["fields"]["entity_type_irs"], "LLC");
        assert_eq!(
            content["fields"]["responsible_party"]["name"],
            "Alice Smith"
        );
        assert_eq!(content["fields"]["reason"], "started_new_business");
        assert_eq!(content["fields"]["expected_employees"], 0);

        let sigs = content["signature_requirements"].as_array().unwrap();
        assert_eq!(sigs.len(), 1);
        assert_eq!(sigs[0]["role"], "responsible_party");
    }

    #[test]
    fn ss4_application_corp() {
        let content =
            generate_ss4_application("Test Corp", EntityType::CCorp, "Delaware", &alice());
        assert_eq!(content["fields"]["entity_type_irs"], "Corporation");
    }

    #[test]
    fn generate_formation_docs_llc() {
        let members = vec![alice(), bob()];
        let docs = generate_formation_documents(
            EntityType::Llc,
            "Test LLC",
            "Delaware",
            "RA Inc.",
            "123 Main St",
            &members,
            None,
            None,
        );

        assert_eq!(docs.len(), 2);
        assert_eq!(docs[0].0, DocumentType::ArticlesOfOrganization);
        assert_eq!(docs[1].0, DocumentType::OperatingAgreement);
        assert!(docs[0].1.contains("Articles of Organization"));
        assert!(docs[1].1.contains("Operating Agreement"));
        assert_eq!(docs[0].2, Some("formation".to_string()));
        assert_eq!(docs[1].2, Some("operating_agreement".to_string()));
    }

    #[test]
    fn generate_formation_docs_corp() {
        let mut alice = alice();
        alice.role = Some(MemberRole::Director);
        let docs = generate_formation_documents(
            EntityType::CCorp,
            "Test Corp",
            "Delaware",
            "RA Inc.",
            "123 Main St",
            &[alice],
            Some(10_000_000),
            Some("0.0001"),
        );

        assert_eq!(docs.len(), 2);
        assert_eq!(docs[0].0, DocumentType::ArticlesOfIncorporation);
        assert_eq!(docs[1].0, DocumentType::Bylaws);
    }

    #[test]
    fn generate_formation_docs_corp_defaults() {
        let mut alice = alice();
        alice.role = Some(MemberRole::Director);
        let docs = generate_formation_documents(
            EntityType::CCorp,
            "Test Corp",
            "Delaware",
            "RA Inc.",
            "123 Main St",
            &[alice],
            None,
            None,
        );

        // Should use default shares/par_value
        let articles_content = &docs[0].3;
        assert_eq!(articles_content["fields"]["authorized_shares"], 10_000_000);
        assert_eq!(articles_content["fields"]["par_value"], "0.0001");
    }

    #[test]
    fn statutory_reference_delaware() {
        let content =
            generate_articles_of_organization("Test LLC", "Delaware", "RA", "123 Main", &alice());
        let ref_str = content["fields"]["statutory_reference"].as_str().unwrap();
        assert!(ref_str.contains("Delaware Limited Liability Company Act"));
        assert!(ref_str.contains("6 Del. C."));
    }

    #[test]
    fn statutory_reference_other_state() {
        let content =
            generate_articles_of_organization("Test LLC", "Wyoming", "RA", "123 Main", &alice());
        let ref_str = content["fields"]["statutory_reference"].as_str().unwrap();
        assert!(ref_str.contains("Wyoming"));
    }

    #[test]
    fn investor_type_serde() {
        let it = InvestorType::NaturalPerson;
        let json = serde_json::to_string(&it).unwrap();
        assert_eq!(json, "\"natural_person\"");

        let parsed: InvestorType = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, it);
    }

    #[test]
    fn member_role_serde() {
        let r = MemberRole::Chair;
        let json = serde_json::to_string(&r).unwrap();
        assert_eq!(json, "\"chair\"");

        let parsed: MemberRole = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, r);
    }
}
