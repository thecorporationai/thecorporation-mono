//! Policy decision — the result of evaluating a governance policy for an action.

use serde::{Deserialize, Serialize};

use super::capability::AuthorityTier;

// ── PolicyDecision ────────────────────────────────────────────────────────────

/// The evaluated outcome of a governance policy check for a given action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyDecision {
    /// The tier at which this action is classified.
    pub tier: AuthorityTier,
    /// Whether the action is permitted at all.
    pub allowed: bool,
    /// Whether an explicit approval workflow must be completed before acting.
    pub requires_approval: bool,
    /// Hard blockers that prevent the action regardless of approvals.
    pub blockers: Vec<String>,
    /// Reasons why the decision was escalated to a higher tier or denied.
    pub escalation_reasons: Vec<String>,
    /// Identifier of the policy source that produced this decision
    /// (e.g. a rule ID or policy document reference).
    pub effective_source: Option<String>,
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tier1_allowed_no_approval() {
        let d = PolicyDecision {
            tier: AuthorityTier::Tier1,
            allowed: true,
            requires_approval: false,
            blockers: vec![],
            escalation_reasons: vec![],
            effective_source: Some("rule:payroll_auto".into()),
        };
        assert!(d.allowed);
        assert!(!d.requires_approval);
    }

    #[test]
    fn blocked_action_not_allowed() {
        let d = PolicyDecision {
            tier: AuthorityTier::Tier3,
            allowed: false,
            requires_approval: true,
            blockers: vec!["entity_dissolved".into()],
            escalation_reasons: vec!["dissolution_pending".into()],
            effective_source: None,
        };
        assert!(!d.allowed);
        assert!(!d.blockers.is_empty());
    }
}
