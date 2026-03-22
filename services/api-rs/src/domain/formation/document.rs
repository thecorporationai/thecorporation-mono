//! Document and signature records.
//!
//! Documents are stored as `formation/{document_id}.json` in the entity's git
//! repository. Signatures are embedded within the document record.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::error::FormationError;
use super::types::{DocumentStatus, DocumentType};
use crate::domain::ids::{DocumentId, EntityId, SignatureId, WorkspaceId};

// ── Document ─────────────────────────────────────────────────────────────

/// A legal document associated with an entity's formation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    document_id: DocumentId,
    entity_id: EntityId,
    workspace_id: WorkspaceId,
    document_type: DocumentType,
    title: String,
    content_hash: String,
    content: serde_json::Value,
    status: DocumentStatus,
    version: u32,
    governance_tag: Option<String>,
    supersedes_document_id: Option<DocumentId>,
    superseded_by_document_id: Option<DocumentId>,
    signatures: Vec<Signature>,
    created_at: DateTime<Utc>,
}

impl Document {
    /// Create a new document. The content hash is computed automatically.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        document_id: DocumentId,
        entity_id: EntityId,
        workspace_id: WorkspaceId,
        document_type: DocumentType,
        title: String,
        content: serde_json::Value,
        governance_tag: Option<String>,
        supersedes_document_id: Option<DocumentId>,
    ) -> Self {
        let content_hash = Self::compute_content_hash(&content);
        Self {
            document_id,
            entity_id,
            workspace_id,
            document_type,
            title,
            content_hash,
            content,
            status: DocumentStatus::Draft,
            version: 1,
            governance_tag,
            supersedes_document_id,
            superseded_by_document_id: None,
            signatures: Vec::new(),
            created_at: Utc::now(),
        }
    }

    /// Add a signature to this document.
    ///
    /// If all required signers have signed, the document status is automatically
    /// advanced to `Signed`.
    pub fn sign(&mut self, req: SignatureRequest) -> Result<SignatureId, FormationError> {
        if self.status == DocumentStatus::Signed {
            return Err(FormationError::DocumentAlreadySigned(self.document_id));
        }

        let sig_id = SignatureId::new();
        let signature = Signature {
            signature_id: sig_id,
            document_id: self.document_id,
            signer_name: req.signer_name,
            signer_role: req.signer_role,
            signer_email: req.signer_email,
            signature_text: req.signature_text,
            signature_svg: req.signature_svg,
            document_hash_at_signing: self.content_hash.clone(),
            ip_address: req.ip_address,
            consent_text: req.consent_text,
            signed_at: Utc::now(),
        };
        self.signatures.push(signature);

        if self.is_fully_signed() {
            self.status = DocumentStatus::Signed;
        }

        Ok(sig_id)
    }

    /// Check whether the given hash matches the current content hash.
    pub fn content_hash_matches(&self, hash: &str) -> bool {
        self.content_hash == hash
    }

    /// Check whether all required signers (from `signature_requirements` in
    /// the document content) have signed.
    pub fn is_fully_signed(&self) -> bool {
        let required = self.required_signer_roles();
        if required.is_empty() {
            // No requirements specified — any signature counts as fully signed
            return !self.signatures.is_empty();
        }

        required.iter().all(|required_role| {
            let required_normalized = normalize_role(required_role);
            self.signatures
                .iter()
                .any(|sig| normalize_role(sig.signer_role()) == required_normalized)
        })
    }

    /// Compute a SHA-256 hash of the canonicalized JSON content.
    pub fn compute_content_hash(content: &serde_json::Value) -> String {
        let canonical = serde_json::to_string(content).unwrap_or_default();
        let mut hasher = Sha256::new();
        hasher.update(canonical.as_bytes());
        hex::encode(hasher.finalize())
    }

    // ── Private helpers ─────────────────────────────────────────────────

    /// Extract required signer roles from the `signature_requirements` field
    /// in the document content, if present.
    fn required_signer_roles(&self) -> Vec<String> {
        self.content
            .get("signature_requirements")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|item| item.get("role").and_then(|r| r.as_str()).map(String::from))
                    .collect()
            })
            .unwrap_or_default()
    }

    // ── Accessors ────────────────────────────────────────────────────────

    pub fn document_id(&self) -> DocumentId {
        self.document_id
    }

    pub fn entity_id(&self) -> EntityId {
        self.entity_id
    }

    pub fn workspace_id(&self) -> WorkspaceId {
        self.workspace_id
    }

    pub fn document_type(&self) -> DocumentType {
        self.document_type
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn content_hash(&self) -> &str {
        &self.content_hash
    }

    pub fn content(&self) -> &serde_json::Value {
        &self.content
    }

    pub fn status(&self) -> DocumentStatus {
        self.status
    }

    pub fn version(&self) -> u32 {
        self.version
    }

    pub fn governance_tag(&self) -> Option<&str> {
        self.governance_tag.as_deref()
    }

    pub fn supersedes_document_id(&self) -> Option<DocumentId> {
        self.supersedes_document_id
    }

    pub fn superseded_by_document_id(&self) -> Option<DocumentId> {
        self.superseded_by_document_id
    }

    pub fn signatures(&self) -> &[Signature] {
        &self.signatures
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    /// Set the superseded-by link (used when a new version replaces this doc).
    pub fn set_superseded_by(&mut self, doc_id: DocumentId) {
        self.superseded_by_document_id = Some(doc_id);
        self.status = DocumentStatus::Amended;
    }
}

fn normalize_role(role: &str) -> String {
    role.trim().to_ascii_lowercase()
}

// ── Signature ────────────────────────────────────────────────────────────

/// A cryptographic signature on a document, embedded within the `Document` record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Signature {
    signature_id: SignatureId,
    document_id: DocumentId,
    signer_name: String,
    signer_role: String,
    signer_email: String,
    signature_text: String,
    signature_svg: Option<String>,
    document_hash_at_signing: String,
    ip_address: Option<String>,
    consent_text: String,
    signed_at: DateTime<Utc>,
}

impl Signature {
    pub fn signature_id(&self) -> SignatureId {
        self.signature_id
    }

    pub fn document_id(&self) -> DocumentId {
        self.document_id
    }

    pub fn signer_name(&self) -> &str {
        &self.signer_name
    }

    pub fn signer_role(&self) -> &str {
        &self.signer_role
    }

    pub fn signer_email(&self) -> &str {
        &self.signer_email
    }

    pub fn signature_text(&self) -> &str {
        &self.signature_text
    }

    pub fn signature_svg(&self) -> Option<&str> {
        self.signature_svg.as_deref()
    }

    pub fn document_hash_at_signing(&self) -> &str {
        &self.document_hash_at_signing
    }

    pub fn ip_address(&self) -> Option<&str> {
        self.ip_address.as_deref()
    }

    pub fn consent_text(&self) -> &str {
        &self.consent_text
    }

    pub fn signed_at(&self) -> DateTime<Utc> {
        self.signed_at
    }
}

// ── SignatureRequest ─────────────────────────────────────────────────────

/// Input for creating a new signature (not stored directly).
#[derive(Debug, Clone)]
pub struct SignatureRequest {
    pub signer_name: String,
    pub signer_role: String,
    pub signer_email: String,
    pub signature_text: String,
    pub consent_text: String,
    pub signature_svg: Option<String>,
    pub ip_address: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn make_document() -> Document {
        Document::new(
            DocumentId::new(),
            EntityId::new(),
            WorkspaceId::new(),
            DocumentType::ArticlesOfIncorporation,
            "Articles of Incorporation".into(),
            json!({
                "body": "...",
                "signature_requirements": [
                    {"role": "incorporator", "required": true},
                    {"role": "registered_agent", "required": true}
                ]
            }),
            None,
            None,
        )
    }

    fn make_sig_request(role: &str) -> SignatureRequest {
        SignatureRequest {
            signer_name: "Jane Doe".into(),
            signer_role: role.into(),
            signer_email: "jane@acme.com".into(),
            signature_text: "/s/ Jane Doe".into(),
            consent_text: "I agree".into(),
            signature_svg: None,
            ip_address: Some("127.0.0.1".into()),
        }
    }

    #[test]
    fn new_document_is_draft() {
        let doc = make_document();
        assert_eq!(doc.status(), DocumentStatus::Draft);
        assert_eq!(doc.version(), 1);
        assert!(doc.signatures().is_empty());
    }

    #[test]
    fn content_hash_is_deterministic() {
        let content = json!({"a": 1, "b": 2});
        let h1 = Document::compute_content_hash(&content);
        let h2 = Document::compute_content_hash(&content);
        assert_eq!(h1, h2);
    }

    #[test]
    fn sign_partial_does_not_advance_status() {
        let mut doc = make_document();
        let sig_id = doc.sign(make_sig_request("incorporator")).unwrap();
        assert_ne!(sig_id.to_string(), "");
        assert_eq!(doc.status(), DocumentStatus::Draft);
        assert!(!doc.is_fully_signed());
    }

    #[test]
    fn sign_all_required_advances_to_signed() {
        let mut doc = make_document();
        doc.sign(make_sig_request("incorporator")).unwrap();
        doc.sign(make_sig_request("registered_agent")).unwrap();
        assert_eq!(doc.status(), DocumentStatus::Signed);
        assert!(doc.is_fully_signed());
    }

    #[test]
    fn required_roles_match_case_insensitively() {
        let mut doc = make_document();
        doc.sign(make_sig_request("Incorporator")).unwrap();
        doc.sign(make_sig_request("REGISTERED_AGENT")).unwrap();
        assert_eq!(doc.status(), DocumentStatus::Signed);
        assert!(doc.is_fully_signed());
    }

    #[test]
    fn cannot_sign_already_signed_document() {
        let mut doc = make_document();
        doc.sign(make_sig_request("incorporator")).unwrap();
        doc.sign(make_sig_request("registered_agent")).unwrap();
        let result = doc.sign(make_sig_request("extra"));
        assert!(result.is_err());
    }

    #[test]
    fn content_hash_matches() {
        let doc = make_document();
        let hash = doc.content_hash().to_string();
        assert!(doc.content_hash_matches(&hash));
        assert!(!doc.content_hash_matches("bad_hash"));
    }

    #[test]
    fn serde_roundtrip() {
        let mut doc = make_document();
        doc.sign(make_sig_request("incorporator")).unwrap();
        let json = serde_json::to_string(&doc).unwrap();
        let parsed: Document = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.document_id(), doc.document_id());
        assert_eq!(parsed.signatures().len(), 1);
    }

    // ── Property-based-style signing FSM tests ───────────────────────────

    /// Signing with a wrong role never transitions the document to Signed when
    /// required roles remain unmet. We test this across multiple wrong-role
    /// strings (none of which satisfy the two required roles).
    #[test]
    fn wrong_role_never_advances_to_signed() {
        let wrong_roles = [
            "ceo",
            "director",
            "notary",
            "witness",
            "attorney",
            "",
            "incorporator_extra",
            "registered agent", // space variant, not underscore
        ];
        for role in wrong_roles {
            let mut doc = make_document();
            // A wrong-role signature is accepted (no identity enforcement) but
            // the document must remain Draft since required roles aren't covered.
            let result = doc.sign(make_sig_request(role));
            assert!(result.is_ok(), "sign() should accept any role, got Err for {role:?}");
            assert_eq!(
                doc.status(),
                DocumentStatus::Draft,
                "status should stay Draft after signing with wrong role {role:?}"
            );
            assert!(!doc.is_fully_signed(), "should not be fully signed with wrong role {role:?}");
        }
    }

    /// After a document reaches Signed, every subsequent sign attempt returns
    /// `DocumentAlreadySigned` regardless of what role is presented.
    #[test]
    fn every_sign_attempt_on_signed_doc_fails() {
        let extra_signers = [
            "incorporator",
            "registered_agent",
            "ceo",
            "witness",
        ];
        for extra_role in extra_signers {
            let mut doc = make_document();
            // Fully sign the document.
            doc.sign(make_sig_request("incorporator")).unwrap();
            doc.sign(make_sig_request("registered_agent")).unwrap();
            assert_eq!(doc.status(), DocumentStatus::Signed);

            let result = doc.sign(make_sig_request(extra_role));
            assert!(
                result.is_err(),
                "expected signing a Signed document to fail for role {extra_role:?}"
            );
            // Verify error variant.
            assert!(
                matches!(result.unwrap_err(), super::super::error::FormationError::DocumentAlreadySigned(_)),
                "expected DocumentAlreadySigned error"
            );
        }
    }

    /// Partial signatures accumulate without changing status; only the
    /// final required role flips the document to Signed.
    #[test]
    fn partial_signatures_accumulate_correctly() {
        // Each iteration adds one more partial signer before the first required role.
        let extra_counts = [0usize, 1, 2, 5];
        for extra_count in extra_counts {
            let mut doc = make_document();
            // Add `extra_count` wrong-role signatures.
            for i in 0..extra_count {
                doc.sign(make_sig_request(&format!("witness_{i}"))).unwrap();
                assert_eq!(doc.status(), DocumentStatus::Draft);
            }
            // Sign first required role — still Draft.
            doc.sign(make_sig_request("incorporator")).unwrap();
            assert_eq!(doc.status(), DocumentStatus::Draft);
            // Sign second required role — now Signed.
            doc.sign(make_sig_request("registered_agent")).unwrap();
            assert_eq!(doc.status(), DocumentStatus::Signed);
            assert_eq!(doc.signatures().len(), extra_count + 2);
        }
    }

    /// A document with no `signature_requirements` flips to Signed after any
    /// single signature (the "no requirements" fast-path).
    #[test]
    fn document_without_requirements_signs_on_first_sig() {
        let roles = ["anyone", "ceo", "founder", "agent"];
        for role in roles {
            let mut doc = Document::new(
                DocumentId::new(),
                EntityId::new(),
                WorkspaceId::new(),
                DocumentType::Resolution,
                "Simple Resolution".into(),
                json!({"body": "resolved"}), // no signature_requirements key
                None,
                None,
            );
            assert_eq!(doc.status(), DocumentStatus::Draft);
            doc.sign(make_sig_request(role)).unwrap();
            assert_eq!(
                doc.status(),
                DocumentStatus::Signed,
                "expected Signed after one sig with role {role:?}"
            );
        }
    }
}
