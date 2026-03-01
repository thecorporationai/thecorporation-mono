//! Shared governance enforcement helpers used across route modules.

use chrono::Utc;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::domain::governance::audit::{GovernanceAuditEntry, GovernanceAuditEventType};
use crate::domain::governance::incident::{GovernanceIncident, IncidentSeverity, IncidentStatus};
use crate::domain::governance::mode::{GovernanceMode, GovernanceModeState};
use crate::domain::governance::mode_history::GovernanceModeChangeEvent;
use crate::domain::governance::trigger::{
    GovernanceTriggerEvent, GovernanceTriggerIdempotencyRecord, GovernanceTriggerSource,
    GovernanceTriggerType,
};
use crate::domain::ids::{
    ComplianceEscalationId, ContactId, EntityId, GovernanceAuditEntryId, GovernanceModeEventId,
    GovernanceTriggerId, IncidentId, IntentId,
};
use crate::error::AppError;
use crate::git::commit::FileWrite;
use crate::git::error::GitStorageError;
use crate::store::entity_store::EntityStore;

pub const GOVERNANCE_MODE_PATH: &str = "governance/mode.json";

pub fn mode_history_path(mode_event_id: GovernanceModeEventId) -> String {
    format!("governance/mode-history/{mode_event_id}.json")
}

pub fn trigger_idempotency_path(idempotency_key_hash: &str) -> String {
    format!("governance/triggers/idempotency/{idempotency_key_hash}.json")
}

pub fn audit_entry_path(audit_entry_id: GovernanceAuditEntryId) -> String {
    format!("governance/audit/entries/{audit_entry_id}.json")
}

fn trigger_idempotency_hash(idempotency_key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(idempotency_key.as_bytes());
    hex::encode(hasher.finalize())
}

pub fn read_mode_or_default(store: &EntityStore<'_>, entity_id: EntityId) -> GovernanceModeState {
    store
        .read_json::<GovernanceModeState>("main", GOVERNANCE_MODE_PATH)
        .unwrap_or_else(|_| GovernanceModeState::new(entity_id))
}

pub fn list_audit_entries_sorted(
    store: &EntityStore<'_>,
    entity_id: EntityId,
) -> Result<Vec<GovernanceAuditEntry>, AppError> {
    let ids = store
        .list_ids::<GovernanceAuditEntry>("main")
        .map_err(|e| AppError::Internal(format!("list governance audit entries: {e}")))?;
    let mut entries = Vec::new();
    for id in ids {
        let entry = store
            .read::<GovernanceAuditEntry>("main", id)
            .map_err(|e| AppError::Internal(format!("read governance audit entry {id}: {e}")))?;
        if entry.entity_id() == entity_id {
            entries.push(entry);
        }
    }
    entries.sort_by(|a, b| {
        a.created_at().cmp(&b.created_at()).then_with(|| {
            a.audit_entry_id()
                .to_string()
                .cmp(&b.audit_entry_id().to_string())
        })
    });
    Ok(entries)
}

#[derive(Debug, Clone)]
pub struct BuildAuditEntryInput {
    pub event_type: GovernanceAuditEventType,
    pub action: String,
    pub details: Value,
    pub evidence_refs: Vec<String>,
    pub linked_intent_id: Option<IntentId>,
    pub linked_incident_id: Option<IncidentId>,
    pub linked_trigger_id: Option<GovernanceTriggerId>,
    pub linked_mode_event_id: Option<GovernanceModeEventId>,
}

pub fn build_audit_entry(
    store: &EntityStore<'_>,
    entity_id: EntityId,
    input: BuildAuditEntryInput,
) -> Result<GovernanceAuditEntry, AppError> {
    let previous_entry_hash = list_audit_entries_sorted(store, entity_id)?
        .last()
        .map(|entry| entry.entry_hash().to_owned());
    Ok(GovernanceAuditEntry::new(
        GovernanceAuditEntryId::new(),
        entity_id,
        input.event_type,
        input.action,
        input.details,
        input.evidence_refs,
        input.linked_intent_id,
        input.linked_incident_id,
        input.linked_trigger_id,
        input.linked_mode_event_id,
        previous_entry_hash,
    ))
}

#[derive(Debug, Clone)]
pub struct SetModeWithHistoryInput {
    pub target_mode: GovernanceMode,
    pub reason: Option<String>,
    pub incident_ids: Vec<IncidentId>,
    pub evidence_refs: Vec<String>,
    pub trigger_id: Option<GovernanceTriggerId>,
    pub updated_by: Option<ContactId>,
    pub commit_message: String,
}

#[derive(Debug, Clone)]
pub struct SetModeWithHistoryResult {
    pub mode: GovernanceModeState,
    pub mode_event: GovernanceModeChangeEvent,
}

pub fn set_mode_with_history(
    store: &EntityStore<'_>,
    entity_id: EntityId,
    input: SetModeWithHistoryInput,
) -> Result<SetModeWithHistoryResult, AppError> {
    let mut mode = read_mode_or_default(store, entity_id);
    let from_mode = mode.mode();
    mode.set_mode(input.target_mode, input.reason.clone(), input.updated_by);

    let mode_event = GovernanceModeChangeEvent::new(
        GovernanceModeEventId::new(),
        entity_id,
        from_mode,
        mode.mode(),
        input.reason,
        input.incident_ids,
        input.evidence_refs,
        input.trigger_id,
        input.updated_by,
    );

    let audit_entry = build_audit_entry(
        store,
        entity_id,
        BuildAuditEntryInput {
            event_type: GovernanceAuditEventType::ModeChanged,
            action: format!("governance mode changed {:?}->{:?}", from_mode, mode.mode()),
            details: json!({
                "from_mode": from_mode,
                "to_mode": mode.mode(),
                "reason": mode_event.reason(),
            }),
            evidence_refs: mode_event.evidence_refs().to_vec(),
            linked_intent_id: None,
            linked_incident_id: mode_event.incident_ids().first().copied(),
            linked_trigger_id: mode_event.trigger_id(),
            linked_mode_event_id: Some(mode_event.mode_event_id()),
        },
    )?;

    let files = vec![
        FileWrite::json(GOVERNANCE_MODE_PATH, &mode)
            .map_err(|e| AppError::Internal(format!("serialize governance mode: {e}")))?,
        FileWrite::json(mode_history_path(mode_event.mode_event_id()), &mode_event)
            .map_err(|e| AppError::Internal(format!("serialize governance mode event: {e}")))?,
        FileWrite::json(audit_entry_path(audit_entry.audit_entry_id()), &audit_entry)
            .map_err(|e| AppError::Internal(format!("serialize governance audit entry: {e}")))?,
    ];
    store
        .commit("main", &input.commit_message, files)
        .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

    Ok(SetModeWithHistoryResult { mode, mode_event })
}

#[derive(Debug, Clone)]
pub struct LockdownTriggerInput {
    pub source: GovernanceTriggerSource,
    pub trigger_type: GovernanceTriggerType,
    pub severity: IncidentSeverity,
    pub title: String,
    pub description: String,
    pub evidence_refs: Vec<String>,
    pub linked_intent_id: Option<IntentId>,
    pub linked_escalation_id: Option<ComplianceEscalationId>,
    pub idempotency_key: Option<String>,
    pub existing_incident_id: Option<IncidentId>,
    pub updated_by: Option<ContactId>,
}

#[derive(Debug, Clone)]
pub struct LockdownTriggerResult {
    pub trigger: GovernanceTriggerEvent,
    pub incident: GovernanceIncident,
    pub mode: GovernanceModeState,
    pub mode_event: GovernanceModeChangeEvent,
    pub idempotent_replay: bool,
    pub incident_created: bool,
}

pub fn apply_lockdown_trigger(
    store: &EntityStore<'_>,
    entity_id: EntityId,
    input: LockdownTriggerInput,
) -> Result<LockdownTriggerResult, AppError> {
    if input.title.trim().is_empty() {
        return Err(AppError::BadRequest(
            "trigger title must not be empty".to_owned(),
        ));
    }
    if input.description.trim().is_empty() {
        return Err(AppError::BadRequest(
            "trigger description must not be empty".to_owned(),
        ));
    }

    let idempotency_hash = input
        .idempotency_key
        .as_deref()
        .map(trigger_idempotency_hash);
    if let Some(hash) = idempotency_hash.as_deref() {
        let existing_path = trigger_idempotency_path(hash);
        match store.read_json::<GovernanceTriggerIdempotencyRecord>("main", &existing_path) {
            Ok(record) => {
                let trigger = store
                    .read::<GovernanceTriggerEvent>("main", record.trigger_id())
                    .map_err(|e| AppError::Internal(format!("read existing trigger: {e}")))?;
                let incident = store
                    .read::<GovernanceIncident>("main", record.incident_id())
                    .map_err(|e| AppError::Internal(format!("read existing incident: {e}")))?;
                let mode_event = store
                    .read::<GovernanceModeChangeEvent>("main", record.mode_event_id())
                    .map_err(|e| AppError::Internal(format!("read existing mode event: {e}")))?;
                let mode = read_mode_or_default(store, entity_id);
                return Ok(LockdownTriggerResult {
                    trigger,
                    incident,
                    mode,
                    mode_event,
                    idempotent_replay: true,
                    incident_created: false,
                });
            }
            Err(GitStorageError::NotFound(_)) => {}
            Err(other) => {
                return Err(AppError::Internal(format!(
                    "read trigger idempotency record: {other}"
                )));
            }
        }
    }

    let (incident, incident_created) = if let Some(incident_id) = input.existing_incident_id {
        let incident = store
            .read::<GovernanceIncident>("main", incident_id)
            .map_err(|_| AppError::NotFound(format!("incident {incident_id} not found")))?;
        if incident.status() == IncidentStatus::Resolved {
            return Err(AppError::UnprocessableEntity(format!(
                "incident {incident_id} is already resolved"
            )));
        }
        (incident, false)
    } else {
        (
            GovernanceIncident::new(
                IncidentId::new(),
                entity_id,
                input.severity,
                input.title.clone(),
                input.description.clone(),
            ),
            true,
        )
    };

    let mut mode = read_mode_or_default(store, entity_id);
    let from_mode = mode.mode();
    let reason = Some(format!(
        "auto-lockdown: {:?} - {}",
        input.trigger_type, input.title
    ));
    mode.set_mode(
        GovernanceMode::IncidentLockdown,
        reason.clone(),
        input.updated_by,
    );

    let mode_event = GovernanceModeChangeEvent::new(
        GovernanceModeEventId::new(),
        entity_id,
        from_mode,
        mode.mode(),
        reason,
        vec![incident.incident_id()],
        input.evidence_refs.clone(),
        None,
        input.updated_by,
    );

    let trigger = GovernanceTriggerEvent::new(
        GovernanceTriggerId::new(),
        entity_id,
        input.trigger_type,
        input.source,
        input.severity,
        input.title,
        input.description,
        input.evidence_refs,
        input.linked_intent_id,
        input.linked_escalation_id,
        incident.incident_id(),
        mode_event.mode_event_id(),
        idempotency_hash.clone(),
    );

    let mode_event = GovernanceModeChangeEvent::new(
        mode_event.mode_event_id(),
        entity_id,
        mode_event.from_mode(),
        mode_event.to_mode(),
        mode_event.reason().map(ToOwned::to_owned),
        mode_event.incident_ids().to_vec(),
        mode_event.evidence_refs().to_vec(),
        Some(trigger.trigger_id()),
        mode_event.updated_by(),
    );

    let audit_entry = build_audit_entry(
        store,
        entity_id,
        BuildAuditEntryInput {
            event_type: GovernanceAuditEventType::LockdownTriggerApplied,
            action: "governance lockdown trigger applied".to_owned(),
            details: json!({
                "trigger_type": trigger.trigger_type(),
                "source": trigger.source(),
                "severity": trigger.severity(),
                "title": trigger.title(),
                "incident_created": incident_created,
            }),
            evidence_refs: trigger.evidence_refs().to_vec(),
            linked_intent_id: trigger.linked_intent_id(),
            linked_incident_id: Some(incident.incident_id()),
            linked_trigger_id: Some(trigger.trigger_id()),
            linked_mode_event_id: Some(mode_event.mode_event_id()),
        },
    )?;

    let mut files = Vec::with_capacity(6);
    if incident_created {
        files.push(
            FileWrite::json(
                format!("governance/incidents/{}.json", incident.incident_id()),
                &incident,
            )
            .map_err(|e| AppError::Internal(format!("serialize incident: {e}")))?,
        );
    }
    files.push(
        FileWrite::json(GOVERNANCE_MODE_PATH, &mode)
            .map_err(|e| AppError::Internal(format!("serialize governance mode: {e}")))?,
    );
    files.push(
        FileWrite::json(mode_history_path(mode_event.mode_event_id()), &mode_event)
            .map_err(|e| AppError::Internal(format!("serialize governance mode event: {e}")))?,
    );
    files.push(
        FileWrite::json(
            format!("governance/triggers/{}.json", trigger.trigger_id()),
            &trigger,
        )
        .map_err(|e| AppError::Internal(format!("serialize trigger: {e}")))?,
    );
    files.push(
        FileWrite::json(audit_entry_path(audit_entry.audit_entry_id()), &audit_entry)
            .map_err(|e| AppError::Internal(format!("serialize governance audit entry: {e}")))?,
    );
    if let Some(hash) = idempotency_hash {
        let record = GovernanceTriggerIdempotencyRecord::new(
            hash.clone(),
            trigger.trigger_id(),
            incident.incident_id(),
            mode_event.mode_event_id(),
        );
        files.push(
            FileWrite::json(trigger_idempotency_path(&hash), &record)
                .map_err(|e| AppError::Internal(format!("serialize idempotency record: {e}")))?,
        );
    }

    store
        .commit(
            "main",
            &format!(
                "GOVERNANCE: lockdown trigger {} {}",
                trigger.trigger_id(),
                Utc::now().to_rfc3339()
            ),
            files,
        )
        .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

    Ok(LockdownTriggerResult {
        trigger,
        incident,
        mode,
        mode_event,
        idempotent_replay: false,
        incident_created,
    })
}
