//! Delegation schedule runtime state and amendment history.

use chrono::{DateTime, Duration, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use super::policy_ast::default_governance_ast;
use crate::domain::ids::{EntityId, ResolutionId, ScheduleAmendmentId};

pub const CURRENT_SCHEDULE_PATH: &str = "governance/delegation-schedule/current.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegationSchedule {
    entity_id: EntityId,
    version: u32,
    tier1_max_amount_cents: i64,
    #[serde(default)]
    allowed_tier1_intent_types: Vec<String>,
    reauth_reduced_limits_at_days: i64,
    reauth_reduced_limits_percent: u32,
    reauth_full_suspension_at_days: i64,
    last_reauthorized_at: DateTime<Utc>,
    next_mandatory_review_at: NaiveDate,
    adopted_resolution_id: Option<ResolutionId>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl DelegationSchedule {
    pub fn default_for_entity(entity_id: EntityId) -> Self {
        let ast = default_governance_ast();
        let reauth = &ast.rules.reauth;
        let now = Utc::now();
        Self {
            entity_id,
            version: 1,
            // Keep permissive unless narrowed by an explicit amendment.
            tier1_max_amount_cents: i64::MAX / 4,
            allowed_tier1_intent_types: Vec::new(),
            reauth_reduced_limits_at_days: i64::from(reauth.reduced_limits_at_days),
            reauth_reduced_limits_percent: reauth.reduced_limits_percent,
            reauth_full_suspension_at_days: i64::from(reauth.full_suspension_at_days),
            last_reauthorized_at: now,
            next_mandatory_review_at: (now + Duration::days(365)).date_naive(),
            adopted_resolution_id: None,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn bump_version(&mut self) {
        self.version = self.version.saturating_add(1);
        self.updated_at = Utc::now();
    }

    pub fn set_tier1_max_amount_cents(&mut self, amount: i64) {
        self.tier1_max_amount_cents = amount.max(1);
        self.updated_at = Utc::now();
    }

    pub fn set_allowed_tier1_intent_types(&mut self, intents: Vec<String>) {
        let mut normalized = intents
            .into_iter()
            .map(|s| s.trim().to_owned())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>();
        normalized.sort();
        normalized.dedup();
        self.allowed_tier1_intent_types = normalized;
        self.updated_at = Utc::now();
    }

    pub fn set_next_mandatory_review_at(&mut self, next_review: NaiveDate) {
        self.next_mandatory_review_at = next_review;
        self.updated_at = Utc::now();
    }

    pub fn set_adopted_resolution_id(&mut self, resolution_id: Option<ResolutionId>) {
        self.adopted_resolution_id = resolution_id;
        self.updated_at = Utc::now();
    }

    pub fn reauthorize(&mut self, adopted_resolution_id: ResolutionId) {
        let now = Utc::now();
        self.last_reauthorized_at = now;
        self.next_mandatory_review_at = (now + Duration::days(365)).date_naive();
        self.adopted_resolution_id = Some(adopted_resolution_id);
        self.updated_at = now;
    }

    pub fn days_since_reauthorization(&self, now: DateTime<Utc>) -> i64 {
        (now - self.last_reauthorized_at).num_days()
    }

    pub fn is_reauth_suspended_at(&self, now: DateTime<Utc>) -> bool {
        self.days_since_reauthorization(now) >= self.reauth_full_suspension_at_days
    }

    pub fn effective_tier1_max_amount_cents(&self, now: DateTime<Utc>) -> i64 {
        let days = self.days_since_reauthorization(now);
        if days >= self.reauth_full_suspension_at_days {
            return 0;
        }
        if days >= self.reauth_reduced_limits_at_days {
            let reduced = i128::from(self.tier1_max_amount_cents)
                .saturating_mul(i128::from(self.reauth_reduced_limits_percent))
                / 100_i128;
            return i64::try_from(reduced).unwrap_or(i64::MAX);
        }
        self.tier1_max_amount_cents
    }

    pub fn allows_tier1_intent(&self, intent_type: &str) -> bool {
        if self.allowed_tier1_intent_types.is_empty() {
            return true;
        }
        self.allowed_tier1_intent_types
            .iter()
            .any(|allowed| allowed == intent_type)
    }

    pub fn added_tier1_intents(&self, next_intents: &[String]) -> Vec<String> {
        let mut added = next_intents
            .iter()
            .filter(|intent| !self.allowed_tier1_intent_types.iter().any(|v| v == *intent))
            .cloned()
            .collect::<Vec<_>>();
        added.sort();
        added.dedup();
        added
    }

    pub fn removed_tier1_intents(&self, next_intents: &[String]) -> Vec<String> {
        let mut removed = self
            .allowed_tier1_intent_types
            .iter()
            .filter(|intent| !next_intents.iter().any(|v| v == *intent))
            .cloned()
            .collect::<Vec<_>>();
        removed.sort();
        removed.dedup();
        removed
    }

    pub fn entity_id(&self) -> EntityId {
        self.entity_id
    }
    pub fn version(&self) -> u32 {
        self.version
    }
    pub fn tier1_max_amount_cents(&self) -> i64 {
        self.tier1_max_amount_cents
    }
    pub fn allowed_tier1_intent_types(&self) -> &[String] {
        &self.allowed_tier1_intent_types
    }
    pub fn reauth_reduced_limits_at_days(&self) -> i64 {
        self.reauth_reduced_limits_at_days
    }
    pub fn reauth_reduced_limits_percent(&self) -> u32 {
        self.reauth_reduced_limits_percent
    }
    pub fn reauth_full_suspension_at_days(&self) -> i64 {
        self.reauth_full_suspension_at_days
    }
    pub fn last_reauthorized_at(&self) -> DateTime<Utc> {
        self.last_reauthorized_at
    }
    pub fn next_mandatory_review_at(&self) -> NaiveDate {
        self.next_mandatory_review_at
    }
    pub fn adopted_resolution_id(&self) -> Option<ResolutionId> {
        self.adopted_resolution_id
    }
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleAmendment {
    schedule_amendment_id: ScheduleAmendmentId,
    entity_id: EntityId,
    from_version: u32,
    to_version: u32,
    previous_tier1_max_amount_cents: i64,
    new_tier1_max_amount_cents: i64,
    added_tier1_intent_types: Vec<String>,
    removed_tier1_intent_types: Vec<String>,
    authority_expansion: bool,
    adopted_resolution_id: Option<ResolutionId>,
    rationale: Option<String>,
    created_at: DateTime<Utc>,
}

impl ScheduleAmendment {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        schedule_amendment_id: ScheduleAmendmentId,
        entity_id: EntityId,
        from_version: u32,
        to_version: u32,
        previous_tier1_max_amount_cents: i64,
        new_tier1_max_amount_cents: i64,
        mut added_tier1_intent_types: Vec<String>,
        mut removed_tier1_intent_types: Vec<String>,
        authority_expansion: bool,
        adopted_resolution_id: Option<ResolutionId>,
        rationale: Option<String>,
    ) -> Self {
        added_tier1_intent_types.sort();
        added_tier1_intent_types.dedup();
        removed_tier1_intent_types.sort();
        removed_tier1_intent_types.dedup();
        Self {
            schedule_amendment_id,
            entity_id,
            from_version,
            to_version,
            previous_tier1_max_amount_cents,
            new_tier1_max_amount_cents,
            added_tier1_intent_types,
            removed_tier1_intent_types,
            authority_expansion,
            adopted_resolution_id,
            rationale,
            created_at: Utc::now(),
        }
    }

    pub fn schedule_amendment_id(&self) -> ScheduleAmendmentId {
        self.schedule_amendment_id
    }
    pub fn entity_id(&self) -> EntityId {
        self.entity_id
    }
    pub fn from_version(&self) -> u32 {
        self.from_version
    }
    pub fn to_version(&self) -> u32 {
        self.to_version
    }
    pub fn previous_tier1_max_amount_cents(&self) -> i64 {
        self.previous_tier1_max_amount_cents
    }
    pub fn new_tier1_max_amount_cents(&self) -> i64 {
        self.new_tier1_max_amount_cents
    }
    pub fn added_tier1_intent_types(&self) -> &[String] {
        &self.added_tier1_intent_types
    }
    pub fn removed_tier1_intent_types(&self) -> &[String] {
        &self.removed_tier1_intent_types
    }
    pub fn authority_expansion(&self) -> bool {
        self.authority_expansion
    }
    pub fn adopted_resolution_id(&self) -> Option<ResolutionId> {
        self.adopted_resolution_id
    }
    pub fn rationale(&self) -> Option<&str> {
        self.rationale.as_deref()
    }
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_schedule_is_not_suspended() {
        let schedule = DelegationSchedule::default_for_entity(EntityId::new());
        assert!(!schedule.is_reauth_suspended_at(Utc::now()));
        assert!(schedule.effective_tier1_max_amount_cents(Utc::now()) > 0);
    }

    #[test]
    fn empty_tier1_list_means_allow_all() {
        let schedule = DelegationSchedule::default_for_entity(EntityId::new());
        assert!(schedule.allows_tier1_intent("authorize_expenditure"));
        assert!(schedule.allows_tier1_intent("anything.else"));
    }
}
