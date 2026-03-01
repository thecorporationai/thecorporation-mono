//! Intent record (stored as `execution/intents/{intent_id}.json`).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::error::ExecutionError;
use super::types::{AuthorityTier, IntentStatus};
use crate::domain::governance::policy_engine::PolicyDecision;
use crate::domain::ids::{ApprovalArtifactId, DocumentRequestId, EntityId, IntentId, WorkspaceId};

/// An intent to perform a corporate action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Intent {
    intent_id: IntentId,
    entity_id: EntityId,
    workspace_id: WorkspaceId,
    intent_type: String,
    authority_tier: AuthorityTier,
    #[serde(default)]
    policy_decision: Option<PolicyDecision>,
    #[serde(default)]
    bound_approval_artifact_id: Option<ApprovalArtifactId>,
    #[serde(default)]
    bound_document_request_ids: Vec<DocumentRequestId>,
    status: IntentStatus,
    description: String,
    metadata: serde_json::Value,
    evaluated_at: Option<DateTime<Utc>>,
    authorized_at: Option<DateTime<Utc>>,
    executed_at: Option<DateTime<Utc>>,
    failed_at: Option<DateTime<Utc>>,
    failure_reason: Option<String>,
    created_at: DateTime<Utc>,
}

impl Intent {
    pub fn new(
        intent_id: IntentId,
        entity_id: EntityId,
        workspace_id: WorkspaceId,
        intent_type: String,
        authority_tier: AuthorityTier,
        description: String,
        metadata: serde_json::Value,
    ) -> Self {
        Self {
            intent_id,
            entity_id,
            workspace_id,
            intent_type,
            authority_tier,
            policy_decision: None,
            bound_approval_artifact_id: None,
            bound_document_request_ids: Vec::new(),
            status: IntentStatus::Pending,
            description,
            metadata,
            evaluated_at: None,
            authorized_at: None,
            executed_at: None,
            failed_at: None,
            failure_reason: None,
            created_at: Utc::now(),
        }
    }

    /// Evaluate the intent. Pending -> Evaluated.
    pub fn evaluate(&mut self) -> Result<(), ExecutionError> {
        if self.status != IntentStatus::Pending {
            return Err(ExecutionError::InvalidIntentTransition {
                from: self.status,
                to: IntentStatus::Evaluated,
            });
        }
        self.status = IntentStatus::Evaluated;
        self.evaluated_at = Some(Utc::now());
        Ok(())
    }

    /// Authorize the intent. Evaluated -> Authorized.
    pub fn authorize(&mut self) -> Result<(), ExecutionError> {
        if self.status != IntentStatus::Evaluated {
            return Err(ExecutionError::InvalidIntentTransition {
                from: self.status,
                to: IntentStatus::Authorized,
            });
        }
        self.status = IntentStatus::Authorized;
        self.authorized_at = Some(Utc::now());
        Ok(())
    }

    /// Mark as executed. Authorized -> Executed.
    pub fn mark_executed(&mut self) -> Result<(), ExecutionError> {
        if self.status != IntentStatus::Authorized {
            return Err(ExecutionError::InvalidIntentTransition {
                from: self.status,
                to: IntentStatus::Executed,
            });
        }
        self.status = IntentStatus::Executed;
        self.executed_at = Some(Utc::now());
        Ok(())
    }

    /// Mark as failed. Can fail from Pending, Evaluated, or Authorized.
    pub fn mark_failed(&mut self, reason: String) -> Result<(), ExecutionError> {
        match self.status {
            IntentStatus::Executed | IntentStatus::Failed => {
                Err(ExecutionError::InvalidIntentTransition {
                    from: self.status,
                    to: IntentStatus::Failed,
                })
            }
            _ => {
                self.status = IntentStatus::Failed;
                self.failed_at = Some(Utc::now());
                self.failure_reason = Some(reason);
                Ok(())
            }
        }
    }

    // ── Accessors ─────────────────────────────────────────────────────

    pub fn intent_id(&self) -> IntentId {
        self.intent_id
    }
    pub fn entity_id(&self) -> EntityId {
        self.entity_id
    }
    pub fn workspace_id(&self) -> WorkspaceId {
        self.workspace_id
    }
    pub fn intent_type(&self) -> &str {
        &self.intent_type
    }
    pub fn authority_tier(&self) -> AuthorityTier {
        self.authority_tier
    }
    pub fn status(&self) -> IntentStatus {
        self.status
    }
    pub fn description(&self) -> &str {
        &self.description
    }
    pub fn metadata(&self) -> &serde_json::Value {
        &self.metadata
    }
    pub fn policy_decision(&self) -> Option<&PolicyDecision> {
        self.policy_decision.as_ref()
    }
    pub fn bound_approval_artifact_id(&self) -> Option<ApprovalArtifactId> {
        self.bound_approval_artifact_id
    }
    pub fn bound_document_request_ids(&self) -> &[DocumentRequestId] {
        &self.bound_document_request_ids
    }
    pub fn evaluated_at(&self) -> Option<DateTime<Utc>> {
        self.evaluated_at
    }
    pub fn authorized_at(&self) -> Option<DateTime<Utc>> {
        self.authorized_at
    }
    pub fn executed_at(&self) -> Option<DateTime<Utc>> {
        self.executed_at
    }
    pub fn failed_at(&self) -> Option<DateTime<Utc>> {
        self.failed_at
    }
    pub fn failure_reason(&self) -> Option<&str> {
        self.failure_reason.as_deref()
    }
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    pub fn update_authority_tier(&mut self, tier: AuthorityTier) {
        self.authority_tier = tier;
    }

    pub fn set_policy_decision(&mut self, decision: PolicyDecision) {
        self.policy_decision = Some(decision);
    }

    pub fn bind_approval_artifact(&mut self, approval_artifact_id: ApprovalArtifactId) {
        self.bound_approval_artifact_id = Some(approval_artifact_id);
    }

    pub fn bind_document_request(&mut self, request_id: DocumentRequestId) {
        if !self.bound_document_request_ids.contains(&request_id) {
            self.bound_document_request_ids.push(request_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn make_intent() -> Intent {
        Intent::new(
            IntentId::new(),
            EntityId::new(),
            WorkspaceId::new(),
            "incorporate".to_owned(),
            AuthorityTier::Tier2,
            "Incorporate ACME LLC in Delaware".to_owned(),
            json!({"state": "DE"}),
        )
    }

    #[test]
    fn full_fsm_pending_to_executed() {
        let mut intent = make_intent();
        assert_eq!(intent.status(), IntentStatus::Pending);
        assert!(intent.evaluated_at().is_none());

        intent.evaluate().unwrap();
        assert_eq!(intent.status(), IntentStatus::Evaluated);
        assert!(intent.evaluated_at().is_some());

        intent.authorize().unwrap();
        assert_eq!(intent.status(), IntentStatus::Authorized);
        assert!(intent.authorized_at().is_some());

        intent.mark_executed().unwrap();
        assert_eq!(intent.status(), IntentStatus::Executed);
        assert!(intent.executed_at().is_some());
    }

    #[test]
    fn mark_failed_from_pending() {
        let mut intent = make_intent();
        intent.mark_failed("bad input".to_owned()).unwrap();
        assert_eq!(intent.status(), IntentStatus::Failed);
        assert_eq!(intent.failure_reason(), Some("bad input"));
        assert!(intent.failed_at().is_some());
    }

    #[test]
    fn mark_failed_from_evaluated() {
        let mut intent = make_intent();
        intent.evaluate().unwrap();
        intent.mark_failed("policy violation".to_owned()).unwrap();
        assert_eq!(intent.status(), IntentStatus::Failed);
    }

    #[test]
    fn mark_failed_from_authorized() {
        let mut intent = make_intent();
        intent.evaluate().unwrap();
        intent.authorize().unwrap();
        intent
            .mark_failed("external service down".to_owned())
            .unwrap();
        assert_eq!(intent.status(), IntentStatus::Failed);
    }

    #[test]
    fn cannot_fail_from_executed() {
        let mut intent = make_intent();
        intent.evaluate().unwrap();
        intent.authorize().unwrap();
        intent.mark_executed().unwrap();
        assert!(intent.mark_failed("too late".to_owned()).is_err());
    }

    #[test]
    fn cannot_fail_from_failed() {
        let mut intent = make_intent();
        intent.mark_failed("first".to_owned()).unwrap();
        assert!(intent.mark_failed("second".to_owned()).is_err());
    }

    #[test]
    fn invalid_transitions() {
        let mut intent = make_intent();
        // Can't authorize from Pending (must evaluate first).
        assert!(intent.authorize().is_err());
        // Can't execute from Pending.
        assert!(intent.mark_executed().is_err());
    }

    #[test]
    fn update_authority_tier_syncs() {
        let mut intent = make_intent();
        assert_eq!(intent.authority_tier(), AuthorityTier::Tier2);

        intent.update_authority_tier(AuthorityTier::Tier3);
        assert_eq!(intent.authority_tier(), AuthorityTier::Tier3);

        intent.update_authority_tier(AuthorityTier::Tier1);
        assert_eq!(intent.authority_tier(), AuthorityTier::Tier1);
    }

    #[test]
    fn serde_roundtrip() {
        let mut intent = make_intent();
        intent.evaluate().unwrap();
        intent.authorize().unwrap();

        let json = serde_json::to_string_pretty(&intent).expect("serialize");
        let parsed: Intent = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(parsed.intent_id(), intent.intent_id());
        assert_eq!(parsed.status(), IntentStatus::Authorized);
        assert_eq!(parsed.intent_type(), "incorporate");
        assert_eq!(parsed.description(), intent.description());
    }
}
