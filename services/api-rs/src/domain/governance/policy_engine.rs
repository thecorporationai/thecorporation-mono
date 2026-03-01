//! Rust policy evaluator backed by the JSON governance AST.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::domain::execution::types::AuthorityTier;

use super::capability::GovernanceCapability;
use super::policy_ast::{EscalationRule, LaneCheck, default_governance_ast};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuthoritySource {
    Law,
    Charter,
    GovernanceDocs,
    Resolution,
    Directive,
    StandingInstruction,
    DelegationSchedule,
    Heuristic,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyPrecedenceTrace {
    pub source: AuthoritySource,
    pub outcome: String,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyConflict {
    pub higher_source: AuthoritySource,
    pub lower_source: AuthoritySource,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyDecision {
    pub tier: AuthorityTier,
    pub policy_mapped: bool,
    pub allowed: bool,
    pub requires_approval: bool,
    pub blockers: Vec<String>,
    pub escalation_reasons: Vec<String>,
    pub clause_refs: Vec<String>,
    #[serde(default)]
    pub precedence_trace: Vec<PolicyPrecedenceTrace>,
    #[serde(default)]
    pub precedence_conflicts: Vec<PolicyConflict>,
    #[serde(default)]
    pub effective_source: Option<AuthoritySource>,
}

/// Canonicalize incoming intent type text for policy evaluation.
///
/// Known governance capabilities are normalized through the typed enum.
/// Unknown values are trimmed and passed through for controlled Tier 2 fallback.
pub fn canonicalize_intent_type(intent_type: &str) -> String {
    let trimmed = intent_type.trim();
    trimmed
        .parse::<GovernanceCapability>()
        .map(|capability| capability.as_str().to_owned())
        .unwrap_or_else(|_| trimmed.to_owned())
}

pub fn evaluate_intent(intent_type: &str, metadata: &Value) -> PolicyDecision {
    let canonical_intent_type = canonicalize_intent_type(intent_type);
    let ast = default_governance_ast();
    let mut blockers = Vec::new();
    let mut reasons = Vec::new();
    let mut clause_refs = vec!["delegation.authority_tiers".to_owned()];

    let policy_mapped = ast
        .rules
        .tier_defaults
        .contains_key(canonical_intent_type.as_str());
    let default_tier_int = ast
        .rules
        .tier_defaults
        .get(canonical_intent_type.as_str())
        .copied()
        .unwrap_or(2);

    if !policy_mapped {
        reasons.push(format!(
            "Unknown capability \"{canonical_intent_type}\" defaulted to Tier 2 pending explicit policy mapping"
        ));
        clause_refs.push("delegation.authority_tiers.tier2".to_owned());
    }

    let mut tier_int = default_tier_int;

    if ast
        .rules
        .non_delegable
        .iter()
        .any(|c| c == canonical_intent_type.as_str())
    {
        tier_int = 3;
        blockers.push(format!(
            "\"{canonical_intent_type}\" is non-delegable and requires Principal/Board authority"
        ));
        clause_refs.push("delegation.authority_tiers.tier3".to_owned());
    }

    for rule in &ast.rules.escalation {
        if escalation_applies(rule, canonical_intent_type.as_str(), metadata)
            && rule.escalate_to > tier_int
        {
            tier_int = rule.escalate_to;
            reasons.push(rule.reason.clone());
            clause_refs.push(format!("rule.escalation.{}", rule.id));
        }
    }

    for lane in ast
        .rules
        .lane_conditions
        .iter()
        .filter(|l| l.capability == canonical_intent_type.as_str())
    {
        for check in &lane.checks {
            if !check_lane(metadata, check) {
                if tier_int < 2 {
                    tier_int = 2;
                }
                reasons.push(format!(
                    "Lane conditions failed ({}): {}",
                    lane.lane_id, check.message
                ));
                clause_refs.push(format!("rule.lane.{}", lane.lane_id));
            }
        }
    }

    let tier = match tier_int {
        1 => AuthorityTier::Tier1,
        2 => AuthorityTier::Tier2,
        _ => AuthorityTier::Tier3,
    };
    let requires_approval = !matches!(tier, AuthorityTier::Tier1);

    PolicyDecision {
        tier,
        policy_mapped,
        allowed: blockers.is_empty(),
        requires_approval,
        blockers,
        escalation_reasons: reasons,
        clause_refs,
        precedence_trace: vec![PolicyPrecedenceTrace {
            source: AuthoritySource::Heuristic,
            outcome: "allow".to_owned(),
            reason: Some("base policy AST evaluation".to_owned()),
        }],
        precedence_conflicts: Vec::new(),
        effective_source: Some(AuthoritySource::Heuristic),
    }
}

pub fn supported_escalation_conditions() -> &'static [&'static str] {
    &[
        "template_approved_false",
        "restricted_modifications_present",
        "is_reversible_false",
    ]
}

fn escalation_applies(rule: &EscalationRule, intent_type: &str, metadata: &Value) -> bool {
    if !rule.applies.is_empty() && !rule.applies.iter().any(|c| c == intent_type) {
        return false;
    }

    match rule.condition.as_deref() {
        Some("template_approved_false") => metadata
            .get("templateApproved")
            .and_then(Value::as_bool)
            .is_some_and(|v| !v),
        Some("restricted_modifications_present") => {
            let mods = metadata
                .get("modifications")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            let restricted = ["indemnification", "governing_law", "ip_assignment"];
            mods.iter().any(|m| {
                m.as_str()
                    .is_some_and(|s| restricted.iter().any(|r| r == &s))
            })
        }
        Some("is_reversible_false") => metadata
            .get("isReversible")
            .and_then(Value::as_bool)
            .is_some_and(|v| !v),
        Some(_) => false,
        None => false,
    }
}

fn check_lane(metadata: &Value, check: &LaneCheck) -> bool {
    let got = get_path(metadata, &check.field);
    match check.op.as_str() {
        "eq" => got == Some(&check.value),
        "neq" => got != Some(&check.value),
        "lte" => got
            .and_then(Value::as_f64)
            .zip(check.value.as_f64())
            .is_some_and(|(g, c)| g <= c),
        "gte" => got
            .and_then(Value::as_f64)
            .zip(check.value.as_f64())
            .is_some_and(|(g, c)| g >= c),
        "contains_none" => {
            let arr = got.and_then(Value::as_array).cloned().unwrap_or_default();
            let banned = check
                .value
                .as_array()
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .filter_map(|v| v.as_str().map(ToOwned::to_owned))
                .collect::<Vec<_>>();
            !arr.iter()
                .any(|v| v.as_str().is_some_and(|s| banned.iter().any(|b| b == s)))
        }
        "contains_any" => {
            let arr = got.and_then(Value::as_array).cloned().unwrap_or_default();
            let allowed = check
                .value
                .as_array()
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .filter_map(|v| v.as_str().map(ToOwned::to_owned))
                .collect::<Vec<_>>();
            arr.iter()
                .any(|v| v.as_str().is_some_and(|s| allowed.iter().any(|a| a == s)))
        }
        _ => true,
    }
}

fn get_path<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
    let mut current = value;
    for part in path.split('.') {
        current = current.get(part)?;
    }
    Some(current)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use serde_json::json;

    #[test]
    fn canonicalize_known_capability_from_trimmed_input() {
        let canonical = canonicalize_intent_type("  equity.round.accept  ");
        assert_eq!(canonical, "equity.round.accept");
    }

    #[test]
    fn non_delegable_is_blocked() {
        let d = evaluate_intent("issue_equity", &json!({}));
        assert!(matches!(d.tier, AuthorityTier::Tier3));
        assert!(!d.allowed);
        assert!(!d.blockers.is_empty());
    }

    #[test]
    fn lane_condition_failure_escalates() {
        let d = evaluate_intent(
            "pay_recurring_obligation",
            &json!({"context": {"priceIncreasePercent": 15}}),
        );
        assert!(matches!(d.tier, AuthorityTier::Tier2));
        assert!(
            d.escalation_reasons
                .iter()
                .any(|r| r.contains("Lane conditions failed"))
        );
    }

    #[test]
    fn approved_template_stays_tier1() {
        let d = evaluate_intent(
            "execute_standard_form_agreement",
            &json!({"templateApproved": true}),
        );
        assert!(matches!(d.tier, AuthorityTier::Tier1));
    }

    #[test]
    fn unknown_capability_defaults_to_tier2_without_blocking() {
        let d = evaluate_intent("totally.unknown.intent", &json!({}));
        assert!(matches!(d.tier, AuthorityTier::Tier2));
        assert!(!d.policy_mapped);
        assert!(d.allowed);
    }

    #[test]
    fn missing_field_fails_lane_check() {
        // pay_recurring_obligation has a lane check on context.priceIncreasePercent <= 10
        // Omitting the field entirely should fail the check (escalate), not pass.
        let d = evaluate_intent("pay_recurring_obligation", &json!({}));
        assert!(
            matches!(d.tier, AuthorityTier::Tier2),
            "missing metadata field should escalate tier"
        );
        assert!(
            d.escalation_reasons
                .iter()
                .any(|r| r.contains("Lane conditions failed")),
            "should report lane condition failure"
        );
    }

    #[test]
    fn ast_escalation_conditions_are_supported() {
        let ast = default_governance_ast();
        let supported: BTreeSet<&str> = supported_escalation_conditions().iter().copied().collect();
        for rule in &ast.rules.escalation {
            let condition = rule
                .condition
                .as_deref()
                .expect("AST escalation rules must have a condition");
            assert!(
                supported.contains(condition),
                "unsupported AST escalation condition: {condition} (rule id: {})",
                rule.id
            );
        }
    }
}
