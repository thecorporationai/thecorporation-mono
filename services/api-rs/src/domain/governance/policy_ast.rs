//! Governance AST loader and typed schema for policy evaluation.

use serde::Deserialize;
use std::collections::HashMap;
use std::sync::OnceLock;

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
    pub clause_type: String,
    pub text: String,
    #[serde(default)]
    pub citations: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GovernanceRules {
    pub tier_defaults: HashMap<String, u8>,
    pub non_delegable: Vec<String>,
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
    pub applies: Vec<String>,
    pub condition: Option<String>,
    pub escalate_to: u8,
    pub reason: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LaneConditionRule {
    pub lane_id: String,
    pub capability: String,
    pub checks: Vec<LaneCheck>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LaneCheck {
    pub field: String,
    pub op: String,
    pub value: serde_json::Value,
    pub message: String,
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

const AST_JSON: &str = include_str!("../../../../../governance/ast/v1/governance-ast.json");

static AST: OnceLock<GovernanceAstV1> = OnceLock::new();

pub fn default_governance_ast() -> &'static GovernanceAstV1 {
    AST.get_or_init(|| {
        serde_json::from_str(AST_JSON)
            .expect("governance AST JSON is invalid; fix governance/ast/v1/governance-ast.json")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_default_ast() {
        let ast = default_governance_ast();
        assert_eq!(ast.version, "1.0.0");
        assert!(ast.rules.tier_defaults.contains_key("new_contract"));
        assert!(ast.rules.non_delegable.contains(&"issue_equity".to_owned()));
    }
}
