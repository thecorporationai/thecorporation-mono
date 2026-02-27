//! Subscription record (stored as `billing/subscription.json` in workspace repo).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::ids::{SubscriptionId, WorkspaceId};

/// A billing subscription for a workspace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subscription {
    subscription_id: SubscriptionId,
    workspace_id: WorkspaceId,
    plan: String,
    status: String,
    current_period_end: Option<String>,
    created_at: DateTime<Utc>,
}

impl Subscription {
    pub fn new(
        subscription_id: SubscriptionId,
        workspace_id: WorkspaceId,
        plan: String,
    ) -> Self {
        Self {
            subscription_id,
            workspace_id,
            plan,
            status: "active".to_owned(),
            current_period_end: None,
            created_at: Utc::now(),
        }
    }

    pub fn subscription_id(&self) -> SubscriptionId { self.subscription_id }
    pub fn workspace_id(&self) -> WorkspaceId { self.workspace_id }
    pub fn plan(&self) -> &str { &self.plan }
    pub fn status(&self) -> &str { &self.status }
    pub fn current_period_end(&self) -> Option<&str> { self.current_period_end.as_deref() }
    pub fn created_at(&self) -> DateTime<Utc> { self.created_at }

    pub fn set_plan(&mut self, plan: String) { self.plan = plan; }
    pub fn set_status(&mut self, status: String) { self.status = status; }
    pub fn set_current_period_end(&mut self, end: String) { self.current_period_end = Some(end); }
}
