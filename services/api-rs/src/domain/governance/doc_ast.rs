//! Governance document AST v0 — typed schema, loader, and validation.
//!
//! The unified v0 AST encodes the full content of all governance documents as
//! typed `ContentNode` trees, plus rules, structured data, and disclaimer.
//! This module owns the deserialization types, a compiled-in loader (same
//! `include_str!` + `OnceLock` pattern as `policy_ast.rs`), and cross-field
//! validation.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

// ── Top-level AST ────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct GovernanceDocAst {
    pub version: String,
    pub entity_types: HashMap<EntityTypeKey, EntityTypeConfig>,
    pub spending_defaults: SpendingDefaults,
    pub compliance: HashMap<EntityTypeKey, ComplianceConfig>,
    pub authority_precedence: Vec<AuthorityPrecedenceEntry>,
    pub documents: Vec<DocumentDefinition>,
    #[serde(default)]
    pub disclaimer: Option<Disclaimer>,
    #[serde(default)]
    pub rules: Option<serde_json::Value>,
    #[serde(default)]
    pub structured_data: Option<StructuredData>,
}

// ── Entity types ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityTypeKey {
    Corporation,
    Llc,
}

impl EntityTypeKey {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Corporation => "corporation",
            Self::Llc => "llc",
        }
    }
}

impl std::fmt::Display for EntityTypeKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct EntityTypeConfig {
    pub jurisdiction: String,
    pub governing_statute: String,
    pub governing_statute_full: String,
    pub filing_authority: String,
    pub governance_body: String,
    pub governing_document: String,
    pub charter_document: String,
    pub approval_authority: String,
    pub human_manager_title: String,
    pub human_manager_role: String,
}

// ── Spending defaults ────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct SpendingDefaults {
    pub parameter_set: String,
    pub categories: Vec<SpendingCategory>,
    pub per_vendor_annual_cap_cents: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SpendingCategory {
    pub id: String,
    pub label: String,
    pub per_transaction_cents: i64,
    pub monthly_aggregate_cents: i64,
}

// ── Compliance ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct ComplianceConfig {
    #[serde(default)]
    pub franchise_tax_deadline: Option<String>,
    #[serde(default)]
    pub franchise_tax_methods: Option<Vec<FranchiseTaxMethod>>,
    #[serde(default)]
    pub annual_report_fee_cents: Option<i64>,
    #[serde(default)]
    pub annual_report_deadline: Option<String>,
    #[serde(default)]
    pub filing_fee_cents: Option<i64>,
    #[serde(default)]
    pub state_income_tax: Option<bool>,
    #[serde(default)]
    pub franchise_tax: Option<bool>,
    #[serde(default)]
    pub name_reservation_fee_cents: Option<i64>,
    #[serde(default)]
    pub name_reservation_validity_days: Option<u32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FranchiseTaxMethod {
    pub id: String,
    pub label: String,
    pub minimum_cents: i64,
}

// ── Authority precedence ─────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct AuthorityPrecedenceEntry {
    pub rank: u32,
    pub source: String,
    pub label: String,
}

// ── Document definition ──────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct DocumentDefinition {
    pub id: String,
    pub category: DocumentCategory,
    pub entity_scope: EntityScope,
    pub path: String,
    pub title: String,
    #[serde(default)]
    pub preamble: Option<String>,
    #[serde(default)]
    pub metadata_fields: Vec<MetadataField>,
    pub content: Vec<ContentNode>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocumentCategory {
    Common,
    Compliance,
    Corporation,
    Llc,
    Transactions,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityScope {
    Common,
    Corporation,
    Llc,
    Both,
}

impl EntityScope {
    /// Whether a document with this scope should be included for the given entity type.
    pub fn matches(self, entity_type: EntityTypeKey) -> bool {
        match self {
            Self::Common | Self::Both => true,
            Self::Corporation => entity_type == EntityTypeKey::Corporation,
            Self::Llc => entity_type == EntityTypeKey::Llc,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct MetadataField {
    pub key: String,
    pub label: String,
    #[serde(default)]
    pub placeholder: Option<String>,
    #[serde(default)]
    pub default: Option<String>,
}

// ── Content nodes ────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentNode {
    Heading {
        level: u8,
        text: String,
    },
    Paragraph {
        text: String,
    },
    OrderedList {
        items: Vec<String>,
    },
    UnorderedList {
        items: Vec<String>,
    },
    Table {
        headers: Vec<String>,
        rows: Vec<Vec<String>>,
    },
    DataTable {
        source: String,
        columns: Vec<DataTableColumn>,
    },
    Conditional {
        when_entity: EntityTypeKey,
        content: Vec<ContentNode>,
    },
    SignatureBlock {
        role: String,
        fields: Vec<String>,
    },
    Placeholder {
        key: String,
        label: String,
    },
    Note {
        text: String,
    },
    CodeBlock {
        #[serde(default)]
        language: Option<String>,
        lines: Vec<String>,
    },
    DocumentRef {
        document_id: String,
        text: String,
    },
    HorizontalRule,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DataTableColumn {
    pub key: String,
    pub header: String,
    #[serde(default)]
    pub format: Option<String>,
}

// ── Disclaimer ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct Disclaimer {
    pub text: String,
    #[serde(default)]
    pub must_accept_before_use: bool,
    #[serde(default)]
    pub key_points: Vec<String>,
}

// ── Structured data ─────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct StructuredData {
    #[serde(default)]
    pub autonomy_lanes: Vec<AutonomyLane>,
    #[serde(default)]
    pub emergency_modes: Vec<EmergencyMode>,
    #[serde(default)]
    pub auto_suspension_triggers: Vec<AutoSuspensionTrigger>,
    #[serde(default)]
    pub credential_custody: Vec<CredentialCustody>,
    #[serde(default)]
    pub adjustment_rules: Vec<AdjustmentRule>,
    #[serde(default)]
    pub approval_validity: Option<ApprovalValidity>,
    #[serde(default)]
    pub retention_schedule: Vec<RetentionRecord>,
    #[serde(default)]
    pub severity_classification: Vec<SeverityLevel>,
    #[serde(default)]
    pub report_schedule: Vec<ReportDefinition>,
    #[serde(default)]
    pub change_control_rules: Vec<ChangeControlRule>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AutonomyLane {
    pub id: String,
    pub category: String,
    pub label: String,
    pub tier: u8,
    pub authority_action: String,
    #[serde(default)]
    pub conditions: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EmergencyMode {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub tier1_allowed: bool,
    #[serde(default)]
    pub tier2_allowed: bool,
    #[serde(default)]
    pub reversible_only: bool,
    #[serde(default)]
    pub activated_by: serde_json::Value,
    #[serde(default)]
    pub deactivated_by: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApprovalValidity {
    pub required_elements: Vec<ApprovalRequirement>,
    #[serde(default)]
    pub negative_consent: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApprovalRequirement {
    pub id: String,
    pub label: String,
    pub rule: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AutoSuspensionTrigger {
    pub id: String,
    pub label: String,
    pub description: String,
    pub severity: String,
    pub activates_mode: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CredentialCustody {
    pub id: String,
    pub label: String,
    pub custodian: String,
    pub agent_access: String,
    #[serde(default)]
    pub agent_access_details: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AdjustmentRule {
    pub id: String,
    pub action: String,
    pub target: String,
    #[serde(default)]
    pub scope: Option<String>,
    pub requires_board_resolution: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RetentionRecord {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub retention_years: Option<u32>,
    #[serde(default)]
    pub permanent: bool,
    #[serde(default)]
    pub retention_from: Option<String>,
    #[serde(default)]
    pub governing_requirement: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SeverityLevel {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub response_sla_hours: Option<u32>,
    #[serde(default)]
    pub auto_lockdown: bool,
    #[serde(default)]
    pub suspend_affected: bool,
    #[serde(default)]
    pub examples: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReportDefinition {
    pub id: String,
    pub label: String,
    pub frequency: String,
    pub authority_to_produce: u8,
    #[serde(default)]
    pub authority_to_change: Option<u8>,
    #[serde(default)]
    pub lookahead_days: Option<u32>,
    #[serde(default)]
    pub content_keys: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChangeControlRule {
    pub id: String,
    pub label: String,
    pub tier: u8,
    #[serde(default)]
    pub requires_impact_assessment: bool,
    #[serde(default)]
    pub requires_governance_amendment: bool,
}

// ── Validation ───────────────────────────────────────────────────────

impl GovernanceDocAst {
    /// Validate cross-field invariants. Returns a list of errors (empty = valid).
    pub fn validate(&self) -> Vec<String> {
        let mut errors = Vec::new();

        // 1. Version check.
        if self.version != "0.1.0" {
            errors.push(format!(
                "expected version \"0.1.0\", got \"{}\"",
                self.version
            ));
        }

        // 2. Both entity types must be present.
        if !self.entity_types.contains_key(&EntityTypeKey::Corporation) {
            errors.push("missing entity type config for \"corporation\"".to_owned());
        }
        if !self.entity_types.contains_key(&EntityTypeKey::Llc) {
            errors.push("missing entity type config for \"llc\"".to_owned());
        }

        // 3. Spending categories: positive amounts.
        for cat in &self.spending_defaults.categories {
            if cat.per_transaction_cents <= 0 {
                errors.push(format!(
                    "spending category '{}': per_transaction_cents must be positive",
                    cat.id
                ));
            }
            if cat.monthly_aggregate_cents <= 0 {
                errors.push(format!(
                    "spending category '{}': monthly_aggregate_cents must be positive",
                    cat.id
                ));
            }
        }
        if self.spending_defaults.per_vendor_annual_cap_cents <= 0 {
            errors.push("per_vendor_annual_cap_cents must be positive".to_owned());
        }

        // 4. Authority precedence: ranks must be monotonically increasing.
        for i in 1..self.authority_precedence.len() {
            if self.authority_precedence[i].rank <= self.authority_precedence[i - 1].rank {
                errors.push(format!(
                    "authority_precedence: rank {} is not greater than previous rank {}",
                    self.authority_precedence[i].rank,
                    self.authority_precedence[i - 1].rank
                ));
            }
        }

        // 5. Unique document IDs and paths.
        let mut seen_ids = HashSet::new();
        let mut seen_paths = HashSet::new();
        for doc in &self.documents {
            if !seen_ids.insert(&doc.id) {
                errors.push(format!("duplicate document id: {}", doc.id));
            }
            if !seen_paths.insert(&doc.path) {
                errors.push(format!("duplicate document path: {}", doc.path));
            }
        }

        // 6. All DocumentRef targets must reference existing document IDs.
        let doc_ids: HashSet<&str> = self.documents.iter().map(|d| d.id.as_str()).collect();
        for doc in &self.documents {
            Self::validate_content_refs(&doc.content, &doc_ids, &doc.id, &mut errors);
        }

        errors
    }

    fn validate_content_refs(
        nodes: &[ContentNode],
        doc_ids: &HashSet<&str>,
        parent_doc_id: &str,
        errors: &mut Vec<String>,
    ) {
        for node in nodes {
            match node {
                ContentNode::DocumentRef { document_id, .. } => {
                    if !doc_ids.contains(document_id.as_str()) {
                        errors.push(format!(
                            "document '{}': DocumentRef target '{}' not found in AST",
                            parent_doc_id, document_id
                        ));
                    }
                }
                ContentNode::Conditional { content, .. } => {
                    Self::validate_content_refs(content, doc_ids, parent_doc_id, errors);
                }
                _ => {}
            }
        }
    }
}

// ── AST loader ───────────────────────────────────────────────────────

const DOC_AST_JSON: &str =
    include_str!("../../../../../governance/ast/governance-ast.json");

static DOC_AST: OnceLock<GovernanceDocAst> = OnceLock::new();

pub fn default_doc_ast() -> &'static GovernanceDocAst {
    DOC_AST.get_or_init(|| {
        let ast: GovernanceDocAst = serde_json::from_str(DOC_AST_JSON).expect(
            "governance AST JSON is invalid; fix governance/ast/governance-ast.json",
        );
        let errors = ast.validate();
        if !errors.is_empty() {
            panic!(
                "governance AST validation failed ({} errors):\n  {}",
                errors.len(),
                errors.join("\n  ")
            );
        }
        ast
    })
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn doc_ast_parses_and_validates() {
        let ast = default_doc_ast();
        assert_eq!(ast.version, "0.1.0");
        assert!(!ast.documents.is_empty());
    }

    #[test]
    fn doc_ast_spending_categories() {
        let ast = default_doc_ast();
        assert_eq!(ast.spending_defaults.categories.len(), 5);
        let recurring = ast
            .spending_defaults
            .categories
            .iter()
            .find(|c| c.id == "recurring_obligations")
            .expect("recurring_obligations category");
        assert_eq!(recurring.per_transaction_cents, 1_000_000);
        assert_eq!(recurring.monthly_aggregate_cents, 4_000_000);

        let supplies = ast
            .spending_defaults
            .categories
            .iter()
            .find(|c| c.id == "office_supplies")
            .expect("office_supplies category");
        assert_eq!(supplies.per_transaction_cents, 50_000);
    }

    #[test]
    fn doc_ast_entity_types_complete() {
        let ast = default_doc_ast();
        let corp = ast
            .entity_types
            .get(&EntityTypeKey::Corporation)
            .expect("corporation config");
        assert_eq!(corp.jurisdiction, "Delaware");
        assert_eq!(corp.governing_statute, "DGCL");

        let llc = ast
            .entity_types
            .get(&EntityTypeKey::Llc)
            .expect("llc config");
        assert_eq!(llc.jurisdiction, "Wyoming");
    }

    #[test]
    fn doc_ast_authority_precedence_monotonic() {
        let ast = default_doc_ast();
        assert_eq!(ast.authority_precedence.len(), 8);
        for i in 1..ast.authority_precedence.len() {
            assert!(
                ast.authority_precedence[i].rank > ast.authority_precedence[i - 1].rank,
                "rank {} must be > rank {}",
                ast.authority_precedence[i].rank,
                ast.authority_precedence[i - 1].rank
            );
        }
    }

    #[test]
    fn doc_ast_common_documents_present() {
        let ast = default_doc_ast();
        let ids: Vec<&str> = ast.documents.iter().map(|d| d.id.as_str()).collect();
        assert!(ids.contains(&"agent_delegation_schedule"));
        assert!(ids.contains(&"assumptions_and_decisions"));
        assert!(ids.contains(&"signing_and_records_standard"));
        assert!(ids.contains(&"agent_operator_service_agreement_template"));
        assert!(ids.contains(&"agent_operator_service_agreement_checklist"));
    }

    #[test]
    fn doc_ast_entity_scope_matches() {
        assert!(EntityScope::Common.matches(EntityTypeKey::Corporation));
        assert!(EntityScope::Common.matches(EntityTypeKey::Llc));
        assert!(EntityScope::Both.matches(EntityTypeKey::Corporation));
        assert!(EntityScope::Both.matches(EntityTypeKey::Llc));
        assert!(EntityScope::Corporation.matches(EntityTypeKey::Corporation));
        assert!(!EntityScope::Corporation.matches(EntityTypeKey::Llc));
        assert!(EntityScope::Llc.matches(EntityTypeKey::Llc));
        assert!(!EntityScope::Llc.matches(EntityTypeKey::Corporation));
    }

    #[test]
    fn doc_ast_validation_catches_duplicate_ids() {
        let mut ast: GovernanceDocAst = serde_json::from_str(DOC_AST_JSON).unwrap();
        // Duplicate a document.
        let dup = ast.documents[0].clone();
        ast.documents.push(dup);
        let errors = ast.validate();
        assert!(
            errors.iter().any(|e| e.contains("duplicate document id")),
            "expected duplicate id error, got: {errors:?}"
        );
    }

    #[test]
    fn doc_ast_validation_catches_bad_spending() {
        let mut ast: GovernanceDocAst = serde_json::from_str(DOC_AST_JSON).unwrap();
        ast.spending_defaults.categories[0].per_transaction_cents = -100;
        let errors = ast.validate();
        assert!(
            errors
                .iter()
                .any(|e| e.contains("per_transaction_cents must be positive")),
            "expected spending error, got: {errors:?}"
        );
    }

    #[test]
    fn doc_ast_compliance_configs() {
        let ast = default_doc_ast();
        let corp = ast
            .compliance
            .get(&EntityTypeKey::Corporation)
            .expect("corp compliance");
        assert_eq!(corp.franchise_tax_deadline.as_deref(), Some("March 1"));
        assert!(corp.franchise_tax_methods.is_some());

        let llc = ast
            .compliance
            .get(&EntityTypeKey::Llc)
            .expect("llc compliance");
        assert_eq!(llc.state_income_tax, Some(false));
        assert_eq!(llc.franchise_tax, Some(false));
    }
}
