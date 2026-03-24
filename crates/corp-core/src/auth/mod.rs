//! Authentication and authorization primitives.
//!
//! This module contains:
//! - [`Scope`] — a discrete capability that can be granted to a principal.
//! - [`ScopeSet`] — an owned, checked collection of scopes.
//! - [`PrincipalType`] — discriminates between human users, internal workers,
//!   and agents.
//! - [`Claims`] — the JWT payload used throughout the platform.

use std::collections::HashSet;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::ids::{ContactId, EntityId, WorkspaceId};

// ── Scope ─────────────────────────────────────────────────────────────────────

/// A discrete capability.  Scopes are serialized as lowercase kebab-case
/// strings (e.g., `"formation-create"`).
///
/// `Scope::All` is a wildcard: [`Scope::satisfies`] returns `true` for any
/// `required` value when `self` is `All`.
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Scope {
    // Formation
    FormationCreate,
    FormationRead,
    FormationSign,

    // Equity
    EquityRead,
    EquityWrite,
    EquityTransfer,

    // Governance
    GovernanceRead,
    GovernanceWrite,
    GovernanceVote,

    // Treasury
    TreasuryRead,
    TreasuryWrite,
    TreasuryApprove,

    // Contacts
    ContactsRead,
    ContactsWrite,

    // Execution
    ExecutionRead,
    ExecutionWrite,

    // Services
    ServicesRead,
    ServicesWrite,

    // Agents
    AgentsRead,
    AgentsWrite,

    // Work items
    WorkItemsRead,
    WorkItemsWrite,

    // Compliance
    ComplianceRead,
    ComplianceWrite,

    // Branch management
    BranchCreate,
    BranchMerge,
    BranchDelete,

    // Git
    GitRead,
    GitWrite,

    // Platform / internal
    Admin,
    InternalWorkerRead,
    InternalWorkerWrite,
    SecretsManage,

    // Wildcard — satisfies every scope check.
    All,
}

impl Scope {
    /// Returns `true` when this scope satisfies a `required` scope check.
    ///
    /// [`Scope::All`] satisfies every check.  Any other scope satisfies only
    /// itself.
    #[inline]
    pub fn satisfies(&self, required: &Scope) -> bool {
        matches!(self, Scope::All) || self == required
    }
}

impl fmt::Display for Scope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Delegate to the serde kebab-case representation for a canonical
        // human-readable form, stripping the surrounding quotes.
        let s = serde_json::to_string(self)
            .unwrap_or_else(|_| "\"unknown\"".to_owned());
        write!(f, "{}", s.trim_matches('"'))
    }
}

// ── ScopeSet ──────────────────────────────────────────────────────────────────

/// An owned, de-duplicated collection of [`Scope`]s.
///
/// Provides convenience methods for checking whether a required scope is
/// covered by any scope in the set.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ScopeSet(HashSet<Scope>);

impl ScopeSet {
    /// An empty set — no capabilities.
    pub fn empty() -> Self {
        Self(HashSet::new())
    }

    /// A set containing only [`Scope::All`] — satisfies every check.
    pub fn all() -> Self {
        let mut s = HashSet::new();
        s.insert(Scope::All);
        Self(s)
    }

    /// Build a `ScopeSet` from a `Vec`, de-duplicating as needed.
    pub fn from_vec(scopes: Vec<Scope>) -> Self {
        Self(scopes.into_iter().collect())
    }

    /// Returns `true` if any scope in this set satisfies `required`.
    pub fn has(&self, required: &Scope) -> bool {
        self.0.iter().any(|s| s.satisfies(required))
    }

    /// Returns `true` if any scope in this set satisfies at least one of
    /// `required`.
    pub fn has_any(&self, required: &[Scope]) -> bool {
        required.iter().any(|r| self.has(r))
    }

    /// Returns `true` if the set contains no scopes.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Return a new `ScopeSet` that is the union of `self` and `other`.
    pub fn union(&self, other: &ScopeSet) -> ScopeSet {
        Self(self.0.union(&other.0).copied().collect())
    }

    /// Return the contained scopes as a `Vec` (unordered).
    pub fn to_vec(&self) -> Vec<Scope> {
        self.0.iter().copied().collect()
    }
}

impl Default for ScopeSet {
    fn default() -> Self {
        Self::empty()
    }
}

// ── PrincipalType ─────────────────────────────────────────────────────────────

/// Discriminates between the three kinds of principals that can hold JWT
/// claims on this platform.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrincipalType {
    /// A human user authenticated via the web or CLI.
    User,
    /// An internal platform worker (cron jobs, background processors).
    InternalWorker,
    /// An autonomous agent acting on behalf of a workspace.
    Agent,
}

// ── Claims ────────────────────────────────────────────────────────────────────

/// JWT payload used throughout the platform.
///
/// The `scopes` field carries a flat `Vec<Scope>` so it can be embedded
/// directly in a JWT as a JSON array.  Use [`ScopeSet::from_vec`] when you
/// need membership queries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Subject — typically the user / worker / agent UUID as a string.
    pub sub: String,

    /// The workspace this token is scoped to.
    pub workspace_id: WorkspaceId,

    /// The primary legal entity, if this token is entity-scoped.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_id: Option<EntityId>,

    /// The contact record associated with this principal, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contact_id: Option<ContactId>,

    /// All entities this principal is authorized to act on behalf of, for
    /// tokens that span multiple entities.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_ids: Option<Vec<EntityId>>,

    /// Discriminates between humans, workers, and agents.
    pub principal_type: PrincipalType,

    /// The capabilities this token grants.
    pub scopes: Vec<Scope>,

    /// Issued-at timestamp (Unix seconds).
    pub iat: i64,

    /// Expiry timestamp (Unix seconds).
    pub exp: i64,
}

impl Claims {
    /// Returns `true` if these claims include a scope that satisfies
    /// `required`.
    pub fn has_scope(&self, required: &Scope) -> bool {
        ScopeSet::from_vec(self.scopes.clone()).has(required)
    }

    /// Return a [`ScopeSet`] view of the scopes embedded in these claims.
    pub fn scope_set(&self) -> ScopeSet {
        ScopeSet::from_vec(self.scopes.clone())
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Scope::satisfies ──────────────────────────────────────────────────────

    #[test]
    fn scope_satisfies_self() {
        assert!(Scope::FormationCreate.satisfies(&Scope::FormationCreate));
    }

    #[test]
    fn scope_does_not_satisfy_other() {
        assert!(!Scope::FormationCreate.satisfies(&Scope::FormationRead));
    }

    #[test]
    fn all_satisfies_everything() {
        let all_variants = [
            Scope::FormationCreate,
            Scope::FormationRead,
            Scope::FormationSign,
            Scope::EquityRead,
            Scope::EquityWrite,
            Scope::EquityTransfer,
            Scope::GovernanceRead,
            Scope::GovernanceWrite,
            Scope::GovernanceVote,
            Scope::TreasuryRead,
            Scope::TreasuryWrite,
            Scope::TreasuryApprove,
            Scope::ContactsRead,
            Scope::ContactsWrite,
            Scope::ExecutionRead,
            Scope::ExecutionWrite,
            Scope::ServicesRead,
            Scope::ServicesWrite,
            Scope::AgentsRead,
            Scope::AgentsWrite,
            Scope::WorkItemsRead,
            Scope::WorkItemsWrite,
            Scope::ComplianceRead,
            Scope::ComplianceWrite,
            Scope::BranchCreate,
            Scope::BranchMerge,
            Scope::BranchDelete,
            Scope::GitRead,
            Scope::GitWrite,
            Scope::Admin,
            Scope::InternalWorkerRead,
            Scope::InternalWorkerWrite,
            Scope::SecretsManage,
            Scope::All,
        ];
        for v in &all_variants {
            assert!(Scope::All.satisfies(v), "All should satisfy {:?}", v);
        }
    }

    // ── Scope serialization ───────────────────────────────────────────────────

    #[test]
    fn scope_serializes_to_kebab_case() {
        let s = serde_json::to_string(&Scope::FormationCreate).unwrap();
        assert_eq!(s, r#""formation-create""#);

        let s = serde_json::to_string(&Scope::InternalWorkerRead).unwrap();
        assert_eq!(s, r#""internal-worker-read""#);

        let s = serde_json::to_string(&Scope::All).unwrap();
        assert_eq!(s, r#""all""#);
    }

    #[test]
    fn scope_deserializes_from_kebab_case() {
        let s: Scope = serde_json::from_str(r#""governance-vote""#).unwrap();
        assert_eq!(s, Scope::GovernanceVote);
    }

    #[test]
    fn scope_display_matches_serde() {
        assert_eq!(Scope::TreasuryApprove.to_string(), "treasury-approve");
        assert_eq!(Scope::SecretsManage.to_string(), "secrets-manage");
        assert_eq!(Scope::All.to_string(), "all");
    }

    // ── ScopeSet ──────────────────────────────────────────────────────────────

    #[test]
    fn empty_scope_set_has_nothing() {
        let ss = ScopeSet::empty();
        assert!(ss.is_empty());
        assert!(!ss.has(&Scope::Admin));
    }

    #[test]
    fn all_scope_set_has_everything() {
        let ss = ScopeSet::all();
        assert!(ss.has(&Scope::FormationCreate));
        assert!(ss.has(&Scope::SecretsManage));
        assert!(ss.has(&Scope::All));
    }

    #[test]
    fn from_vec_deduplicates() {
        let ss =
            ScopeSet::from_vec(vec![Scope::GitRead, Scope::GitRead, Scope::GitWrite]);
        assert_eq!(ss.to_vec().len(), 2);
    }

    #[test]
    fn has_any_returns_true_on_first_match() {
        let ss = ScopeSet::from_vec(vec![Scope::TreasuryRead]);
        assert!(ss.has_any(&[Scope::FormationCreate, Scope::TreasuryRead]));
        assert!(!ss.has_any(&[Scope::FormationCreate, Scope::TreasuryWrite]));
    }

    #[test]
    fn union_combines_sets() {
        let a = ScopeSet::from_vec(vec![Scope::GitRead]);
        let b = ScopeSet::from_vec(vec![Scope::GitWrite]);
        let c = a.union(&b);
        assert!(c.has(&Scope::GitRead));
        assert!(c.has(&Scope::GitWrite));
    }

    #[test]
    fn scope_set_roundtrip_json() {
        let ss = ScopeSet::from_vec(vec![Scope::Admin, Scope::EquityRead]);
        let json = serde_json::to_string(&ss).unwrap();
        let de: ScopeSet = serde_json::from_str(&json).unwrap();
        assert!(de.has(&Scope::Admin));
        assert!(de.has(&Scope::EquityRead));
        assert!(!de.has(&Scope::EquityWrite));
    }

    // ── PrincipalType serialization ───────────────────────────────────────────

    #[test]
    fn principal_type_serialization() {
        assert_eq!(
            serde_json::to_string(&PrincipalType::InternalWorker).unwrap(),
            r#""internal_worker""#
        );
        assert_eq!(
            serde_json::to_string(&PrincipalType::Agent).unwrap(),
            r#""agent""#
        );
        assert_eq!(
            serde_json::to_string(&PrincipalType::User).unwrap(),
            r#""user""#
        );
    }

    // ── Claims ────────────────────────────────────────────────────────────────

    #[test]
    fn claims_roundtrip_json() {
        let ws_id = WorkspaceId::new();
        let claims = Claims {
            sub: "user-abc".to_owned(),
            workspace_id: ws_id,
            entity_id: None,
            contact_id: None,
            entity_ids: None,
            principal_type: PrincipalType::User,
            scopes: vec![Scope::FormationRead, Scope::GovernanceRead],
            iat: 1_700_000_000,
            exp: 1_700_003_600,
        };

        let json = serde_json::to_string(&claims).unwrap();
        let de: Claims = serde_json::from_str(&json).unwrap();

        assert_eq!(de.sub, "user-abc");
        assert_eq!(de.workspace_id, ws_id);
        assert!(de.entity_id.is_none());
        assert_eq!(de.principal_type, PrincipalType::User);
        assert!(de.has_scope(&Scope::FormationRead));
        assert!(!de.has_scope(&Scope::Admin));
    }

    #[test]
    fn claims_optional_fields_omitted_in_json() {
        let claims = Claims {
            sub: "worker-1".to_owned(),
            workspace_id: WorkspaceId::new(),
            entity_id: None,
            contact_id: None,
            entity_ids: None,
            principal_type: PrincipalType::InternalWorker,
            scopes: vec![Scope::InternalWorkerRead],
            iat: 0,
            exp: 9999,
        };
        let json = serde_json::to_string(&claims).unwrap();
        assert!(!json.contains("entity_id"));
        assert!(!json.contains("contact_id"));
        assert!(!json.contains("entity_ids"));
    }

    #[test]
    fn claims_with_all_optional_fields() {
        let ws_id = WorkspaceId::new();
        let entity_id = EntityId::new();
        let contact_id = ContactId::new();
        let extra_entity = EntityId::new();

        let claims = Claims {
            sub: "agent-xyz".to_owned(),
            workspace_id: ws_id,
            entity_id: Some(entity_id),
            contact_id: Some(contact_id),
            entity_ids: Some(vec![entity_id, extra_entity]),
            principal_type: PrincipalType::Agent,
            scopes: vec![Scope::All],
            iat: 100,
            exp: 200,
        };

        let json = serde_json::to_string(&claims).unwrap();
        let de: Claims = serde_json::from_str(&json).unwrap();

        assert_eq!(de.entity_id, Some(entity_id));
        assert_eq!(de.contact_id, Some(contact_id));
        assert_eq!(de.entity_ids.as_ref().map(|v| v.len()), Some(2));
        // All wildcard scope satisfies any check.
        assert!(de.has_scope(&Scope::SecretsManage));
        assert!(de.has_scope(&Scope::EquityTransfer));
    }

    #[test]
    fn scope_set_method_on_claims() {
        let claims = Claims {
            sub: "u".to_owned(),
            workspace_id: WorkspaceId::new(),
            entity_id: None,
            contact_id: None,
            entity_ids: None,
            principal_type: PrincipalType::User,
            scopes: vec![Scope::BranchCreate, Scope::BranchMerge],
            iat: 0,
            exp: 1,
        };
        let ss = claims.scope_set();
        assert!(ss.has(&Scope::BranchCreate));
        assert!(!ss.has(&Scope::BranchDelete));
    }

    // ── All 34 Scope variants serde roundtrips ────────────────────────────────

    #[test]
    fn all_scope_variants_serde_roundtrip() {
        let all_variants = [
            Scope::FormationCreate,
            Scope::FormationRead,
            Scope::FormationSign,
            Scope::EquityRead,
            Scope::EquityWrite,
            Scope::EquityTransfer,
            Scope::GovernanceRead,
            Scope::GovernanceWrite,
            Scope::GovernanceVote,
            Scope::TreasuryRead,
            Scope::TreasuryWrite,
            Scope::TreasuryApprove,
            Scope::ContactsRead,
            Scope::ContactsWrite,
            Scope::ExecutionRead,
            Scope::ExecutionWrite,
            Scope::ServicesRead,
            Scope::ServicesWrite,
            Scope::AgentsRead,
            Scope::AgentsWrite,
            Scope::WorkItemsRead,
            Scope::WorkItemsWrite,
            Scope::ComplianceRead,
            Scope::ComplianceWrite,
            Scope::BranchCreate,
            Scope::BranchMerge,
            Scope::BranchDelete,
            Scope::GitRead,
            Scope::GitWrite,
            Scope::Admin,
            Scope::InternalWorkerRead,
            Scope::InternalWorkerWrite,
            Scope::SecretsManage,
            Scope::All,
        ];
        assert_eq!(all_variants.len(), 34);
        for variant in &all_variants {
            let s = serde_json::to_string(variant).unwrap();
            let de: Scope = serde_json::from_str(&s).unwrap();
            assert_eq!(de, *variant, "roundtrip failed for {:?}", variant);
        }
    }

    #[test]
    fn all_scope_variants_kebab_case_values() {
        assert_eq!(serde_json::to_string(&Scope::FormationCreate).unwrap(), r#""formation-create""#);
        assert_eq!(serde_json::to_string(&Scope::FormationRead).unwrap(), r#""formation-read""#);
        assert_eq!(serde_json::to_string(&Scope::FormationSign).unwrap(), r#""formation-sign""#);
        assert_eq!(serde_json::to_string(&Scope::EquityRead).unwrap(), r#""equity-read""#);
        assert_eq!(serde_json::to_string(&Scope::EquityWrite).unwrap(), r#""equity-write""#);
        assert_eq!(serde_json::to_string(&Scope::EquityTransfer).unwrap(), r#""equity-transfer""#);
        assert_eq!(serde_json::to_string(&Scope::GovernanceRead).unwrap(), r#""governance-read""#);
        assert_eq!(serde_json::to_string(&Scope::GovernanceWrite).unwrap(), r#""governance-write""#);
        assert_eq!(serde_json::to_string(&Scope::GovernanceVote).unwrap(), r#""governance-vote""#);
        assert_eq!(serde_json::to_string(&Scope::TreasuryRead).unwrap(), r#""treasury-read""#);
        assert_eq!(serde_json::to_string(&Scope::TreasuryWrite).unwrap(), r#""treasury-write""#);
        assert_eq!(serde_json::to_string(&Scope::TreasuryApprove).unwrap(), r#""treasury-approve""#);
        assert_eq!(serde_json::to_string(&Scope::ContactsRead).unwrap(), r#""contacts-read""#);
        assert_eq!(serde_json::to_string(&Scope::ContactsWrite).unwrap(), r#""contacts-write""#);
        assert_eq!(serde_json::to_string(&Scope::ExecutionRead).unwrap(), r#""execution-read""#);
        assert_eq!(serde_json::to_string(&Scope::ExecutionWrite).unwrap(), r#""execution-write""#);
        assert_eq!(serde_json::to_string(&Scope::ServicesRead).unwrap(), r#""services-read""#);
        assert_eq!(serde_json::to_string(&Scope::ServicesWrite).unwrap(), r#""services-write""#);
        assert_eq!(serde_json::to_string(&Scope::AgentsRead).unwrap(), r#""agents-read""#);
        assert_eq!(serde_json::to_string(&Scope::AgentsWrite).unwrap(), r#""agents-write""#);
        assert_eq!(serde_json::to_string(&Scope::WorkItemsRead).unwrap(), r#""work-items-read""#);
        assert_eq!(serde_json::to_string(&Scope::WorkItemsWrite).unwrap(), r#""work-items-write""#);
        assert_eq!(serde_json::to_string(&Scope::ComplianceRead).unwrap(), r#""compliance-read""#);
        assert_eq!(serde_json::to_string(&Scope::ComplianceWrite).unwrap(), r#""compliance-write""#);
        assert_eq!(serde_json::to_string(&Scope::BranchCreate).unwrap(), r#""branch-create""#);
        assert_eq!(serde_json::to_string(&Scope::BranchMerge).unwrap(), r#""branch-merge""#);
        assert_eq!(serde_json::to_string(&Scope::BranchDelete).unwrap(), r#""branch-delete""#);
        assert_eq!(serde_json::to_string(&Scope::GitRead).unwrap(), r#""git-read""#);
        assert_eq!(serde_json::to_string(&Scope::GitWrite).unwrap(), r#""git-write""#);
        assert_eq!(serde_json::to_string(&Scope::Admin).unwrap(), r#""admin""#);
        assert_eq!(serde_json::to_string(&Scope::InternalWorkerRead).unwrap(), r#""internal-worker-read""#);
        assert_eq!(serde_json::to_string(&Scope::InternalWorkerWrite).unwrap(), r#""internal-worker-write""#);
        assert_eq!(serde_json::to_string(&Scope::SecretsManage).unwrap(), r#""secrets-manage""#);
        assert_eq!(serde_json::to_string(&Scope::All).unwrap(), r#""all""#);
    }

    // ── Scope::satisfies against All ──────────────────────────────────────────

    #[test]
    fn scope_satisfies_against_all_wildcard() {
        // Every scope satisfies itself when checked against All
        let all_variants = [
            Scope::FormationCreate, Scope::FormationRead, Scope::FormationSign,
            Scope::EquityRead, Scope::EquityWrite, Scope::EquityTransfer,
            Scope::GovernanceRead, Scope::GovernanceWrite, Scope::GovernanceVote,
            Scope::TreasuryRead, Scope::TreasuryWrite, Scope::TreasuryApprove,
            Scope::ContactsRead, Scope::ContactsWrite,
            Scope::ExecutionRead, Scope::ExecutionWrite,
            Scope::ServicesRead, Scope::ServicesWrite,
            Scope::AgentsRead, Scope::AgentsWrite,
            Scope::WorkItemsRead, Scope::WorkItemsWrite,
            Scope::ComplianceRead, Scope::ComplianceWrite,
            Scope::BranchCreate, Scope::BranchMerge, Scope::BranchDelete,
            Scope::GitRead, Scope::GitWrite,
            Scope::Admin, Scope::InternalWorkerRead, Scope::InternalWorkerWrite,
            Scope::SecretsManage, Scope::All,
        ];
        for v in &all_variants {
            assert!(Scope::All.satisfies(v), "All should satisfy {:?}", v);
            // Non-All scopes only satisfy themselves
            assert!(v.satisfies(v), "{:?} should satisfy itself", v);
        }
    }

    #[test]
    fn non_all_scope_does_not_satisfy_different_scope() {
        assert!(!Scope::TreasuryRead.satisfies(&Scope::TreasuryWrite));
        assert!(!Scope::Admin.satisfies(&Scope::SecretsManage));
        assert!(!Scope::GitRead.satisfies(&Scope::GitWrite));
        assert!(!Scope::FormationCreate.satisfies(&Scope::All));
    }

    // ── ScopeSet additional tests ─────────────────────────────────────────────

    #[test]
    fn scope_set_empty_has_nothing() {
        let ss = ScopeSet::empty();
        assert!(ss.is_empty());
        assert!(!ss.has(&Scope::Admin));
        assert!(!ss.has(&Scope::All));
    }

    #[test]
    fn scope_set_from_vec_single() {
        let ss = ScopeSet::from_vec(vec![Scope::TreasuryRead]);
        assert!(ss.has(&Scope::TreasuryRead));
        assert!(!ss.has(&Scope::TreasuryWrite));
        assert!(!ss.is_empty());
    }

    #[test]
    fn scope_set_default_is_empty() {
        let ss = ScopeSet::default();
        assert!(ss.is_empty());
    }

    #[test]
    fn scope_set_all_satisfies_everything() {
        let ss = ScopeSet::all();
        let all_variants = [
            Scope::FormationCreate, Scope::Admin, Scope::SecretsManage,
            Scope::EquityTransfer, Scope::GovernanceVote,
        ];
        for v in &all_variants {
            assert!(ss.has(v), "all() set should satisfy {:?}", v);
        }
        assert!(!ss.is_empty());
    }

    #[test]
    fn scope_set_has_any_empty_required() {
        let ss = ScopeSet::from_vec(vec![Scope::Admin]);
        assert!(!ss.has_any(&[]));
    }

    #[test]
    fn scope_set_has_any_multiple_required() {
        let ss = ScopeSet::from_vec(vec![Scope::TreasuryRead, Scope::TreasuryWrite]);
        assert!(ss.has_any(&[Scope::TreasuryRead, Scope::EquityRead]));
        assert!(ss.has_any(&[Scope::TreasuryWrite]));
        assert!(!ss.has_any(&[Scope::Admin, Scope::SecretsManage]));
    }

    #[test]
    fn scope_set_union_is_additive() {
        let a = ScopeSet::from_vec(vec![Scope::GitRead, Scope::Admin]);
        let b = ScopeSet::from_vec(vec![Scope::GitWrite, Scope::Admin]);
        let c = a.union(&b);
        assert!(c.has(&Scope::GitRead));
        assert!(c.has(&Scope::GitWrite));
        assert!(c.has(&Scope::Admin));
        // de-duplication: Admin appears once
        assert_eq!(c.to_vec().iter().filter(|&&s| s == Scope::Admin).count(), 1);
    }

    #[test]
    fn scope_set_to_vec_correct_length() {
        let ss = ScopeSet::from_vec(vec![Scope::GitRead, Scope::GitWrite, Scope::Admin]);
        assert_eq!(ss.to_vec().len(), 3);
    }

    // ── PrincipalType additional serde ────────────────────────────────────────

    #[test]
    fn all_principal_types_serde_roundtrip() {
        for pt in [PrincipalType::User, PrincipalType::InternalWorker, PrincipalType::Agent] {
            let s = serde_json::to_string(&pt).unwrap();
            let de: PrincipalType = serde_json::from_str(&s).unwrap();
            assert_eq!(de, pt);
        }
    }

    // ── Claims additional tests ───────────────────────────────────────────────

    #[test]
    fn claims_has_scope_with_all_wildcard() {
        let claims = Claims {
            sub: "super-agent".to_owned(),
            workspace_id: WorkspaceId::new(),
            entity_id: None,
            contact_id: None,
            entity_ids: None,
            principal_type: PrincipalType::Agent,
            scopes: vec![Scope::All],
            iat: 0,
            exp: 9999,
        };
        assert!(claims.has_scope(&Scope::TreasuryApprove));
        assert!(claims.has_scope(&Scope::SecretsManage));
        assert!(claims.has_scope(&Scope::FormationSign));
    }

    #[test]
    fn claims_has_scope_false_when_not_present() {
        let claims = Claims {
            sub: "limited-user".to_owned(),
            workspace_id: WorkspaceId::new(),
            entity_id: None,
            contact_id: None,
            entity_ids: None,
            principal_type: PrincipalType::User,
            scopes: vec![Scope::FormationRead],
            iat: 0,
            exp: 9999,
        };
        assert!(!claims.has_scope(&Scope::Admin));
        assert!(!claims.has_scope(&Scope::TreasuryWrite));
    }

    #[test]
    fn scope_set_json_roundtrip_all_scopes() {
        let all_scopes: Vec<Scope> = vec![
            Scope::Admin, Scope::GitRead, Scope::TreasuryApprove,
        ];
        let ss = ScopeSet::from_vec(all_scopes);
        let json = serde_json::to_string(&ss).unwrap();
        let de: ScopeSet = serde_json::from_str(&json).unwrap();
        assert!(de.has(&Scope::Admin));
        assert!(de.has(&Scope::GitRead));
        assert!(de.has(&Scope::TreasuryApprove));
        assert!(!de.has(&Scope::TreasuryRead));
    }
}
