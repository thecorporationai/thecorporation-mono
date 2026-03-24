//! Execution intents — a request to perform a governed action.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::types::IntentStatus;
use crate::governance::capability::AuthorityTier;
use crate::ids::{EntityId, IntentId, WorkspaceId};

// ── Error ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum IntentError {
    #[error("intent must be Pending to be evaluated; current status: {0:?}")]
    NotPending(IntentStatus),
    #[error("intent must be Evaluated to be authorized; current status: {0:?}")]
    NotEvaluated(IntentStatus),
    #[error("intent must be Authorized to be executed; current status: {0:?}")]
    NotAuthorized(IntentStatus),
    #[error("intent is already in a terminal state: {0:?}")]
    AlreadyTerminal(IntentStatus),
}

// ── Intent ────────────────────────────────────────────────────────────────────

/// A request to perform a governed action within a workspace/entity context.
///
/// The FSM is:
/// ```text
/// Pending → Evaluated → Authorized → Executed
///     ↓           ↓           ↓
///  Failed / Cancelled (from any non-terminal state)
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Intent {
    pub intent_id: IntentId,
    pub entity_id: EntityId,
    pub workspace_id: WorkspaceId,
    /// A free-form type tag identifying what kind of action this intent
    /// represents (e.g. `"equity.grant.issue"`, `"treasury.payment.send"`).
    pub intent_type: String,
    pub authority_tier: AuthorityTier,
    pub description: String,
    pub status: IntentStatus,
    /// Arbitrary JSON metadata attached to this intent.
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub evaluated_at: Option<DateTime<Utc>>,
    pub authorized_at: Option<DateTime<Utc>>,
    pub executed_at: Option<DateTime<Utc>>,
    pub failed_at: Option<DateTime<Utc>>,
    pub failure_reason: Option<String>,
    pub cancelled_at: Option<DateTime<Utc>>,
}

impl Intent {
    /// Create a new intent in `Pending` status.
    pub fn new(
        entity_id: EntityId,
        workspace_id: WorkspaceId,
        intent_type: impl Into<String>,
        authority_tier: AuthorityTier,
        description: impl Into<String>,
        metadata: serde_json::Value,
    ) -> Self {
        Self {
            intent_id: IntentId::new(),
            entity_id,
            workspace_id,
            intent_type: intent_type.into(),
            authority_tier,
            description: description.into(),
            status: IntentStatus::Pending,
            metadata,
            created_at: Utc::now(),
            evaluated_at: None,
            authorized_at: None,
            executed_at: None,
            failed_at: None,
            failure_reason: None,
            cancelled_at: None,
        }
    }

    /// Transition `Pending → Evaluated`.
    pub fn evaluate(&mut self) -> Result<(), IntentError> {
        match self.status {
            IntentStatus::Pending => {
                self.status = IntentStatus::Evaluated;
                self.evaluated_at = Some(Utc::now());
                Ok(())
            }
            s => Err(IntentError::NotPending(s)),
        }
    }

    /// Transition `Evaluated → Authorized`.
    pub fn authorize(&mut self) -> Result<(), IntentError> {
        match self.status {
            IntentStatus::Evaluated => {
                self.status = IntentStatus::Authorized;
                self.authorized_at = Some(Utc::now());
                Ok(())
            }
            s => Err(IntentError::NotEvaluated(s)),
        }
    }

    /// Transition `Authorized → Executed`.
    pub fn mark_executed(&mut self) -> Result<(), IntentError> {
        match self.status {
            IntentStatus::Authorized => {
                self.status = IntentStatus::Executed;
                self.executed_at = Some(Utc::now());
                Ok(())
            }
            s => Err(IntentError::NotAuthorized(s)),
        }
    }

    /// Mark this intent as failed with a reason string. Allowed from any
    /// non-terminal state.
    pub fn mark_failed(&mut self, reason: impl Into<String>) -> Result<(), IntentError> {
        if self.is_terminal() {
            return Err(IntentError::AlreadyTerminal(self.status));
        }
        self.status = IntentStatus::Failed;
        self.failed_at = Some(Utc::now());
        self.failure_reason = Some(reason.into());
        Ok(())
    }

    /// Cancel this intent. Allowed from any non-terminal state.
    pub fn cancel(&mut self) -> Result<(), IntentError> {
        if self.is_terminal() {
            return Err(IntentError::AlreadyTerminal(self.status));
        }
        self.status = IntentStatus::Cancelled;
        self.cancelled_at = Some(Utc::now());
        Ok(())
    }

    /// Returns `true` if the intent is in a terminal state (`Executed`,
    /// `Failed`, or `Cancelled`).
    pub fn is_terminal(&self) -> bool {
        matches!(
            self.status,
            IntentStatus::Executed | IntentStatus::Failed | IntentStatus::Cancelled
        )
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_intent(tier: AuthorityTier) -> Intent {
        Intent::new(
            EntityId::new(),
            WorkspaceId::new(),
            "equity.grant.issue",
            tier,
            "Issue new equity grant",
            serde_json::json!({ "shares": 10000 }),
        )
    }

    fn pending_intent() -> Intent {
        make_intent(AuthorityTier::Tier1)
    }

    // ── new() ─────────────────────────────────────────────────────────────────

    #[test]
    fn new_intent_is_pending() {
        let intent = pending_intent();
        assert_eq!(intent.status, IntentStatus::Pending);
    }

    #[test]
    fn new_intent_all_timestamps_none() {
        let intent = pending_intent();
        assert!(intent.evaluated_at.is_none());
        assert!(intent.authorized_at.is_none());
        assert!(intent.executed_at.is_none());
        assert!(intent.failed_at.is_none());
        assert!(intent.failure_reason.is_none());
        assert!(intent.cancelled_at.is_none());
    }

    #[test]
    fn new_intent_stores_type_and_description() {
        let intent = pending_intent();
        assert_eq!(intent.intent_type, "equity.grant.issue");
        assert_eq!(intent.description, "Issue new equity grant");
    }

    #[test]
    fn new_intent_with_tier1() {
        let intent = make_intent(AuthorityTier::Tier1);
        assert_eq!(intent.authority_tier, AuthorityTier::Tier1);
    }

    #[test]
    fn new_intent_with_tier2() {
        let intent = make_intent(AuthorityTier::Tier2);
        assert_eq!(intent.authority_tier, AuthorityTier::Tier2);
    }

    #[test]
    fn new_intent_with_tier3() {
        let intent = make_intent(AuthorityTier::Tier3);
        assert_eq!(intent.authority_tier, AuthorityTier::Tier3);
    }

    // ── evaluate() ───────────────────────────────────────────────────────────

    #[test]
    fn evaluate_from_pending() {
        let mut intent = pending_intent();
        assert!(intent.evaluate().is_ok());
        assert_eq!(intent.status, IntentStatus::Evaluated);
        assert!(intent.evaluated_at.is_some());
    }

    #[test]
    fn evaluate_from_evaluated_is_error() {
        let mut intent = pending_intent();
        intent.evaluate().unwrap();
        assert!(matches!(intent.evaluate(), Err(IntentError::NotPending(_))));
    }

    #[test]
    fn evaluate_from_authorized_is_error() {
        let mut intent = pending_intent();
        intent.evaluate().unwrap();
        intent.authorize().unwrap();
        assert!(matches!(intent.evaluate(), Err(IntentError::NotPending(_))));
    }

    // ── authorize() ──────────────────────────────────────────────────────────

    #[test]
    fn authorize_from_evaluated() {
        let mut intent = pending_intent();
        intent.evaluate().unwrap();
        assert!(intent.authorize().is_ok());
        assert_eq!(intent.status, IntentStatus::Authorized);
        assert!(intent.authorized_at.is_some());
    }

    #[test]
    fn authorize_from_pending_is_error() {
        let mut intent = pending_intent();
        assert!(matches!(
            intent.authorize(),
            Err(IntentError::NotEvaluated(_))
        ));
    }

    #[test]
    fn authorize_from_authorized_is_error() {
        let mut intent = pending_intent();
        intent.evaluate().unwrap();
        intent.authorize().unwrap();
        assert!(matches!(
            intent.authorize(),
            Err(IntentError::NotEvaluated(_))
        ));
    }

    // ── mark_executed() ──────────────────────────────────────────────────────

    #[test]
    fn mark_executed_from_authorized() {
        let mut intent = pending_intent();
        intent.evaluate().unwrap();
        intent.authorize().unwrap();
        assert!(intent.mark_executed().is_ok());
        assert_eq!(intent.status, IntentStatus::Executed);
        assert!(intent.executed_at.is_some());
    }

    #[test]
    fn mark_executed_from_pending_is_error() {
        let mut intent = pending_intent();
        assert!(matches!(
            intent.mark_executed(),
            Err(IntentError::NotAuthorized(_))
        ));
    }

    #[test]
    fn mark_executed_from_evaluated_is_error() {
        let mut intent = pending_intent();
        intent.evaluate().unwrap();
        assert!(matches!(
            intent.mark_executed(),
            Err(IntentError::NotAuthorized(_))
        ));
    }

    // ── mark_failed() ────────────────────────────────────────────────────────

    #[test]
    fn mark_failed_from_pending() {
        let mut intent = pending_intent();
        assert!(intent.mark_failed("network error").is_ok());
        assert_eq!(intent.status, IntentStatus::Failed);
        assert!(intent.failed_at.is_some());
        assert_eq!(intent.failure_reason.as_deref(), Some("network error"));
    }

    #[test]
    fn mark_failed_from_evaluated() {
        let mut intent = pending_intent();
        intent.evaluate().unwrap();
        assert!(intent.mark_failed("policy violation").is_ok());
        assert_eq!(intent.status, IntentStatus::Failed);
    }

    #[test]
    fn mark_failed_from_authorized() {
        let mut intent = pending_intent();
        intent.evaluate().unwrap();
        intent.authorize().unwrap();
        assert!(intent.mark_failed("timeout").is_ok());
        assert_eq!(intent.status, IntentStatus::Failed);
    }

    #[test]
    fn mark_failed_from_terminal_is_error() {
        let mut intent = pending_intent();
        intent.evaluate().unwrap();
        intent.authorize().unwrap();
        intent.mark_executed().unwrap();
        assert!(matches!(
            intent.mark_failed("late"),
            Err(IntentError::AlreadyTerminal(_))
        ));
    }

    #[test]
    fn mark_failed_from_cancelled_is_error() {
        let mut intent = pending_intent();
        intent.cancel().unwrap();
        assert!(matches!(
            intent.mark_failed("late"),
            Err(IntentError::AlreadyTerminal(_))
        ));
    }

    // ── cancel() ─────────────────────────────────────────────────────────────

    #[test]
    fn cancel_from_pending() {
        let mut intent = pending_intent();
        assert!(intent.cancel().is_ok());
        assert_eq!(intent.status, IntentStatus::Cancelled);
        assert!(intent.cancelled_at.is_some());
    }

    #[test]
    fn cancel_from_evaluated() {
        let mut intent = pending_intent();
        intent.evaluate().unwrap();
        assert!(intent.cancel().is_ok());
        assert_eq!(intent.status, IntentStatus::Cancelled);
    }

    #[test]
    fn cancel_from_authorized() {
        let mut intent = pending_intent();
        intent.evaluate().unwrap();
        intent.authorize().unwrap();
        assert!(intent.cancel().is_ok());
        assert_eq!(intent.status, IntentStatus::Cancelled);
    }

    #[test]
    fn cancel_from_executed_is_error() {
        let mut intent = pending_intent();
        intent.evaluate().unwrap();
        intent.authorize().unwrap();
        intent.mark_executed().unwrap();
        assert!(matches!(
            intent.cancel(),
            Err(IntentError::AlreadyTerminal(_))
        ));
    }

    #[test]
    fn cancel_from_failed_is_error() {
        let mut intent = pending_intent();
        intent.mark_failed("oops").unwrap();
        assert!(matches!(
            intent.cancel(),
            Err(IntentError::AlreadyTerminal(_))
        ));
    }

    // ── is_terminal() ────────────────────────────────────────────────────────

    #[test]
    fn is_terminal_for_executed() {
        let mut intent = pending_intent();
        intent.evaluate().unwrap();
        intent.authorize().unwrap();
        intent.mark_executed().unwrap();
        assert!(intent.is_terminal());
    }

    #[test]
    fn is_terminal_for_failed() {
        let mut intent = pending_intent();
        intent.mark_failed("err").unwrap();
        assert!(intent.is_terminal());
    }

    #[test]
    fn is_terminal_for_cancelled() {
        let mut intent = pending_intent();
        intent.cancel().unwrap();
        assert!(intent.is_terminal());
    }

    #[test]
    fn is_not_terminal_for_pending() {
        assert!(!pending_intent().is_terminal());
    }

    #[test]
    fn is_not_terminal_for_evaluated() {
        let mut intent = pending_intent();
        intent.evaluate().unwrap();
        assert!(!intent.is_terminal());
    }

    // ── IntentStatus serde roundtrips ────────────────────────────────────────

    #[test]
    fn intent_status_serde_roundtrip() {
        for status in [
            IntentStatus::Pending,
            IntentStatus::Evaluated,
            IntentStatus::Authorized,
            IntentStatus::Executed,
            IntentStatus::Failed,
            IntentStatus::Cancelled,
        ] {
            let s = serde_json::to_string(&status).unwrap();
            let de: IntentStatus = serde_json::from_str(&s).unwrap();
            assert_eq!(de, status);
        }
    }

    #[test]
    fn intent_status_serde_values() {
        assert_eq!(
            serde_json::to_string(&IntentStatus::Pending).unwrap(),
            r#""pending""#
        );
        assert_eq!(
            serde_json::to_string(&IntentStatus::Evaluated).unwrap(),
            r#""evaluated""#
        );
        assert_eq!(
            serde_json::to_string(&IntentStatus::Authorized).unwrap(),
            r#""authorized""#
        );
        assert_eq!(
            serde_json::to_string(&IntentStatus::Executed).unwrap(),
            r#""executed""#
        );
        assert_eq!(
            serde_json::to_string(&IntentStatus::Failed).unwrap(),
            r#""failed""#
        );
        assert_eq!(
            serde_json::to_string(&IntentStatus::Cancelled).unwrap(),
            r#""cancelled""#
        );
    }

    #[test]
    fn intent_ids_are_unique() {
        let a = pending_intent();
        let b = pending_intent();
        assert_ne!(a.intent_id, b.intent_id);
    }
}
