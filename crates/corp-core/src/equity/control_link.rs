//! Control links: directed edges in the legal-entity control graph.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::ids::{ControlLinkId, LegalEntityId};

// ── ControlType ───────────────────────────────────────────────────────────────

/// The nature of control one entity exercises over another.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ControlType {
    /// Voting control (e.g. majority shareholder).
    Voting,
    /// Board appointment rights.
    Board,
    /// Economic but not necessarily voting control.
    Economic,
    /// Contractual control (management agreement, etc.).
    Contractual,
}

// ── ControlLink ───────────────────────────────────────────────────────────────

/// A directed control relationship between two [`LegalEntity`] records.
///
/// `parent_legal_entity_id` controls `child_legal_entity_id`.
/// `voting_power_bps` is expressed as basis points (0–10 000 = 0–100%).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlLink {
    pub control_link_id: ControlLinkId,
    pub parent_legal_entity_id: LegalEntityId,
    pub child_legal_entity_id: LegalEntityId,
    pub control_type: ControlType,
    /// Voting power in basis points (100 bps = 1%, 10 000 bps = 100%).
    /// `None` when not applicable (e.g. pure contractual control).
    pub voting_power_bps: Option<u32>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl ControlLink {
    /// Create a new control link.
    pub fn new(
        parent_legal_entity_id: LegalEntityId,
        child_legal_entity_id: LegalEntityId,
        control_type: ControlType,
        voting_power_bps: Option<u32>,
        notes: Option<String>,
    ) -> Self {
        Self {
            control_link_id: ControlLinkId::new(),
            parent_legal_entity_id,
            child_legal_entity_id,
            control_type,
            voting_power_bps,
            notes,
            created_at: Utc::now(),
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_link(control_type: ControlType, voting_power_bps: Option<u32>) -> ControlLink {
        ControlLink::new(
            LegalEntityId::new(),
            LegalEntityId::new(),
            control_type,
            voting_power_bps,
            None,
        )
    }

    #[test]
    fn new_link_stores_control_type() {
        let link = make_link(ControlType::Voting, Some(10_000));
        assert_eq!(link.control_type, ControlType::Voting);
    }

    #[test]
    fn new_link_stores_voting_power() {
        let link = make_link(ControlType::Voting, Some(5_100));
        assert_eq!(link.voting_power_bps, Some(5_100));
    }

    #[test]
    fn new_link_no_voting_power_for_contractual() {
        let link = make_link(ControlType::Contractual, None);
        assert!(link.voting_power_bps.is_none());
    }

    #[test]
    fn new_link_has_unique_id() {
        let a = make_link(ControlType::Board, Some(3_333));
        let b = make_link(ControlType::Board, Some(3_333));
        assert_ne!(a.control_link_id, b.control_link_id);
    }

    #[test]
    fn new_link_with_notes() {
        let link = ControlLink::new(
            LegalEntityId::new(),
            LegalEntityId::new(),
            ControlType::Economic,
            Some(10_000),
            Some("Wholly owned subsidiary".to_string()),
        );
        assert_eq!(link.notes, Some("Wholly owned subsidiary".to_string()));
    }

    #[test]
    fn control_link_serde_roundtrip() {
        let link = make_link(ControlType::Board, Some(6_667));
        let json = serde_json::to_string(&link).unwrap();
        let de: ControlLink = serde_json::from_str(&json).unwrap();
        assert_eq!(de.control_link_id, link.control_link_id);
        assert_eq!(de.control_type, ControlType::Board);
        assert_eq!(de.voting_power_bps, Some(6_667));
    }

    // ── ControlType serde ─────────────────────────────────────────────────────

    #[test]
    fn control_type_serde_voting() {
        let json = serde_json::to_string(&ControlType::Voting).unwrap();
        assert_eq!(json, r#""voting""#);
        let de: ControlType = serde_json::from_str(&json).unwrap();
        assert_eq!(de, ControlType::Voting);
    }

    #[test]
    fn control_type_serde_board() {
        let json = serde_json::to_string(&ControlType::Board).unwrap();
        assert_eq!(json, r#""board""#);
    }

    #[test]
    fn control_type_serde_economic() {
        let json = serde_json::to_string(&ControlType::Economic).unwrap();
        assert_eq!(json, r#""economic""#);
    }

    #[test]
    fn control_type_serde_contractual() {
        let json = serde_json::to_string(&ControlType::Contractual).unwrap();
        assert_eq!(json, r#""contractual""#);
    }
}
