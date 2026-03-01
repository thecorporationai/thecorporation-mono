//! Contract record (stored as `contracts/{contract_id}.json`).

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::ids::{ContractId, DocumentId, EntityId};

/// Template type for generated contracts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContractTemplateType {
    ConsultingAgreement,
    EmploymentOffer,
    ContractorAgreement,
    Nda,
    Custom,
}

/// Lifecycle status of a contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContractStatus {
    Draft,
    Active,
    Expired,
    Terminated,
}

/// A contract associated with an entity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contract {
    contract_id: ContractId,
    entity_id: EntityId,
    template_type: ContractTemplateType,
    counterparty_name: String,
    effective_date: NaiveDate,
    #[serde(default)]
    parameters: serde_json::Value,
    status: ContractStatus,
    document_id: DocumentId,
    created_at: DateTime<Utc>,
}

impl Contract {
    pub fn new(
        contract_id: ContractId,
        entity_id: EntityId,
        template_type: ContractTemplateType,
        counterparty_name: String,
        effective_date: NaiveDate,
        parameters: serde_json::Value,
        document_id: DocumentId,
    ) -> Self {
        Self {
            contract_id,
            entity_id,
            template_type,
            counterparty_name,
            effective_date,
            parameters,
            status: ContractStatus::Draft,
            document_id,
            created_at: Utc::now(),
        }
    }

    pub fn activate(&mut self) {
        self.status = ContractStatus::Active;
    }

    pub fn terminate(&mut self) {
        self.status = ContractStatus::Terminated;
    }

    // Accessors
    pub fn contract_id(&self) -> ContractId {
        self.contract_id
    }
    pub fn entity_id(&self) -> EntityId {
        self.entity_id
    }
    pub fn template_type(&self) -> ContractTemplateType {
        self.template_type
    }
    pub fn counterparty_name(&self) -> &str {
        &self.counterparty_name
    }
    pub fn effective_date(&self) -> NaiveDate {
        self.effective_date
    }
    pub fn parameters(&self) -> &serde_json::Value {
        &self.parameters
    }
    pub fn status(&self) -> ContractStatus {
        self.status
    }
    pub fn document_id(&self) -> DocumentId {
        self.document_id
    }
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_contract_is_draft() {
        let c = Contract::new(
            ContractId::new(),
            EntityId::new(),
            ContractTemplateType::Nda,
            "Acme Corp".to_owned(),
            NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            serde_json::json!({}),
            DocumentId::new(),
        );
        assert_eq!(c.status(), ContractStatus::Draft);
        assert_eq!(c.counterparty_name(), "Acme Corp");
    }

    #[test]
    fn activate_and_terminate() {
        let mut c = Contract::new(
            ContractId::new(),
            EntityId::new(),
            ContractTemplateType::ConsultingAgreement,
            "Bob".to_owned(),
            NaiveDate::from_ymd_opt(2026, 3, 1).unwrap(),
            serde_json::json!({}),
            DocumentId::new(),
        );
        c.activate();
        assert_eq!(c.status(), ContractStatus::Active);
        c.terminate();
        assert_eq!(c.status(), ContractStatus::Terminated);
    }

    #[test]
    fn serde_roundtrip() {
        let c = Contract::new(
            ContractId::new(),
            EntityId::new(),
            ContractTemplateType::EmploymentOffer,
            "Alice".to_owned(),
            NaiveDate::from_ymd_opt(2026, 6, 15).unwrap(),
            serde_json::json!({"salary": 120000}),
            DocumentId::new(),
        );
        let json = serde_json::to_string(&c).unwrap();
        let parsed: Contract = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.contract_id(), c.contract_id());
        assert_eq!(
            parsed.template_type(),
            ContractTemplateType::EmploymentOffer
        );
    }
}
