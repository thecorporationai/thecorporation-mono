//! Formation domain — legal entity creation and lifecycle management.
//!
//! This module covers everything from the initial filing of formation documents
//! through state acceptance, EIN acquisition, and eventual dissolution.
//!
//! ## Submodules
//!
//! | Module | Purpose |
//! |--------|---------|
//! | [`entity`] | Root aggregate: legal name, type, jurisdiction, status FSM |
//! | [`document`] | Formation documents with signature collection |
//! | [`filing`] | State filing submissions and confirmations |
//! | [`tax_profile`] | EIN and IRS tax classification |

pub mod document;
pub mod entity;
pub mod filing;
pub mod tax_profile;

// ── Re-exports ────────────────────────────────────────────────────────────────

pub use document::{Document, DocumentError, DocumentStatus, DocumentType, Signature};
pub use entity::{Entity, EntityError, EntityType, FormationStatus, Jurisdiction};
pub use filing::{Filing, FilingError, FilingStatus, FilingType};
pub use tax_profile::{EinStatus, IrsTaxClassification, TaxProfile, TaxProfileError};
