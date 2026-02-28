//! Capability scopes for API authorization.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fmt;

/// A single capability scope that can be granted to an API key or token.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scope {
    // ── Formation ─────────────────────────────────────────────────────
    FormationCreate,
    FormationRead,
    FormationSign,

    // ── Equity ────────────────────────────────────────────────────────
    EquityRead,
    EquityWrite,
    EquityTransfer,

    // ── Governance ────────────────────────────────────────────────────
    GovernanceRead,
    GovernanceWrite,
    GovernanceVote,

    // ── Treasury ──────────────────────────────────────────────────────
    TreasuryRead,
    TreasuryWrite,
    TreasuryApprove,

    // ── Contacts ──────────────────────────────────────────────────────
    ContactsRead,
    ContactsWrite,

    // ── Execution ─────────────────────────────────────────────────────
    ExecutionRead,
    ExecutionWrite,

    // ── Branch management ─────────────────────────────────────────────
    BranchCreate,
    BranchMerge,
    BranchDelete,

    // ── Admin ─────────────────────────────────────────────────────────
    Admin,

    // ── Internal service-to-service ──────────────────────────────────
    InternalWorkerRead,
    InternalWorkerWrite,
    SecretsManage,

    // ── Wildcard — all permissions ────────────────────────────────────
    All,
}

impl Scope {
    /// Returns `true` if `self` satisfies the `required` scope.
    ///
    /// `All` satisfies any scope. Otherwise, an exact match is required.
    pub fn satisfies(self, required: Scope) -> bool {
        self == Scope::All || self == required
    }
}

impl fmt::Display for Scope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Use the serde snake_case representation.
        let json = serde_json::to_string(self).unwrap_or_else(|_| format!("{self:?}"));
        // Strip surrounding quotes from JSON string.
        let trimmed = json.trim_matches('"');
        write!(f, "{trimmed}")
    }
}

// ── ScopeSet ──────────────────────────────────────────────────────────────

/// A set of scopes for efficient membership checks.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ScopeSet(HashSet<Scope>);

impl ScopeSet {
    /// Create an empty scope set.
    pub fn empty() -> Self {
        Self(HashSet::new())
    }

    /// Create a scope set from a `Vec<Scope>`.
    pub fn from_vec(scopes: Vec<Scope>) -> Self {
        Self(scopes.into_iter().collect())
    }

    /// Returns `true` if the set satisfies the required scope.
    ///
    /// Any scope in the set that satisfies the required scope is sufficient.
    pub fn has(&self, required: Scope) -> bool {
        self.0.iter().any(|s| s.satisfies(required))
    }

    /// Returns `true` if the set satisfies at least one of the required scopes.
    pub fn has_any(&self, required: &[Scope]) -> bool {
        required.iter().any(|r| self.has(*r))
    }

    /// Returns a reference to the inner set.
    pub fn inner(&self) -> &HashSet<Scope> {
        &self.0
    }

    /// Converts the set into a sorted `Vec<Scope>` (for deterministic serialization).
    pub fn to_vec(&self) -> Vec<Scope> {
        let mut v: Vec<Scope> = self.0.iter().copied().collect();
        v.sort_by(|a, b| format!("{a}").cmp(&format!("{b}")));
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_satisfies_any_scope() {
        assert!(Scope::All.satisfies(Scope::FormationCreate));
        assert!(Scope::All.satisfies(Scope::Admin));
        assert!(Scope::All.satisfies(Scope::TreasuryApprove));
        assert!(Scope::All.satisfies(Scope::All));
    }

    #[test]
    fn exact_match_satisfies() {
        assert!(Scope::FormationCreate.satisfies(Scope::FormationCreate));
        assert!(Scope::Admin.satisfies(Scope::Admin));
    }

    #[test]
    fn non_all_does_not_satisfy_different_scope() {
        assert!(!Scope::FormationCreate.satisfies(Scope::FormationRead));
        assert!(!Scope::EquityRead.satisfies(Scope::EquityWrite));
        assert!(!Scope::Admin.satisfies(Scope::All));
    }

    #[test]
    fn scope_set_has() {
        let set = ScopeSet::from_vec(vec![Scope::FormationCreate, Scope::EquityRead]);
        assert!(set.has(Scope::FormationCreate));
        assert!(set.has(Scope::EquityRead));
        assert!(!set.has(Scope::Admin));
    }

    #[test]
    fn scope_set_has_with_all() {
        let set = ScopeSet::from_vec(vec![Scope::All]);
        assert!(set.has(Scope::FormationCreate));
        assert!(set.has(Scope::Admin));
        assert!(set.has(Scope::TreasuryApprove));
    }

    #[test]
    fn scope_set_has_any() {
        let set = ScopeSet::from_vec(vec![Scope::FormationCreate]);
        assert!(set.has_any(&[Scope::FormationCreate, Scope::EquityRead]));
        assert!(!set.has_any(&[Scope::EquityRead, Scope::Admin]));
    }

    #[test]
    fn scope_set_empty_has_nothing() {
        let set = ScopeSet::empty();
        assert!(!set.has(Scope::Admin));
        assert!(!set.has_any(&[Scope::All]));
    }

    #[test]
    fn scope_serde_roundtrip() {
        let scope = Scope::TreasuryApprove;
        let json = serde_json::to_string(&scope).expect("serialize Scope");
        assert_eq!(json, "\"treasury_approve\"");
        let parsed: Scope = serde_json::from_str(&json).expect("deserialize Scope");
        assert_eq!(scope, parsed);
    }

    #[test]
    fn scope_set_serde_roundtrip() {
        let set = ScopeSet::from_vec(vec![Scope::FormationCreate, Scope::EquityRead]);
        let json = serde_json::to_string(&set).expect("serialize ScopeSet");
        let parsed: ScopeSet = serde_json::from_str(&json).expect("deserialize ScopeSet");
        assert!(parsed.has(Scope::FormationCreate));
        assert!(parsed.has(Scope::EquityRead));
        assert!(!parsed.has(Scope::Admin));
    }

    #[test]
    fn scope_display() {
        assert_eq!(Scope::FormationCreate.to_string(), "formation_create");
        assert_eq!(Scope::All.to_string(), "all");
        assert_eq!(Scope::BranchMerge.to_string(), "branch_merge");
    }
}
