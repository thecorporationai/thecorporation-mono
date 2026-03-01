//! Rust policy evaluator backed by the JSON governance AST.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::domain::execution::types::AuthorityTier;

use super::capability::GovernanceCapability;
use super::delegation_schedule::DelegationSchedule;
use super::mode::GovernanceMode;
use super::policy_ast::{
    EscalationCondition, EscalationRule, LaneCheck, LaneField, LaneScalarValue,
    default_governance_ast,
};
use super::proof_obligations::enforce_proof_obligations;
use super::typed_intent::{ParsedGovernanceMetadata, TypedIntent};

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
    let typed_intent = TypedIntent::parse(intent_type, metadata);
    evaluate_intent_typed(&typed_intent)
}

pub fn evaluate_intent_typed(intent: &TypedIntent<'_>) -> PolicyDecision {
    let canonical_intent_type = intent.canonical_intent_type().to_owned();
    let parsed_cap = intent.capability();
    let metadata = intent.raw_metadata();
    let parsed_metadata = intent.metadata();
    let ast = default_governance_ast();
    let mut blockers = Vec::new();
    let mut reasons = Vec::new();
    let mut clause_refs = vec!["delegation.authority_tiers".to_owned()];

    let policy_mapped = parsed_cap
        .is_some_and(|cap| ast.rules.tier_defaults.contains_key(&cap));
    let default_tier = parsed_cap
        .and_then(|cap| ast.rules.tier_defaults.get(&cap))
        .map(|t| t.into_inner())
        .unwrap_or(AuthorityTier::Tier2);

    if !policy_mapped {
        reasons.push(format!(
            "Unknown capability \"{canonical_intent_type}\" defaulted to Tier 2 pending explicit policy mapping"
        ));
        clause_refs.push("delegation.authority_tiers.tier2".to_owned());
    }

    let mut tier = default_tier;

    if parsed_cap.is_some_and(|cap| ast.rules.non_delegable.contains(&cap)) {
        tier = AuthorityTier::Tier3;
        blockers.push(format!(
            "\"{canonical_intent_type}\" is non-delegable and requires Principal/Board authority"
        ));
        clause_refs.push("delegation.authority_tiers.tier3".to_owned());
    }

    for rule in &ast.rules.escalation {
        let escalation_tier = rule.escalate_to.into_inner();
        if escalation_applies(rule, parsed_cap, parsed_metadata, metadata) && escalation_tier > tier
        {
            tier = escalation_tier;
            reasons.push(rule.reason.clone());
            clause_refs.push(format!("rule.escalation.{}", rule.id));
        }
    }

    let requested_lane_id = parsed_metadata.lane_id.as_deref();

    let lane_conditions = ast
        .rules
        .lane_conditions
        .iter()
        .filter(|l| parsed_cap.is_some_and(|cap| l.capability == cap))
        .collect::<Vec<_>>();

    if let Some(lane_id) = requested_lane_id {
        if !lane_conditions.is_empty()
            && !lane_conditions.iter().any(|lane| lane.lane_id == lane_id)
        {
            if tier < AuthorityTier::Tier2 {
                tier = AuthorityTier::Tier2;
            }
            reasons.push(format!(
                "Unknown laneId \"{lane_id}\" for capability \"{canonical_intent_type}\""
            ));
            clause_refs.push("rule.lane.invalid_lane_id".to_owned());
        }
    }

    for lane in lane_conditions
        .into_iter()
        .filter(|l| requested_lane_id.is_none_or(|id| id == l.lane_id))
    {
        for check in &lane.checks {
            if !check_lane(metadata, check) {
                if tier < AuthorityTier::Tier2 {
                    tier = AuthorityTier::Tier2;
                }
                reasons.push(format!(
                    "Lane conditions failed ({}): {}",
                    lane.lane_id,
                    check.message()
                ));
                clause_refs.push(format!("rule.lane.{}", lane.lane_id));
            }
        }
    }

    if !parsed_metadata.decode_issues.is_empty() {
        if tier < AuthorityTier::Tier2 {
            tier = AuthorityTier::Tier2;
        }
        reasons.push(format!(
            "Metadata decoding issues detected: {}",
            parsed_metadata.decode_issues.join("; ")
        ));
        clause_refs.push("rule.metadata.decode_failure".to_owned());
    }

    let requires_approval = tier > AuthorityTier::Tier1;

    let mut decision = PolicyDecision {
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
    };

    enforce_proof_obligations(&mut decision);
    decision
}

pub fn supported_escalation_conditions() -> &'static [EscalationCondition] {
    &[
        EscalationCondition::TemplateApprovedFalse,
        EscalationCondition::RestrictedModificationsPresent,
        EscalationCondition::IsReversibleFalse,
    ]
}

fn escalation_applies(
    rule: &EscalationRule,
    parsed_cap: Option<GovernanceCapability>,
    parsed_metadata: &ParsedGovernanceMetadata,
    metadata: &Value,
) -> bool {
    if !rule.applies.is_empty() {
        match parsed_cap {
            Some(cap) if rule.applies.contains(&cap) => {}
            _ => return false,
        }
    }

    match rule.condition {
        Some(EscalationCondition::TemplateApprovedFalse) => {
            parsed_metadata.template_approved.is_some_and(|v| !v)
        }
        Some(EscalationCondition::RestrictedModificationsPresent) => {
            let mods = if parsed_metadata.modifications.is_empty() {
                normalized_string_list(metadata.get("modifications"))
            } else {
                parsed_metadata.modifications.clone()
            };
            let restricted = ["indemnification", "governing_law", "ip_assignment"];
            mods.iter()
                .any(|m| restricted.iter().any(|restricted| restricted == m))
        }
        Some(EscalationCondition::IsReversibleFalse) => {
            parsed_metadata.is_reversible.is_some_and(|v| !v)
        }
        None => false,
    }
}

fn check_lane(metadata: &Value, check: &LaneCheck) -> bool {
    match check {
        LaneCheck::Eq { field, value, .. } => {
            let got = get_field(metadata, *field);
            got == scalar_value_as_json(value).as_ref()
        }
        LaneCheck::Neq { field, value, .. } => {
            let got = get_field(metadata, *field);
            got != scalar_value_as_json(value).as_ref()
        }
        LaneCheck::Lte { field, value, .. } => get_field(metadata, *field)
            .and_then(Value::as_f64)
            .is_some_and(|g| g <= *value),
        LaneCheck::Gte { field, value, .. } => get_field(metadata, *field)
            .and_then(Value::as_f64)
            .is_some_and(|g| g >= *value),
        LaneCheck::ContainsNone { field, value, .. } => {
            let got = get_field(metadata, *field);
            let arr = normalized_string_list(got);
            let banned = value
                .iter()
                .map(|s| s.trim().to_ascii_lowercase())
                .collect::<Vec<_>>();
            !arr.iter().any(|item| banned.iter().any(|blocked| blocked == item))
        }
        LaneCheck::ContainsAny { field, value, .. } => {
            let got = get_field(metadata, *field);
            let arr = normalized_string_list(got);
            let required = value
                .iter()
                .map(|s| s.trim().to_ascii_lowercase())
                .collect::<Vec<_>>();
            arr.iter()
                .any(|item| required.iter().any(|expected| expected == item))
        }
    }
}

fn scalar_value_as_json(value: &LaneScalarValue) -> Option<Value> {
    match value {
        LaneScalarValue::Bool(v) => Some(Value::Bool(*v)),
        LaneScalarValue::Number(v) => serde_json::Number::from_f64(*v).map(Value::Number),
        LaneScalarValue::String(v) => Some(Value::String(v.clone())),
        LaneScalarValue::Null => Some(Value::Null),
    }
}

fn normalized_string_list(value: Option<&Value>) -> Vec<String> {
    match value {
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(Value::as_str)
            .map(|s| s.trim().to_ascii_lowercase())
            .collect(),
        Some(Value::String(s)) => vec![s.trim().to_ascii_lowercase()],
        _ => Vec::new(),
    }
}

fn get_field(value: &Value, field: LaneField) -> Option<&Value> {
    match field {
        LaneField::TemplateApproved => value.get("templateApproved"),
        LaneField::Modifications => value.get("modifications"),
        LaneField::ContextRateIncreasePercent => value
            .get("context")
            .and_then(|v| v.get("rateIncreasePercent")),
        LaneField::ContextPriceIncreasePercent => value
            .get("context")
            .and_then(|v| v.get("priceIncreasePercent")),
        LaneField::ContextPremiumIncreasePercent => value
            .get("context")
            .and_then(|v| v.get("premiumIncreasePercent")),
    }
}

// ── Full evaluation with mode, schedule, and service agreement ────────

/// Full context for a complete governance policy evaluation.
pub struct PolicyEvaluationContext<'a> {
    pub intent_type: &'a str,
    pub metadata: &'a Value,
    pub mode: GovernanceMode,
    pub schedule: &'a DelegationSchedule,
    pub now: DateTime<Utc>,
    pub entity_is_active: bool,
    pub service_agreement_executed: bool,
}

/// Run the full policy evaluation pipeline: AST rules + mode + schedule + service agreement.
pub fn evaluate_full(ctx: &PolicyEvaluationContext) -> PolicyDecision {
    let canonical = canonicalize_intent_type(ctx.intent_type);
    let decision = evaluate_intent(&canonical, ctx.metadata);
    let decision = apply_mode_overrides(decision, ctx.mode, &canonical, ctx.metadata);
    let decision = apply_schedule_overrides(
        decision,
        ctx.schedule,
        &canonical,
        ctx.metadata,
        ctx.now,
    );
    let decision = apply_service_agreement_overrides(
        decision,
        ctx.entity_is_active,
        ctx.service_agreement_executed,
    );
    let mut decision = apply_conflict_fail_closed(decision);
    enforce_proof_obligations(&mut decision);
    decision
}

/// Capabilities that remain operational during incident lockdown.
const LOCKDOWN_ALLOWLIST: &[&str] = &[
    "maintain_books_records",
    "prepare_compliance_docs",
    "compliance_deadline_tracking",
    "information_gathering",
    "routine_correspondence",
];

pub fn apply_mode_overrides(
    mut decision: PolicyDecision,
    mode: GovernanceMode,
    intent_type: &str,
    metadata: &Value,
) -> PolicyDecision {
    let was_allowed = decision.allowed;
    let initial_tier = decision.tier;
    match mode {
        GovernanceMode::Normal => {
            push_precedence_trace(
                &mut decision,
                AuthoritySource::Resolution,
                "allow",
                Some("normal governance mode".to_owned()),
            );
        }
        GovernanceMode::PrincipalUnavailable => {
            let reversible = metadata
                .get("isReversible")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            if decision.tier != AuthorityTier::Tier1 || !reversible {
                decision.allowed = false;
                decision.blockers.push(
                    "principal_unavailable mode only permits reversible tier_1 actions".to_owned(),
                );
                if was_allowed {
                    push_precedence_conflict(
                        &mut decision,
                        AuthoritySource::Resolution,
                        AuthoritySource::Heuristic,
                        "principal_unavailable overrides previously-allowed action".to_owned(),
                    );
                }
                push_precedence_trace(
                    &mut decision,
                    AuthoritySource::Resolution,
                    "block",
                    Some("principal_unavailable mode requires reversible tier_1".to_owned()),
                );
            } else {
                push_precedence_trace(
                    &mut decision,
                    AuthoritySource::Resolution,
                    "allow",
                    Some("principal_unavailable mode constraints satisfied".to_owned()),
                );
            }
            decision
                .clause_refs
                .push("rule.mode.principal_unavailable".to_owned());
        }
        GovernanceMode::IncidentLockdown => {
            if !LOCKDOWN_ALLOWLIST.contains(&intent_type) {
                decision.allowed = false;
                decision
                    .blockers
                    .push("incident_lockdown mode blocks this capability".to_owned());
                if was_allowed {
                    push_precedence_conflict(
                        &mut decision,
                        AuthoritySource::Resolution,
                        AuthoritySource::Heuristic,
                        "incident_lockdown overrides previously-allowed action".to_owned(),
                    );
                }
                push_precedence_trace(
                    &mut decision,
                    AuthoritySource::Resolution,
                    "block",
                    Some("incident_lockdown blocks non-allowlisted capabilities".to_owned()),
                );
            } else {
                push_precedence_trace(
                    &mut decision,
                    AuthoritySource::Resolution,
                    "allow",
                    Some("incident_lockdown allowlisted capability".to_owned()),
                );
            }
            decision
                .clause_refs
                .push("rule.mode.incident_lockdown".to_owned());
        }
    }
    if initial_tier != decision.tier {
        let updated_tier = decision.tier;
        push_precedence_trace(
            &mut decision,
            AuthoritySource::Resolution,
            "escalate",
            Some(format!(
                "mode adjusted tier from {initial_tier:?} to {updated_tier:?}"
            )),
        );
    }
    decision
}

pub fn apply_schedule_overrides(
    mut decision: PolicyDecision,
    schedule: &DelegationSchedule,
    intent_type: &str,
    metadata: &Value,
    now: DateTime<Utc>,
) -> PolicyDecision {
    if decision.tier != AuthorityTier::Tier1 {
        push_precedence_trace(
            &mut decision,
            AuthoritySource::DelegationSchedule,
            "allow",
            Some("schedule tier_1 checks skipped for non-tier_1 action".to_owned()),
        );
        return decision;
    }

    let was_allowed = decision.allowed;
    if schedule.is_reauth_suspended_at(now) {
        decision.allowed = false;
        decision
            .blockers
            .push("delegation schedule autonomy is suspended pending reauthorization".to_owned());
        decision
            .clause_refs
            .push("rule.reauth.full_suspension".to_owned());
        if was_allowed {
            push_precedence_conflict(
                &mut decision,
                AuthoritySource::DelegationSchedule,
                AuthoritySource::Heuristic,
                "reauthorization suspension overrides previously-allowed tier_1 action".to_owned(),
            );
        }
        push_precedence_trace(
            &mut decision,
            AuthoritySource::DelegationSchedule,
            "block",
            Some("delegation schedule full suspension active".to_owned()),
        );
        return decision;
    }

    if !schedule.allows_tier1_intent(intent_type) {
        decision.tier = AuthorityTier::Tier2;
        decision.requires_approval = true;
        decision.escalation_reasons.push(
            "intent is outside the tier_1 delegation lane and requires explicit approval"
                .to_owned(),
        );
        decision
            .clause_refs
            .push("delegation.schedule.tier1_lane".to_owned());
        push_precedence_conflict(
            &mut decision,
            AuthoritySource::DelegationSchedule,
            AuthoritySource::Heuristic,
            "tier_1 lane restriction escalated action to tier_2".to_owned(),
        );
        push_precedence_trace(
            &mut decision,
            AuthoritySource::DelegationSchedule,
            "escalate",
            Some("intent outside allowed tier_1 lane".to_owned()),
        );
    }

    if let Some(amount_cents) = amount_from_metadata_cents(metadata) {
        let effective_limit = schedule.effective_tier1_max_amount_cents(now);
        if amount_cents > effective_limit {
            decision.tier = AuthorityTier::Tier2;
            decision.requires_approval = true;
            decision.escalation_reasons.push(format!(
                "amount {amount_cents} exceeds current tier_1 limit {effective_limit}"
            ));
            decision
                .clause_refs
                .push("delegation.schedule.tier1_limit".to_owned());
            push_precedence_conflict(
                &mut decision,
                AuthoritySource::DelegationSchedule,
                AuthoritySource::Heuristic,
                "tier_1 spending limit exceeded".to_owned(),
            );
            push_precedence_trace(
                &mut decision,
                AuthoritySource::DelegationSchedule,
                "escalate",
                Some(format!(
                    "tier_1 amount limit exceeded: {amount_cents} > {effective_limit}"
                )),
            );
        }
    }

    if decision.allowed && decision.tier == AuthorityTier::Tier1 {
        push_precedence_trace(
            &mut decision,
            AuthoritySource::DelegationSchedule,
            "allow",
            Some("delegation schedule constraints satisfied".to_owned()),
        );
    }

    decision
}

pub fn apply_service_agreement_overrides(
    mut decision: PolicyDecision,
    entity_is_active: bool,
    service_agreement_executed: bool,
) -> PolicyDecision {
    let was_allowed = decision.allowed;
    if entity_is_active
        && decision.tier == AuthorityTier::Tier1
        && !service_agreement_executed
    {
        decision.allowed = false;
        decision.blockers.push(
            "active entities require an executed service agreement for tier_1 autonomy".to_owned(),
        );
        decision
            .clause_refs
            .push("rule.precondition.service_agreement".to_owned());
        if was_allowed {
            push_precedence_conflict(
                &mut decision,
                AuthoritySource::GovernanceDocs,
                AuthoritySource::Heuristic,
                "service agreement precondition overrides autonomous tier_1 execution".to_owned(),
            );
        }
        push_precedence_trace(
            &mut decision,
            AuthoritySource::GovernanceDocs,
            "block",
            Some("service agreement precondition not satisfied".to_owned()),
        );
    } else {
        push_precedence_trace(
            &mut decision,
            AuthoritySource::GovernanceDocs,
            "allow",
            Some("service agreement precondition satisfied or not applicable".to_owned()),
        );
    }
    decision
}

pub fn apply_conflict_fail_closed(mut decision: PolicyDecision) -> PolicyDecision {
    if !decision.precedence_conflicts.is_empty() {
        decision.allowed = false;
        decision.blockers.push(format!(
            "precedence conflict requires explicit human governance resolution ({} conflict(s))",
            decision.precedence_conflicts.len()
        ));
    }
    decision
}

/// Returns true if the intent maps to a known policy tier above Tier 1
/// (i.e., it requires manual approval artifacts).
pub fn mapped_tier_requires_manual_artifacts(decision: &PolicyDecision) -> bool {
    decision.policy_mapped && decision.tier != AuthorityTier::Tier1
}

pub fn amount_from_metadata_cents(metadata: &Value) -> Option<i64> {
    metadata
        .get("amount_cents")
        .and_then(Value::as_i64)
        .or_else(|| {
            metadata
                .get("amount")
                .and_then(Value::as_i64)
                .filter(|amount| *amount > 0 && *amount < i64::MAX / 100)
                .map(|dollars| dollars.saturating_mul(100))
        })
}

fn push_precedence_trace(
    decision: &mut PolicyDecision,
    source: AuthoritySource,
    outcome: &str,
    reason: Option<String>,
) {
    decision.precedence_trace.push(PolicyPrecedenceTrace {
        source,
        outcome: outcome.to_owned(),
        reason,
    });
    decision.effective_source = Some(source);
}

fn push_precedence_conflict(
    decision: &mut PolicyDecision,
    higher_source: AuthoritySource,
    lower_source: AuthoritySource,
    reason: String,
) {
    decision.precedence_conflicts.push(PolicyConflict {
        higher_source,
        lower_source,
        reason,
    });
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
    fn lane_id_filter_targets_specific_lane() {
        // Without laneId, both renewal and insurance lanes are checked for pay_recurring_obligation.
        // Insurance lane fails (missing premiumIncreasePercent), escalating to Tier 2.
        let d = evaluate_intent(
            "pay_recurring_obligation",
            &json!({"context": {"priceIncreasePercent": 5}}),
        );
        assert_eq!(d.tier, AuthorityTier::Tier2, "without laneId, insurance lane should fail");

        // With laneId targeting renewal lane, only that lane is checked.
        let d = evaluate_intent(
            "pay_recurring_obligation",
            &json!({"laneId": "lane-3.3-renewal", "context": {"priceIncreasePercent": 5}}),
        );
        assert_eq!(d.tier, AuthorityTier::Tier1, "with laneId, only renewal lane should be checked");
    }

    #[test]
    fn invalid_lane_id_escalates_instead_of_bypassing_checks() {
        let d = evaluate_intent(
            "pay_recurring_obligation",
            &json!({"laneId": "lane-does-not-exist", "context": {"priceIncreasePercent": 5}}),
        );
        assert_eq!(d.tier, AuthorityTier::Tier2);
        assert!(
            d.escalation_reasons
                .iter()
                .any(|r| r.contains("Unknown laneId"))
        );
    }

    #[test]
    fn saas_lane_blocks_exclusivity() {
        let d = evaluate_intent(
            "execute_standard_form_agreement",
            &json!({"laneId": "lane-3.1-saas", "modifications": ["exclusivity"]}),
        );
        assert_eq!(d.tier, AuthorityTier::Tier2);
        assert!(d.escalation_reasons.iter().any(|r| r.contains("lane-3.1-saas")));
    }

    #[test]
    fn string_modifications_payload_does_not_bypass_restricted_check() {
        let d = evaluate_intent(
            "execute_standard_form_agreement",
            &json!({"laneId": "lane-3.1-saas", "modifications": "exclusivity"}),
        );
        assert_eq!(d.tier, AuthorityTier::Tier2);
        assert!(d.escalation_reasons.iter().any(|r| r.contains("lane-3.1-saas")));
    }

    #[test]
    fn restricted_modification_case_variant_is_detected() {
        let d = evaluate_intent(
            "execute_standard_form_agreement",
            &json!({"laneId": "lane-3.1-vendor-po", "modifications": ["InDemnification"]}),
        );
        assert_eq!(d.tier, AuthorityTier::Tier2);
        assert!(d.escalation_reasons.iter().any(|r| r.contains("restricted areas")));
    }

    #[test]
    fn contractor_lane_blocks_equity_compensation() {
        let d = evaluate_intent(
            "engage_contractor",
            &json!({"laneId": "lane-3.2-1099", "modifications": ["equity_compensation"]}),
        );
        assert_eq!(d.tier, AuthorityTier::Tier2);
        assert!(d.escalation_reasons.iter().any(|r| r.contains("lane-3.2-1099")));
    }

    #[test]
    fn admin_lane_blocks_economics_changes() {
        let d = evaluate_intent(
            "routine_correspondence",
            &json!({"laneId": "lane-3.3-admin", "modifications": ["economics"]}),
        );
        assert_eq!(d.tier, AuthorityTier::Tier2);
        assert!(d.escalation_reasons.iter().any(|r| r.contains("lane-3.3-admin")));
    }

    #[test]
    fn insurance_lane_premium_increase_within_limit() {
        let d = evaluate_intent(
            "pay_recurring_obligation",
            &json!({"laneId": "lane-3.3-insurance", "context": {"premiumIncreasePercent": 10}}),
        );
        assert_eq!(d.tier, AuthorityTier::Tier1);
    }

    #[test]
    fn insurance_lane_premium_increase_exceeds_limit() {
        let d = evaluate_intent(
            "pay_recurring_obligation",
            &json!({"laneId": "lane-3.3-insurance", "context": {"premiumIncreasePercent": 20}}),
        );
        assert_eq!(d.tier, AuthorityTier::Tier2);
    }

    #[test]
    fn empty_checks_lane_always_passes() {
        // lane-3.4-transfer has no checks, should stay Tier 1.
        let d = evaluate_intent(
            "internal_account_transfer",
            &json!({"laneId": "lane-3.4-transfer"}),
        );
        assert_eq!(d.tier, AuthorityTier::Tier1);
    }

    #[test]
    fn ast_escalation_conditions_are_supported() {
        let ast = default_governance_ast();
        let supported: BTreeSet<EscalationCondition> =
            supported_escalation_conditions().iter().copied().collect();
        for rule in &ast.rules.escalation {
            let condition = rule.condition.expect("AST escalation rules must have a condition");
            assert!(
                supported.contains(&condition),
                "unsupported AST escalation condition: {:?} (rule id: {})",
                condition,
                rule.id
            );
        }
    }

    // ── evaluate_full tests ───────────────────────────────────────────

    use crate::domain::ids::EntityId;

    fn make_default_schedule() -> DelegationSchedule {
        DelegationSchedule::default_for_entity(EntityId::new())
    }

    fn make_ctx<'a>(
        intent_type: &'a str,
        metadata: &'a Value,
        mode: GovernanceMode,
        schedule: &'a DelegationSchedule,
    ) -> PolicyEvaluationContext<'a> {
        PolicyEvaluationContext {
            intent_type,
            metadata,
            mode,
            schedule,
            now: Utc::now(),
            entity_is_active: true,
            service_agreement_executed: true,
        }
    }

    #[test]
    fn full_tier1_normal_mode_allowed() {
        let schedule = make_default_schedule();
        let metadata = json!({"laneId": "lane-3.3-renewal", "context": {"priceIncreasePercent": 5}});
        let ctx = make_ctx(
            "pay_recurring_obligation",
            &metadata,
            GovernanceMode::Normal,
            &schedule,
        );
        let d = evaluate_full(&ctx);
        assert_eq!(d.tier, AuthorityTier::Tier1);
        assert!(d.allowed, "blockers: {:?}", d.blockers);
    }

    #[test]
    fn full_principal_unavailable_blocks_irreversible() {
        let schedule = make_default_schedule();
        let metadata = json!({"isReversible": false, "laneId": "lane-3.3-renewal", "context": {"priceIncreasePercent": 5}});
        let ctx = make_ctx(
            "pay_recurring_obligation",
            &metadata,
            GovernanceMode::PrincipalUnavailable,
            &schedule,
        );
        let d = evaluate_full(&ctx);
        assert!(!d.allowed);
        assert!(d.blockers.iter().any(|b| b.contains("principal_unavailable")));
    }

    #[test]
    fn full_principal_unavailable_allows_reversible_tier1() {
        let schedule = make_default_schedule();
        let metadata = json!({"isReversible": true, "laneId": "lane-3.3-renewal", "context": {"priceIncreasePercent": 5}});
        let ctx = make_ctx(
            "pay_recurring_obligation",
            &metadata,
            GovernanceMode::PrincipalUnavailable,
            &schedule,
        );
        let d = evaluate_full(&ctx);
        assert!(d.allowed, "blockers: {:?}", d.blockers);
    }

    #[test]
    fn full_incident_lockdown_blocks_non_allowlisted() {
        let schedule = make_default_schedule();
        let metadata = json!({});
        let ctx = make_ctx(
            "new_contract",
            &metadata,
            GovernanceMode::IncidentLockdown,
            &schedule,
        );
        let d = evaluate_full(&ctx);
        assert!(!d.allowed);
        assert!(d.blockers.iter().any(|b| b.contains("incident_lockdown")));
    }

    #[test]
    fn full_incident_lockdown_allows_record_keeping() {
        let schedule = make_default_schedule();
        let metadata = json!({});
        let ctx = make_ctx(
            "maintain_books_records",
            &metadata,
            GovernanceMode::IncidentLockdown,
            &schedule,
        );
        let d = evaluate_full(&ctx);
        assert!(d.allowed, "blockers: {:?}", d.blockers);
    }

    #[test]
    fn full_service_agreement_blocks_when_missing() {
        let schedule = make_default_schedule();
        let metadata = json!({"laneId": "lane-3.3-renewal", "context": {"priceIncreasePercent": 5}});
        let mut ctx = make_ctx(
            "pay_recurring_obligation",
            &metadata,
            GovernanceMode::Normal,
            &schedule,
        );
        ctx.service_agreement_executed = false;
        let d = evaluate_full(&ctx);
        assert!(!d.allowed);
        assert!(d.blockers.iter().any(|b| b.contains("service agreement")));
    }

    #[test]
    fn full_reauth_suspension_blocks() {
        let mut schedule = make_default_schedule();
        // Set last reauthorized 100 days ago (> 90 day threshold).
        let old_date = Utc::now() - chrono::Duration::days(100);
        schedule.set_last_reauthorized_at(old_date);
        let metadata = json!({"laneId": "lane-3.3-renewal", "context": {"priceIncreasePercent": 5}});
        let ctx = make_ctx(
            "pay_recurring_obligation",
            &metadata,
            GovernanceMode::Normal,
            &schedule,
        );
        let d = evaluate_full(&ctx);
        assert!(!d.allowed);
        assert!(d.blockers.iter().any(|b| b.contains("suspended")));
    }

    #[test]
    fn full_spending_limit_escalates() {
        let mut schedule = make_default_schedule();
        schedule.set_tier1_max_amount_cents(1000);
        let metadata = json!({"amount_cents": 5000, "laneId": "lane-3.3-renewal", "context": {"priceIncreasePercent": 5}});
        let ctx = make_ctx(
            "pay_recurring_obligation",
            &metadata,
            GovernanceMode::Normal,
            &schedule,
        );
        let d = evaluate_full(&ctx);
        assert_eq!(d.tier, AuthorityTier::Tier2);
        assert!(d.requires_approval);
    }
}
