//! Decision lattice utilities for governance policy laws.

use crate::domain::execution::types::AuthorityTier;

use super::policy_engine::PolicyDecision;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecisionLatticePoint {
    tier_level: u8,
    denied: bool,
    requires_approval: bool,
    blockers: usize,
    escalation_reasons: usize,
    conflicts: usize,
}

impl DecisionLatticePoint {
    pub fn from_decision(decision: &PolicyDecision) -> Self {
        Self {
            tier_level: decision.tier.level(),
            denied: !decision.allowed,
            requires_approval: decision.requires_approval,
            blockers: decision.blockers.len(),
            escalation_reasons: decision.escalation_reasons.len(),
            conflicts: decision.precedence_conflicts.len(),
        }
    }

    /// Partial order for restrictiveness:
    /// `a <= b` means `b` is at least as restrictive as `a`.
    pub fn less_or_equal_restrictive(self, other: Self) -> bool {
        self.tier_level <= other.tier_level
            && (self.denied as u8) <= (other.denied as u8)
            && (self.requires_approval as u8) <= (other.requires_approval as u8)
            && self.blockers <= other.blockers
            && self.escalation_reasons <= other.escalation_reasons
            && self.conflicts <= other.conflicts
    }
}

pub fn is_monotone_restriction_step(before: &PolicyDecision, after: &PolicyDecision) -> bool {
    let a = DecisionLatticePoint::from_decision(before);
    let b = DecisionLatticePoint::from_decision(after);
    a.less_or_equal_restrictive(b)
}

pub fn has_valid_tier_approval_relation(decision: &PolicyDecision) -> bool {
    match decision.tier {
        AuthorityTier::Tier1 => !decision.requires_approval,
        AuthorityTier::Tier2 | AuthorityTier::Tier3 => decision.requires_approval,
    }
}
