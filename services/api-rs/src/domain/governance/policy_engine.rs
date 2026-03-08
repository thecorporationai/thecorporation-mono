//! Rust policy evaluator backed by the JSON governance AST.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::domain::execution::types::AuthorityTier;

use super::capability::GovernanceCapability;
use super::delegation_schedule::DelegationSchedule;
use super::mode::GovernanceMode;
use super::policy_ast::{
    BoolField, EscalationCondition, EscalationRule, LaneCheck, LaneField, LaneScalarValue,
    NumericField, StringListField, default_governance_ast,
};
use super::proof_obligations::{enforce_proof_obligations, verify_decision};
use super::typed_intent::{ParsedGovernanceMetadata, TypedIntent};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, utoipa::ToSchema)]
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

/// All authority sources in descending rank order (highest authority first).
const AUTHORITY_SOURCES_BY_RANK: [AuthoritySource; 8] = [
    AuthoritySource::Law,
    AuthoritySource::Charter,
    AuthoritySource::GovernanceDocs,
    AuthoritySource::Resolution,
    AuthoritySource::Directive,
    AuthoritySource::StandingInstruction,
    AuthoritySource::DelegationSchedule,
    AuthoritySource::Heuristic,
];

// Compile-time assertion: the array is monotonically ranked (each rank > next rank).
const _: () = {
    let sources = AUTHORITY_SOURCES_BY_RANK;
    let mut i = 0;
    while i + 1 < sources.len() {
        assert!(
            sources[i].rank() > sources[i + 1].rank(),
            "AUTHORITY_SOURCES_BY_RANK is not monotonically ranked"
        );
        i += 1;
    }
};

impl AuthoritySource {
    /// Numeric rank: higher means greater authority. Law=7, Heuristic=0.
    pub const fn rank(self) -> u8 {
        match self {
            Self::Law => 7,
            Self::Charter => 6,
            Self::GovernanceDocs => 5,
            Self::Resolution => 4,
            Self::Directive => 3,
            Self::StandingInstruction => 2,
            Self::DelegationSchedule => 1,
            Self::Heuristic => 0,
        }
    }

    /// Returns true if `self` has strictly greater authority than `other`.
    pub fn outranks(self, other: Self) -> bool {
        self.rank() > other.rank()
    }
}

impl PartialOrd for AuthoritySource {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for AuthoritySource {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.rank().cmp(&other.rank())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct PolicyPrecedenceTrace {
    pub source: AuthoritySource,
    pub outcome: String,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct PolicyConflict {
    pub higher_source: AuthoritySource,
    pub lower_source: AuthoritySource,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct PolicyDecision {
    pub(super) tier: AuthorityTier,
    pub(super) policy_mapped: bool,
    pub(super) allowed: bool,
    pub(super) requires_approval: bool,
    pub(super) blockers: Vec<String>,
    pub(super) escalation_reasons: Vec<String>,
    pub(super) clause_refs: Vec<String>,
    #[serde(default)]
    pub(super) precedence_trace: Vec<PolicyPrecedenceTrace>,
    #[serde(default)]
    pub(super) precedence_conflicts: Vec<PolicyConflict>,
    #[serde(default)]
    pub(super) effective_source: Option<AuthoritySource>,
}

impl PolicyDecision {
    /// Construct a new PolicyDecision with derived invariants:
    /// - `allowed` is derived from `blockers.is_empty()`
    /// - `requires_approval` is derived from `tier > Tier1`
    pub fn new(
        tier: AuthorityTier,
        policy_mapped: bool,
        blockers: Vec<String>,
        escalation_reasons: Vec<String>,
        clause_refs: Vec<String>,
        precedence_trace: Vec<PolicyPrecedenceTrace>,
        precedence_conflicts: Vec<PolicyConflict>,
        effective_source: Option<AuthoritySource>,
    ) -> Self {
        Self {
            allowed: blockers.is_empty(),
            requires_approval: tier > AuthorityTier::Tier1,
            tier,
            policy_mapped,
            blockers,
            escalation_reasons,
            clause_refs,
            precedence_trace,
            precedence_conflicts,
            effective_source,
        }
    }

    // ── Getters ──────────────────────────────────────────────────────

    pub fn tier(&self) -> AuthorityTier {
        self.tier
    }

    pub fn policy_mapped(&self) -> bool {
        self.policy_mapped
    }

    pub fn allowed(&self) -> bool {
        self.allowed
    }

    pub fn requires_approval(&self) -> bool {
        self.requires_approval
    }

    pub fn blockers(&self) -> &[String] {
        &self.blockers
    }

    pub fn escalation_reasons(&self) -> &[String] {
        &self.escalation_reasons
    }

    pub fn clause_refs(&self) -> &[String] {
        &self.clause_refs
    }

    pub fn precedence_trace(&self) -> &[PolicyPrecedenceTrace] {
        &self.precedence_trace
    }

    pub fn precedence_conflicts(&self) -> &[PolicyConflict] {
        &self.precedence_conflicts
    }

    pub fn effective_source(&self) -> Option<AuthoritySource> {
        self.effective_source
    }

    // ── Invariant-preserving mutators ────────────────────────────────

    /// Escalate the tier — only raises, never lowers. Updates `requires_approval`.
    pub fn escalate_tier(&mut self, new_tier: AuthorityTier) {
        if new_tier > self.tier {
            self.tier = new_tier;
            self.requires_approval = self.tier > AuthorityTier::Tier1;
        }
    }

    /// Add a blocker, which sets `allowed = false`.
    pub fn add_blocker(&mut self, msg: String) {
        self.blockers.push(msg);
        self.allowed = false;
    }

    /// Add an escalation reason.
    pub fn add_escalation_reason(&mut self, msg: String) {
        self.escalation_reasons.push(msg);
    }

    /// Add a clause reference.
    pub fn add_clause_ref(&mut self, clause_ref: String) {
        self.clause_refs.push(clause_ref);
    }

    /// Record a precedence trace entry and update the effective source.
    pub fn push_precedence_trace(
        &mut self,
        source: AuthoritySource,
        outcome: &str,
        reason: Option<String>,
    ) {
        self.precedence_trace.push(PolicyPrecedenceTrace {
            source,
            outcome: outcome.to_owned(),
            reason,
        });
        self.effective_source = Some(source);
    }

    /// Record a precedence conflict. The higher source must outrank the lower source.
    pub fn push_precedence_conflict(
        &mut self,
        higher_source: AuthoritySource,
        lower_source: AuthoritySource,
        reason: String,
    ) {
        debug_assert!(
            higher_source.outranks(lower_source),
            "push_precedence_conflict: higher_source {:?} (rank {}) does not outrank lower_source {:?} (rank {})",
            higher_source,
            higher_source.rank(),
            lower_source,
            lower_source.rank()
        );
        self.precedence_conflicts.push(PolicyConflict {
            higher_source,
            lower_source,
            reason,
        });
    }

    /// Override tier for unmapped capabilities (legacy execution.rs behavior).
    /// Only valid when `policy_mapped` is false.
    pub fn override_tier_for_unmapped(&mut self, tier: AuthorityTier) {
        debug_assert!(
            !self.policy_mapped,
            "override_tier_for_unmapped called on policy-mapped decision"
        );
        self.tier = tier;
        self.requires_approval = self.tier > AuthorityTier::Tier1;
    }
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

    let policy_mapped = parsed_cap.is_some_and(|cap| ast.rules.tier_defaults.contains_key(&cap));
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

    let mut decision = PolicyDecision::new(
        tier,
        policy_mapped,
        blockers,
        reasons,
        clause_refs,
        vec![PolicyPrecedenceTrace {
            source: AuthoritySource::Heuristic,
            outcome: "allow".to_owned(),
            reason: Some("base policy AST evaluation".to_owned()),
        }],
        Vec::new(),
        Some(AuthoritySource::Heuristic),
    );

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
            let got = get_field(metadata, LaneField::from(*field));
            got == scalar_value_as_json(value).as_ref()
        }
        LaneCheck::Neq { field, value, .. } => {
            let got = get_field(metadata, LaneField::from(*field));
            got != scalar_value_as_json(value).as_ref()
        }
        LaneCheck::Lte { field, value, .. } => get_field(metadata, LaneField::from(*field))
            .and_then(Value::as_f64)
            .is_some_and(|g| g <= *value),
        LaneCheck::Gte { field, value, .. } => get_field(metadata, LaneField::from(*field))
            .and_then(Value::as_f64)
            .is_some_and(|g| g >= *value),
        LaneCheck::ContainsNone { field, value, .. } => {
            let got = get_field(metadata, LaneField::from(*field));
            let arr = normalized_string_list(got);
            let banned = value
                .iter()
                .map(|s| s.trim().to_ascii_lowercase())
                .collect::<Vec<_>>();
            !arr.iter()
                .any(|item| banned.iter().any(|blocked| blocked == item))
        }
        LaneCheck::ContainsAny { field, value, .. } => {
            let got = get_field(metadata, LaneField::from(*field));
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
        LaneField::Bool(BoolField::TemplateApproved) => value.get("templateApproved"),
        LaneField::StringList(StringListField::Modifications) => value.get("modifications"),
        LaneField::Numeric(NumericField::ContextRateIncreasePercent) => value
            .get("context")
            .and_then(|v| v.get("rateIncreasePercent")),
        LaneField::Numeric(NumericField::ContextPriceIncreasePercent) => value
            .get("context")
            .and_then(|v| v.get("priceIncreasePercent")),
        LaneField::Numeric(NumericField::ContextPremiumIncreasePercent) => value
            .get("context")
            .and_then(|v| v.get("premiumIncreasePercent")),
    }
}

// ── Typestate pipeline ────────────────────────────────────────────────

mod sealed {
    pub trait PipelineStage {}
}

/// Base evaluation complete — no overrides applied yet.
pub enum Raw {}
/// Mode overrides (normal/principal_unavailable/incident_lockdown) applied.
pub enum ModeApplied {}
/// Delegation schedule overrides applied.
pub enum ScheduleApplied {}
/// Service agreement precondition applied.
pub enum AgreementApplied {}
/// Precedence conflict fail-closed check applied.
pub enum ConflictChecked {}
/// Proof obligations verified — terminal state.
pub enum Verified {}

impl sealed::PipelineStage for Raw {}
impl sealed::PipelineStage for ModeApplied {}
impl sealed::PipelineStage for ScheduleApplied {}
impl sealed::PipelineStage for AgreementApplied {}
impl sealed::PipelineStage for ConflictChecked {}
impl sealed::PipelineStage for Verified {}

/// A `PolicyDecision` tagged with its pipeline stage.
///
/// Only `PipelineDecision<Verified>` can produce the final `PolicyDecision`.
/// Calling stages out of order is a compile error.
pub(crate) struct PipelineDecision<S: sealed::PipelineStage> {
    decision: PolicyDecision,
    _stage: std::marker::PhantomData<S>,
}

impl<S: sealed::PipelineStage> PipelineDecision<S> {
    fn new(decision: PolicyDecision) -> Self {
        Self {
            decision,
            _stage: std::marker::PhantomData,
        }
    }

    fn transition<T: sealed::PipelineStage>(self) -> PipelineDecision<T> {
        PipelineDecision::new(self.decision)
    }

    /// Access the inner decision (read-only) for inspection at any stage.
    pub(crate) fn decision(&self) -> &PolicyDecision {
        &self.decision
    }
}

impl PipelineDecision<Raw> {
    pub(crate) fn apply_mode(
        self,
        mode: GovernanceMode,
        intent_type: &str,
        metadata: &Value,
    ) -> PipelineDecision<ModeApplied> {
        let decision = apply_mode_overrides(self.decision, mode, intent_type, metadata);
        PipelineDecision::new(decision)
    }
}

impl PipelineDecision<ModeApplied> {
    pub(crate) fn apply_tier_override(
        mut self,
        tier_override: Option<AuthorityTier>,
    ) -> PipelineDecision<ModeApplied> {
        if let Some(tier) = tier_override {
            if !self.decision.policy_mapped() {
                self.decision.override_tier_for_unmapped(tier);
            }
        }
        self
    }

    pub(crate) fn apply_schedule(
        self,
        schedule: &DelegationSchedule,
        intent_type: &str,
        metadata: &Value,
        now: DateTime<Utc>,
    ) -> PipelineDecision<ScheduleApplied> {
        let decision =
            apply_schedule_overrides(self.decision, schedule, intent_type, metadata, now);
        PipelineDecision::new(decision)
    }
}

impl PipelineDecision<ScheduleApplied> {
    pub(crate) fn apply_agreement(
        self,
        entity_is_active: bool,
        service_agreement_executed: bool,
    ) -> PipelineDecision<AgreementApplied> {
        let decision = apply_service_agreement_overrides(
            self.decision,
            entity_is_active,
            service_agreement_executed,
        );
        PipelineDecision::new(decision)
    }
}

impl PipelineDecision<AgreementApplied> {
    pub(crate) fn apply_conflict_check(self) -> PipelineDecision<ConflictChecked> {
        let decision = apply_conflict_fail_closed(self.decision);
        PipelineDecision::new(decision)
    }
}

impl PipelineDecision<ConflictChecked> {
    pub(crate) fn verify(self) -> PipelineDecision<Verified> {
        let verified = verify_decision(self.decision);
        PipelineDecision {
            decision: verified.into_decision(),
            _stage: std::marker::PhantomData,
        }
    }
}

impl PipelineDecision<Verified> {
    /// Extract the final `PolicyDecision`. Only available after full pipeline verification.
    pub(crate) fn into_decision(self) -> PolicyDecision {
        self.decision
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
    evaluate_full_with_override(ctx, None)
}

/// Run the full policy evaluation pipeline with an optional tier override for unmapped capabilities.
///
/// When `tier_override` is `Some(tier)` and the capability is not policy-mapped,
/// the tier is overridden to the requested value. This supports the execution route's
/// legacy behavior where intents can specify their own tier for unmapped capabilities.
///
/// The full pipeline is always executed including `enforce_proof_obligations`, preventing
/// the bug where proof obligations were skipped after mode/schedule/agreement overrides.
pub fn evaluate_full_with_override(
    ctx: &PolicyEvaluationContext,
    tier_override: Option<AuthorityTier>,
) -> PolicyDecision {
    let canonical = canonicalize_intent_type(ctx.intent_type);
    evaluate_pipeline(&canonical, ctx.metadata)
        .apply_mode(ctx.mode, &canonical, ctx.metadata)
        .apply_tier_override(tier_override)
        .apply_schedule(ctx.schedule, &canonical, ctx.metadata, ctx.now)
        .apply_agreement(ctx.entity_is_active, ctx.service_agreement_executed)
        .apply_conflict_check()
        .verify()
        .into_decision()
}

/// Entry point for the typed pipeline: evaluates base intent and returns `PipelineDecision<Raw>`.
pub(crate) fn evaluate_pipeline(intent_type: &str, metadata: &Value) -> PipelineDecision<Raw> {
    let decision = evaluate_intent(intent_type, metadata);
    PipelineDecision::new(decision)
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
    let was_allowed = decision.allowed();
    let initial_tier = decision.tier();
    match mode {
        GovernanceMode::Normal => {
            decision.push_precedence_trace(
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
            if decision.tier() != AuthorityTier::Tier1 || !reversible {
                decision.add_blocker(
                    "principal_unavailable mode only permits reversible tier_1 actions".to_owned(),
                );
                if was_allowed {
                    decision.push_precedence_conflict(
                        AuthoritySource::Resolution,
                        AuthoritySource::Heuristic,
                        "principal_unavailable overrides previously-allowed action".to_owned(),
                    );
                }
                decision.push_precedence_trace(
                    AuthoritySource::Resolution,
                    "block",
                    Some("principal_unavailable mode requires reversible tier_1".to_owned()),
                );
            } else {
                decision.push_precedence_trace(
                    AuthoritySource::Resolution,
                    "allow",
                    Some("principal_unavailable mode constraints satisfied".to_owned()),
                );
            }
            decision.add_clause_ref("rule.mode.principal_unavailable".to_owned());
        }
        GovernanceMode::IncidentLockdown => {
            if !LOCKDOWN_ALLOWLIST.contains(&intent_type) {
                decision.add_blocker("incident_lockdown mode blocks this capability".to_owned());
                if was_allowed {
                    decision.push_precedence_conflict(
                        AuthoritySource::Resolution,
                        AuthoritySource::Heuristic,
                        "incident_lockdown overrides previously-allowed action".to_owned(),
                    );
                }
                decision.push_precedence_trace(
                    AuthoritySource::Resolution,
                    "block",
                    Some("incident_lockdown blocks non-allowlisted capabilities".to_owned()),
                );
            } else {
                decision.push_precedence_trace(
                    AuthoritySource::Resolution,
                    "allow",
                    Some("incident_lockdown allowlisted capability".to_owned()),
                );
            }
            decision.add_clause_ref("rule.mode.incident_lockdown".to_owned());
        }
    }
    if initial_tier != decision.tier() {
        let updated_tier = decision.tier();
        decision.push_precedence_trace(
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
    if decision.tier() != AuthorityTier::Tier1 {
        decision.push_precedence_trace(
            AuthoritySource::DelegationSchedule,
            "allow",
            Some("schedule tier_1 checks skipped for non-tier_1 action".to_owned()),
        );
        return decision;
    }

    let was_allowed = decision.allowed();
    if schedule.is_reauth_suspended_at(now) {
        decision.add_blocker(
            "delegation schedule autonomy is suspended pending reauthorization".to_owned(),
        );
        decision.add_clause_ref("rule.reauth.full_suspension".to_owned());
        if was_allowed {
            decision.push_precedence_conflict(
                AuthoritySource::DelegationSchedule,
                AuthoritySource::Heuristic,
                "reauthorization suspension overrides previously-allowed tier_1 action".to_owned(),
            );
        }
        decision.push_precedence_trace(
            AuthoritySource::DelegationSchedule,
            "block",
            Some("delegation schedule full suspension active".to_owned()),
        );
        return decision;
    }

    if !schedule.allows_tier1_intent(intent_type) {
        decision.escalate_tier(AuthorityTier::Tier2);
        decision.add_escalation_reason(
            "intent is outside the tier_1 delegation lane and requires explicit approval"
                .to_owned(),
        );
        decision.add_clause_ref("delegation.schedule.tier1_lane".to_owned());
        decision.push_precedence_conflict(
            AuthoritySource::DelegationSchedule,
            AuthoritySource::Heuristic,
            "tier_1 lane restriction escalated action to tier_2".to_owned(),
        );
        decision.push_precedence_trace(
            AuthoritySource::DelegationSchedule,
            "escalate",
            Some("intent outside allowed tier_1 lane".to_owned()),
        );
    }

    if let Some(amount_cents) = amount_from_metadata_cents(metadata) {
        let effective_limit = schedule.effective_tier1_max_amount_cents(now);
        if amount_cents > effective_limit {
            decision.escalate_tier(AuthorityTier::Tier2);
            decision.add_escalation_reason(format!(
                "amount {amount_cents} exceeds current tier_1 limit {effective_limit}"
            ));
            decision.add_clause_ref("delegation.schedule.tier1_limit".to_owned());
            decision.push_precedence_conflict(
                AuthoritySource::DelegationSchedule,
                AuthoritySource::Heuristic,
                "tier_1 spending limit exceeded".to_owned(),
            );
            decision.push_precedence_trace(
                AuthoritySource::DelegationSchedule,
                "escalate",
                Some(format!(
                    "tier_1 amount limit exceeded: {amount_cents} > {effective_limit}"
                )),
            );
        }
    }

    if decision.allowed() && decision.tier() == AuthorityTier::Tier1 {
        decision.push_precedence_trace(
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
    let was_allowed = decision.allowed();
    if entity_is_active && decision.tier() == AuthorityTier::Tier1 && !service_agreement_executed {
        decision.add_blocker(
            "active entities require an executed service agreement for tier_1 autonomy".to_owned(),
        );
        decision.add_clause_ref("rule.precondition.service_agreement".to_owned());
        if was_allowed {
            decision.push_precedence_conflict(
                AuthoritySource::GovernanceDocs,
                AuthoritySource::Heuristic,
                "service agreement precondition overrides autonomous tier_1 execution".to_owned(),
            );
        }
        decision.push_precedence_trace(
            AuthoritySource::GovernanceDocs,
            "block",
            Some("service agreement precondition not satisfied".to_owned()),
        );
    } else {
        decision.push_precedence_trace(
            AuthoritySource::GovernanceDocs,
            "allow",
            Some("service agreement precondition satisfied or not applicable".to_owned()),
        );
    }
    decision
}

pub fn apply_conflict_fail_closed(mut decision: PolicyDecision) -> PolicyDecision {
    if !decision.precedence_conflicts().is_empty() {
        let count = decision.precedence_conflicts().len();
        decision.add_blocker(format!(
            "precedence conflict requires explicit human governance resolution ({count} conflict(s))",
        ));
    }
    decision
}

/// Returns true if the intent maps to a known policy tier above Tier 1
/// (i.e., it requires manual approval artifacts).
pub fn mapped_tier_requires_manual_artifacts(decision: &PolicyDecision) -> bool {
    decision.policy_mapped() && decision.tier() != AuthorityTier::Tier1
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
        assert_eq!(
            d.tier,
            AuthorityTier::Tier2,
            "without laneId, insurance lane should fail"
        );

        // With laneId targeting renewal lane, only that lane is checked.
        let d = evaluate_intent(
            "pay_recurring_obligation",
            &json!({"laneId": "lane-3.3-renewal", "context": {"priceIncreasePercent": 5}}),
        );
        assert_eq!(
            d.tier,
            AuthorityTier::Tier1,
            "with laneId, only renewal lane should be checked"
        );
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
        assert!(
            d.escalation_reasons
                .iter()
                .any(|r| r.contains("lane-3.1-saas"))
        );
    }

    #[test]
    fn string_modifications_payload_does_not_bypass_restricted_check() {
        let d = evaluate_intent(
            "execute_standard_form_agreement",
            &json!({"laneId": "lane-3.1-saas", "modifications": "exclusivity"}),
        );
        assert_eq!(d.tier, AuthorityTier::Tier2);
        assert!(
            d.escalation_reasons
                .iter()
                .any(|r| r.contains("lane-3.1-saas"))
        );
    }

    #[test]
    fn restricted_modification_case_variant_is_detected() {
        let d = evaluate_intent(
            "execute_standard_form_agreement",
            &json!({"laneId": "lane-3.1-vendor-po", "modifications": ["InDemnification"]}),
        );
        assert_eq!(d.tier, AuthorityTier::Tier2);
        assert!(
            d.escalation_reasons
                .iter()
                .any(|r| r.contains("restricted areas"))
        );
    }

    #[test]
    fn contractor_lane_blocks_equity_compensation() {
        let d = evaluate_intent(
            "engage_contractor",
            &json!({"laneId": "lane-3.2-1099", "modifications": ["equity_compensation"]}),
        );
        assert_eq!(d.tier, AuthorityTier::Tier2);
        assert!(
            d.escalation_reasons
                .iter()
                .any(|r| r.contains("lane-3.2-1099"))
        );
    }

    #[test]
    fn admin_lane_blocks_economics_changes() {
        let d = evaluate_intent(
            "routine_correspondence",
            &json!({"laneId": "lane-3.3-admin", "modifications": ["economics"]}),
        );
        assert_eq!(d.tier, AuthorityTier::Tier2);
        assert!(
            d.escalation_reasons
                .iter()
                .any(|r| r.contains("lane-3.3-admin"))
        );
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
            let condition = rule
                .condition
                .expect("AST escalation rules must have a condition");
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
        let metadata =
            json!({"laneId": "lane-3.3-renewal", "context": {"priceIncreasePercent": 5}});
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
        assert!(
            d.blockers
                .iter()
                .any(|b| b.contains("principal_unavailable"))
        );
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
        let metadata =
            json!({"laneId": "lane-3.3-renewal", "context": {"priceIncreasePercent": 5}});
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
        let metadata =
            json!({"laneId": "lane-3.3-renewal", "context": {"priceIncreasePercent": 5}});
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

    // ── AuthoritySource ordering tests ───────────────────────────────

    #[test]
    fn authority_source_rank_ordering() {
        assert!(AuthoritySource::Law.outranks(AuthoritySource::Charter));
        assert!(AuthoritySource::Charter.outranks(AuthoritySource::GovernanceDocs));
        assert!(AuthoritySource::GovernanceDocs.outranks(AuthoritySource::Resolution));
        assert!(AuthoritySource::Resolution.outranks(AuthoritySource::Directive));
        assert!(AuthoritySource::Directive.outranks(AuthoritySource::StandingInstruction));
        assert!(AuthoritySource::StandingInstruction.outranks(AuthoritySource::DelegationSchedule));
        assert!(AuthoritySource::DelegationSchedule.outranks(AuthoritySource::Heuristic));
    }

    #[test]
    fn authority_source_ord_is_total() {
        let sources = [
            AuthoritySource::Law,
            AuthoritySource::Charter,
            AuthoritySource::GovernanceDocs,
            AuthoritySource::Resolution,
            AuthoritySource::Directive,
            AuthoritySource::StandingInstruction,
            AuthoritySource::DelegationSchedule,
            AuthoritySource::Heuristic,
        ];
        // Trichotomy: for all pairs, exactly one of <, ==, > holds
        for (i, a) in sources.iter().enumerate() {
            for (j, b) in sources.iter().enumerate() {
                if i < j {
                    assert!(a > b, "{a:?} should outrank {b:?}");
                } else if i == j {
                    assert_eq!(a, b);
                } else {
                    assert!(a < b, "{a:?} should be outranked by {b:?}");
                }
            }
        }
    }

    #[test]
    fn authority_source_self_does_not_outrank_self() {
        let sources = [
            AuthoritySource::Law,
            AuthoritySource::Charter,
            AuthoritySource::GovernanceDocs,
            AuthoritySource::Resolution,
            AuthoritySource::Directive,
            AuthoritySource::StandingInstruction,
            AuthoritySource::DelegationSchedule,
            AuthoritySource::Heuristic,
        ];
        for s in sources {
            assert!(!s.outranks(s), "{s:?} should not outrank itself");
        }
    }

    // ── Typestate pipeline tests ─────────────────────────────────────

    #[test]
    fn pipeline_stages_produce_same_result_as_evaluate_full() {
        let schedule = make_default_schedule();
        let metadata =
            json!({"laneId": "lane-3.3-renewal", "context": {"priceIncreasePercent": 5}});
        let ctx = make_ctx(
            "pay_recurring_obligation",
            &metadata,
            GovernanceMode::Normal,
            &schedule,
        );

        // evaluate_full uses the pipeline internally — verify it matches direct evaluation
        let d = evaluate_full(&ctx);
        assert_eq!(d.tier, AuthorityTier::Tier1);
        assert!(d.allowed, "blockers: {:?}", d.blockers);

        // Also verify through evaluate_full_with_override
        let d2 = evaluate_full_with_override(&ctx, None);
        assert_eq!(d2.tier, d.tier);
        assert_eq!(d2.allowed, d.allowed);
    }

    #[test]
    fn pipeline_with_tier_override_for_unmapped() {
        let schedule = make_default_schedule();
        let metadata = json!({});
        let ctx = make_ctx(
            "totally.unknown.intent",
            &metadata,
            GovernanceMode::Normal,
            &schedule,
        );

        // Without override: unmapped defaults to Tier2
        let d_default = evaluate_full(&ctx);
        assert_eq!(d_default.tier, AuthorityTier::Tier2);

        // With override to Tier1: unmapped uses the override
        let d_override = evaluate_full_with_override(&ctx, Some(AuthorityTier::Tier1));
        assert_eq!(d_override.tier, AuthorityTier::Tier1);
    }
}
