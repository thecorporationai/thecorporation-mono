//! Formation documents — governance documents associated with an entity.
//!
//! A `Document` holds the structured content of a legal document (articles,
//! bylaws, consents, etc.) plus all `Signature`s collected for it.  The
//! `sign()` method appends a signature and automatically transitions the
//! document to `Signed` once every required signer has signed.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::ids::{DocumentId, EntityId, SignatureId, WorkspaceId};

// ── Errors ────────────────────────────────────────────────────────────────────

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum DocumentError {
    #[error("signer {0:?} has already signed this document")]
    AlreadySigned(String),

    #[error("document hash mismatch: expected {expected}, got {actual}")]
    HashMismatch { expected: String, actual: String },

    #[error("document is not in a state that accepts new signatures (status: {0:?})")]
    NotSignable(DocumentStatus),
}

// ── DocumentType ──────────────────────────────────────────────────────────────

/// The category of a formation document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocumentType {
    ArticlesOfIncorporation,
    ArticlesOfOrganization,
    Bylaws,
    IncorporatorAction,
    InitialBoardConsent,
    OperatingAgreement,
    InitialWrittenConsent,
    Ss4Application,
    Resolution,
    SafeAgreement,
    StockTransferAgreement,
    StockPurchaseAgreement,
    StockOptionPlan,
    OptionGrantAgreement,
    RestrictedStockPurchaseAgreement,
    IndemnificationAgreement,
    OfferLetter,
    EmploymentAgreement,
    ContractorAgreement,
    NondisclosureAgreement,
    IntellectualPropertyAssignment,
    BoardConsent,
    StockholderConsent,
    CertificateOfIncorporation,
    Other,
}

// ── DocumentStatus ────────────────────────────────────────────────────────────

/// Lifecycle status of a document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocumentStatus {
    /// Content has been generated but no signatures collected.
    Draft,
    /// All required signers have signed.
    Signed,
    /// The document has been amended after signing.
    Amended,
    /// The document has been filed with a government authority.
    Filed,
}

// ── Signature ─────────────────────────────────────────────────────────────────

/// A single signature applied to a document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Signature {
    pub signature_id: SignatureId,
    pub document_id: DocumentId,

    /// Full legal name of the signer.
    pub signer_name: String,

    /// Organizational role of the signer (e.g. "CEO", "Director").
    pub signer_role: String,

    pub signer_email: String,

    /// Typed or drawn signature representation.
    pub signature_text: String,

    /// Optional SVG representation of the handwritten signature.
    pub signature_svg: Option<String>,

    /// SHA-256 of the document content at the moment of signing.
    /// Signing over a stale hash is rejected by `Document::sign`.
    pub document_hash_at_signing: String,

    pub signed_at: DateTime<Utc>,
}

impl Signature {
    /// Construct a new signature record.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        document_id: DocumentId,
        signer_name: impl Into<String>,
        signer_role: impl Into<String>,
        signer_email: impl Into<String>,
        signature_text: impl Into<String>,
        signature_svg: Option<String>,
        document_hash_at_signing: impl Into<String>,
    ) -> Self {
        Self {
            signature_id: SignatureId::new(),
            document_id,
            signer_name: signer_name.into(),
            signer_role: signer_role.into(),
            signer_email: signer_email.into(),
            signature_text: signature_text.into(),
            signature_svg,
            document_hash_at_signing: document_hash_at_signing.into(),
            signed_at: Utc::now(),
        }
    }
}

// ── Document ──────────────────────────────────────────────────────────────────

/// A formation or governance document belonging to an entity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub document_id: DocumentId,
    pub entity_id: EntityId,
    pub workspace_id: WorkspaceId,

    pub document_type: DocumentType,

    /// Human-readable title (e.g. "Certificate of Incorporation").
    pub title: String,

    /// SHA-256 hex digest of `content` at creation / last amendment.
    pub content_hash: String,

    /// Structured document content (template variables, rendered text, etc.).
    pub content: serde_json::Value,

    pub status: DocumentStatus,

    /// Monotonically increasing amendment counter; starts at 1.
    pub version: u32,

    /// Ordered list of signatures collected so far.
    pub signatures: Vec<Signature>,

    pub created_at: DateTime<Utc>,
}

impl Document {
    /// Create a new document in `Draft` status.
    pub fn new(
        entity_id: EntityId,
        workspace_id: WorkspaceId,
        document_type: DocumentType,
        title: impl Into<String>,
        content: serde_json::Value,
        content_hash: impl Into<String>,
    ) -> Self {
        Self {
            document_id: DocumentId::new(),
            entity_id,
            workspace_id,
            document_type,
            title: title.into(),
            content_hash: content_hash.into(),
            content,
            status: DocumentStatus::Draft,
            version: 1,
            signatures: Vec::new(),
            created_at: Utc::now(),
        }
    }

    // ── Signing ───────────────────────────────────────────────────────────────

    /// Add a signature to the document.
    ///
    /// The caller must supply the SHA-256 hash of the document content *as the
    /// signer observed it*.  If it does not match `self.content_hash` the call
    /// is rejected — this prevents signing a stale version of the document.
    ///
    /// After appending the signature, if `required_signers` is non-empty and
    /// every email in that list has now signed, the document is automatically
    /// transitioned to `Signed`.
    ///
    /// # Parameters
    /// * `signature` — The signature to append.
    /// * `required_signers` — Email addresses of all parties whose signature is
    ///   required.  Pass an empty slice to mark the document as `Signed`
    ///   immediately after any signature.
    ///
    /// # Errors
    /// * [`DocumentError::NotSignable`] — document is already `Filed`.
    /// * [`DocumentError::HashMismatch`] — `document_hash_at_signing` does not
    ///   match `self.content_hash`.
    /// * [`DocumentError::AlreadySigned`] — the signer's email appears in
    ///   `self.signatures` already.
    pub fn sign(
        &mut self,
        signature: Signature,
        required_signers: &[&str],
    ) -> Result<(), DocumentError> {
        if self.status == DocumentStatus::Filed {
            return Err(DocumentError::NotSignable(self.status));
        }

        if signature.document_hash_at_signing != self.content_hash {
            return Err(DocumentError::HashMismatch {
                expected: self.content_hash.clone(),
                actual: signature.document_hash_at_signing.clone(),
            });
        }

        // Idempotency check: reject duplicate signers by email.
        let email = signature.signer_email.clone();
        if self.signatures.iter().any(|s| s.signer_email == email) {
            return Err(DocumentError::AlreadySigned(email));
        }

        self.signatures.push(signature);

        // Transition to Signed if all required signers are present.
        let all_signed = if required_signers.is_empty() {
            true
        } else {
            required_signers
                .iter()
                .all(|e| self.signatures.iter().any(|s| s.signer_email == *e))
        };

        if all_signed {
            self.status = DocumentStatus::Signed;
        }

        Ok(())
    }

    // ── Amendment ─────────────────────────────────────────────────────────────

    /// Record an amendment: update content, hash, bump version, and move to
    /// `Amended` status.  Existing signatures are cleared because the content
    /// has changed.
    pub fn amend(&mut self, new_content: serde_json::Value, new_content_hash: impl Into<String>) {
        self.content = new_content;
        self.content_hash = new_content_hash.into();
        self.version += 1;
        self.signatures.clear();
        self.status = DocumentStatus::Amended;
    }

    /// Mark the document as filed.
    pub fn mark_filed(&mut self) {
        self.status = DocumentStatus::Filed;
    }

    // ── Accessors ─────────────────────────────────────────────────────────────

    /// Returns `true` if the document has been signed by the given email.
    pub fn is_signed_by(&self, email: &str) -> bool {
        self.signatures.iter().any(|s| s.signer_email == email)
    }

    /// Returns the number of signatures collected.
    pub fn signature_count(&self) -> usize {
        self.signatures.len()
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_document() -> Document {
        Document::new(
            EntityId::new(),
            WorkspaceId::new(),
            DocumentType::ArticlesOfIncorporation,
            "Articles of Incorporation",
            serde_json::json!({"company": "Acme Corp"}),
            "abc123hash",
        )
    }

    fn make_document_of_type(dt: DocumentType) -> Document {
        Document::new(
            EntityId::new(),
            WorkspaceId::new(),
            dt,
            "Test Document",
            serde_json::json!({}),
            "deadbeef",
        )
    }

    fn make_signature(doc: &Document, email: &str) -> Signature {
        Signature::new(
            doc.document_id,
            "Jane Founder",
            "CEO",
            email,
            "Jane Founder",
            None,
            doc.content_hash.clone(),
        )
    }

    // ── Document::new() ───────────────────────────────────────────────────────

    #[test]
    fn new_document_is_draft_v1() {
        let d = make_document();
        assert_eq!(d.status, DocumentStatus::Draft);
        assert_eq!(d.version, 1);
        assert!(d.signatures.is_empty());
    }

    #[test]
    fn new_document_stores_title() {
        let d = make_document();
        assert_eq!(d.title, "Articles of Incorporation");
    }

    #[test]
    fn new_document_stores_content_hash() {
        let d = make_document();
        assert_eq!(d.content_hash, "abc123hash");
    }

    #[test]
    fn new_document_articles_of_incorporation() {
        let d = make_document_of_type(DocumentType::ArticlesOfIncorporation);
        assert_eq!(d.document_type, DocumentType::ArticlesOfIncorporation);
    }

    #[test]
    fn new_document_articles_of_organization() {
        let d = make_document_of_type(DocumentType::ArticlesOfOrganization);
        assert_eq!(d.document_type, DocumentType::ArticlesOfOrganization);
    }

    #[test]
    fn new_document_bylaws() {
        let d = make_document_of_type(DocumentType::Bylaws);
        assert_eq!(d.document_type, DocumentType::Bylaws);
    }

    #[test]
    fn new_document_initial_board_consent() {
        let d = make_document_of_type(DocumentType::InitialBoardConsent);
        assert_eq!(d.document_type, DocumentType::InitialBoardConsent);
    }

    #[test]
    fn new_document_operating_agreement() {
        let d = make_document_of_type(DocumentType::OperatingAgreement);
        assert_eq!(d.document_type, DocumentType::OperatingAgreement);
    }

    #[test]
    fn new_document_ss4_application() {
        let d = make_document_of_type(DocumentType::Ss4Application);
        assert_eq!(d.document_type, DocumentType::Ss4Application);
    }

    #[test]
    fn new_document_safe_agreement() {
        let d = make_document_of_type(DocumentType::SafeAgreement);
        assert_eq!(d.document_type, DocumentType::SafeAgreement);
    }

    #[test]
    fn new_document_stock_option_plan() {
        let d = make_document_of_type(DocumentType::StockOptionPlan);
        assert_eq!(d.document_type, DocumentType::StockOptionPlan);
    }

    #[test]
    fn new_document_nda() {
        let d = make_document_of_type(DocumentType::NondisclosureAgreement);
        assert_eq!(d.document_type, DocumentType::NondisclosureAgreement);
    }

    #[test]
    fn new_document_other() {
        let d = make_document_of_type(DocumentType::Other);
        assert_eq!(d.document_type, DocumentType::Other);
    }

    // ── DocumentType serde roundtrip ──────────────────────────────────────────

    #[test]
    fn document_type_serde_articles_of_incorporation() {
        let json = serde_json::to_string(&DocumentType::ArticlesOfIncorporation).unwrap();
        assert_eq!(json, r#""articles_of_incorporation""#);
        let de: DocumentType = serde_json::from_str(&json).unwrap();
        assert_eq!(de, DocumentType::ArticlesOfIncorporation);
    }

    #[test]
    fn document_type_serde_articles_of_organization() {
        let json = serde_json::to_string(&DocumentType::ArticlesOfOrganization).unwrap();
        assert_eq!(json, r#""articles_of_organization""#);
    }

    #[test]
    fn document_type_serde_bylaws() {
        let json = serde_json::to_string(&DocumentType::Bylaws).unwrap();
        assert_eq!(json, r#""bylaws""#);
        let de: DocumentType = serde_json::from_str(&json).unwrap();
        assert_eq!(de, DocumentType::Bylaws);
    }

    #[test]
    fn document_type_serde_incorporator_action() {
        let json = serde_json::to_string(&DocumentType::IncorporatorAction).unwrap();
        assert_eq!(json, r#""incorporator_action""#);
    }

    #[test]
    fn document_type_serde_initial_board_consent() {
        let json = serde_json::to_string(&DocumentType::InitialBoardConsent).unwrap();
        assert_eq!(json, r#""initial_board_consent""#);
    }

    #[test]
    fn document_type_serde_operating_agreement() {
        let json = serde_json::to_string(&DocumentType::OperatingAgreement).unwrap();
        assert_eq!(json, r#""operating_agreement""#);
    }

    #[test]
    fn document_type_serde_initial_written_consent() {
        let json = serde_json::to_string(&DocumentType::InitialWrittenConsent).unwrap();
        assert_eq!(json, r#""initial_written_consent""#);
    }

    #[test]
    fn document_type_serde_ss4_application() {
        let json = serde_json::to_string(&DocumentType::Ss4Application).unwrap();
        assert_eq!(json, r#""ss4_application""#);
    }

    #[test]
    fn document_type_serde_resolution() {
        let json = serde_json::to_string(&DocumentType::Resolution).unwrap();
        assert_eq!(json, r#""resolution""#);
    }

    #[test]
    fn document_type_serde_safe_agreement() {
        let json = serde_json::to_string(&DocumentType::SafeAgreement).unwrap();
        assert_eq!(json, r#""safe_agreement""#);
    }

    #[test]
    fn document_type_serde_stock_transfer_agreement() {
        let json = serde_json::to_string(&DocumentType::StockTransferAgreement).unwrap();
        assert_eq!(json, r#""stock_transfer_agreement""#);
    }

    #[test]
    fn document_type_serde_stock_purchase_agreement() {
        let json = serde_json::to_string(&DocumentType::StockPurchaseAgreement).unwrap();
        assert_eq!(json, r#""stock_purchase_agreement""#);
    }

    #[test]
    fn document_type_serde_stock_option_plan() {
        let json = serde_json::to_string(&DocumentType::StockOptionPlan).unwrap();
        assert_eq!(json, r#""stock_option_plan""#);
    }

    #[test]
    fn document_type_serde_option_grant_agreement() {
        let json = serde_json::to_string(&DocumentType::OptionGrantAgreement).unwrap();
        assert_eq!(json, r#""option_grant_agreement""#);
    }

    #[test]
    fn document_type_serde_restricted_stock_purchase_agreement() {
        let json = serde_json::to_string(&DocumentType::RestrictedStockPurchaseAgreement).unwrap();
        assert_eq!(json, r#""restricted_stock_purchase_agreement""#);
    }

    #[test]
    fn document_type_serde_indemnification_agreement() {
        let json = serde_json::to_string(&DocumentType::IndemnificationAgreement).unwrap();
        assert_eq!(json, r#""indemnification_agreement""#);
    }

    #[test]
    fn document_type_serde_offer_letter() {
        let json = serde_json::to_string(&DocumentType::OfferLetter).unwrap();
        assert_eq!(json, r#""offer_letter""#);
    }

    #[test]
    fn document_type_serde_employment_agreement() {
        let json = serde_json::to_string(&DocumentType::EmploymentAgreement).unwrap();
        assert_eq!(json, r#""employment_agreement""#);
    }

    #[test]
    fn document_type_serde_contractor_agreement() {
        let json = serde_json::to_string(&DocumentType::ContractorAgreement).unwrap();
        assert_eq!(json, r#""contractor_agreement""#);
    }

    #[test]
    fn document_type_serde_nda() {
        let json = serde_json::to_string(&DocumentType::NondisclosureAgreement).unwrap();
        assert_eq!(json, r#""nondisclosure_agreement""#);
    }

    #[test]
    fn document_type_serde_ip_assignment() {
        let json = serde_json::to_string(&DocumentType::IntellectualPropertyAssignment).unwrap();
        assert_eq!(json, r#""intellectual_property_assignment""#);
    }

    #[test]
    fn document_type_serde_board_consent() {
        let json = serde_json::to_string(&DocumentType::BoardConsent).unwrap();
        assert_eq!(json, r#""board_consent""#);
    }

    #[test]
    fn document_type_serde_stockholder_consent() {
        let json = serde_json::to_string(&DocumentType::StockholderConsent).unwrap();
        assert_eq!(json, r#""stockholder_consent""#);
    }

    #[test]
    fn document_type_serde_certificate_of_incorporation() {
        let json = serde_json::to_string(&DocumentType::CertificateOfIncorporation).unwrap();
        assert_eq!(json, r#""certificate_of_incorporation""#);
    }

    #[test]
    fn document_type_serde_other() {
        let json = serde_json::to_string(&DocumentType::Other).unwrap();
        assert_eq!(json, r#""other""#);
        let de: DocumentType = serde_json::from_str(&json).unwrap();
        assert_eq!(de, DocumentType::Other);
    }

    // ── DocumentStatus serde roundtrip ────────────────────────────────────────

    #[test]
    fn document_status_serde_draft() {
        let json = serde_json::to_string(&DocumentStatus::Draft).unwrap();
        assert_eq!(json, r#""draft""#);
        let de: DocumentStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(de, DocumentStatus::Draft);
    }

    #[test]
    fn document_status_serde_signed() {
        let json = serde_json::to_string(&DocumentStatus::Signed).unwrap();
        assert_eq!(json, r#""signed""#);
    }

    #[test]
    fn document_status_serde_amended() {
        let json = serde_json::to_string(&DocumentStatus::Amended).unwrap();
        assert_eq!(json, r#""amended""#);
    }

    #[test]
    fn document_status_serde_filed() {
        let json = serde_json::to_string(&DocumentStatus::Filed).unwrap();
        assert_eq!(json, r#""filed""#);
    }

    // ── sign() ────────────────────────────────────────────────────────────────

    #[test]
    fn sign_with_no_required_signers_transitions_to_signed() {
        let mut d = make_document();
        let sig = make_signature(&d, "jane@example.com");
        d.sign(sig, &[]).unwrap();
        assert_eq!(d.status, DocumentStatus::Signed);
        assert_eq!(d.signature_count(), 1);
    }

    #[test]
    fn sign_stores_signer_info() {
        let mut d = make_document();
        let sig = make_signature(&d, "jane@example.com");
        d.sign(sig, &[]).unwrap();
        assert!(d.is_signed_by("jane@example.com"));
    }

    #[test]
    fn sign_with_pending_required_signers_stays_draft() {
        let mut d = make_document();
        let sig = make_signature(&d, "jane@example.com");
        d.sign(sig, &["jane@example.com", "bob@example.com"])
            .unwrap();
        assert_eq!(d.status, DocumentStatus::Draft);
    }

    #[test]
    fn sign_all_required_signers_transitions_to_signed() {
        let mut d = make_document();
        let sig1 = make_signature(&d, "jane@example.com");
        let sig2 = Signature::new(
            d.document_id,
            "Bob Director",
            "Director",
            "bob@example.com",
            "Bob Director",
            None,
            d.content_hash.clone(),
        );
        let required = &["jane@example.com", "bob@example.com"];
        d.sign(sig1, required).unwrap();
        d.sign(sig2, required).unwrap();
        assert_eq!(d.status, DocumentStatus::Signed);
    }

    #[test]
    fn sign_three_signers_all_required() {
        let mut d = make_document();
        let emails = ["a@x.com", "b@x.com", "c@x.com"];
        let required: Vec<&str> = emails.iter().copied().collect();
        for email in &emails {
            let sig = make_signature(&d, email);
            d.sign(sig, &required).unwrap();
        }
        assert_eq!(d.status, DocumentStatus::Signed);
        assert_eq!(d.signature_count(), 3);
    }

    #[test]
    fn sign_rejects_hash_mismatch() {
        let mut d = make_document();
        let mut sig = make_signature(&d, "jane@example.com");
        sig.document_hash_at_signing = "stalehash".into();
        assert!(matches!(
            d.sign(sig, &[]),
            Err(DocumentError::HashMismatch { .. })
        ));
    }

    #[test]
    fn sign_hash_mismatch_error_contains_expected_and_actual() {
        let mut d = make_document();
        let mut sig = make_signature(&d, "jane@example.com");
        sig.document_hash_at_signing = "wronghash".into();
        match d.sign(sig, &[]) {
            Err(DocumentError::HashMismatch { expected, actual }) => {
                assert_eq!(expected, "abc123hash");
                assert_eq!(actual, "wronghash");
            }
            _ => panic!("expected HashMismatch"),
        }
    }

    #[test]
    fn sign_rejects_duplicate_signer() {
        let mut d = make_document();
        let sig1 = make_signature(&d, "jane@example.com");
        let sig2 = make_signature(&d, "jane@example.com");
        d.sign(sig1, &[]).unwrap();
        assert!(matches!(
            d.sign(sig2, &[]),
            Err(DocumentError::AlreadySigned(_))
        ));
    }

    #[test]
    fn sign_with_svg_signature() {
        let mut d = make_document();
        let sig = Signature::new(
            d.document_id,
            "Jane Founder",
            "CEO",
            "jane@example.com",
            "Jane Founder",
            Some("<svg>...</svg>".to_string()),
            d.content_hash.clone(),
        );
        d.sign(sig, &[]).unwrap();
        assert_eq!(d.signature_count(), 1);
        assert!(d.signatures[0].signature_svg.is_some());
    }

    #[test]
    fn sign_filed_document_fails() {
        let mut d = make_document();
        d.mark_filed();
        let sig = make_signature(&d, "jane@example.com");
        assert!(matches!(
            d.sign(sig, &[]),
            Err(DocumentError::NotSignable(DocumentStatus::Filed))
        ));
    }

    #[test]
    fn is_signed_by_returns_false_for_unknown_email() {
        let d = make_document();
        assert!(!d.is_signed_by("unknown@example.com"));
    }

    #[test]
    fn signature_count_zero_initially() {
        let d = make_document();
        assert_eq!(d.signature_count(), 0);
    }

    // ── amend() ───────────────────────────────────────────────────────────────

    #[test]
    fn amend_resets_signatures_and_bumps_version() {
        let mut d = make_document();
        let sig = make_signature(&d, "jane@example.com");
        d.sign(sig, &[]).unwrap();
        assert_eq!(d.status, DocumentStatus::Signed);

        d.amend(serde_json::json!({"company": "Acme Inc"}), "newhash456");
        assert_eq!(d.version, 2);
        assert_eq!(d.status, DocumentStatus::Amended);
        assert!(d.signatures.is_empty());
        assert_eq!(d.content_hash, "newhash456");
    }

    #[test]
    fn amend_from_draft_bumps_version() {
        let mut d = make_document();
        d.amend(serde_json::json!({"v": 2}), "hash2");
        assert_eq!(d.version, 2);
        assert_eq!(d.status, DocumentStatus::Amended);
    }

    #[test]
    fn amend_multiple_times_increments_version() {
        let mut d = make_document();
        d.amend(serde_json::json!({}), "h2");
        d.amend(serde_json::json!({}), "h3");
        d.amend(serde_json::json!({}), "h4");
        assert_eq!(d.version, 4);
    }

    #[test]
    fn amend_updates_content_hash_so_new_signatures_work() {
        let mut d = make_document();
        d.amend(serde_json::json!({"updated": true}), "newhash");
        // After amendment the hash is updated; a new signature against newhash should work.
        let sig = Signature::new(
            d.document_id,
            "Jane",
            "CEO",
            "jane@example.com",
            "Jane",
            None,
            "newhash",
        );
        d.sign(sig, &[]).unwrap();
        assert_eq!(d.status, DocumentStatus::Signed);
    }

    // ── mark_filed() ──────────────────────────────────────────────────────────

    #[test]
    fn mark_filed_sets_status() {
        let mut d = make_document();
        d.mark_filed();
        assert_eq!(d.status, DocumentStatus::Filed);
    }

    #[test]
    fn mark_filed_after_signing() {
        let mut d = make_document();
        let sig = make_signature(&d, "jane@example.com");
        d.sign(sig, &[]).unwrap();
        d.mark_filed();
        assert_eq!(d.status, DocumentStatus::Filed);
    }

    // ── JSON roundtrip ────────────────────────────────────────────────────────

    #[test]
    fn json_roundtrip() {
        let d = make_document();
        let json = serde_json::to_string(&d).unwrap();
        let de: Document = serde_json::from_str(&json).unwrap();
        assert_eq!(d.document_id, de.document_id);
    }

    #[test]
    fn json_roundtrip_with_signatures() {
        let mut d = make_document();
        let sig = make_signature(&d, "jane@example.com");
        d.sign(sig, &[]).unwrap();
        let json = serde_json::to_string(&d).unwrap();
        let de: Document = serde_json::from_str(&json).unwrap();
        assert_eq!(de.signature_count(), 1);
        assert_eq!(de.status, DocumentStatus::Signed);
    }
}
