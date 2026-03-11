//! Governance body record (stored as `governance/bodies/{body_id}.json`).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::error::GovernanceError;
use super::types::{BodyStatus, BodyType, QuorumThreshold, VotingMethod};
use crate::domain::ids::{EntityId, GovernanceBodyId};

/// Maximum length for a governance body name.
const MAX_BODY_NAME_LEN: usize = 200;

/// A governance body (e.g., board of directors, LLC member vote).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceBody {
    body_id: GovernanceBodyId,
    entity_id: EntityId,
    body_type: BodyType,
    name: String,
    quorum_rule: QuorumThreshold,
    voting_method: VotingMethod,
    status: BodyStatus,
    created_at: DateTime<Utc>,
}

impl GovernanceBody {
    /// Create a new governance body.
    pub fn new(
        body_id: GovernanceBodyId,
        entity_id: EntityId,
        body_type: BodyType,
        name: String,
        quorum_rule: QuorumThreshold,
        voting_method: VotingMethod,
    ) -> Result<Self, GovernanceError> {
        let trimmed = name.trim();
        if trimmed.is_empty() || trimmed.len() > MAX_BODY_NAME_LEN {
            return Err(GovernanceError::Validation(format!(
                "body name must be between 1 and {MAX_BODY_NAME_LEN} characters, got {}",
                trimmed.len()
            )));
        }
        if trimmed.contains('<')
            || trimmed.contains('>')
            || trimmed.contains("{{")
            || trimmed.contains("}}")
            || trimmed.chars().any(|ch| ch == '\n' || ch == '\r')
        {
            return Err(GovernanceError::Validation(
                "body name cannot contain markup, template syntax, or newlines".to_owned(),
            ));
        }
        Ok(Self {
            body_id,
            entity_id,
            body_type,
            name: trimmed.to_owned(),
            quorum_rule,
            voting_method,
            status: BodyStatus::Active,
            created_at: Utc::now(),
        })
    }

    /// Deactivate this governance body.
    pub fn deactivate(&mut self) {
        self.status = BodyStatus::Inactive;
    }

    // ── Accessors ────────────────────────────────────────────────────────

    pub fn body_id(&self) -> GovernanceBodyId {
        self.body_id
    }

    pub fn entity_id(&self) -> EntityId {
        self.entity_id
    }

    pub fn body_type(&self) -> BodyType {
        self.body_type
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn quorum_rule(&self) -> QuorumThreshold {
        self.quorum_rule
    }

    pub fn voting_method(&self) -> VotingMethod {
        self.voting_method
    }

    pub fn status(&self) -> BodyStatus {
        self.status
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_body() -> GovernanceBody {
        GovernanceBody::new(
            GovernanceBodyId::new(),
            EntityId::new(),
            BodyType::BoardOfDirectors,
            "Board of Directors".into(),
            QuorumThreshold::Majority,
            VotingMethod::PerCapita,
        )
        .unwrap()
    }

    #[test]
    fn new_body_defaults_to_active() {
        let b = make_body();
        assert_eq!(b.status(), BodyStatus::Active);
        assert_eq!(b.name(), "Board of Directors");
    }

    #[test]
    fn deactivate_changes_status() {
        let mut b = make_body();
        b.deactivate();
        assert_eq!(b.status(), BodyStatus::Inactive);
    }

    #[test]
    fn rejects_empty_name() {
        let result = GovernanceBody::new(
            GovernanceBodyId::new(),
            EntityId::new(),
            BodyType::LlcMemberVote,
            "".into(),
            QuorumThreshold::Unanimous,
            VotingMethod::PerUnit,
        );
        assert!(result.is_err());
    }

    #[test]
    fn rejects_oversized_name() {
        let long = "X".repeat(201);
        let result = GovernanceBody::new(
            GovernanceBodyId::new(),
            EntityId::new(),
            BodyType::LlcMemberVote,
            long,
            QuorumThreshold::Unanimous,
            VotingMethod::PerUnit,
        );
        assert!(result.is_err());
    }

    #[test]
    fn rejects_markup_and_template_name() {
        let result = GovernanceBody::new(
            GovernanceBodyId::new(),
            EntityId::new(),
            BodyType::BoardOfDirectors,
            "<script>{{7*7}}</script>".into(),
            QuorumThreshold::Majority,
            VotingMethod::PerCapita,
        );
        assert!(result.is_err());
    }

    #[test]
    fn serde_roundtrip() {
        let b = make_body();
        let json = serde_json::to_string(&b).unwrap();
        let parsed: GovernanceBody = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.body_id(), b.body_id());
        assert_eq!(parsed.name(), b.name());
        assert_eq!(parsed.status(), b.status());
    }
}
