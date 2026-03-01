//! Approval artifact record (stored as `execution/approval-artifacts/{approval_artifact_id}.json`).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::ids::{ApprovalArtifactId, EntityId};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalArtifact {
    approval_artifact_id: ApprovalArtifactId,
    entity_id: EntityId,
    intent_type: String,
    scope: String,
    approver_identity: String,
    explicit: bool,
    approved_at: DateTime<Utc>,
    expires_at: Option<DateTime<Utc>>,
    channel: String,
    max_amount_cents: Option<i64>,
    revoked_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

impl ApprovalArtifact {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        approval_artifact_id: ApprovalArtifactId,
        entity_id: EntityId,
        intent_type: String,
        scope: String,
        approver_identity: String,
        explicit: bool,
        approved_at: DateTime<Utc>,
        expires_at: Option<DateTime<Utc>>,
        channel: String,
        max_amount_cents: Option<i64>,
    ) -> Self {
        Self {
            approval_artifact_id,
            entity_id,
            intent_type,
            scope,
            approver_identity,
            explicit,
            approved_at,
            expires_at,
            channel,
            max_amount_cents,
            revoked_at: None,
            created_at: Utc::now(),
        }
    }

    pub fn revoke(&mut self) {
        self.revoked_at = Some(Utc::now());
    }

    pub fn covers_intent(
        &self,
        intent_type: &str,
        amount_cents: Option<i64>,
        now: DateTime<Utc>,
    ) -> bool {
        if self.intent_type != intent_type {
            return false;
        }
        if !self.explicit {
            return false;
        }
        if self.revoked_at.is_some() {
            return false;
        }
        if let Some(expires_at) = self.expires_at
            && now > expires_at
        {
            return false;
        }
        if let (Some(max), Some(amount)) = (self.max_amount_cents, amount_cents)
            && amount > max
        {
            return false;
        }
        true
    }

    pub fn approval_artifact_id(&self) -> ApprovalArtifactId {
        self.approval_artifact_id
    }
    pub fn entity_id(&self) -> EntityId {
        self.entity_id
    }
    pub fn intent_type(&self) -> &str {
        &self.intent_type
    }
    pub fn scope(&self) -> &str {
        &self.scope
    }
    pub fn approver_identity(&self) -> &str {
        &self.approver_identity
    }
    pub fn explicit(&self) -> bool {
        self.explicit
    }
    pub fn approved_at(&self) -> DateTime<Utc> {
        self.approved_at
    }
    pub fn expires_at(&self) -> Option<DateTime<Utc>> {
        self.expires_at
    }
    pub fn channel(&self) -> &str {
        &self.channel
    }
    pub fn max_amount_cents(&self) -> Option<i64> {
        self.max_amount_cents
    }
    pub fn revoked_at(&self) -> Option<DateTime<Utc>> {
        self.revoked_at
    }
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn artifact_covers_matching_intent() {
        let now = Utc::now();
        let artifact = ApprovalArtifact::new(
            ApprovalArtifactId::new(),
            EntityId::new(),
            "new_contract".to_owned(),
            "Vendor MSA under 50k".to_owned(),
            "Board Chair".to_owned(),
            true,
            now,
            Some(now + Duration::days(30)),
            "written_consent".to_owned(),
            Some(50_000_00),
        );
        assert!(artifact.covers_intent("new_contract", Some(20_000_00), now));
    }

    #[test]
    fn artifact_fails_after_expiry() {
        let now = Utc::now();
        let artifact = ApprovalArtifact::new(
            ApprovalArtifactId::new(),
            EntityId::new(),
            "new_contract".to_owned(),
            "Scope".to_owned(),
            "Board".to_owned(),
            true,
            now - Duration::days(40),
            Some(now - Duration::days(1)),
            "board_resolution".to_owned(),
            None,
        );
        assert!(!artifact.covers_intent("new_contract", None, now));
    }
}
