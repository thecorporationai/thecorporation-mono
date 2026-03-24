//! The resolved identity for an authenticated request.
//!
//! A [`Principal`] is built from validated [`Claims`] and provides
//! scope-checking helpers used by route handlers and extractors.

use corp_core::auth::{Claims, PrincipalType, Scope, ScopeSet};
use corp_core::ids::{ContactId, EntityId, WorkspaceId};

use crate::error::AuthError;

/// The resolved identity attached to every authenticated request.
///
/// Constructed from validated [`Claims`] via [`Principal::from_claims`].
#[derive(Debug, Clone)]
pub struct Principal {
    /// The workspace this principal belongs to.
    pub workspace_id: WorkspaceId,

    /// The primary entity this principal acts on behalf of, if any.
    pub entity_id: Option<EntityId>,

    /// The contact record for this principal, if any.
    pub contact_id: Option<ContactId>,

    /// All entities this principal is authorized to act on behalf of.
    ///
    /// Populated from the `entity_ids` claim; falls back to a single-element
    /// vec containing `entity_id` when `entity_ids` is absent.
    pub entity_ids: Vec<EntityId>,

    /// Whether this is a human user, an internal worker, or an agent.
    pub principal_type: PrincipalType,

    /// The capability set granted by this principal's credential.
    pub scopes: ScopeSet,
}

impl Principal {
    /// Build a `Principal` from already-validated [`Claims`].
    pub fn from_claims(claims: Claims) -> Self {
        let scopes = ScopeSet::from_vec(claims.scopes.clone());

        // entity_ids: use the multi-entity claim when present; otherwise
        // promote entity_id to a single-element list.
        let entity_ids = match claims.entity_ids {
            Some(ids) => ids,
            None => claims.entity_id.into_iter().collect(),
        };

        Self {
            workspace_id: claims.workspace_id,
            entity_id: claims.entity_id,
            contact_id: claims.contact_id,
            entity_ids,
            principal_type: claims.principal_type,
            scopes,
        }
    }

    /// Returns `true` if this principal's scope set satisfies `scope`.
    #[inline]
    pub fn has_scope(&self, scope: &Scope) -> bool {
        self.scopes.has(scope)
    }

    /// Returns `Ok(())` when the required scope is present, or an
    /// [`AuthError::InsufficientScope`] error that includes the scope name.
    #[inline]
    pub fn require_scope(&self, scope: &Scope) -> Result<(), AuthError> {
        if self.has_scope(scope) {
            Ok(())
        } else {
            Err(AuthError::InsufficientScope(scope.to_string()))
        }
    }

    /// Return the most specific entity ID available.
    ///
    /// Returns `entity_id` if set; otherwise the first element of
    /// `entity_ids`; otherwise `None`.
    pub fn effective_entity_id(&self) -> Option<EntityId> {
        self.entity_id.or_else(|| self.entity_ids.first().copied())
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use corp_core::ids::WorkspaceId;

    fn make_claims_with_scopes(scopes: Vec<Scope>) -> Claims {
        Claims {
            sub: "u".to_owned(),
            workspace_id: WorkspaceId::new(),
            entity_id: None,
            contact_id: None,
            entity_ids: None,
            principal_type: PrincipalType::User,
            scopes,
            iat: 0,
            exp: i64::MAX,
        }
    }

    #[test]
    fn has_scope_matches() {
        let claims = make_claims_with_scopes(vec![Scope::FormationRead]);
        let p = Principal::from_claims(claims);
        assert!(p.has_scope(&Scope::FormationRead));
        assert!(!p.has_scope(&Scope::FormationCreate));
    }

    #[test]
    fn all_scope_satisfies_everything() {
        let claims = make_claims_with_scopes(vec![Scope::All]);
        let p = Principal::from_claims(claims);
        assert!(p.has_scope(&Scope::Admin));
        assert!(p.has_scope(&Scope::TreasuryWrite));
    }

    #[test]
    fn require_scope_ok() {
        let claims = make_claims_with_scopes(vec![Scope::GovernanceRead]);
        let p = Principal::from_claims(claims);
        assert!(p.require_scope(&Scope::GovernanceRead).is_ok());
    }

    #[test]
    fn require_scope_err_has_scope_name() {
        let claims = make_claims_with_scopes(vec![]);
        let p = Principal::from_claims(claims);
        match p.require_scope(&Scope::TreasuryWrite) {
            Err(AuthError::InsufficientScope(s)) => {
                assert!(s.contains("treasury-write"), "scope name: {s}");
            }
            other => panic!("expected InsufficientScope, got {:?}", other),
        }
    }

    #[test]
    fn effective_entity_id_uses_entity_id_first() {
        let eid = EntityId::new();
        let extra = EntityId::new();
        let mut claims = make_claims_with_scopes(vec![]);
        claims.entity_id = Some(eid);
        claims.entity_ids = Some(vec![extra]);
        let p = Principal::from_claims(claims);
        assert_eq!(p.effective_entity_id(), Some(eid));
    }

    #[test]
    fn effective_entity_id_falls_back_to_entity_ids() {
        let extra = EntityId::new();
        let mut claims = make_claims_with_scopes(vec![]);
        claims.entity_id = None;
        claims.entity_ids = Some(vec![extra]);
        let p = Principal::from_claims(claims);
        assert_eq!(p.effective_entity_id(), Some(extra));
    }

    #[test]
    fn effective_entity_id_none_when_absent() {
        let claims = make_claims_with_scopes(vec![]);
        let p = Principal::from_claims(claims);
        assert_eq!(p.effective_entity_id(), None);
    }

    #[test]
    fn entity_ids_promoted_from_entity_id_when_absent() {
        let eid = EntityId::new();
        let mut claims = make_claims_with_scopes(vec![]);
        claims.entity_id = Some(eid);
        claims.entity_ids = None;
        let p = Principal::from_claims(claims);
        assert_eq!(p.entity_ids, vec![eid]);
    }
}
