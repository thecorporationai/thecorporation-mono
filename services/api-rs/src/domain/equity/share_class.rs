//! Share class record (stored as `cap-table/classes/{share_class_id}.json`).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::types::{ShareCount, StockType};
use crate::domain::ids::{CapTableId, ShareClassId};

/// A class of shares (Common, Preferred, Unit, etc.) within a cap table.
///
/// NOTE: No `outstanding_shares` field — computed dynamically by summing grants.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareClass {
    share_class_id: ShareClassId,
    cap_table_id: CapTableId,
    class_code: String,
    stock_type: StockType,
    par_value: String,
    authorized_shares: ShareCount,
    liquidation_preference: Option<String>,
    created_at: DateTime<Utc>,
}

impl ShareClass {
    /// Create a new share class.
    pub fn new(
        share_class_id: ShareClassId,
        cap_table_id: CapTableId,
        class_code: String,
        stock_type: StockType,
        par_value: String,
        authorized_shares: ShareCount,
        liquidation_preference: Option<String>,
    ) -> Self {
        Self {
            share_class_id,
            cap_table_id,
            class_code,
            stock_type,
            par_value,
            authorized_shares,
            liquidation_preference,
            created_at: Utc::now(),
        }
    }

    pub fn share_class_id(&self) -> ShareClassId {
        self.share_class_id
    }

    pub fn cap_table_id(&self) -> CapTableId {
        self.cap_table_id
    }

    pub fn class_code(&self) -> &str {
        &self.class_code
    }

    pub fn stock_type(&self) -> StockType {
        self.stock_type
    }

    pub fn par_value(&self) -> &str {
        &self.par_value
    }

    pub fn authorized_shares(&self) -> ShareCount {
        self.authorized_shares
    }

    pub fn liquidation_preference(&self) -> Option<&str> {
        self.liquidation_preference.as_deref()
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_share_class() {
        let sc = ShareClass::new(
            ShareClassId::new(),
            CapTableId::new(),
            "COMMON".to_string(),
            StockType::Common,
            "0.0001".to_string(),
            ShareCount::new(10_000_000),
            None,
        );
        assert_eq!(sc.class_code(), "COMMON");
        assert_eq!(sc.stock_type(), StockType::Common);
        assert_eq!(sc.authorized_shares().raw(), 10_000_000);
        assert!(sc.liquidation_preference().is_none());
    }

    #[test]
    fn serde_roundtrip() {
        let sc = ShareClass::new(
            ShareClassId::new(),
            CapTableId::new(),
            "PREFERRED".to_string(),
            StockType::Preferred,
            "0.01".to_string(),
            ShareCount::new(5_000_000),
            Some("1x".to_string()),
        );
        let json = serde_json::to_string(&sc).unwrap();
        let parsed: ShareClass = serde_json::from_str(&json).unwrap();
        assert_eq!(sc.share_class_id(), parsed.share_class_id());
        assert_eq!(sc.class_code(), parsed.class_code());
        assert_eq!(sc.liquidation_preference(), parsed.liquidation_preference());
    }
}
