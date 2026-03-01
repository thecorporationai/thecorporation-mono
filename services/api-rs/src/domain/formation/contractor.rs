//! Contractor classification record (stored as `contractors/{classification_id}.json`).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::ids::{ClassificationId, EntityId};

/// Risk level for contractor classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    Low,
    Medium,
    High,
}

/// Classification result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClassificationResult {
    Independent,
    Employee,
    Uncertain,
}

/// A contractor classification assessment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractorClassification {
    classification_id: ClassificationId,
    entity_id: EntityId,
    contractor_name: String,
    state: String,
    risk_level: RiskLevel,
    #[serde(default)]
    flags: Vec<String>,
    classification: ClassificationResult,
    created_at: DateTime<Utc>,
}

impl ContractorClassification {
    pub fn new(
        classification_id: ClassificationId,
        entity_id: EntityId,
        contractor_name: String,
        state: String,
        risk_level: RiskLevel,
        flags: Vec<String>,
        classification: ClassificationResult,
    ) -> Self {
        Self {
            classification_id,
            entity_id,
            contractor_name,
            state,
            risk_level,
            flags,
            classification,
            created_at: Utc::now(),
        }
    }

    // Accessors
    pub fn classification_id(&self) -> ClassificationId {
        self.classification_id
    }
    pub fn entity_id(&self) -> EntityId {
        self.entity_id
    }
    pub fn contractor_name(&self) -> &str {
        &self.contractor_name
    }
    pub fn state(&self) -> &str {
        &self.state
    }
    pub fn risk_level(&self) -> RiskLevel {
        self.risk_level
    }
    pub fn flags(&self) -> &[String] {
        &self.flags
    }
    pub fn classification(&self) -> ClassificationResult {
        self.classification
    }
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serde_roundtrip() {
        let c = ContractorClassification::new(
            ClassificationId::new(),
            EntityId::new(),
            "Jane Freelancer".to_owned(),
            "CA".to_owned(),
            RiskLevel::Medium,
            vec!["ab5_risk".to_owned()],
            ClassificationResult::Independent,
        );
        let json = serde_json::to_string(&c).unwrap();
        let parsed: ContractorClassification = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.classification_id(), c.classification_id());
        assert_eq!(parsed.classification(), ClassificationResult::Independent);
        assert_eq!(parsed.risk_level(), RiskLevel::Medium);
    }
}
