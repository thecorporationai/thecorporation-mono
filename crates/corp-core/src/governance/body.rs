//! Governance body — the board, LLC member vote, or similar decision-making organ.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::ids::{EntityId, GovernanceBodyId};

use super::types::{BodyStatus, BodyType, QuorumThreshold, VotingMethod};

// ── Constants ─────────────────────────────────────────────────────────────────

/// Maximum byte length of a governance body name.
const NAME_MAX_LEN: usize = 200;

// ── Error ─────────────────────────────────────────────────────────────────────

/// Errors that can arise when constructing or mutating a [`GovernanceBody`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum GovernanceBodyError {
    #[error("body name exceeds maximum length of {NAME_MAX_LEN} characters")]
    NameTooLong,
    #[error("body name must not be empty")]
    NameEmpty,
    #[error("body name contains disallowed markup characters")]
    NameContainsMarkup,
}

// ── GovernanceBody ────────────────────────────────────────────────────────────

/// A decision-making organ of a corporate entity (board, member vote, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceBody {
    pub body_id: GovernanceBodyId,
    pub entity_id: EntityId,
    pub body_type: BodyType,
    /// Human-readable name. Max 200 characters, no markup.
    pub name: String,
    pub quorum_rule: QuorumThreshold,
    pub voting_method: VotingMethod,
    pub status: BodyStatus,
    pub created_at: DateTime<Utc>,
}

impl GovernanceBody {
    /// Create a new active governance body after validating the name.
    pub fn new(
        entity_id: EntityId,
        body_type: BodyType,
        name: String,
        quorum_rule: QuorumThreshold,
        voting_method: VotingMethod,
    ) -> Result<Self, GovernanceBodyError> {
        Self::validate_name(&name)?;
        Ok(Self {
            body_id: GovernanceBodyId::new(),
            entity_id,
            body_type,
            name,
            quorum_rule,
            voting_method,
            status: BodyStatus::Active,
            created_at: Utc::now(),
        })
    }

    /// Transition the body to `Inactive`.
    pub fn deactivate(&mut self) {
        self.status = BodyStatus::Inactive;
    }

    /// Validate a prospective body name.
    ///
    /// Rules:
    /// - Must be non-empty.
    /// - Must not exceed [`NAME_MAX_LEN`] characters.
    /// - Must not contain HTML/markup characters (`<`, `>`, `&`, `"`, `'`).
    pub fn validate_name(name: &str) -> Result<(), GovernanceBodyError> {
        if name.is_empty() {
            return Err(GovernanceBodyError::NameEmpty);
        }
        if name.len() > NAME_MAX_LEN {
            return Err(GovernanceBodyError::NameTooLong);
        }
        if name.contains(['<', '>', '&', '"', '\'']) {
            return Err(GovernanceBodyError::NameContainsMarkup);
        }
        Ok(())
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_body(name: &str) -> Result<GovernanceBody, GovernanceBodyError> {
        GovernanceBody::new(
            EntityId::new(),
            BodyType::BoardOfDirectors,
            name.to_string(),
            QuorumThreshold::Majority,
            VotingMethod::PerCapita,
        )
    }

    fn make_body_full(
        body_type: BodyType,
        quorum_rule: QuorumThreshold,
        voting_method: VotingMethod,
    ) -> GovernanceBody {
        GovernanceBody::new(
            EntityId::new(),
            body_type,
            "Test Body".to_string(),
            quorum_rule,
            voting_method,
        )
        .unwrap()
    }

    // ── name validation ───────────────────────────────────────────────────────

    #[test]
    fn valid_name() {
        assert!(make_body("Board of Directors").is_ok());
    }

    #[test]
    fn valid_name_max_length() {
        // exactly 200 chars — must pass
        let name = "a".repeat(200);
        assert!(make_body(&name).is_ok());
    }

    #[test]
    fn empty_name_rejected() {
        assert_eq!(make_body("").unwrap_err(), GovernanceBodyError::NameEmpty);
    }

    #[test]
    fn whitespace_only_name_is_valid() {
        // The validator only checks for empty string, not blank strings
        // (validate_name uses is_empty(), not trim().is_empty())
        // A single space is non-empty and passes.
        assert!(make_body(" ").is_ok());
    }

    #[test]
    fn name_too_long_rejected() {
        let long = "a".repeat(201);
        assert_eq!(
            make_body(&long).unwrap_err(),
            GovernanceBodyError::NameTooLong
        );
    }

    #[test]
    fn name_200_chars_is_ok() {
        let name = "b".repeat(200);
        assert!(make_body(&name).is_ok());
    }

    #[test]
    fn markup_in_name_rejected_lt() {
        assert_eq!(
            make_body("<script>").unwrap_err(),
            GovernanceBodyError::NameContainsMarkup
        );
    }

    #[test]
    fn markup_in_name_rejected_gt() {
        assert_eq!(
            make_body("foo>bar").unwrap_err(),
            GovernanceBodyError::NameContainsMarkup
        );
    }

    #[test]
    fn markup_in_name_rejected_ampersand() {
        assert_eq!(
            make_body("Foo & Bar").unwrap_err(),
            GovernanceBodyError::NameContainsMarkup
        );
    }

    #[test]
    fn markup_in_name_rejected_double_quote() {
        assert_eq!(
            make_body(r#"Foo "Bar""#).unwrap_err(),
            GovernanceBodyError::NameContainsMarkup
        );
    }

    #[test]
    fn markup_in_name_rejected_single_quote() {
        assert_eq!(
            make_body("Foo's Board").unwrap_err(),
            GovernanceBodyError::NameContainsMarkup
        );
    }

    #[test]
    fn name_with_numbers_and_hyphens_is_ok() {
        assert!(make_body("Committee-2025").is_ok());
    }

    #[test]
    fn name_with_parens_is_ok() {
        assert!(make_body("Board (Class A)").is_ok());
    }

    // ── status transitions ────────────────────────────────────────────────────

    #[test]
    fn new_body_is_active() {
        let body = make_body("Audit Committee").unwrap();
        assert_eq!(body.status, BodyStatus::Active);
    }

    #[test]
    fn deactivate() {
        let mut body = make_body("Audit Committee").unwrap();
        assert_eq!(body.status, BodyStatus::Active);
        body.deactivate();
        assert_eq!(body.status, BodyStatus::Inactive);
    }

    #[test]
    fn double_deactivate_stays_inactive() {
        let mut body = make_body("Audit Committee").unwrap();
        body.deactivate();
        body.deactivate(); // idempotent
        assert_eq!(body.status, BodyStatus::Inactive);
    }

    // ── body_type / quorum / voting combinations ──────────────────────────────

    #[test]
    fn board_of_directors_majority_per_capita() {
        let body = make_body_full(
            BodyType::BoardOfDirectors,
            QuorumThreshold::Majority,
            VotingMethod::PerCapita,
        );
        assert_eq!(body.body_type, BodyType::BoardOfDirectors);
        assert_eq!(body.quorum_rule, QuorumThreshold::Majority);
        assert_eq!(body.voting_method, VotingMethod::PerCapita);
    }

    #[test]
    fn board_of_directors_supermajority_per_unit() {
        let body = make_body_full(
            BodyType::BoardOfDirectors,
            QuorumThreshold::Supermajority,
            VotingMethod::PerUnit,
        );
        assert_eq!(body.quorum_rule, QuorumThreshold::Supermajority);
        assert_eq!(body.voting_method, VotingMethod::PerUnit);
    }

    #[test]
    fn llc_member_vote_unanimous_per_capita() {
        let body = make_body_full(
            BodyType::LlcMemberVote,
            QuorumThreshold::Unanimous,
            VotingMethod::PerCapita,
        );
        assert_eq!(body.body_type, BodyType::LlcMemberVote);
        assert_eq!(body.quorum_rule, QuorumThreshold::Unanimous);
    }

    #[test]
    fn llc_member_vote_majority_per_unit() {
        let body = make_body_full(
            BodyType::LlcMemberVote,
            QuorumThreshold::Majority,
            VotingMethod::PerUnit,
        );
        assert_eq!(body.body_type, BodyType::LlcMemberVote);
        assert_eq!(body.voting_method, VotingMethod::PerUnit);
    }

    #[test]
    fn body_has_unique_ids() {
        let b1 = make_body("Board A").unwrap();
        let b2 = make_body("Board B").unwrap();
        assert_ne!(b1.body_id, b2.body_id);
    }

    #[test]
    fn body_stores_entity_id() {
        let entity_id = EntityId::new();
        let body = GovernanceBody::new(
            entity_id,
            BodyType::BoardOfDirectors,
            "Test Board".to_string(),
            QuorumThreshold::Majority,
            VotingMethod::PerCapita,
        )
        .unwrap();
        assert_eq!(body.entity_id, entity_id);
    }

    #[test]
    fn body_serde_roundtrip() {
        let body = make_body("Compensation Committee").unwrap();
        let json = serde_json::to_string(&body).unwrap();
        let back: GovernanceBody = serde_json::from_str(&json).unwrap();
        assert_eq!(body.body_id, back.body_id);
        assert_eq!(body.name, back.name);
        assert_eq!(body.status, back.status);
    }
}
