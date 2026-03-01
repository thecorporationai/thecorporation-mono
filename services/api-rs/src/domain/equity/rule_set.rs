//! Financing rule sets used for conversion and anti-dilution logic.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::instrument::InstrumentKind;
use crate::domain::ids::EquityRuleSetId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AntiDilutionMethod {
    None,
    BroadBasedWeightedAverage,
    NarrowBasedWeightedAverage,
    FullRatchet,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EquityRuleSet {
    rule_set_id: EquityRuleSetId,
    anti_dilution_method: AntiDilutionMethod,
    conversion_precedence: Vec<InstrumentKind>,
    protective_provisions: serde_json::Value,
    created_at: DateTime<Utc>,
}

impl EquityRuleSet {
    pub fn new(
        rule_set_id: EquityRuleSetId,
        anti_dilution_method: AntiDilutionMethod,
        conversion_precedence: Vec<InstrumentKind>,
        protective_provisions: serde_json::Value,
    ) -> Self {
        Self {
            rule_set_id,
            anti_dilution_method,
            conversion_precedence,
            protective_provisions,
            created_at: Utc::now(),
        }
    }

    pub fn rule_set_id(&self) -> EquityRuleSetId {
        self.rule_set_id
    }

    pub fn anti_dilution_method(&self) -> AntiDilutionMethod {
        self.anti_dilution_method
    }

    pub fn conversion_precedence(&self) -> &[InstrumentKind] {
        &self.conversion_precedence
    }

    pub fn protective_provisions(&self) -> &serde_json::Value {
        &self.protective_provisions
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}
