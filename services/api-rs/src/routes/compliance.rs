//! Compliance HTTP routes.
//!
//! Endpoints for tax filings, deadlines, and contractor classification.

use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{get, post},
};
use chrono::{NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use super::AppState;
use crate::auth::RequireAdmin;
use crate::domain::formation::{
    contractor::{ClassificationResult, ContractorClassification, RiskLevel},
    deadline::{Deadline, DeadlineSeverity, DeadlineStatus, Recurrence},
    escalation::ComplianceEscalation,
    evidence_link::ComplianceEvidenceLink,
    tax_filing::{TaxFiling, TaxFilingStatus},
};
use crate::domain::governance::incident::{GovernanceIncident, IncidentSeverity, IncidentStatus};
use crate::domain::governance::trigger::{GovernanceTriggerSource, GovernanceTriggerType};
use crate::domain::ids::*;
use crate::domain::{
    execution::{
        obligation::Obligation,
        transaction_packet::TransactionPacket,
        types::{AssigneeType, ObligationStatus, ObligationType},
    },
    formation::escalation::EscalationStatus,
};
use crate::error::AppError;
use crate::git::commit::FileWrite;
use crate::routes::governance_enforcement::{LockdownTriggerInput, apply_lockdown_trigger};
use crate::store::entity_store::EntityStore;

// ── Request types ────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct FileTaxDocumentRequest {
    pub entity_id: EntityId,
    pub document_type: String,
    pub tax_year: i32,
}

#[derive(Deserialize)]
pub struct CreateDeadlineRequest {
    pub entity_id: EntityId,
    pub deadline_type: String,
    pub due_date: NaiveDate,
    pub description: String,
    #[serde(default = "default_recurrence")]
    pub recurrence: Recurrence,
    #[serde(default = "default_deadline_severity")]
    pub severity: DeadlineSeverity,
}

fn default_recurrence() -> Recurrence {
    Recurrence::OneTime
}

fn default_deadline_severity() -> DeadlineSeverity {
    DeadlineSeverity::Medium
}

#[derive(Deserialize)]
pub struct ScanComplianceRequest {
    pub entity_id: EntityId,
}

#[derive(Deserialize)]
pub struct ClassifyContractorRequest {
    pub entity_id: EntityId,
    pub contractor_name: String,
    #[serde(default = "default_state")]
    pub state: String,
    #[serde(default)]
    pub factors: serde_json::Value,
}

fn default_state() -> String {
    "CA".to_owned()
}

#[derive(Deserialize)]
pub struct ResolveEscalationWithEvidenceRequest {
    pub entity_id: EntityId,
    #[serde(default)]
    pub packet_id: Option<PacketId>,
    #[serde(default)]
    pub filing_reference: Option<String>,
    #[serde(default)]
    pub evidence_type: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(default)]
    pub resolve_obligation: bool,
    #[serde(default)]
    pub resolve_incident: bool,
}

// ── Response types ───────────────────────────────────────────────────

#[derive(Serialize)]
pub struct TaxFilingResponse {
    pub filing_id: TaxFilingId,
    pub entity_id: EntityId,
    pub document_type: String,
    pub tax_year: i32,
    pub document_id: DocumentId,
    pub status: TaxFilingStatus,
    pub created_at: String,
}

#[derive(Serialize)]
pub struct DeadlineResponse {
    pub deadline_id: DeadlineId,
    pub entity_id: EntityId,
    pub deadline_type: String,
    pub due_date: NaiveDate,
    pub description: String,
    pub recurrence: Recurrence,
    pub severity: DeadlineSeverity,
    pub status: DeadlineStatus,
    pub completed_at: Option<String>,
    pub created_at: String,
}

#[derive(Serialize)]
pub struct ClassificationResponse {
    pub classification_id: ClassificationId,
    pub entity_id: EntityId,
    pub contractor_name: String,
    pub state: String,
    pub risk_level: RiskLevel,
    pub flags: Vec<String>,
    pub classification: ClassificationResult,
    pub created_at: String,
}

#[derive(Serialize)]
pub struct ComplianceEscalationResponse {
    pub escalation_id: ComplianceEscalationId,
    pub entity_id: EntityId,
    pub deadline_id: DeadlineId,
    pub milestone: String,
    pub action: String,
    pub authority: String,
    pub status: EscalationStatus,
    pub obligation_id: Option<ObligationId>,
    pub incident_id: Option<IncidentId>,
    pub created_at: String,
}

#[derive(Serialize)]
pub struct ComplianceScanResponse {
    pub scanned_deadlines: usize,
    pub escalations_created: usize,
    pub incidents_created: usize,
}

#[derive(Serialize)]
pub struct ResolveEscalationWithEvidenceResponse {
    pub escalation: ComplianceEscalationResponse,
    pub evidence_link_id: ComplianceEvidenceLinkId,
    pub obligation_resolved: bool,
    pub incident_resolved: bool,
}

// ── Helper ───────────────────────────────────────────────────────────

fn open_store<'a>(
    layout: &'a crate::store::RepoLayout,
    workspace_id: WorkspaceId,
    entity_id: EntityId,
) -> Result<EntityStore<'a>, AppError> {
    EntityStore::open(layout, workspace_id, entity_id).map_err(|e| match e {
        crate::git::error::GitStorageError::RepoNotFound(_) => {
            AppError::NotFound(format!("entity {} not found", entity_id))
        }
        other => AppError::Internal(other.to_string()),
    })
}

fn deadline_to_response(deadline: &Deadline) -> DeadlineResponse {
    DeadlineResponse {
        deadline_id: deadline.deadline_id(),
        entity_id: deadline.entity_id(),
        deadline_type: deadline.deadline_type().to_owned(),
        due_date: deadline.due_date(),
        description: deadline.description().to_owned(),
        recurrence: deadline.recurrence(),
        severity: deadline.severity(),
        status: deadline.status(),
        completed_at: deadline.completed_at().map(|t| t.to_rfc3339()),
        created_at: deadline.created_at().to_rfc3339(),
    }
}

fn escalation_to_response(escalation: &ComplianceEscalation) -> ComplianceEscalationResponse {
    ComplianceEscalationResponse {
        escalation_id: escalation.escalation_id(),
        entity_id: escalation.entity_id(),
        deadline_id: escalation.deadline_id(),
        milestone: escalation.milestone().to_owned(),
        action: escalation.action().to_owned(),
        authority: escalation.authority().to_owned(),
        status: escalation.status(),
        obligation_id: escalation.obligation_id(),
        incident_id: escalation.incident_id(),
        created_at: escalation.created_at().to_rfc3339(),
    }
}

fn milestone_specs() -> [(&'static str, i64, &'static str, &'static str); 6] {
    [
        (
            "D-30",
            30,
            "Prepare filing package and evidence",
            "operator",
        ),
        (
            "D-14",
            14,
            "Escalate with owner reminder and checklist",
            "operator",
        ),
        ("D-7", 7, "Require execution plan with assignee", "officer"),
        (
            "D-1",
            1,
            "Immediate owner notification and hold risky actions",
            "officer",
        ),
        (
            "D+0",
            0,
            "Deadline due today: block non-essential changes",
            "officer",
        ),
        (
            "D+1",
            -1,
            "Missed deadline: incident + board escalation",
            "board",
        ),
    ]
}

// ── Handlers ─────────────────────────────────────────────────────────

async fn file_tax_document(
    RequireAdmin(auth): RequireAdmin,
    State(state): State<AppState>,
    Json(req): Json<FileTaxDocumentRequest>,
) -> Result<Json<TaxFilingResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = req.entity_id;

    let filing = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;

            let filing_id = TaxFilingId::new();
            let document_id = DocumentId::new();
            let filing = TaxFiling::new(
                filing_id,
                entity_id,
                req.document_type,
                req.tax_year,
                document_id,
            );

            let path = format!("tax/filings/{}.json", filing_id);
            store
                .write_json(
                    "main",
                    &path,
                    &filing,
                    &format!("File tax document {filing_id}"),
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

            Ok::<_, AppError>(filing)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(TaxFilingResponse {
        filing_id: filing.filing_id(),
        entity_id: filing.entity_id(),
        document_type: filing.document_type().to_owned(),
        tax_year: filing.tax_year(),
        document_id: filing.document_id(),
        status: filing.status(),
        created_at: filing.created_at().to_rfc3339(),
    }))
}

async fn create_deadline(
    RequireAdmin(auth): RequireAdmin,
    State(state): State<AppState>,
    Json(req): Json<CreateDeadlineRequest>,
) -> Result<Json<DeadlineResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = req.entity_id;

    let deadline = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;

            let deadline_id = DeadlineId::new();
            let deadline = Deadline::new(
                deadline_id,
                entity_id,
                req.deadline_type,
                req.due_date,
                req.description,
                req.recurrence,
                req.severity,
            );

            let path = format!("deadlines/{}.json", deadline_id);
            store
                .write_json(
                    "main",
                    &path,
                    &deadline,
                    &format!("COMPLIANCE: create deadline {deadline_id}"),
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

            Ok::<_, AppError>(deadline)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(deadline_to_response(&deadline)))
}

async fn classify_contractor(
    RequireAdmin(auth): RequireAdmin,
    State(state): State<AppState>,
    Json(req): Json<ClassifyContractorRequest>,
) -> Result<Json<ClassificationResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = req.entity_id;

    let classification = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;

            let classification_id = ClassificationId::new();
            // Simple risk assessment based on state
            let risk_level = match req.state.as_str() {
                "CA" | "NY" | "MA" => RiskLevel::High,
                "TX" | "FL" | "WA" => RiskLevel::Low,
                _ => RiskLevel::Medium,
            };
            let flags: Vec<String> = if risk_level == RiskLevel::High {
                vec![format!(
                    "{}_strict_classification_laws",
                    req.state.to_lowercase()
                )]
            } else {
                vec![]
            };

            let classification = ContractorClassification::new(
                classification_id,
                entity_id,
                req.contractor_name,
                req.state,
                risk_level,
                flags,
                ClassificationResult::Independent,
            );

            let path = format!("contractors/{}.json", classification_id);
            store
                .write_json(
                    "main",
                    &path,
                    &classification,
                    &format!("Classify contractor {classification_id}"),
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

            Ok::<_, AppError>(classification)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(ClassificationResponse {
        classification_id: classification.classification_id(),
        entity_id: classification.entity_id(),
        contractor_name: classification.contractor_name().to_owned(),
        state: classification.state().to_owned(),
        risk_level: classification.risk_level(),
        flags: classification.flags().to_vec(),
        classification: classification.classification(),
        created_at: classification.created_at().to_rfc3339(),
    }))
}

async fn scan_compliance_escalations(
    RequireAdmin(auth): RequireAdmin,
    State(state): State<AppState>,
    Json(req): Json<ScanComplianceRequest>,
) -> Result<Json<ComplianceScanResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = req.entity_id;

    let response = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let deadline_ids = store
                .list_ids::<Deadline>("main")
                .map_err(|e| AppError::Internal(format!("list deadlines: {e}")))?;
            let escalation_ids = store
                .list_ids::<ComplianceEscalation>("main")
                .map_err(|e| AppError::Internal(format!("list escalations: {e}")))?;

            let mut existing = std::collections::HashSet::<(DeadlineId, String)>::new();
            for escalation_id in escalation_ids {
                if let Ok(escalation) = store.read::<ComplianceEscalation>("main", escalation_id) {
                    existing.insert((escalation.deadline_id(), escalation.milestone().to_owned()));
                }
            }

            let today = Utc::now().date_naive();
            let mut scanned_deadlines = 0usize;
            let mut escalations_created = 0usize;
            let mut incidents_created = 0usize;

            for deadline_id in deadline_ids {
                let deadline = match store.read::<Deadline>("main", deadline_id) {
                    Ok(deadline) => deadline,
                    Err(_) => continue,
                };
                if deadline.status() == DeadlineStatus::Completed {
                    continue;
                }
                scanned_deadlines += 1;
                let days_until = (deadline.due_date() - today).num_days();

                for (milestone, threshold, action, authority) in milestone_specs() {
                    if days_until > threshold {
                        continue;
                    }
                    let key = (deadline.deadline_id(), milestone.to_owned());
                    if existing.contains(&key) {
                        continue;
                    }

                    let obligation_id = ObligationId::new();
                    let obligation = Obligation::new(
                        obligation_id,
                        entity_id,
                        None,
                        ObligationType::new(format!(
                            "compliance_escalation_{}",
                            milestone.to_lowercase().replace('+', "plus")
                        )),
                        AssigneeType::Human,
                        None,
                        format!(
                            "{} for deadline {} ({})",
                            action,
                            deadline.deadline_type(),
                            deadline.deadline_id()
                        ),
                        Some(deadline.due_date()),
                    );
                    store
                        .write_json(
                            "main",
                            &format!("execution/obligations/{}.json", obligation_id),
                            &obligation,
                            &format!(
                                "COMPLIANCE: create escalation obligation {obligation_id} for {}",
                                deadline.deadline_id()
                            ),
                        )
                        .map_err(|e| AppError::Internal(format!("commit: {e}")))?;

                    let incident_id = if milestone == "D+1" {
                        let severity = match deadline.severity() {
                            DeadlineSeverity::Low => IncidentSeverity::Low,
                            DeadlineSeverity::Medium => IncidentSeverity::Medium,
                            DeadlineSeverity::High => IncidentSeverity::High,
                            DeadlineSeverity::Critical => IncidentSeverity::Critical,
                        };
                        let lockdown = apply_lockdown_trigger(
                            &store,
                            entity_id,
                            LockdownTriggerInput {
                                source: GovernanceTriggerSource::ComplianceScanner,
                                trigger_type: GovernanceTriggerType::ComplianceDeadlineMissedDPlus1,
                                severity,
                                title: format!("Compliance miss: {}", deadline.deadline_type()),
                                description: format!(
                                    "Deadline {} missed by at least one day",
                                    deadline.deadline_id()
                                ),
                                evidence_refs: vec![format!("deadline:{}", deadline.deadline_id())],
                                linked_intent_id: None,
                                linked_escalation_id: None,
                                idempotency_key: Some(format!(
                                    "compliance-d-plus-1:{}",
                                    deadline.deadline_id()
                                )),
                                existing_incident_id: None,
                                updated_by: None,
                            },
                        )?;
                        if lockdown.incident_created {
                            incidents_created += 1;
                        }
                        Some(lockdown.incident.incident_id())
                    } else {
                        None
                    };

                    let escalation = ComplianceEscalation::new(
                        ComplianceEscalationId::new(),
                        entity_id,
                        deadline.deadline_id(),
                        milestone.to_owned(),
                        action.to_owned(),
                        authority.to_owned(),
                        Some(obligation_id),
                        incident_id,
                    );
                    store
                        .write_json(
                            "main",
                            &format!("compliance/escalations/{}.json", escalation.escalation_id()),
                            &escalation,
                            &format!(
                                "COMPLIANCE: create escalation {} {}",
                                escalation.escalation_id(),
                                milestone
                            ),
                        )
                        .map_err(|e| AppError::Internal(format!("commit: {e}")))?;
                    existing.insert((deadline.deadline_id(), milestone.to_owned()));
                    escalations_created += 1;
                }
            }

            Ok::<_, AppError>(ComplianceScanResponse {
                scanned_deadlines,
                escalations_created,
                incidents_created,
            })
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(response))
}

async fn list_entity_escalations(
    RequireAdmin(auth): RequireAdmin,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
) -> Result<Json<Vec<ComplianceEscalationResponse>>, AppError> {
    let workspace_id = auth.workspace_id();

    let escalations = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let ids = store
                .list_ids::<ComplianceEscalation>("main")
                .map_err(|e| AppError::Internal(format!("list escalations: {e}")))?;
            let mut out = Vec::new();
            for id in ids {
                if let Ok(escalation) = store.read::<ComplianceEscalation>("main", id) {
                    out.push(escalation_to_response(&escalation));
                }
            }
            out.sort_by(|a, b| b.created_at.cmp(&a.created_at));
            Ok::<_, AppError>(out)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(escalations))
}

async fn resolve_escalation_with_evidence(
    RequireAdmin(auth): RequireAdmin,
    State(state): State<AppState>,
    Path(escalation_id): Path<ComplianceEscalationId>,
    Json(req): Json<ResolveEscalationWithEvidenceRequest>,
) -> Result<Json<ResolveEscalationWithEvidenceResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = req.entity_id;

    let response = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let mut escalation = store
                .read::<ComplianceEscalation>("main", escalation_id)
                .map_err(|_| {
                    AppError::NotFound(format!("escalation {} not found", escalation_id))
                })?;
            if escalation.entity_id() != entity_id {
                return Err(AppError::Forbidden(
                    "escalation belongs to a different entity".to_owned(),
                ));
            }

            let evidence_link_id = ComplianceEvidenceLinkId::new();
            let evidence_type = req
                .evidence_type
                .unwrap_or_else(|| "escalation_resolution_evidence".to_owned());
            let evidence_link = ComplianceEvidenceLink::new(
                evidence_link_id,
                entity_id,
                escalation_id,
                evidence_type,
                req.packet_id,
                req.filing_reference,
                req.notes,
            );

            let mut files = vec![
                FileWrite::json(
                    format!(
                        "compliance/evidence-links/{}.json",
                        evidence_link.evidence_link_id()
                    ),
                    &evidence_link,
                )
                .map_err(|e| AppError::Internal(format!("serialize evidence link: {e}")))?,
            ];

            if let Some(packet_id) = req.packet_id {
                let mut packet = store
                    .read::<TransactionPacket>("main", packet_id)
                    .map_err(|_| AppError::NotFound(format!("packet {} not found", packet_id)))?;
                if packet.entity_id() != entity_id {
                    return Err(AppError::Forbidden(
                        "packet belongs to a different entity".to_owned(),
                    ));
                }
                packet.add_evidence_ref(format!("escalation:{}", escalation_id));
                packet.add_evidence_ref(format!("evidence_link:{}", evidence_link_id));
                files.push(
                    FileWrite::json(format!("execution/packets/{}.json", packet_id), &packet)
                        .map_err(|e| AppError::Internal(format!("serialize packet: {e}")))?,
                );
            }

            let mut obligation_resolved = false;
            if req.resolve_obligation
                && let Some(obligation_id) = escalation.obligation_id()
            {
                let mut obligation =
                    store
                        .read::<Obligation>("main", obligation_id)
                        .map_err(|_| {
                            AppError::NotFound(format!("obligation {} not found", obligation_id))
                        })?;
                if matches!(
                    obligation.status(),
                    ObligationStatus::Required | ObligationStatus::InProgress
                ) {
                    obligation.fulfill()?;
                    obligation_resolved = true;
                    files.push(
                        FileWrite::json(
                            format!("execution/obligations/{}.json", obligation_id),
                            &obligation,
                        )
                        .map_err(|e| AppError::Internal(format!("serialize obligation: {e}")))?,
                    );
                }
            }

            let mut incident_resolved = false;
            if req.resolve_incident
                && let Some(incident_id) = escalation.incident_id()
            {
                let mut incident = store
                    .read::<GovernanceIncident>("main", incident_id)
                    .map_err(|_| {
                        AppError::NotFound(format!("incident {} not found", incident_id))
                    })?;
                if incident.status() == IncidentStatus::Open {
                    incident.resolve();
                    incident_resolved = true;
                    files.push(
                        FileWrite::json(
                            format!("governance/incidents/{}.json", incident_id),
                            &incident,
                        )
                        .map_err(|e| AppError::Internal(format!("serialize incident: {e}")))?,
                    );
                }
            }

            escalation.resolve();
            files.push(
                FileWrite::json(
                    format!("compliance/escalations/{}.json", escalation_id),
                    &escalation,
                )
                .map_err(|e| AppError::Internal(format!("serialize escalation: {e}")))?,
            );

            store
                .commit(
                    "main",
                    &format!(
                        "COMPLIANCE: resolve escalation {} with evidence {}",
                        escalation_id, evidence_link_id
                    ),
                    files,
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

            Ok::<_, AppError>(ResolveEscalationWithEvidenceResponse {
                escalation: escalation_to_response(&escalation),
                evidence_link_id,
                obligation_resolved,
                incident_resolved,
            })
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(response))
}

// ── Router ───────────────────────────────────────────────────────────

pub fn compliance_routes() -> Router<AppState> {
    Router::new()
        .route("/v1/tax/filings", post(file_tax_document))
        .route("/v1/deadlines", post(create_deadline))
        .route("/v1/contractors/classify", post(classify_contractor))
        .route(
            "/v1/compliance/escalations/scan",
            post(scan_compliance_escalations),
        )
        .route(
            "/v1/entities/{entity_id}/compliance/escalations",
            get(list_entity_escalations),
        )
        .route(
            "/v1/compliance/escalations/{escalation_id}/resolve-with-evidence",
            post(resolve_escalation_with_evidence),
        )
}
