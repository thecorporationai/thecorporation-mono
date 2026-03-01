//! Governance AST loader and typed schema for policy evaluation.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use crate::domain::execution::types::AuthorityTier;

use super::capability::GovernanceCapability;

// ── AST-specific typed wrappers ───────────────────────────────────────

/// Authority tier as represented in the governance AST (integer 1/2/3).
///
/// The core `AuthorityTier` serializes as `"tier_1"` etc. for wire format
/// compatibility with stored intents. This wrapper deserializes from the
/// plain integers used in the AST JSON.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AstAuthorityTier(AuthorityTier);

impl AstAuthorityTier {
    pub fn into_inner(self) -> AuthorityTier {
        self.0
    }
}

impl From<AstAuthorityTier> for AuthorityTier {
    fn from(t: AstAuthorityTier) -> Self {
        t.0
    }
}

impl Serialize for AstAuthorityTier {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_u8(self.0.level())
    }
}

impl<'de> Deserialize<'de> for AstAuthorityTier {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let n = u8::deserialize(deserializer)?;
        AuthorityTier::from_level(n)
            .map(AstAuthorityTier)
            .ok_or_else(|| {
                serde::de::Error::custom(format!(
                    "invalid authority tier: {n}, expected 1, 2, or 3"
                ))
            })
    }
}

/// Lane check operators supported in the governance AST.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LaneCheckOp {
    Eq,
    Neq,
    Lte,
    Gte,
    ContainsNone,
    ContainsAny,
}

/// Escalation condition keys supported in the governance AST.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EscalationCondition {
    TemplateApprovedFalse,
    RestrictedModificationsPresent,
    IsReversibleFalse,
}

/// Supported metadata field paths for lane checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LaneField {
    #[serde(rename = "templateApproved")]
    TemplateApproved,
    #[serde(rename = "modifications")]
    Modifications,
    #[serde(rename = "context.rateIncreasePercent")]
    ContextRateIncreasePercent,
    #[serde(rename = "context.priceIncreasePercent")]
    ContextPriceIncreasePercent,
    #[serde(rename = "context.premiumIncreasePercent")]
    ContextPremiumIncreasePercent,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum LaneScalarValue {
    Bool(bool),
    Number(f64),
    String(String),
    Null,
}

/// Governance clause types from the AST document structure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GovernanceClauseType {
    Threshold,
    ApprovalRequirement,
    Prohibition,
    AttestationRequirement,
}

// ── AST schema types ──────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct GovernanceAstV1 {
    pub version: String,
    pub entity_types: Vec<String>,
    pub documents: Vec<GovernanceDocument>,
    pub rules: GovernanceRules,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GovernanceDocument {
    pub id: String,
    pub path: String,
    pub title: String,
    pub sections: Vec<GovernanceSection>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GovernanceSection {
    pub id: String,
    pub heading: String,
    pub clauses: Vec<GovernanceClause>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GovernanceClause {
    pub id: String,
    #[serde(rename = "type")]
    pub clause_type: GovernanceClauseType,
    pub text: String,
    #[serde(default)]
    pub citations: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GovernanceRules {
    pub tier_defaults: HashMap<GovernanceCapability, AstAuthorityTier>,
    pub non_delegable: Vec<GovernanceCapability>,
    pub escalation: Vec<EscalationRule>,
    pub lane_conditions: Vec<LaneConditionRule>,
    pub approval: ApprovalRule,
    pub mode: ModeRule,
    pub reauth: ReauthRule,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EscalationRule {
    pub id: String,
    #[serde(default)]
    pub applies: Vec<GovernanceCapability>,
    pub condition: Option<EscalationCondition>,
    pub escalate_to: AstAuthorityTier,
    pub reason: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LaneConditionRule {
    pub lane_id: String,
    pub capability: GovernanceCapability,
    pub checks: Vec<LaneCheck>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum LaneCheck {
    Eq {
        field: LaneField,
        value: LaneScalarValue,
        message: String,
    },
    Neq {
        field: LaneField,
        value: LaneScalarValue,
        message: String,
    },
    Lte {
        field: LaneField,
        value: f64,
        message: String,
    },
    Gte {
        field: LaneField,
        value: f64,
        message: String,
    },
    ContainsNone {
        field: LaneField,
        value: Vec<String>,
        message: String,
    },
    ContainsAny {
        field: LaneField,
        value: Vec<String>,
        message: String,
    },
}

impl LaneCheck {
    pub fn message(&self) -> &str {
        match self {
            Self::Eq { message, .. }
            | Self::Neq { message, .. }
            | Self::Lte { message, .. }
            | Self::Gte { message, .. }
            | Self::ContainsNone { message, .. }
            | Self::ContainsAny { message, .. } => message,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApprovalRule {
    pub expiry_days: u32,
    pub silence_is_approval: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ModeRule {
    pub principal_unavailable_requires_reversible_tier1: bool,
    pub incident_lockdown_blocks_all: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReauthRule {
    pub reduced_limits_at_days: u32,
    pub reduced_limits_percent: u32,
    pub full_suspension_at_days: u32,
}

// ── AST validation ────────────────────────────────────────────────────

impl GovernanceAstV1 {
    /// Validate cross-field invariants that the type system alone cannot enforce.
    ///
    /// Returns a list of errors. An empty list means the AST is valid.
    pub fn validate(&self) -> Vec<String> {
        let mut errors = Vec::new();
        let rules = &self.rules;

        // 1. Non-delegable capabilities must be Tier 3.
        for cap in &rules.non_delegable {
            if let Some(tier) = rules.tier_defaults.get(cap) {
                if tier.into_inner() != AuthorityTier::Tier3 {
                    errors.push(format!(
                        "non_delegable capability {cap} must be Tier 3 but is Tier {}",
                        tier.into_inner().level()
                    ));
                }
            } else {
                errors.push(format!(
                    "non_delegable capability {cap} not found in tier_defaults"
                ));
            }
        }

        // 2. Escalation rule `applies` must reference known capabilities.
        for rule in &rules.escalation {
            for cap in &rule.applies {
                if !rules.tier_defaults.contains_key(cap) {
                    errors.push(format!(
                        "escalation rule '{}' applies to {cap} which is not in tier_defaults",
                        rule.id
                    ));
                }
            }
        }

        // 3. Lane conditions must reference known capabilities.
        for lane in &rules.lane_conditions {
            if !rules.tier_defaults.contains_key(&lane.capability) {
                errors.push(format!(
                    "lane '{}' references capability {} not in tier_defaults",
                    lane.lane_id, lane.capability
                ));
            }
        }

        // 4. Reauth timing: reduced limits must come before full suspension.
        if rules.reauth.reduced_limits_at_days >= rules.reauth.full_suspension_at_days {
            errors.push(format!(
                "reauth.reduced_limits_at_days ({}) must be < full_suspension_at_days ({})",
                rules.reauth.reduced_limits_at_days, rules.reauth.full_suspension_at_days
            ));
        }

        // 5. No duplicate escalation rule IDs.
        let mut seen_esc = HashSet::new();
        for rule in &rules.escalation {
            if !seen_esc.insert(&rule.id) {
                errors.push(format!("duplicate escalation rule id: {}", rule.id));
            }
        }

        // 6. No duplicate lane IDs.
        let mut seen_lane = HashSet::new();
        for lane in &rules.lane_conditions {
            if !seen_lane.insert(&lane.lane_id) {
                errors.push(format!("duplicate lane id: {}", lane.lane_id));
            }
        }

        // 7. Silence must never be approval.
        if rules.approval.silence_is_approval {
            errors.push("silence_is_approval must be false".to_owned());
        }

        // 8. Lane checks must use field/operator combinations that are type-safe.
        for lane in &rules.lane_conditions {
            for check in &lane.checks {
                let valid = match check {
                    LaneCheck::Eq { field, .. } | LaneCheck::Neq { field, .. } => {
                        matches!(field, LaneField::TemplateApproved)
                    }
                    LaneCheck::Lte { field, .. } | LaneCheck::Gte { field, .. } => matches!(
                        field,
                        LaneField::ContextRateIncreasePercent
                            | LaneField::ContextPriceIncreasePercent
                            | LaneField::ContextPremiumIncreasePercent
                    ),
                    LaneCheck::ContainsNone { field, .. } | LaneCheck::ContainsAny { field, .. } => {
                        matches!(field, LaneField::Modifications)
                    }
                };
                if !valid {
                    errors.push(format!(
                        "lane '{}' has invalid field/operator pairing for check {:?}",
                        lane.lane_id, check
                    ));
                }
            }
        }

        // 9. Document/section/clause IDs must be unique across the AST.
        let mut seen_doc_ids = HashSet::new();
        let mut seen_section_ids = HashSet::new();
        let mut seen_clause_ids = HashSet::new();
        for doc in &self.documents {
            if !seen_doc_ids.insert(&doc.id) {
                errors.push(format!("duplicate document id: {}", doc.id));
            }
            for section in &doc.sections {
                if !seen_section_ids.insert(&section.id) {
                    errors.push(format!("duplicate section id: {}", section.id));
                }
                for clause in &section.clauses {
                    if !seen_clause_ids.insert(&clause.id) {
                        errors.push(format!("duplicate clause id: {}", clause.id));
                    }
                }
            }
        }

        errors
    }
}

// ── AST loader ────────────────────────────────────────────────────────

const AST_JSON: &str = include_str!("../../../../../governance/ast/v1/governance-ast.json");

static AST: OnceLock<GovernanceAstV1> = OnceLock::new();

pub fn default_governance_ast() -> &'static GovernanceAstV1 {
    AST.get_or_init(|| {
        let ast: GovernanceAstV1 = serde_json::from_str(AST_JSON)
            .expect("governance AST JSON is invalid; fix governance/ast/v1/governance-ast.json");
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_and_validates_default_ast() {
        let ast = default_governance_ast();
        assert_eq!(ast.version, "1.0.0");
        assert!(ast
            .rules
            .tier_defaults
            .contains_key(&GovernanceCapability::NewContract));
        assert!(ast
            .rules
            .non_delegable
            .contains(&GovernanceCapability::IssueEquity));
        // Validation runs during load — reaching here means no invariant violations.
    }

    #[test]
    fn validation_catches_non_delegable_tier_mismatch() {
        let mut ast: GovernanceAstV1 = serde_json::from_str(AST_JSON).unwrap();
        // Demote a non-delegable capability to Tier 1 — this is a contradiction.
        ast.rules.tier_defaults.insert(
            GovernanceCapability::IssueEquity,
            AstAuthorityTier(AuthorityTier::Tier1),
        );
        let errors = ast.validate();
        assert!(
            errors.iter().any(|e| e.contains("non_delegable") && e.contains("issue_equity")),
            "expected non-delegable tier mismatch error, got: {errors:?}"
        );
    }

    #[test]
    fn validation_catches_bad_reauth_ordering() {
        let mut ast: GovernanceAstV1 = serde_json::from_str(AST_JSON).unwrap();
        ast.rules.reauth.reduced_limits_at_days = 100;
        ast.rules.reauth.full_suspension_at_days = 50;
        let errors = ast.validate();
        assert!(
            errors.iter().any(|e| e.contains("reduced_limits_at_days")),
            "expected reauth ordering error, got: {errors:?}"
        );
    }

    #[test]
    fn validation_catches_silence_is_approval() {
        let mut ast: GovernanceAstV1 = serde_json::from_str(AST_JSON).unwrap();
        ast.rules.approval.silence_is_approval = true;
        let errors = ast.validate();
        assert!(
            errors.iter().any(|e| e.contains("silence_is_approval")),
            "expected silence_is_approval error, got: {errors:?}"
        );
    }

    #[test]
    fn validation_catches_invalid_lane_field_operator_pairing() {
        let mut ast_json: serde_json::Value = serde_json::from_str(AST_JSON).unwrap();
        ast_json["rules"]["lane_conditions"][0]["checks"][0]["field"] = json!("modifications");
        let ast: GovernanceAstV1 = serde_json::from_value(ast_json).unwrap();
        let errors = ast.validate();
        assert!(
            errors.iter().any(|e| e.contains("invalid field/operator pairing")),
            "expected lane field/op pairing error, got: {errors:?}"
        );
    }

    #[test]
    fn unknown_escalation_condition_fails_deserialization() {
        let mut ast_json: serde_json::Value = serde_json::from_str(AST_JSON).unwrap();
        ast_json["rules"]["escalation"][0]["condition"] = json!("unknown_condition");
        let err = serde_json::from_value::<GovernanceAstV1>(ast_json).unwrap_err();
        assert!(
            err.to_string().contains("unknown variant"),
            "expected unknown variant error, got: {err}"
        );
    }
}
