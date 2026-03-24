//! Proof obligations — structural invariant verification of policy decisions.

use serde::{Deserialize, Serialize};

use super::capability::AuthorityTier;
use super::policy::PolicyDecision;

// ── ProofViolation ────────────────────────────────────────────────────────────

/// A single invariant that was found to be violated.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofViolation {
    /// Machine-readable rule identifier.
    pub rule: String,
    /// Human-readable explanation of the violation.
    pub message: String,
}

// ── ProofReport ───────────────────────────────────────────────────────────────

/// The aggregate result of running all proof obligations against a decision.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofReport {
    pub violations: Vec<ProofViolation>,
}

impl ProofReport {
    /// Returns `true` if no invariants were violated.
    pub fn passed(&self) -> bool {
        self.violations.is_empty()
    }
}

// ── verify_decision ───────────────────────────────────────────────────────────

/// Run all structural proof obligations against a [`PolicyDecision`] and return
/// a [`ProofReport`] describing any violations found.
///
/// ## Checked invariants
///
/// | Rule | Description |
/// |------|-------------|
/// | `tier_approval_relation` | Tier1 decisions must **not** require approval; Tier2+ **must**. |
/// | `conflict_fail_closed`   | If any blockers are present, `allowed` must be `false`. |
pub fn verify_decision(decision: &PolicyDecision) -> ProofReport {
    let mut violations = Vec::new();

    // Rule: tier_approval_relation
    match decision.tier {
        AuthorityTier::Tier1 => {
            if decision.requires_approval {
                violations.push(ProofViolation {
                    rule: "tier_approval_relation".into(),
                    message: "Tier1 decisions must not require approval — \
                              autonomous actions should be pre-authorised by policy."
                        .into(),
                });
            }
        }
        AuthorityTier::Tier2 | AuthorityTier::Tier3 => {
            if !decision.requires_approval && decision.allowed {
                violations.push(ProofViolation {
                    rule: "tier_approval_relation".into(),
                    message: format!(
                        "{:?} decisions must require approval before acting.",
                        decision.tier
                    ),
                });
            }
        }
    }

    // Rule: conflict_fail_closed
    if !decision.blockers.is_empty() && decision.allowed {
        violations.push(ProofViolation {
            rule: "conflict_fail_closed".into(),
            message: "Blockers are present but `allowed` is true — \
                      a blocked action must be denied (fail-closed)."
                .into(),
        });
    }

    ProofReport { violations }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn decision(
        tier: AuthorityTier,
        allowed: bool,
        requires_approval: bool,
        blockers: Vec<String>,
    ) -> PolicyDecision {
        PolicyDecision {
            tier,
            allowed,
            requires_approval,
            blockers,
            escalation_reasons: vec![],
            effective_source: None,
        }
    }

    // ── ProofReport helpers ───────────────────────────────────────────────────

    #[test]
    fn empty_violations_passes() {
        let report = ProofReport { violations: vec![] };
        assert!(report.passed());
    }

    #[test]
    fn non_empty_violations_fails() {
        let report = ProofReport {
            violations: vec![ProofViolation {
                rule: "test_rule".into(),
                message: "test message".into(),
            }],
        };
        assert!(!report.passed());
    }

    // ── Tier1 decisions ───────────────────────────────────────────────────────

    #[test]
    fn tier1_no_approval_passes() {
        let d = decision(AuthorityTier::Tier1, true, false, vec![]);
        assert!(verify_decision(&d).passed());
    }

    #[test]
    fn tier1_with_approval_violates() {
        let d = decision(AuthorityTier::Tier1, true, true, vec![]);
        let report = verify_decision(&d);
        assert!(!report.passed());
        assert!(
            report
                .violations
                .iter()
                .any(|v| v.rule == "tier_approval_relation")
        );
    }

    #[test]
    fn tier1_denied_no_approval_passes() {
        // Tier1 denied (not allowed), no approval required — valid
        let d = decision(AuthorityTier::Tier1, false, false, vec![]);
        assert!(verify_decision(&d).passed());
    }

    #[test]
    fn tier1_empty_blockers_no_violation() {
        let d = decision(AuthorityTier::Tier1, true, false, vec![]);
        let report = verify_decision(&d);
        assert!(report.violations.is_empty());
    }

    #[test]
    fn tier1_with_blockers_and_allowed_double_violation() {
        // requires_approval violates tier_approval_relation AND blockers+allowed violates conflict_fail_closed
        let d = decision(
            AuthorityTier::Tier1,
            true,
            true,
            vec!["some_blocker".into()],
        );
        let report = verify_decision(&d);
        assert!(!report.passed());
        // Should have both violations
        assert!(
            report
                .violations
                .iter()
                .any(|v| v.rule == "tier_approval_relation")
        );
        assert!(
            report
                .violations
                .iter()
                .any(|v| v.rule == "conflict_fail_closed")
        );
    }

    // ── Tier2 decisions ───────────────────────────────────────────────────────

    #[test]
    fn tier2_requires_approval_passes() {
        let d = decision(AuthorityTier::Tier2, true, true, vec![]);
        assert!(verify_decision(&d).passed());
    }

    #[test]
    fn tier2_no_approval_violates() {
        let d = decision(AuthorityTier::Tier2, true, false, vec![]);
        let report = verify_decision(&d);
        assert!(!report.passed());
        assert!(
            report
                .violations
                .iter()
                .any(|v| v.rule == "tier_approval_relation")
        );
    }

    #[test]
    fn tier2_denied_no_approval_ok() {
        // Tier2 denied + no requires_approval: allowed=false so tier rule doesn't fire
        let d = decision(AuthorityTier::Tier2, false, false, vec![]);
        assert!(verify_decision(&d).passed());
    }

    #[test]
    fn tier2_with_blockers_allowed_violates_fail_closed() {
        let d = decision(
            AuthorityTier::Tier2,
            true,
            true,
            vec!["entity_not_in_good_standing".into()],
        );
        let report = verify_decision(&d);
        assert!(!report.passed());
        assert!(
            report
                .violations
                .iter()
                .any(|v| v.rule == "conflict_fail_closed")
        );
    }

    #[test]
    fn tier2_with_blockers_denied_passes_fail_closed() {
        let d = decision(
            AuthorityTier::Tier2,
            false,
            true,
            vec!["pending_litigation".into()],
        );
        // `allowed` is false, so fail-closed rule is satisfied.
        let report = verify_decision(&d);
        assert!(report.passed());
    }

    #[test]
    fn tier2_multiple_blockers_and_allowed_violates() {
        let d = decision(
            AuthorityTier::Tier2,
            true,
            true,
            vec!["blocker_a".into(), "blocker_b".into()],
        );
        let report = verify_decision(&d);
        assert!(!report.passed());
        assert!(
            report
                .violations
                .iter()
                .any(|v| v.rule == "conflict_fail_closed")
        );
    }

    // ── Tier3 decisions ───────────────────────────────────────────────────────

    #[test]
    fn tier3_with_approval_passes() {
        let d = decision(AuthorityTier::Tier3, true, true, vec![]);
        assert!(verify_decision(&d).passed());
    }

    #[test]
    fn tier3_no_approval_allowed_violates() {
        // Tier3 allowed without requires_approval violates tier_approval_relation
        let d = decision(AuthorityTier::Tier3, true, false, vec![]);
        let report = verify_decision(&d);
        assert!(!report.passed());
        assert!(
            report
                .violations
                .iter()
                .any(|v| v.rule == "tier_approval_relation")
        );
    }

    #[test]
    fn tier3_denied_no_approval_needed_is_ok() {
        // A denied Tier3 action does not need to set requires_approval.
        let d = decision(AuthorityTier::Tier3, false, false, vec![]);
        assert!(verify_decision(&d).passed());
    }

    #[test]
    fn tier3_with_blockers_and_allowed_violates() {
        let d = decision(
            AuthorityTier::Tier3,
            true,
            true,
            vec!["dissolution_pending".into()],
        );
        let report = verify_decision(&d);
        assert!(!report.passed());
        assert!(
            report
                .violations
                .iter()
                .any(|v| v.rule == "conflict_fail_closed")
        );
    }

    #[test]
    fn tier3_with_blockers_denied_is_ok() {
        let d = decision(
            AuthorityTier::Tier3,
            false,
            true,
            vec!["charter_not_filed".into()],
        );
        assert!(verify_decision(&d).passed());
    }

    // ── empty blockers cases ──────────────────────────────────────────────────

    #[test]
    fn empty_blockers_no_fail_closed_violation() {
        let d = decision(AuthorityTier::Tier2, true, true, vec![]);
        let report = verify_decision(&d);
        assert!(
            !report
                .violations
                .iter()
                .any(|v| v.rule == "conflict_fail_closed")
        );
    }

    // ── violation message content ─────────────────────────────────────────────

    #[test]
    fn tier1_approval_violation_message_references_tier1() {
        let d = decision(AuthorityTier::Tier1, true, true, vec![]);
        let report = verify_decision(&d);
        let v = report
            .violations
            .iter()
            .find(|v| v.rule == "tier_approval_relation")
            .unwrap();
        assert!(!v.message.is_empty());
        assert!(v.message.contains("Tier1") || v.message.contains("autonomous"));
    }

    #[test]
    fn conflict_fail_closed_violation_message_mentions_blocked() {
        let d = decision(AuthorityTier::Tier2, true, true, vec!["blocker".into()]);
        let report = verify_decision(&d);
        let v = report
            .violations
            .iter()
            .find(|v| v.rule == "conflict_fail_closed")
            .unwrap();
        assert!(!v.message.is_empty());
    }
}
