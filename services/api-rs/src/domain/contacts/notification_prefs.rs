//! Notification preferences (stored as `contacts/{contact_id}/notification-prefs.json`).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::ids::ContactId;

/// Notification preferences for a contact.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationPrefs {
    contact_id: ContactId,
    email_enabled: bool,
    sms_enabled: bool,
    webhook_enabled: bool,
    updated_at: DateTime<Utc>,
}

impl NotificationPrefs {
    pub fn new(contact_id: ContactId) -> Self {
        Self {
            contact_id,
            email_enabled: true,
            sms_enabled: false,
            webhook_enabled: false,
            updated_at: Utc::now(),
        }
    }

    pub fn contact_id(&self) -> ContactId { self.contact_id }
    pub fn email_enabled(&self) -> bool { self.email_enabled }
    pub fn sms_enabled(&self) -> bool { self.sms_enabled }
    pub fn webhook_enabled(&self) -> bool { self.webhook_enabled }
    pub fn updated_at(&self) -> DateTime<Utc> { self.updated_at }

    pub fn set_email_enabled(&mut self, v: bool) { self.email_enabled = v; self.updated_at = Utc::now(); }
    pub fn set_sms_enabled(&mut self, v: bool) { self.sms_enabled = v; self.updated_at = Utc::now(); }
    pub fn set_webhook_enabled(&mut self, v: bool) { self.webhook_enabled = v; self.updated_at = Utc::now(); }
}
