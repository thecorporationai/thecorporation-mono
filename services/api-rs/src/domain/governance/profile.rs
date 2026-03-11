//! Persisted governance generation profile.
//!
//! Stored in each entity repo at `governance/profile.json`.

use chrono::{DateTime, Duration, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::formation::{entity::Entity, types::EntityType};
use crate::domain::ids::EntityId;

pub const GOVERNANCE_PROFILE_PATH: &str = "governance/profile.json";

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CompanyAddress {
    pub street: String,
    pub city: String,
    pub county: Option<String>,
    pub state: String,
    pub zip: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct FounderInfo {
    pub name: String,
    #[serde(default)]
    pub shares: Option<u64>,
    #[serde(default)]
    pub vesting: Option<VestingSchedule>,
    #[serde(default)]
    pub ip_contribution: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub address: Option<CompanyAddress>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct VestingSchedule {
    pub total_months: u32,
    pub cliff_months: u32,
    #[serde(default)]
    pub acceleration_on_termination: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct DirectorInfo {
    pub name: String,
    #[serde(default)]
    pub address: Option<CompanyAddress>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct OfficerInfo {
    pub name: String,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct StockDetails {
    pub authorized_shares: u64,
    pub par_value_cents: u64,
    #[serde(default = "default_share_class")]
    pub share_class: String,
}

fn default_share_class() -> String {
    "Common Stock".to_owned()
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct FiscalYearEnd {
    pub month: u32,
    pub day: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct DocumentOptions {
    #[serde(default = "default_dating_format")]
    pub dating_format: String,
    #[serde(default)]
    pub transfer_restrictions: bool,
    #[serde(default)]
    pub right_of_first_refusal: bool,
    #[serde(default)]
    pub s_corp_election: bool,
}

fn default_dating_format() -> String {
    "blank_line".to_owned()
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct GovernanceProfile {
    entity_id: EntityId,
    entity_type: EntityType,
    legal_name: String,
    jurisdiction: String,
    effective_date: NaiveDate,
    adopted_by: String,
    last_reviewed: NaiveDate,
    next_mandatory_review: NaiveDate,
    #[serde(default)]
    registered_agent_name: Option<String>,
    #[serde(default)]
    registered_agent_address: Option<String>,
    #[serde(default)]
    board_size: Option<u32>,
    #[serde(default)]
    incorporator_name: Option<String>,
    #[serde(default)]
    incorporator_address: Option<String>,
    #[serde(default)]
    principal_name: Option<String>,
    #[serde(default)]
    principal_title: Option<String>,
    #[serde(default)]
    company_address: Option<CompanyAddress>,
    #[serde(default)]
    founders: Vec<FounderInfo>,
    #[serde(default)]
    directors: Vec<DirectorInfo>,
    #[serde(default)]
    officers: Vec<OfficerInfo>,
    #[serde(default)]
    stock_details: Option<StockDetails>,
    #[serde(default)]
    fiscal_year_end: Option<FiscalYearEnd>,
    #[serde(default)]
    document_options: Option<DocumentOptions>,
    #[serde(default)]
    incomplete_profile: bool,
    version: u32,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl GovernanceProfile {
    pub fn default_for_entity(entity: &Entity) -> Self {
        let now = Utc::now();
        let today = now.date_naive();
        let next = (now + Duration::days(365)).date_naive();
        Self {
            entity_id: entity.entity_id(),
            entity_type: entity.entity_type(),
            legal_name: entity.legal_name().to_owned(),
            jurisdiction: entity.jurisdiction().to_string(),
            effective_date: today,
            adopted_by: match entity.entity_type() {
                EntityType::CCorp => "Board of Directors".to_owned(),
                EntityType::Llc => "Members".to_owned(),
            },
            last_reviewed: today,
            next_mandatory_review: next,
            registered_agent_name: entity.registered_agent_name().map(ToOwned::to_owned),
            registered_agent_address: entity.registered_agent_address().map(ToOwned::to_owned),
            board_size: None,
            incorporator_name: None,
            incorporator_address: None,
            principal_name: None,
            principal_title: None,
            company_address: None,
            founders: Vec::new(),
            directors: Vec::new(),
            officers: Vec::new(),
            stock_details: None,
            fiscal_year_end: Some(FiscalYearEnd { month: 12, day: 31 }),
            document_options: None,
            incomplete_profile: true,
            version: 1,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.legal_name.trim().is_empty() {
            return Err("legal_name must not be empty".to_owned());
        }
        if self.jurisdiction.trim().is_empty() {
            return Err("jurisdiction must not be empty".to_owned());
        }
        if self.last_reviewed < self.effective_date {
            return Err("last_reviewed must be on or after effective_date".to_owned());
        }
        if self.next_mandatory_review <= self.last_reviewed {
            return Err("next_mandatory_review must be after last_reviewed".to_owned());
        }
        if self.version == 0 {
            return Err("version must be >= 1".to_owned());
        }
        if let Some(addr) = &self.company_address {
            if addr.street.trim().is_empty() {
                return Err("company_address.street must not be empty".to_owned());
            }
            if addr.city.trim().is_empty() {
                return Err("company_address.city must not be empty".to_owned());
            }
            if addr.state.trim().is_empty() {
                return Err("company_address.state must not be empty".to_owned());
            }
            if addr.zip.trim().is_empty() {
                return Err("company_address.zip must not be empty".to_owned());
            }
        }
        if let Some(stock) = &self.stock_details {
            if stock.authorized_shares == 0 {
                return Err("stock_details.authorized_shares must be > 0".to_owned());
            }
        }
        if let Some(fy) = &self.fiscal_year_end {
            if fy.month == 0 || fy.month > 12 {
                return Err("fiscal_year_end.month must be 1-12".to_owned());
            }
            if fy.day == 0 || fy.day > 31 {
                return Err("fiscal_year_end.day must be 1-31".to_owned());
            }
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn update(
        &mut self,
        legal_name: String,
        jurisdiction: String,
        effective_date: NaiveDate,
        adopted_by: String,
        last_reviewed: NaiveDate,
        next_mandatory_review: NaiveDate,
        registered_agent_name: Option<String>,
        registered_agent_address: Option<String>,
        board_size: Option<u32>,
        incorporator_name: Option<String>,
        incorporator_address: Option<String>,
        principal_name: Option<String>,
        principal_title: Option<String>,
        incomplete_profile: Option<bool>,
    ) {
        self.legal_name = legal_name;
        self.jurisdiction = jurisdiction;
        self.effective_date = effective_date;
        self.adopted_by = adopted_by;
        self.last_reviewed = last_reviewed;
        self.next_mandatory_review = next_mandatory_review;
        self.registered_agent_name = registered_agent_name;
        self.registered_agent_address = registered_agent_address;
        self.board_size = board_size;
        self.incorporator_name = incorporator_name;
        self.incorporator_address = incorporator_address;
        self.principal_name = principal_name;
        self.principal_title = principal_title;
        if let Some(v) = incomplete_profile {
            self.incomplete_profile = v;
        }
        self.version = self.version.saturating_add(1);
        self.updated_at = Utc::now();
    }

    pub fn retype_for_entity(&mut self, entity: &Entity) {
        self.entity_type = entity.entity_type();
        self.legal_name = entity.legal_name().to_owned();
        self.jurisdiction = entity.jurisdiction().to_string();
        self.registered_agent_name = entity.registered_agent_name().map(ToOwned::to_owned);
        self.registered_agent_address = entity.registered_agent_address().map(ToOwned::to_owned);
        self.adopted_by = match entity.entity_type() {
            EntityType::CCorp => "Board of Directors".to_owned(),
            EntityType::Llc => "Members".to_owned(),
        };
        match entity.entity_type() {
            EntityType::CCorp => {
                self.principal_name = None;
                self.principal_title = None;
            }
            EntityType::Llc => {
                self.board_size = None;
                self.incorporator_name = None;
                self.incorporator_address = None;
                self.directors.clear();
                self.stock_details = None;
            }
        }
        self.version = self.version.saturating_add(1);
        self.updated_at = Utc::now();
    }

    pub fn set_company_address(&mut self, address: CompanyAddress) {
        self.company_address = Some(address);
        self.updated_at = Utc::now();
    }

    pub fn set_founders(&mut self, founders: Vec<FounderInfo>) {
        self.founders = founders;
        self.updated_at = Utc::now();
    }

    pub fn set_directors(&mut self, directors: Vec<DirectorInfo>) {
        self.directors = directors;
        self.updated_at = Utc::now();
    }

    pub fn set_officers(&mut self, officers: Vec<OfficerInfo>) {
        self.officers = officers;
        self.updated_at = Utc::now();
    }

    pub fn set_stock_details(&mut self, details: StockDetails) {
        self.stock_details = Some(details);
        self.updated_at = Utc::now();
    }

    pub fn set_fiscal_year_end(&mut self, fy: FiscalYearEnd) {
        self.fiscal_year_end = Some(fy);
        self.updated_at = Utc::now();
    }

    pub fn set_document_options(&mut self, options: DocumentOptions) {
        self.document_options = Some(options);
        self.updated_at = Utc::now();
    }

    pub fn entity_id(&self) -> EntityId {
        self.entity_id
    }
    pub fn entity_type(&self) -> EntityType {
        self.entity_type
    }
    pub fn legal_name(&self) -> &str {
        &self.legal_name
    }
    pub fn jurisdiction(&self) -> &str {
        &self.jurisdiction
    }
    pub fn effective_date(&self) -> NaiveDate {
        self.effective_date
    }
    pub fn adopted_by(&self) -> &str {
        &self.adopted_by
    }
    pub fn last_reviewed(&self) -> NaiveDate {
        self.last_reviewed
    }
    pub fn next_mandatory_review(&self) -> NaiveDate {
        self.next_mandatory_review
    }
    pub fn registered_agent_name(&self) -> Option<&str> {
        self.registered_agent_name.as_deref()
    }
    pub fn registered_agent_address(&self) -> Option<&str> {
        self.registered_agent_address.as_deref()
    }
    pub fn board_size(&self) -> Option<u32> {
        self.board_size
    }
    pub fn incorporator_name(&self) -> Option<&str> {
        self.incorporator_name.as_deref()
    }
    pub fn incorporator_address(&self) -> Option<&str> {
        self.incorporator_address.as_deref()
    }
    /// Override incorporator fields (used when finalize provides them explicitly).
    pub fn patch_incorporator(&mut self, name: Option<String>, address: Option<String>) {
        if let Some(name) = name {
            self.incorporator_name = Some(name);
        }
        if let Some(address) = address {
            self.incorporator_address = Some(address);
        }
    }
    pub fn principal_name(&self) -> Option<&str> {
        self.principal_name.as_deref()
    }
    pub fn principal_title(&self) -> Option<&str> {
        self.principal_title.as_deref()
    }
    pub fn company_address(&self) -> Option<&CompanyAddress> {
        self.company_address.as_ref()
    }
    pub fn founders(&self) -> &[FounderInfo] {
        &self.founders
    }
    pub fn directors(&self) -> &[DirectorInfo] {
        &self.directors
    }
    pub fn officers(&self) -> &[OfficerInfo] {
        &self.officers
    }
    pub fn stock_details(&self) -> Option<&StockDetails> {
        self.stock_details.as_ref()
    }
    pub fn fiscal_year_end(&self) -> Option<&FiscalYearEnd> {
        self.fiscal_year_end.as_ref()
    }
    pub fn document_options(&self) -> Option<&DocumentOptions> {
        self.document_options.as_ref()
    }
    pub fn incomplete_profile(&self) -> bool {
        self.incomplete_profile
    }
    pub fn version(&self) -> u32 {
        self.version
    }
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::formation::types::Jurisdiction;
    use crate::domain::ids::WorkspaceId;

    fn make_entity() -> Entity {
        Entity::new(
            EntityId::new(),
            WorkspaceId::new(),
            "Acme, Inc.".to_owned(),
            EntityType::CCorp,
            Jurisdiction::new("Delaware").expect("jurisdiction"),
            Some("Delaware RA".to_owned()),
            Some("123 Main St, Wilmington, DE".to_owned()),
        )
        .expect("entity")
    }

    #[test]
    fn default_profile_validates() {
        let entity = make_entity();
        let profile = GovernanceProfile::default_for_entity(&entity);
        assert_eq!(profile.entity_id(), entity.entity_id());
        assert!(profile.validate().is_ok());
        assert!(profile.incomplete_profile());
    }

    #[test]
    fn update_bumps_version() {
        let entity = make_entity();
        let mut profile = GovernanceProfile::default_for_entity(&entity);
        let start = profile.version();
        profile.update(
            "Acme Corporation".to_owned(),
            "Delaware".to_owned(),
            profile.effective_date(),
            "Board".to_owned(),
            profile.last_reviewed(),
            profile.next_mandatory_review(),
            profile.registered_agent_name().map(ToOwned::to_owned),
            profile.registered_agent_address().map(ToOwned::to_owned),
            Some(3),
            Some("Incorporator".to_owned()),
            None,
            Some("CEO".to_owned()),
            Some("Chief Executive Officer".to_owned()),
            Some(false),
        );
        assert_eq!(profile.version(), start + 1);
        assert!(!profile.incomplete_profile());
        assert!(profile.validate().is_ok());
    }
}
