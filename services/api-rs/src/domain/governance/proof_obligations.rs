//! Runtime proof-obligation checks for governance decisions.

use std::collections::HashSet;
use std::sync::OnceLock;

use crate::domain::execution::types::AuthorityTier;

use super::policy_ast::default_governance_ast;
use super::policy_engine::PolicyDecision;
use super::policy_lattice::has_valid_tier_approval_relation;

#[derive(Debug, Clone)]
pub struct ProofViolation {
    pub code: &'static str,
    pub detail: String,
}

#[derive(Debug, Clone, Default)]
pub struct ProofReport {
    pub violations: Vec<ProofViolation>,
}

impl ProofReport {
    pub fn passed(&self) -> bool {
        self.violations.is_empty()
    }
}

static KNOWN_CLAUSE_REFS: OnceLock<HashSet<String>> = OnceLock::new();

fn known_clause_refs() -> &'static HashSet<String> {
    KNOWN_CLAUSE_REFS.get_or_init(|| {
        let ast = default_governance_ast();
        let mut refs = HashSet::new();
        for doc in &ast.documents {
            for section in &doc.sections {
                refs.insert(section.id.clone());
                for clause in &section.clauses {
                    refs.insert(clause.id.clone());
                }
            }
        }
        for esc in &ast.rules.escalation {
            refs.insert(format!("rule.escalation.{}", esc.id));
        }
        for lane in &ast.rules.lane_conditions {
            refs.insert(format!("rule.lane.{}", lane.lane_id));
        }
        refs.insert("rule.lane.invalid_lane_id".to_owned());
        refs.insert("rule.mode.principal_unavailable".to_owned());
        refs.insert("rule.mode.incident_lockdown".to_owned());
        refs.insert("rule.reauth.full_suspension".to_owned());
        refs.insert("rule.precondition.service_agreement".to_owned());
        refs.insert("rule.metadata.decode_failure".to_owned());
        refs.insert("delegation.schedule.tier1_lane".to_owned());
        refs.insert("delegation.schedule.tier1_limit".to_owned());
        refs
    })
}

pub fn evaluate_proof_obligations(decision: &PolicyDecision) -> ProofReport {
    let mut report = ProofReport::default();

    if !has_valid_tier_approval_relation(decision) {
        report.violations.push(ProofViolation {
            code: "tier_approval_relation",
            detail: format!(
                "tier {:?} has inconsistent requires_approval={}",
                decision.tier, decision.requires_approval
            ),
        });
    }

    if !decision.policy_mapped && decision.tier != AuthorityTier::Tier2 {
        report.violations.push(ProofViolation {
            code: "unknown_capability_tier_fallback",
            detail: format!(
                "policy_mapped=false must default to tier_2; got {:?}",
                decision.tier
            ),
        });
    }

    if !decision.precedence_conflicts.is_empty() && decision.allowed {
        report.violations.push(ProofViolation {
            code: "conflict_fail_closed",
            detail: "precedence conflicts present while decision.allowed=true".to_owned(),
        });
    }

    let known_refs = known_clause_refs();
    for clause_ref in &decision.clause_refs {
        if !known_refs.contains(clause_ref) {
            report.violations.push(ProofViolation {
                code: "unknown_clause_ref",
                detail: format!("unknown clause ref emitted: {clause_ref}"),
            });
        }
    }

    report
}

pub fn enforce_proof_obligations(decision: &mut PolicyDecision) -> ProofReport {
    let report = evaluate_proof_obligations(decision);
    if !report.passed() {
        for violation in &report.violations {
            decision.add_blocker(format!(
                "proof obligation failed [{}]: {}",
                violation.code, violation.detail
            ));
        }
    }
    report
}

/// A `PolicyDecision` that has been verified against all proof obligations.
///
/// This type bundles the decision with its proof report, ensuring callers
/// can only obtain a decision that has been checked. Constructed by
/// `verify_decision()` or through the typestate pipeline.
#[derive(Debug, Clone)]
pub struct VerifiedDecision {
    decision: PolicyDecision,
    report: ProofReport,
}

impl VerifiedDecision {
    pub fn decision(&self) -> &PolicyDecision {
        &self.decision
    }

    pub fn report(&self) -> &ProofReport {
        &self.report
    }

    pub fn into_decision(self) -> PolicyDecision {
        self.decision
    }

    pub fn into_parts(self) -> (PolicyDecision, ProofReport) {
        (self.decision, self.report)
    }
}

/// Consume a `PolicyDecision` and verify all proof obligations.
/// Returns a `VerifiedDecision` bundling the decision with its proof report.
pub fn verify_decision(mut decision: PolicyDecision) -> VerifiedDecision {
    let report = enforce_proof_obligations(&mut decision);
    VerifiedDecision { decision, report }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::execution::types::AuthorityTier;

    #[test]
    fn catches_conflict_fail_closed_violation() {
        let mut d = PolicyDecision::new(
            AuthorityTier::Tier2,
            true,
            Vec::new(),
            Vec::new(),
            vec!["delegation.authority_tiers".to_owned()],
            Vec::new(),
            vec![crate::domain::governance::policy_engine::PolicyConflict {
                higher_source: crate::domain::governance::policy_engine::AuthoritySource::Law,
                lower_source: crate::domain::governance::policy_engine::AuthoritySource::Heuristic,
                reason: "conflict".to_owned(),
            }],
            None,
        );
        let report = enforce_proof_obligations(&mut d);
        assert!(!report.passed());
        assert!(!d.allowed);
    }
}
