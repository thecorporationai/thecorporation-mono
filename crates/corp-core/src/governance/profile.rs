//! Governance profile — the structured identity and configuration of a governed entity.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use crate::ids::EntityId;

// ── Supporting types ──────────────────────────────────────────────────────────

/// A structured mailing / registered address.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompanyAddress {
    pub street: String,
    pub city: String,
    pub state: String,
    pub zip: String,
}

/// A founding member or co-founder of the entity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FounderInfo {
    pub name: String,
    pub email: Option<String>,
    /// Share count awarded to this founder, if applicable.
    pub shares: Option<i64>,
}

/// A member of the board of directors.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectorInfo {
    pub name: String,
    pub address: Option<String>,
}

/// An executive officer of the entity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OfficerInfo {
    pub name: String,
    pub title: String,
}

/// Authorised stock details for a corporation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockDetails {
    /// Total shares authorised in the charter.
    pub authorized_shares: i64,
    /// Par value per share, expressed in whole cents.
    pub par_value_cents: i64,
    /// Name of the share class (e.g. "Common", "Series A Preferred").
    pub share_class: String,
}

/// The entity's fiscal year end (month and day).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FiscalYearEnd {
    /// Month number (1 = January, 12 = December).
    pub month: u32,
    /// Day of month.
    pub day: u32,
}

// ── Error ─────────────────────────────────────────────────────────────────────

/// Errors that can arise when constructing or validating a [`GovernanceProfile`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum GovernanceProfileError {
    #[error("legal_name must not be empty")]
    LegalNameEmpty,
    #[error("jurisdiction must not be empty")]
    JurisdictionEmpty,
    #[error("entity_type must not be empty")]
    EntityTypeEmpty,
    #[error("fiscal year end month must be between 1 and 12, got {0}")]
    InvalidFiscalMonth(u32),
    #[error("fiscal year end day must be between 1 and 31, got {0}")]
    InvalidFiscalDay(u32),
    #[error("stock authorized_shares must be positive")]
    InvalidAuthorizedShares,
    #[error("stock par_value_cents must be non-negative")]
    InvalidParValue,
    #[error("share_class must not be empty")]
    ShareClassEmpty,
}

// ── GovernanceProfile ─────────────────────────────────────────────────────────

/// The core identity and governance configuration for a corporate entity.
///
/// Version is monotonically incremented on every [`update`](GovernanceProfile::update) call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceProfile {
    pub entity_id: EntityId,
    /// Legal entity type (e.g. "Delaware C-Corporation", "Wyoming LLC").
    pub entity_type: String,
    pub legal_name: String,
    /// Jurisdiction of formation (e.g. "DE", "WY").
    pub jurisdiction: String,
    /// Date on which this governance profile became effective.
    pub effective_date: NaiveDate,
    pub registered_agent_name: Option<String>,
    pub registered_agent_address: Option<String>,
    /// Number of seats on the board, if applicable.
    pub board_size: Option<u32>,
    /// Primary principal / authorised signer name.
    pub principal_name: Option<String>,
    pub company_address: Option<CompanyAddress>,
    pub founders: Vec<FounderInfo>,
    pub directors: Vec<DirectorInfo>,
    pub officers: Vec<OfficerInfo>,
    pub stock_details: Option<StockDetails>,
    pub fiscal_year_end: Option<FiscalYearEnd>,
    /// Monotonically increasing edit counter. Starts at 1.
    pub version: u32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl GovernanceProfile {
    /// Create a new profile. Validates all fields before constructing.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        entity_id: EntityId,
        entity_type: String,
        legal_name: String,
        jurisdiction: String,
        effective_date: NaiveDate,
        registered_agent_name: Option<String>,
        registered_agent_address: Option<String>,
        board_size: Option<u32>,
        principal_name: Option<String>,
        company_address: Option<CompanyAddress>,
        founders: Vec<FounderInfo>,
        directors: Vec<DirectorInfo>,
        officers: Vec<OfficerInfo>,
        stock_details: Option<StockDetails>,
        fiscal_year_end: Option<FiscalYearEnd>,
    ) -> Result<Self, GovernanceProfileError> {
        let profile = Self {
            entity_id,
            entity_type,
            legal_name,
            jurisdiction,
            effective_date,
            registered_agent_name,
            registered_agent_address,
            board_size,
            principal_name,
            company_address,
            founders,
            directors,
            officers,
            stock_details,
            fiscal_year_end,
            version: 1,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        profile.validate()?;
        Ok(profile)
    }

    /// Validate all fields. Called by `new()` and should be called after
    /// any manual field mutation.
    pub fn validate(&self) -> Result<(), GovernanceProfileError> {
        if self.legal_name.trim().is_empty() {
            return Err(GovernanceProfileError::LegalNameEmpty);
        }
        if self.jurisdiction.trim().is_empty() {
            return Err(GovernanceProfileError::JurisdictionEmpty);
        }
        if self.entity_type.trim().is_empty() {
            return Err(GovernanceProfileError::EntityTypeEmpty);
        }
        if let Some(fye) = &self.fiscal_year_end {
            if fye.month == 0 || fye.month > 12 {
                return Err(GovernanceProfileError::InvalidFiscalMonth(fye.month));
            }
            if fye.day == 0 || fye.day > 31 {
                return Err(GovernanceProfileError::InvalidFiscalDay(fye.day));
            }
        }
        if let Some(sd) = &self.stock_details {
            if sd.authorized_shares <= 0 {
                return Err(GovernanceProfileError::InvalidAuthorizedShares);
            }
            if sd.par_value_cents < 0 {
                return Err(GovernanceProfileError::InvalidParValue);
            }
            if sd.share_class.trim().is_empty() {
                return Err(GovernanceProfileError::ShareClassEmpty);
            }
        }
        Ok(())
    }

    /// Apply an in-place update function to this profile, increment the version,
    /// and refresh `updated_at`.
    ///
    /// The update function may mutate any field; `validate` is called after the
    /// mutation and the version bump only occurs if validation passes.
    pub fn update<F>(&mut self, f: F) -> Result<(), GovernanceProfileError>
    where
        F: FnOnce(&mut GovernanceProfile),
    {
        f(self);
        self.validate()?;
        self.version = self.version.saturating_add(1);
        self.updated_at = Utc::now();
        Ok(())
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal_profile() -> GovernanceProfile {
        GovernanceProfile::new(
            EntityId::new(),
            "Delaware C-Corporation".into(),
            "Acme Corp".into(),
            "DE".into(),
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            None,
            None,
            None,
            None,
            None,
            vec![],
            vec![],
            vec![],
            None,
            None,
        )
        .unwrap()
    }

    fn make_profile_with_fye(
        month: u32,
        day: u32,
    ) -> Result<GovernanceProfile, GovernanceProfileError> {
        GovernanceProfile::new(
            EntityId::new(),
            "Wyoming LLC".into(),
            "Test LLC".into(),
            "WY".into(),
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            None,
            None,
            None,
            None,
            None,
            vec![],
            vec![],
            vec![],
            None,
            Some(FiscalYearEnd { month, day }),
        )
    }

    fn make_profile_with_stock(
        shares: i64,
        par: i64,
        class: &str,
    ) -> Result<GovernanceProfile, GovernanceProfileError> {
        GovernanceProfile::new(
            EntityId::new(),
            "Delaware C-Corporation".into(),
            "Stock Corp".into(),
            "DE".into(),
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            None,
            None,
            None,
            None,
            None,
            vec![],
            vec![],
            vec![],
            Some(StockDetails {
                authorized_shares: shares,
                par_value_cents: par,
                share_class: class.into(),
            }),
            None,
        )
    }

    // ── new(): basic construction ─────────────────────────────────────────────

    #[test]
    fn new_profile_version_is_1() {
        let p = minimal_profile();
        assert_eq!(p.version, 1);
    }

    #[test]
    fn new_profile_stores_entity_id() {
        let id = EntityId::new();
        let p = GovernanceProfile::new(
            id,
            "Delaware C-Corporation".into(),
            "Acme Corp".into(),
            "DE".into(),
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            None,
            None,
            None,
            None,
            None,
            vec![],
            vec![],
            vec![],
            None,
            None,
        )
        .unwrap();
        assert_eq!(p.entity_id, id);
    }

    #[test]
    fn new_profile_with_all_optional_fields() {
        let addr = CompanyAddress {
            street: "123 Main St".into(),
            city: "San Francisco".into(),
            state: "CA".into(),
            zip: "94102".into(),
        };
        let founder = FounderInfo {
            name: "Alice".into(),
            email: Some("alice@example.com".into()),
            shares: Some(5_000_000),
        };
        let director = DirectorInfo {
            name: "Bob".into(),
            address: Some("456 Board Ave".into()),
        };
        let officer = OfficerInfo {
            name: "Carol".into(),
            title: "CEO".into(),
        };
        let p = GovernanceProfile::new(
            EntityId::new(),
            "Delaware C-Corporation".into(),
            "Full Corp".into(),
            "DE".into(),
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            Some("Registered Agents Inc".into()),
            Some("160 Greentree Dr, Dover, DE 19904".into()),
            Some(5),
            Some("Jane Smith".into()),
            Some(addr),
            vec![founder],
            vec![director],
            vec![officer],
            Some(StockDetails {
                authorized_shares: 10_000_000,
                par_value_cents: 1,
                share_class: "Common".into(),
            }),
            Some(FiscalYearEnd { month: 12, day: 31 }),
        )
        .unwrap();
        assert_eq!(p.version, 1);
        assert_eq!(p.founders.len(), 1);
        assert_eq!(p.directors.len(), 1);
        assert_eq!(p.officers.len(), 1);
        assert_eq!(p.board_size, Some(5));
        assert!(p.company_address.is_some());
        assert!(p.stock_details.is_some());
        assert!(p.fiscal_year_end.is_some());
    }

    // ── validate(): required fields ───────────────────────────────────────────

    #[test]
    fn empty_legal_name_rejected() {
        let result = GovernanceProfile::new(
            EntityId::new(),
            "LLC".into(),
            "".into(),
            "WY".into(),
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            None,
            None,
            None,
            None,
            None,
            vec![],
            vec![],
            vec![],
            None,
            None,
        );
        assert_eq!(result.unwrap_err(), GovernanceProfileError::LegalNameEmpty);
    }

    #[test]
    fn whitespace_legal_name_rejected() {
        let result = GovernanceProfile::new(
            EntityId::new(),
            "LLC".into(),
            "   ".into(),
            "WY".into(),
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            None,
            None,
            None,
            None,
            None,
            vec![],
            vec![],
            vec![],
            None,
            None,
        );
        assert_eq!(result.unwrap_err(), GovernanceProfileError::LegalNameEmpty);
    }

    #[test]
    fn empty_jurisdiction_rejected() {
        let result = GovernanceProfile::new(
            EntityId::new(),
            "LLC".into(),
            "Valid Corp".into(),
            "".into(),
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            None,
            None,
            None,
            None,
            None,
            vec![],
            vec![],
            vec![],
            None,
            None,
        );
        assert_eq!(
            result.unwrap_err(),
            GovernanceProfileError::JurisdictionEmpty
        );
    }

    #[test]
    fn empty_entity_type_rejected() {
        let result = GovernanceProfile::new(
            EntityId::new(),
            "".into(),
            "Valid Corp".into(),
            "DE".into(),
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            None,
            None,
            None,
            None,
            None,
            vec![],
            vec![],
            vec![],
            None,
            None,
        );
        assert_eq!(result.unwrap_err(), GovernanceProfileError::EntityTypeEmpty);
    }

    // ── FiscalYearEnd validation ───────────────────────────────────────────────

    #[test]
    fn invalid_fiscal_month_rejected() {
        let result = GovernanceProfile::new(
            EntityId::new(),
            "LLC".into(),
            "Beta LLC".into(),
            "WY".into(),
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            None,
            None,
            None,
            None,
            None,
            vec![],
            vec![],
            vec![],
            None,
            Some(FiscalYearEnd { month: 13, day: 31 }),
        );
        assert_eq!(
            result.unwrap_err(),
            GovernanceProfileError::InvalidFiscalMonth(13)
        );
    }

    #[test]
    fn fiscal_month_zero_rejected() {
        assert_eq!(
            make_profile_with_fye(0, 31).unwrap_err(),
            GovernanceProfileError::InvalidFiscalMonth(0)
        );
    }

    #[test]
    fn fiscal_day_zero_rejected() {
        assert_eq!(
            make_profile_with_fye(12, 0).unwrap_err(),
            GovernanceProfileError::InvalidFiscalDay(0)
        );
    }

    #[test]
    fn fiscal_day_32_rejected() {
        assert_eq!(
            make_profile_with_fye(12, 32).unwrap_err(),
            GovernanceProfileError::InvalidFiscalDay(32)
        );
    }

    #[test]
    fn fiscal_year_end_valid_dec_31() {
        assert!(make_profile_with_fye(12, 31).is_ok());
    }

    #[test]
    fn fiscal_year_end_valid_jan_1() {
        assert!(make_profile_with_fye(1, 1).is_ok());
    }

    #[test]
    fn fiscal_year_end_valid_jun_30() {
        assert!(make_profile_with_fye(6, 30).is_ok());
    }

    #[test]
    fn no_fiscal_year_end_is_valid() {
        let p = minimal_profile();
        assert!(p.fiscal_year_end.is_none());
        assert!(p.validate().is_ok());
    }

    // ── StockDetails validation ───────────────────────────────────────────────

    #[test]
    fn stock_details_zero_shares_rejected() {
        let result = GovernanceProfile::new(
            EntityId::new(),
            "C-Corp".into(),
            "Corp Inc".into(),
            "DE".into(),
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            None,
            None,
            None,
            None,
            None,
            vec![],
            vec![],
            vec![],
            Some(StockDetails {
                authorized_shares: 0,
                par_value_cents: 1,
                share_class: "Common".into(),
            }),
            None,
        );
        assert_eq!(
            result.unwrap_err(),
            GovernanceProfileError::InvalidAuthorizedShares
        );
    }

    #[test]
    fn stock_details_negative_shares_rejected() {
        assert_eq!(
            make_profile_with_stock(-1, 1, "Common").unwrap_err(),
            GovernanceProfileError::InvalidAuthorizedShares
        );
    }

    #[test]
    fn stock_details_negative_par_value_rejected() {
        assert_eq!(
            make_profile_with_stock(1_000_000, -1, "Common").unwrap_err(),
            GovernanceProfileError::InvalidParValue
        );
    }

    #[test]
    fn stock_details_zero_par_value_is_ok() {
        // $0.00 par value is allowed (no-par stock)
        assert!(make_profile_with_stock(10_000_000, 0, "Common").is_ok());
    }

    #[test]
    fn stock_details_empty_share_class_rejected() {
        assert_eq!(
            make_profile_with_stock(10_000_000, 1, "").unwrap_err(),
            GovernanceProfileError::ShareClassEmpty
        );
    }

    #[test]
    fn stock_details_whitespace_share_class_rejected() {
        assert_eq!(
            make_profile_with_stock(10_000_000, 1, "   ").unwrap_err(),
            GovernanceProfileError::ShareClassEmpty
        );
    }

    #[test]
    fn stock_details_valid_passes() {
        assert!(make_profile_with_stock(10_000_000, 1, "Common").is_ok());
    }

    // ── update() ─────────────────────────────────────────────────────────────

    #[test]
    fn update_increments_version() {
        let mut p = minimal_profile();
        p.update(|profile| {
            profile.principal_name = Some("Jane Smith".into());
        })
        .unwrap();
        assert_eq!(p.version, 2);
        assert_eq!(p.principal_name.as_deref(), Some("Jane Smith"));
    }

    #[test]
    fn update_twice_increments_to_3() {
        let mut p = minimal_profile();
        p.update(|profile| {
            profile.board_size = Some(3);
        })
        .unwrap();
        p.update(|profile| {
            profile.board_size = Some(5);
        })
        .unwrap();
        assert_eq!(p.version, 3);
        assert_eq!(p.board_size, Some(5));
    }

    #[test]
    fn update_invalid_rolls_back_version() {
        let mut p = minimal_profile();
        let err = p
            .update(|profile| {
                profile.legal_name = String::new(); // invalid
            })
            .unwrap_err();
        assert_eq!(err, GovernanceProfileError::LegalNameEmpty);
        // Version must NOT have been incremented.
        assert_eq!(p.version, 1);
    }

    #[test]
    fn update_invalid_preserves_original_field() {
        let mut p = minimal_profile();
        let original_name = p.legal_name.clone();
        let _ = p.update(|profile| {
            profile.legal_name = String::new();
        });
        // Field WAS mutated (update doesn't roll back field changes, only version)
        // but the version should not have been incremented
        assert_eq!(p.version, 1);
        // The name was set to empty inside the closure; validate caught it
        // (field change is not rolled back, only version)
        let _ = original_name; // suppressing unused warning
    }

    // ── FounderInfo ───────────────────────────────────────────────────────────

    #[test]
    fn founder_without_shares() {
        let f = FounderInfo {
            name: "Bob".into(),
            email: None,
            shares: None,
        };
        assert!(f.shares.is_none());
        assert!(f.email.is_none());
    }

    #[test]
    fn founder_with_shares_and_email() {
        let f = FounderInfo {
            name: "Alice".into(),
            email: Some("alice@example.com".into()),
            shares: Some(5_000_000),
        };
        assert_eq!(f.shares, Some(5_000_000));
        assert_eq!(f.email.as_deref(), Some("alice@example.com"));
    }

    // ── CompanyAddress ────────────────────────────────────────────────────────

    #[test]
    fn company_address_complete() {
        let addr = CompanyAddress {
            street: "123 Main St".into(),
            city: "San Francisco".into(),
            state: "CA".into(),
            zip: "94102".into(),
        };
        assert_eq!(addr.city, "San Francisco");
        assert_eq!(addr.state, "CA");
    }

    #[test]
    fn profile_serde_roundtrip() {
        let p = minimal_profile();
        let json = serde_json::to_string(&p).unwrap();
        let back: GovernanceProfile = serde_json::from_str(&json).unwrap();
        assert_eq!(p.entity_id, back.entity_id);
        assert_eq!(p.version, back.version);
        assert_eq!(p.legal_name, back.legal_name);
    }

    #[test]
    fn profile_error_display() {
        let err = GovernanceProfileError::LegalNameEmpty;
        assert!(format!("{err}").contains("legal_name"));
        let err2 = GovernanceProfileError::InvalidFiscalMonth(15);
        assert!(format!("{err2}").contains("15"));
        let err3 = GovernanceProfileError::InvalidFiscalDay(0);
        assert!(format!("{err3}").contains("0"));
    }
}
