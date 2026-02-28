//! Equity grant record (stored as `cap-table/grants/{grant_id}.json`).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::error::EquityError;
use super::types::{GrantStatus, GrantType, RecipientType, ShareCount, VotingRights};
use crate::domain::ids::{
    AgentId, ContactId, EntityId, EquityGrantId, FilingId, ShareClassId,
};

/// Validate data-integrity invariants shared by constructors and deserialization.
fn validate_grant(share_count: &ShareCount, recipient_name: &str) -> Result<(), EquityError> {
    if share_count.raw() <= 0 {
        return Err(EquityError::Validation("share_count must be positive".into()));
    }
    if recipient_name.is_empty() {
        return Err(EquityError::Validation("recipient_name must not be empty".into()));
    }
    Ok(())
}

// ── Raw mirror for deserialization ──────────────────────────────────────

#[derive(Deserialize)]
struct RawEquityGrant {
    grant_id: EquityGrantId,
    entity_id: EntityId,
    share_class_id: ShareClassId,
    issuance_id: String,
    recipient_name: String,
    recipient_type: RecipientType,
    grant_type: Option<GrantType>,
    share_count: ShareCount,
    board_approval_reference: Option<String>,
    status: GrantStatus,
    contact_id: Option<ContactId>,
    agent_id: Option<AgentId>,
    entity_investor_id: Option<EntityId>,
    voting_rights: VotingRights,
    issued_at: DateTime<Utc>,
    created_at: DateTime<Utc>,
}

impl TryFrom<RawEquityGrant> for EquityGrant {
    type Error = EquityError;

    fn try_from(raw: RawEquityGrant) -> Result<Self, Self::Error> {
        validate_grant(&raw.share_count, &raw.recipient_name)?;
        Ok(EquityGrant {
            grant_id: raw.grant_id,
            entity_id: raw.entity_id,
            share_class_id: raw.share_class_id,
            issuance_id: raw.issuance_id,
            recipient_name: raw.recipient_name,
            recipient_type: raw.recipient_type,
            grant_type: raw.grant_type,
            share_count: raw.share_count,
            board_approval_reference: raw.board_approval_reference,
            status: raw.status,
            contact_id: raw.contact_id,
            agent_id: raw.agent_id,
            entity_investor_id: raw.entity_investor_id,
            voting_rights: raw.voting_rights,
            issued_at: raw.issued_at,
            created_at: raw.created_at,
        })
    }
}

// ── EquityGrant ─────────────────────────────────────────────────────────

/// An equity grant issued to a recipient.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(try_from = "RawEquityGrant")]
pub struct EquityGrant {
    grant_id: EquityGrantId,
    entity_id: EntityId,
    share_class_id: ShareClassId,
    issuance_id: String,
    recipient_name: String,
    recipient_type: RecipientType,
    grant_type: Option<GrantType>,
    share_count: ShareCount,
    board_approval_reference: Option<String>,
    status: GrantStatus,
    contact_id: Option<ContactId>,
    agent_id: Option<AgentId>,
    entity_investor_id: Option<EntityId>,
    voting_rights: VotingRights,
    issued_at: DateTime<Utc>,
    created_at: DateTime<Utc>,
}

impl EquityGrant {
    /// Create a new equity grant.
    ///
    /// Returns `Err` if `share_count` is not positive or `recipient_name` is empty.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        grant_id: EquityGrantId,
        entity_id: EntityId,
        share_class_id: ShareClassId,
        issuance_id: String,
        recipient_name: String,
        recipient_type: RecipientType,
        grant_type: Option<GrantType>,
        share_count: ShareCount,
        board_approval_reference: Option<String>,
        contact_id: Option<ContactId>,
        agent_id: Option<AgentId>,
        entity_investor_id: Option<EntityId>,
        voting_rights: VotingRights,
    ) -> Result<Self, EquityError> {
        validate_grant(&share_count, &recipient_name)?;
        let now = Utc::now();
        Ok(Self {
            grant_id,
            entity_id,
            share_class_id,
            issuance_id,
            recipient_name,
            recipient_type,
            grant_type,
            share_count,
            board_approval_reference,
            status: GrantStatus::Issued,
            contact_id,
            agent_id,
            entity_investor_id,
            voting_rights,
            issued_at: now,
            created_at: now,
        })
    }

    /// Create a formation grant (issued at formation time).
    ///
    /// Returns `Err` if `share_count` is not positive or `recipient_name` is empty.
    #[allow(clippy::too_many_arguments)]
    pub fn formation_grant(
        entity_id: EntityId,
        share_class_id: ShareClassId,
        filing_id: FilingId,
        recipient_name: String,
        recipient_type: RecipientType,
        share_count: ShareCount,
        agent_id: Option<AgentId>,
        entity_investor_id: Option<EntityId>,
    ) -> Result<Self, EquityError> {
        validate_grant(&share_count, &recipient_name)?;
        let now = Utc::now();
        Ok(Self {
            grant_id: EquityGrantId::new(),
            entity_id,
            share_class_id,
            issuance_id: format!("formation-{}", filing_id),
            recipient_name,
            recipient_type,
            grant_type: None,
            share_count,
            board_approval_reference: Some(format!("filing-{}", filing_id)),
            status: GrantStatus::Issued,
            contact_id: None,
            agent_id,
            entity_investor_id,
            voting_rights: VotingRights::Granted,
            issued_at: now,
            created_at: now,
        })
    }

    /// Reduce share count (used during transfers).
    pub fn reduce_shares(&mut self, count: ShareCount) -> Result<(), EquityError> {
        if count.raw() > self.share_count.raw() {
            return Err(EquityError::InsufficientShares {
                available: self.share_count,
                requested: count,
            });
        }
        self.share_count = self.share_count - count;
        Ok(())
    }

    /// Advance grant status. Validates the transition.
    ///
    /// Valid transitions:
    ///   Issued -> Vested | Forfeited | Cancelled
    ///   Vested -> Exercised | Forfeited
    ///   Exercised, Forfeited, Cancelled are terminal
    pub fn set_status(&mut self, to: GrantStatus) -> Result<(), EquityError> {
        let valid = matches!(
            (self.status, to),
            (GrantStatus::Issued, GrantStatus::Vested)
            | (GrantStatus::Issued, GrantStatus::Forfeited)
            | (GrantStatus::Issued, GrantStatus::Cancelled)
            | (GrantStatus::Vested, GrantStatus::Exercised)
            | (GrantStatus::Vested, GrantStatus::Forfeited)
        );
        if !valid {
            return Err(EquityError::InvalidGrantTransition {
                from: self.status,
                to,
            });
        }
        self.status = to;
        Ok(())
    }

    pub fn grant_id(&self) -> EquityGrantId {
        self.grant_id
    }

    pub fn entity_id(&self) -> EntityId {
        self.entity_id
    }

    pub fn share_class_id(&self) -> ShareClassId {
        self.share_class_id
    }

    pub fn issuance_id(&self) -> &str {
        &self.issuance_id
    }

    pub fn recipient_name(&self) -> &str {
        &self.recipient_name
    }

    pub fn recipient_type(&self) -> RecipientType {
        self.recipient_type
    }

    pub fn grant_type(&self) -> Option<GrantType> {
        self.grant_type
    }

    pub fn share_count(&self) -> ShareCount {
        self.share_count
    }

    pub fn board_approval_reference(&self) -> Option<&str> {
        self.board_approval_reference.as_deref()
    }

    pub fn status(&self) -> GrantStatus {
        self.status
    }

    pub fn contact_id(&self) -> Option<ContactId> {
        self.contact_id
    }

    pub fn agent_id(&self) -> Option<AgentId> {
        self.agent_id
    }

    pub fn entity_investor_id(&self) -> Option<EntityId> {
        self.entity_investor_id
    }

    pub fn voting_rights(&self) -> VotingRights {
        self.voting_rights
    }

    pub fn issued_at(&self) -> DateTime<Utc> {
        self.issued_at
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_grant() -> EquityGrant {
        EquityGrant::new(
            EquityGrantId::new(),
            EntityId::new(),
            ShareClassId::new(),
            "ISS-001".to_string(),
            "Alice".to_string(),
            RecipientType::NaturalPerson,
            Some(GrantType::CommonStock),
            ShareCount::new(1000),
            None,
            None,
            None,
            None,
            VotingRights::Granted,
        )
        .unwrap()
    }

    #[test]
    fn new_grant() {
        let g = make_grant();
        assert_eq!(g.status(), GrantStatus::Issued);
        assert_eq!(g.share_count().raw(), 1000);
        assert_eq!(g.recipient_name(), "Alice");
    }

    #[test]
    fn formation_grant() {
        let g = EquityGrant::formation_grant(
            EntityId::new(),
            ShareClassId::new(),
            FilingId::new(),
            "Bob".to_string(),
            RecipientType::NaturalPerson,
            ShareCount::new(5000),
            None,
            None,
        )
        .unwrap();
        assert_eq!(g.status(), GrantStatus::Issued);
        assert_eq!(g.share_count().raw(), 5000);
        assert!(g.issuance_id().starts_with("formation-"));
        assert_eq!(g.voting_rights(), VotingRights::Granted);
    }

    #[test]
    fn reduce_shares() {
        let mut g = make_grant();
        g.reduce_shares(ShareCount::new(300)).unwrap();
        assert_eq!(g.share_count().raw(), 700);
    }

    #[test]
    fn set_status_valid_transitions() {
        let mut g = make_grant();
        assert_eq!(g.status(), GrantStatus::Issued);
        g.set_status(GrantStatus::Vested).unwrap();
        assert_eq!(g.status(), GrantStatus::Vested);
        g.set_status(GrantStatus::Exercised).unwrap();
        assert_eq!(g.status(), GrantStatus::Exercised);
    }

    #[test]
    fn set_status_invalid_transition() {
        let mut g = make_grant();
        // Can't go directly from Issued to Exercised
        let result = g.set_status(GrantStatus::Exercised);
        assert!(result.is_err());
    }

    #[test]
    fn set_status_terminal() {
        let mut g = make_grant();
        g.set_status(GrantStatus::Cancelled).unwrap();
        // Cancelled is terminal, can't transition further
        let result = g.set_status(GrantStatus::Issued);
        assert!(result.is_err());
    }

    #[test]
    fn reduce_shares_error() {
        let mut g = make_grant();
        let result = g.reduce_shares(ShareCount::new(2000));
        assert!(result.is_err());
        // Share count unchanged
        assert_eq!(g.share_count().raw(), 1000);
    }

    #[test]
    fn serde_roundtrip() {
        let g = make_grant();
        let json = serde_json::to_string(&g).unwrap();
        let parsed: EquityGrant = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.grant_id(), g.grant_id());
        assert_eq!(parsed.share_count(), g.share_count());
    }

    #[test]
    fn deserialize_rejects_zero_shares() {
        let g = make_grant();
        let mut json: serde_json::Value = serde_json::to_value(&g).unwrap();
        json["share_count"] = serde_json::json!(0);
        let result: Result<EquityGrant, _> = serde_json::from_value(json);
        assert!(result.is_err());
    }

    #[test]
    fn deserialize_rejects_empty_recipient() {
        let g = make_grant();
        let mut json: serde_json::Value = serde_json::to_value(&g).unwrap();
        json["recipient_name"] = serde_json::json!("");
        let result: Result<EquityGrant, _> = serde_json::from_value(json);
        assert!(result.is_err());
    }
}
