//! SAFE note record (stored as `safe-notes/{safe_note_id}.json`).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::error::EquityError;
use super::types::{SafeStatus, SafeType, ShareCount};
use crate::domain::ids::{ContactId, DocumentId, EntityId, MeetingId, ResolutionId, SafeNoteId};
use crate::domain::treasury::types::Cents;

/// Validate data-integrity invariants shared by `new()` and `TryFrom<RawSafeNote>`.
fn validate_safe(
    principal_amount_cents: Cents,
    discount_rate: Option<f64>,
    valuation_cap_cents: Option<Cents>,
) -> Result<(), EquityError> {
    if principal_amount_cents.raw() <= 0 {
        return Err(EquityError::Validation(
            "principal amount must be positive".into(),
        ));
    }
    if let Some(rate) = discount_rate {
        if !(0.0..=1.0).contains(&rate) {
            return Err(EquityError::Validation(
                "discount_rate must be between 0.0 and 1.0".into(),
            ));
        }
    }
    if let Some(cap) = valuation_cap_cents {
        if cap.raw() < principal_amount_cents.raw() {
            return Err(EquityError::ValuationCapBelowPrincipal {
                cap,
                principal: principal_amount_cents,
            });
        }
    }
    Ok(())
}

// ── Raw mirror for deserialization ──────────────────────────────────────

#[derive(Deserialize)]
struct RawSafeNote {
    safe_note_id: SafeNoteId,
    entity_id: EntityId,
    investor_name: String,
    investor_id: Option<ContactId>,
    principal_amount_cents: Cents,
    valuation_cap_cents: Option<Cents>,
    discount_rate: Option<f64>,
    safe_type: SafeType,
    pro_rata_rights: bool,
    status: SafeStatus,
    document_id: Option<DocumentId>,
    board_approval_meeting_id: Option<MeetingId>,
    board_approval_resolution_id: Option<ResolutionId>,
    conversion_unit_type: String,
    issued_at: DateTime<Utc>,
    created_at: DateTime<Utc>,
    converted_at: Option<DateTime<Utc>>,
    conversion_shares: Option<ShareCount>,
    conversion_price_cents: Option<Cents>,
}

impl TryFrom<RawSafeNote> for SafeNote {
    type Error = EquityError;

    fn try_from(raw: RawSafeNote) -> Result<Self, Self::Error> {
        validate_safe(
            raw.principal_amount_cents,
            raw.discount_rate,
            raw.valuation_cap_cents,
        )?;
        Ok(SafeNote {
            safe_note_id: raw.safe_note_id,
            entity_id: raw.entity_id,
            investor_name: raw.investor_name,
            investor_id: raw.investor_id,
            principal_amount_cents: raw.principal_amount_cents,
            valuation_cap_cents: raw.valuation_cap_cents,
            discount_rate: raw.discount_rate,
            safe_type: raw.safe_type,
            pro_rata_rights: raw.pro_rata_rights,
            status: raw.status,
            document_id: raw.document_id,
            board_approval_meeting_id: raw.board_approval_meeting_id,
            board_approval_resolution_id: raw.board_approval_resolution_id,
            conversion_unit_type: raw.conversion_unit_type,
            issued_at: raw.issued_at,
            created_at: raw.created_at,
            converted_at: raw.converted_at,
            conversion_shares: raw.conversion_shares,
            conversion_price_cents: raw.conversion_price_cents,
        })
    }
}

// ── SafeNote ────────────────────────────────────────────────────────────

/// A SAFE (Simple Agreement for Future Equity) note.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(try_from = "RawSafeNote")]
pub struct SafeNote {
    safe_note_id: SafeNoteId,
    entity_id: EntityId,
    investor_name: String,
    investor_id: Option<ContactId>,
    principal_amount_cents: Cents,
    valuation_cap_cents: Option<Cents>,
    discount_rate: Option<f64>,
    safe_type: SafeType,
    pro_rata_rights: bool,
    status: SafeStatus,
    document_id: Option<DocumentId>,
    board_approval_meeting_id: Option<MeetingId>,
    board_approval_resolution_id: Option<ResolutionId>,
    conversion_unit_type: String,
    issued_at: DateTime<Utc>,
    created_at: DateTime<Utc>,
    converted_at: Option<DateTime<Utc>>,
    conversion_shares: Option<ShareCount>,
    conversion_price_cents: Option<Cents>,
}

impl SafeNote {
    /// Create a new SAFE note.
    ///
    /// Validates that valuation cap >= principal amount if both are set.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        safe_note_id: SafeNoteId,
        entity_id: EntityId,
        investor_name: String,
        investor_id: Option<ContactId>,
        principal_amount_cents: Cents,
        valuation_cap_cents: Option<Cents>,
        discount_rate: Option<f64>,
        safe_type: SafeType,
        pro_rata_rights: bool,
        document_id: Option<DocumentId>,
        conversion_unit_type: String,
    ) -> Result<Self, EquityError> {
        validate_safe(principal_amount_cents, discount_rate, valuation_cap_cents)?;
        let now = Utc::now();
        Ok(Self {
            safe_note_id,
            entity_id,
            investor_name,
            investor_id,
            principal_amount_cents,
            valuation_cap_cents,
            discount_rate,
            safe_type,
            pro_rata_rights,
            status: SafeStatus::Issued,
            document_id,
            board_approval_meeting_id: None,
            board_approval_resolution_id: None,
            conversion_unit_type,
            issued_at: now,
            created_at: now,
            converted_at: None,
            conversion_shares: None,
            conversion_price_cents: None,
        })
    }

    /// Convert the SAFE into equity.
    pub fn convert(&mut self, shares: ShareCount, price_cents: Cents) -> Result<(), EquityError> {
        if self.status != SafeStatus::Issued {
            return Err(EquityError::InvalidSafeTransition {
                from: self.status,
                to: SafeStatus::Converted,
            });
        }
        self.status = SafeStatus::Converted;
        self.converted_at = Some(Utc::now());
        self.conversion_shares = Some(shares);
        self.conversion_price_cents = Some(price_cents);
        Ok(())
    }

    /// Cancel the SAFE.
    pub fn cancel(&mut self) -> Result<(), EquityError> {
        if self.status != SafeStatus::Issued {
            return Err(EquityError::InvalidSafeTransition {
                from: self.status,
                to: SafeStatus::Cancelled,
            });
        }
        self.status = SafeStatus::Cancelled;
        Ok(())
    }

    pub fn safe_note_id(&self) -> SafeNoteId {
        self.safe_note_id
    }

    pub fn entity_id(&self) -> EntityId {
        self.entity_id
    }

    pub fn investor_name(&self) -> &str {
        &self.investor_name
    }

    pub fn investor_id(&self) -> Option<ContactId> {
        self.investor_id
    }

    pub fn principal_amount_cents(&self) -> Cents {
        self.principal_amount_cents
    }

    pub fn valuation_cap_cents(&self) -> Option<Cents> {
        self.valuation_cap_cents
    }

    pub fn discount_rate(&self) -> Option<f64> {
        self.discount_rate
    }

    pub fn safe_type(&self) -> SafeType {
        self.safe_type
    }

    pub fn pro_rata_rights(&self) -> bool {
        self.pro_rata_rights
    }

    pub fn status(&self) -> SafeStatus {
        self.status
    }

    pub fn document_id(&self) -> Option<DocumentId> {
        self.document_id
    }

    pub fn board_approval_meeting_id(&self) -> Option<MeetingId> {
        self.board_approval_meeting_id
    }

    pub fn board_approval_resolution_id(&self) -> Option<ResolutionId> {
        self.board_approval_resolution_id
    }

    pub fn conversion_unit_type(&self) -> &str {
        &self.conversion_unit_type
    }

    pub fn issued_at(&self) -> DateTime<Utc> {
        self.issued_at
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    pub fn converted_at(&self) -> Option<DateTime<Utc>> {
        self.converted_at
    }

    pub fn conversion_shares(&self) -> Option<ShareCount> {
        self.conversion_shares
    }

    pub fn conversion_price_cents(&self) -> Option<Cents> {
        self.conversion_price_cents
    }

    pub fn record_board_approval(&mut self, meeting_id: MeetingId, resolution_id: ResolutionId) {
        self.board_approval_meeting_id = Some(meeting_id);
        self.board_approval_resolution_id = Some(resolution_id);
    }
}

/// Result of a SAFE conversion calculation.
#[derive(Debug, Clone)]
pub struct SafeConversionResult {
    pub conversion_price_cents: Cents,
    pub conversion_shares: ShareCount,
    pub price_basis: String,
}

/// Calculate the conversion price and shares for a SAFE note.
///
/// Returns `Err` if `post_money_shares_outstanding` or `pre_money_shares_outstanding` is zero
/// (depending on SAFE type), since that would cause a division-by-zero.
#[allow(clippy::too_many_arguments)]
pub fn calculate_safe_conversion(
    safe_type: SafeType,
    principal_amount_cents: Cents,
    valuation_cap_cents: Option<Cents>,
    discount_rate: Option<f64>,
    financing_price_per_share_cents: Cents,
    pre_money_shares_outstanding: ShareCount,
    post_money_shares_outstanding: ShareCount,
) -> Result<SafeConversionResult, EquityError> {
    if post_money_shares_outstanding.is_zero() {
        return Err(EquityError::Validation(
            "post_money_shares_outstanding must not be zero".into(),
        ));
    }
    if pre_money_shares_outstanding.is_zero() {
        return Err(EquityError::Validation(
            "pre_money_shares_outstanding must not be zero".into(),
        ));
    }
    let financing_price = financing_price_per_share_cents.raw();

    match safe_type {
        SafeType::PostMoney => {
            // Post-money: price = valuation_cap / post_money_shares
            let cap = valuation_cap_cents
                .map(|c| c.raw())
                .unwrap_or(financing_price * post_money_shares_outstanding.raw());
            let cap_price = cap / post_money_shares_outstanding.raw();

            // Use discount if available
            let discount_price =
                discount_rate.map(|r| ((financing_price as f64) * (1.0 - r)) as i64);

            let (price, basis) = match discount_price {
                Some(dp) if dp < cap_price => (dp, "discount".to_string()),
                _ => (cap_price, "valuation_cap".to_string()),
            };

            let price = price.max(1);
            let shares = principal_amount_cents.raw() / price;
            Ok(SafeConversionResult {
                conversion_price_cents: Cents::new(price),
                conversion_shares: ShareCount::new(shares),
                price_basis: basis,
            })
        }
        SafeType::PreMoney => {
            // Pre-money: price = valuation_cap / pre_money_shares
            let cap = valuation_cap_cents
                .map(|c| c.raw())
                .unwrap_or(financing_price * pre_money_shares_outstanding.raw());
            let cap_price = cap / pre_money_shares_outstanding.raw();

            let discount_price =
                discount_rate.map(|r| ((financing_price as f64) * (1.0 - r)) as i64);

            let (price, basis) = match discount_price {
                Some(dp) if dp < cap_price => (dp, "discount".to_string()),
                _ => (cap_price, "valuation_cap".to_string()),
            };

            let price = price.max(1);
            let shares = principal_amount_cents.raw() / price;
            Ok(SafeConversionResult {
                conversion_price_cents: Cents::new(price),
                conversion_shares: ShareCount::new(shares),
                price_basis: basis,
            })
        }
        SafeType::Mfn => {
            // MFN: convert at the most favorable (lowest) price for investor
            let discount_price =
                discount_rate.map(|r| ((financing_price as f64) * (1.0 - r)) as i64);

            let cap_price =
                valuation_cap_cents.map(|c| c.raw() / post_money_shares_outstanding.raw());

            let candidates: Vec<(i64, &str)> = [
                Some((financing_price, "financing_price")),
                discount_price.map(|p| (p, "discount")),
                cap_price.map(|p| (p, "valuation_cap")),
            ]
            .into_iter()
            .flatten()
            .collect();

            let (price, basis) = candidates
                .into_iter()
                .min_by_key(|(p, _)| *p)
                .unwrap_or((financing_price, "financing_price"));

            let price = price.max(1);
            let shares = principal_amount_cents.raw() / price;
            Ok(SafeConversionResult {
                conversion_price_cents: Cents::new(price),
                conversion_shares: ShareCount::new(shares),
                price_basis: basis.to_string(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_safe() -> SafeNote {
        SafeNote::new(
            SafeNoteId::new(),
            EntityId::new(),
            "Investor A".to_string(),
            None,
            Cents::new(100_000_00),          // $100,000
            Some(Cents::new(10_000_000_00)), // $10M cap
            None,
            SafeType::PostMoney,
            true,
            None,
            "shares".to_string(),
        )
        .unwrap()
    }

    #[test]
    fn new_safe() {
        let s = make_safe();
        assert_eq!(s.status(), SafeStatus::Issued);
        assert_eq!(s.investor_name(), "Investor A");
        assert!(s.pro_rata_rights());
    }

    #[test]
    fn convert_safe() {
        let mut s = make_safe();
        s.convert(ShareCount::new(10_000), Cents::new(10_00))
            .unwrap();
        assert_eq!(s.status(), SafeStatus::Converted);
        assert_eq!(s.conversion_shares(), Some(ShareCount::new(10_000)));
        assert!(s.converted_at().is_some());
    }

    #[test]
    fn convert_already_converted() {
        let mut s = make_safe();
        s.convert(ShareCount::new(10_000), Cents::new(10_00))
            .unwrap();
        let result = s.convert(ShareCount::new(5_000), Cents::new(20_00));
        assert!(result.is_err());
    }

    #[test]
    fn cancel_safe() {
        let mut s = make_safe();
        s.cancel().unwrap();
        assert_eq!(s.status(), SafeStatus::Cancelled);
    }

    #[test]
    fn serde_roundtrip() {
        let s = make_safe();
        let json = serde_json::to_string(&s).unwrap();
        let parsed: SafeNote = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.safe_note_id(), s.safe_note_id());
        assert_eq!(parsed.principal_amount_cents(), s.principal_amount_cents());
    }

    #[test]
    fn deserialize_rejects_zero_principal() {
        let s = make_safe();
        let mut json: serde_json::Value = serde_json::to_value(&s).unwrap();
        json["principal_amount_cents"] = serde_json::json!(0);
        let result: Result<SafeNote, _> = serde_json::from_value(json);
        assert!(result.is_err());
    }

    #[test]
    fn deserialize_rejects_bad_discount_rate() {
        let s = make_safe();
        let mut json: serde_json::Value = serde_json::to_value(&s).unwrap();
        json["discount_rate"] = serde_json::json!(1.5);
        let result: Result<SafeNote, _> = serde_json::from_value(json);
        assert!(result.is_err());
    }

    #[test]
    fn conversion_post_money() {
        // $100K SAFE, $10M cap, 10M post-money shares -> price = $1.00/share -> 100K shares
        let result = calculate_safe_conversion(
            SafeType::PostMoney,
            Cents::new(100_000_00),
            Some(Cents::new(10_000_000_00)),
            None,
            Cents::new(2_00), // financing price $2/share
            ShareCount::new(8_000_000),
            ShareCount::new(10_000_000),
        )
        .unwrap();
        assert_eq!(result.conversion_price_cents.raw(), 1_00);
        assert_eq!(result.conversion_shares.raw(), 100_000);
        assert_eq!(result.price_basis, "valuation_cap");
    }

    #[test]
    fn conversion_pre_money() {
        // $100K SAFE, $8M cap, 8M pre-money shares -> price = $1.00/share -> 100K shares
        let result = calculate_safe_conversion(
            SafeType::PreMoney,
            Cents::new(100_000_00),
            Some(Cents::new(8_000_000_00)),
            None,
            Cents::new(2_00),
            ShareCount::new(8_000_000),
            ShareCount::new(10_000_000),
        )
        .unwrap();
        assert_eq!(result.conversion_price_cents.raw(), 1_00);
        assert_eq!(result.conversion_shares.raw(), 100_000);
        assert_eq!(result.price_basis, "valuation_cap");
    }

    #[test]
    fn conversion_mfn() {
        // MFN: takes the best (lowest) price for investor
        let result = calculate_safe_conversion(
            SafeType::Mfn,
            Cents::new(100_000_00),
            Some(Cents::new(10_000_000_00)), // cap price = $1.00
            Some(0.20),                      // discount price = $2.00 * 0.80 = $1.60
            Cents::new(2_00),
            ShareCount::new(8_000_000),
            ShareCount::new(10_000_000),
        )
        .unwrap();
        // Cap price ($1.00) < discount price ($1.60) < financing ($2.00)
        assert_eq!(result.conversion_price_cents.raw(), 1_00);
        assert_eq!(result.price_basis, "valuation_cap");
    }

    #[test]
    fn conversion_discount() {
        // Discount produces lower price than cap
        let result = calculate_safe_conversion(
            SafeType::PostMoney,
            Cents::new(100_000_00),
            Some(Cents::new(20_000_000_00)), // cap price = $2.00
            Some(0.20),                      // discount = $2.50 * 0.80 = $2.00
            Cents::new(2_50),                // financing $2.50
            ShareCount::new(8_000_000),
            ShareCount::new(10_000_000),
        )
        .unwrap();
        // Cap price = $2.00, discount price = $2.00 — cap wins on tie (not strictly less)
        assert_eq!(result.conversion_price_cents.raw(), 2_00);
    }

    #[test]
    fn conversion_rejects_zero_shares() {
        let result = calculate_safe_conversion(
            SafeType::PostMoney,
            Cents::new(100_000_00),
            Some(Cents::new(10_000_000_00)),
            None,
            Cents::new(2_00),
            ShareCount::new(0),
            ShareCount::new(10_000_000),
        );
        assert!(result.is_err());

        let result = calculate_safe_conversion(
            SafeType::PostMoney,
            Cents::new(100_000_00),
            Some(Cents::new(10_000_000_00)),
            None,
            Cents::new(2_00),
            ShareCount::new(8_000_000),
            ShareCount::new(0),
        );
        assert!(result.is_err());
    }
}
