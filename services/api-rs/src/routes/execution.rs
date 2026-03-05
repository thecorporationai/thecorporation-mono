//! Execution HTTP routes.
//!
//! Endpoints for intents, obligations, and receipts.

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{get, patch, post},
};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use super::AppState;
use crate::auth::{RequireExecutionRead, RequireExecutionWrite};
use crate::domain::execution::{
    approval_artifact::ApprovalArtifact,
    document_request::DocumentRequest,
    intent::Intent,
    obligation::Obligation,
    receipt::Receipt,
    transaction_packet::{PacketItem, TransactionPacket, TransactionPacketStatus, WorkflowType},
    types::*,
};
use crate::domain::formation::types::FormationStatus;
use crate::domain::governance::delegation_schedule::{CURRENT_SCHEDULE_PATH, DelegationSchedule};
use crate::domain::governance::incident::IncidentSeverity;
use crate::domain::governance::policy_engine::{
    PolicyDecision, PolicyEvaluationContext, amount_from_metadata_cents,
    canonicalize_intent_type, evaluate_full_with_override,
    mapped_tier_requires_manual_artifacts,
};
use crate::domain::governance::trigger::{GovernanceTriggerSource, GovernanceTriggerType};
use crate::domain::ids::{
    ApprovalArtifactId, ContactId, DocumentRequestId, EntityId, IntentId, ObligationId, PacketId,
    PacketSignatureId, ReceiptId, WorkspaceId,
};
use crate::error::AppError;
use crate::routes::governance_enforcement::{
    LockdownTriggerInput, apply_lockdown_trigger, read_mode_or_default,
};
use crate::store::entity_store::EntityStore;

// ── Request types ────────────────────────────────────────────────────

#[derive(Deserialize, utoipa::ToSchema)]
pub struct CreateIntentRequest {
    pub entity_id: EntityId,
    pub intent_type: String,
    /// Deprecated: authority is always derived server-side from policy.
    #[serde(default)]
    pub authority_tier: Option<AuthorityTier>,
    pub description: String,
    #[serde(default = "default_metadata")]
    #[schema(value_type = Object)]
    pub metadata: serde_json::Value,
}

fn default_metadata() -> serde_json::Value {
    serde_json::Value::Object(serde_json::Map::new())
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct CreateObligationRequest {
    pub entity_id: EntityId,
    #[serde(default)]
    pub intent_id: Option<IntentId>,
    pub obligation_type: String,
    pub assignee_type: AssigneeType,
    #[serde(default)]
    pub assignee_id: Option<ContactId>,
    pub description: String,
    #[serde(default)]
    pub due_date: Option<NaiveDate>,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct CreateApprovalArtifactRequest {
    pub entity_id: EntityId,
    pub intent_type: String,
    pub scope: String,
    pub approver_identity: String,
    #[serde(default = "default_explicit_approval")]
    pub explicit: bool,
    #[serde(default)]
    pub approved_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub expires_at: Option<DateTime<Utc>>,
    pub channel: String,
    #[serde(default)]
    pub max_amount_cents: Option<i64>,
}

fn default_explicit_approval() -> bool {
    true
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct BindApprovalArtifactRequest {
    pub entity_id: EntityId,
    pub approval_artifact_id: ApprovalArtifactId,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct BindDocumentRequestRequest {
    pub entity_id: EntityId,
    pub request_id: DocumentRequestId,
}

// ── Response types ───────────────────────────────────────────────────

#[derive(Serialize, utoipa::ToSchema)]
pub struct IntentResponse {
    pub intent_id: IntentId,
    pub entity_id: EntityId,
    pub intent_type: String,
    pub authority_tier: AuthorityTier,
    pub policy_decision: Option<PolicyDecision>,
    pub bound_approval_artifact_id: Option<ApprovalArtifactId>,
    pub bound_document_request_ids: Vec<DocumentRequestId>,
    pub status: IntentStatus,
    pub description: String,
    pub evaluated_at: Option<String>,
    pub authorized_at: Option<String>,
    pub executed_at: Option<String>,
    pub failed_at: Option<String>,
    pub failure_reason: Option<String>,
    pub cancelled_at: Option<String>,
    pub created_at: String,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct ObligationResponse {
    pub obligation_id: ObligationId,
    pub entity_id: EntityId,
    pub intent_id: Option<IntentId>,
    pub obligation_type: String,
    pub assignee_type: AssigneeType,
    pub assignee_id: Option<ContactId>,
    pub description: String,
    pub due_date: Option<NaiveDate>,
    pub status: ObligationStatus,
    pub fulfilled_at: Option<String>,
    pub waived_at: Option<String>,
    pub expired_at: Option<String>,
    pub created_at: String,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct ApprovalArtifactResponse {
    pub approval_artifact_id: ApprovalArtifactId,
    pub entity_id: EntityId,
    pub intent_type: String,
    pub scope: String,
    pub approver_identity: String,
    pub explicit: bool,
    pub approved_at: String,
    pub expires_at: Option<String>,
    pub channel: String,
    pub max_amount_cents: Option<i64>,
    pub revoked_at: Option<String>,
    pub created_at: String,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct PacketSignatureResponse {
    pub signature_id: PacketSignatureId,
    pub signer_identity: String,
    pub channel: String,
    pub signed_at: String,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct TransactionPacketResponse {
    pub packet_id: PacketId,
    pub entity_id: EntityId,
    pub intent_id: IntentId,
    pub workflow_type: WorkflowType,
    pub workflow_id: String,
    pub status: TransactionPacketStatus,
    pub manifest_hash: String,
    pub items: Vec<PacketItem>,
    pub required_signers: Vec<String>,
    pub signatures: Vec<PacketSignatureResponse>,
    pub evidence_refs: Vec<String>,
    pub created_at: String,
    pub finalized_at: Option<String>,
}

// ── Conversion helpers ───────────────────────────────────────────────

fn intent_to_response(i: &Intent) -> IntentResponse {
    IntentResponse {
        intent_id: i.intent_id(),
        entity_id: i.entity_id(),
        intent_type: i.intent_type().to_owned(),
        authority_tier: i.authority_tier(),
        policy_decision: i.policy_decision().cloned(),
        bound_approval_artifact_id: i.bound_approval_artifact_id(),
        bound_document_request_ids: i.bound_document_request_ids().to_vec(),
        status: i.status(),
        description: i.description().to_owned(),
        evaluated_at: i.evaluated_at().map(|t| t.to_rfc3339()),
        authorized_at: i.authorized_at().map(|t| t.to_rfc3339()),
        executed_at: i.executed_at().map(|t| t.to_rfc3339()),
        failed_at: i.failed_at().map(|t| t.to_rfc3339()),
        failure_reason: i.failure_reason().map(|s| s.to_owned()),
        cancelled_at: i.cancelled_at().map(|t| t.to_rfc3339()),
        created_at: i.created_at().to_rfc3339(),
    }
}

fn approval_to_response(a: &ApprovalArtifact) -> ApprovalArtifactResponse {
    ApprovalArtifactResponse {
        approval_artifact_id: a.approval_artifact_id(),
        entity_id: a.entity_id(),
        intent_type: a.intent_type().to_owned(),
        scope: a.scope().to_owned(),
        approver_identity: a.approver_identity().to_owned(),
        explicit: a.explicit(),
        approved_at: a.approved_at().to_rfc3339(),
        expires_at: a.expires_at().map(|t| t.to_rfc3339()),
        channel: a.channel().to_owned(),
        max_amount_cents: a.max_amount_cents(),
        revoked_at: a.revoked_at().map(|t| t.to_rfc3339()),
        created_at: a.created_at().to_rfc3339(),
    }
}

fn obligation_to_response(o: &Obligation) -> ObligationResponse {
    ObligationResponse {
        obligation_id: o.obligation_id(),
        entity_id: o.entity_id(),
        intent_id: o.intent_id(),
        obligation_type: o.obligation_type().as_str().to_owned(),
        assignee_type: o.assignee_type(),
        assignee_id: o.assignee_id(),
        description: o.description().to_owned(),
        due_date: o.due_date(),
        status: o.status(),
        fulfilled_at: o.fulfilled_at().map(|t| t.to_rfc3339()),
        waived_at: o.waived_at().map(|t| t.to_rfc3339()),
        expired_at: o.expired_at().map(|t| t.to_rfc3339()),
        created_at: o.created_at().to_rfc3339(),
    }
}

fn packet_to_response(packet: &TransactionPacket) -> TransactionPacketResponse {
    TransactionPacketResponse {
        packet_id: packet.packet_id(),
        entity_id: packet.entity_id(),
        intent_id: packet.intent_id(),
        workflow_type: packet.workflow_type(),
        workflow_id: packet.workflow_id().to_owned(),
        status: packet.status(),
        manifest_hash: packet.manifest_hash().to_owned(),
        items: packet.items().to_vec(),
        required_signers: packet.required_signers().to_vec(),
        signatures: packet
            .signatures()
            .iter()
            .map(|s| PacketSignatureResponse {
                signature_id: s.signature_id(),
                signer_identity: s.signer_identity().to_owned(),
                channel: s.channel().to_owned(),
                signed_at: s.signed_at().to_rfc3339(),
            })
            .collect(),
        evidence_refs: packet.evidence_refs().to_vec(),
        created_at: packet.created_at().to_rfc3339(),
        finalized_at: packet.finalized_at().map(|t| t.to_rfc3339()),
    }
}

// ── Helper to open a store ───────────────────────────────────────────

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

fn read_schedule_or_default(store: &EntityStore<'_>, entity_id: EntityId) -> DelegationSchedule {
    store
        .read_json::<DelegationSchedule>("main", CURRENT_SCHEDULE_PATH)
        .unwrap_or_else(|_| DelegationSchedule::default_for_entity(entity_id))
}

// Override functions (apply_mode_overrides, apply_schedule_overrides, etc.)
// and helpers (amount_from_metadata_cents, mapped_tier_requires_manual_artifacts)
// are now in crate::domain::governance::policy_engine.

fn required_document_types(intent_type: &str) -> &'static [&'static str] {
    match intent_type {
        "equity.transfer.execute" => &["stock_transfer_agreement", "transfer_board_consent"],
        "equity.fundraising.accept" => &[
            "board_consent",
            "equity_issuance_approval",
            "subscription_agreement",
        ],
        "equity.fundraising.close" => &["investor_rights_agreement", "subscription_agreement"],
        _ => &[],
    }
}

#[derive(Debug, Clone)]
enum ExecutionPrerequisiteViolation {
    MissingApprovalArtifact {
        intent_id: IntentId,
        tier: AuthorityTier,
    },
    ApprovalArtifactNotFound {
        approval_artifact_id: ApprovalArtifactId,
    },
    ApprovalScopeMismatch {
        approval_artifact_id: ApprovalArtifactId,
        intent_id: IntentId,
    },
    MissingRequiredDocumentRequest {
        intent_id: IntentId,
        required_document_types: Vec<String>,
    },
    DocumentRequestNotFound {
        request_id: DocumentRequestId,
    },
    DocumentRequestUnsatisfied {
        intent_id: IntentId,
        required_document_type: String,
    },
}

impl ExecutionPrerequisiteViolation {
    fn should_trigger_lockdown(&self) -> bool {
        matches!(
            self,
            Self::ApprovalScopeMismatch { .. }
                | Self::MissingRequiredDocumentRequest { .. }
                | Self::DocumentRequestUnsatisfied { .. }
        )
    }

    fn reason(&self) -> String {
        match self {
            Self::MissingApprovalArtifact { intent_id, tier } => {
                format!("intent {intent_id} requires bound approval artifact for {tier}")
            }
            Self::ApprovalArtifactNotFound {
                approval_artifact_id,
            } => {
                format!("approval artifact {approval_artifact_id} not found")
            }
            Self::ApprovalScopeMismatch {
                approval_artifact_id,
                intent_id,
            } => format!(
                "approval artifact {approval_artifact_id} does not cover intent {intent_id}"
            ),
            Self::MissingRequiredDocumentRequest {
                intent_id,
                required_document_types,
            } => format!(
                "intent {intent_id} requires supporting document requests: {}",
                required_document_types.join(", ")
            ),
            Self::DocumentRequestNotFound { request_id } => {
                format!("document request {request_id} not found")
            }
            Self::DocumentRequestUnsatisfied {
                intent_id,
                required_document_type,
            } => format!(
                "missing satisfied document request for {required_document_type} on intent {intent_id}"
            ),
        }
    }
}

fn enforce_execution_prerequisites(
    store: &EntityStore<'_>,
    intent: &Intent,
    decision: &PolicyDecision,
) -> Result<(), ExecutionPrerequisiteViolation> {
    if !mapped_tier_requires_manual_artifacts(decision) {
        return Ok(());
    }

    let approval_id = intent.bound_approval_artifact_id().ok_or(
        ExecutionPrerequisiteViolation::MissingApprovalArtifact {
            intent_id: intent.intent_id(),
            tier: decision.tier(),
        },
    )?;

    let approval = store
        .read::<ApprovalArtifact>("main", approval_id)
        .map_err(
            |_| ExecutionPrerequisiteViolation::ApprovalArtifactNotFound {
                approval_artifact_id: approval_id,
            },
        )?;
    let canonical_intent_type = canonicalize_intent_type(intent.intent_type());
    if !approval.covers_intent(
        &canonical_intent_type,
        amount_from_metadata_cents(intent.metadata()),
        Utc::now(),
    ) {
        return Err(ExecutionPrerequisiteViolation::ApprovalScopeMismatch {
            approval_artifact_id: approval_id,
            intent_id: intent.intent_id(),
        });
    }

    let required = required_document_types(&canonical_intent_type);
    if required.is_empty() {
        return Ok(());
    }

    let bound_ids = intent.bound_document_request_ids();
    if bound_ids.is_empty() {
        return Err(
            ExecutionPrerequisiteViolation::MissingRequiredDocumentRequest {
                intent_id: intent.intent_id(),
                required_document_types: required.iter().map(|v| (*v).to_owned()).collect(),
            },
        );
    }

    let mut bound_requests = Vec::new();
    for request_id in bound_ids {
        let request = store
            .read::<DocumentRequest>("main", *request_id)
            .map_err(
                |_| ExecutionPrerequisiteViolation::DocumentRequestNotFound {
                    request_id: *request_id,
                },
            )?;
        bound_requests.push(request);
    }

    for required_doc_type in required {
        let satisfied = bound_requests
            .iter()
            .any(|request| request.document_type() == *required_doc_type && request.is_satisfied());
        if !satisfied {
            return Err(ExecutionPrerequisiteViolation::DocumentRequestUnsatisfied {
                intent_id: intent.intent_id(),
                required_document_type: (*required_doc_type).to_owned(),
            });
        }
    }

    Ok(())
}

fn trigger_policy_evidence_lockdown(
    store: &EntityStore<'_>,
    entity_id: EntityId,
    intent_id: IntentId,
    reason: &str,
) -> Result<(), AppError> {
    apply_lockdown_trigger(
        store,
        entity_id,
        LockdownTriggerInput {
            source: GovernanceTriggerSource::ExecutionGate,
            trigger_type: GovernanceTriggerType::PolicyEvidenceMismatch,
            severity: IncidentSeverity::High,
            title: "Policy evidence mismatch".to_owned(),
            description: reason.to_owned(),
            evidence_refs: vec![format!("intent:{intent_id}")],
            linked_intent_id: Some(intent_id),
            linked_escalation_id: None,
            idempotency_key: Some(format!("policy-evidence-mismatch:{intent_id}:{reason}")),
            existing_incident_id: None,
            updated_by: None,
        },
    )?;
    Ok(())
}

// ── Handlers: Intents ────────────────────────────────────────────────

#[utoipa::path(
    post,
    path = "/v1/execution/intents",
    tag = "execution",
    request_body = CreateIntentRequest,
    responses(
        (status = 200, body = IntentResponse),
        (status = 400, description = "Bad request"),
        (status = 422, description = "Unprocessable entity"),
    ),
)]
async fn create_intent(
    RequireExecutionWrite(auth): RequireExecutionWrite,
    State(state): State<AppState>,
    Json(req): Json<CreateIntentRequest>,
) -> Result<Json<IntentResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = req.entity_id;

    let intent = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let canonical_intent_type = canonicalize_intent_type(&req.intent_type);
            let mode = read_mode_or_default(&store, entity_id);
            let schedule = read_schedule_or_default(&store, entity_id);
            let entity = store
                .read_entity("main")
                .map_err(|e| AppError::Internal(format!("read entity: {e}")))?;
            let ctx = PolicyEvaluationContext {
                intent_type: &canonical_intent_type,
                metadata: &req.metadata,
                mode: mode.mode(),
                schedule: &schedule,
                now: Utc::now(),
                entity_is_active: matches!(entity.formation_status(), FormationStatus::Active),
                service_agreement_executed: entity.service_agreement_executed(),
            };
            let decision = evaluate_full_with_override(&ctx, req.authority_tier);

            let intent_id = IntentId::new();
            let mut intent = Intent::new(
                intent_id,
                entity_id,
                workspace_id,
                canonical_intent_type,
                decision.tier(),
                req.description,
                req.metadata,
            );
            intent.set_policy_decision(decision);

            let path = format!("execution/intents/{}.json", intent_id);
            store
                .write_json(
                    "main",
                    &path,
                    &intent,
                    &format!("EXECUTION: create intent {intent_id}"),
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

            Ok::<_, AppError>(intent)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(intent_to_response(&intent)))
}

#[utoipa::path(
    get,
    path = "/v1/entities/{entity_id}/intents",
    tag = "execution",
    params(("entity_id" = EntityId, Path, description = "Entity ID")),
    responses(
        (status = 200, body = Vec<IntentResponse>),
        (status = 404, description = "Entity not found"),
    ),
)]
async fn list_intents(
    RequireExecutionRead(auth): RequireExecutionRead,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
) -> Result<Json<Vec<IntentResponse>>, AppError> {
    let workspace_id = auth.workspace_id();

    let intents = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let ids = store
                .list_ids::<Intent>("main")
                .map_err(|e| AppError::Internal(format!("list intents: {e}")))?;

            let mut results = Vec::new();
            for id in ids {
                let i = store
                    .read::<Intent>("main", id)
                    .map_err(|e| AppError::Internal(format!("read intent {id}: {e}")))?;
                results.push(intent_to_response(&i));
            }
            Ok::<_, AppError>(results)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(intents))
}

#[utoipa::path(
    post,
    path = "/v1/intents/{intent_id}/evaluate",
    tag = "execution",
    params(
        ("intent_id" = IntentId, Path, description = "Intent ID"),
        ("entity_id" = EntityId, Query, description = "Entity ID"),
    ),
    responses(
        (status = 200, body = IntentResponse),
        (status = 404, description = "Intent not found"),
    ),
)]
async fn evaluate_intent(
    RequireExecutionWrite(auth): RequireExecutionWrite,
    State(state): State<AppState>,
    Path(intent_id): Path<IntentId>,
    Query(query): Query<super::EntityIdQuery>,
) -> Result<Json<IntentResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = query.entity_id;

    let intent = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let mut intent = store
                .read::<Intent>("main", intent_id)
                .map_err(|_| AppError::NotFound(format!("intent {} not found", intent_id)))?;

            let canonical_intent_type = canonicalize_intent_type(intent.intent_type());
            let mode = read_mode_or_default(&store, entity_id);
            let schedule = read_schedule_or_default(&store, entity_id);
            let entity = store
                .read_entity("main")
                .map_err(|e| AppError::Internal(format!("read entity: {e}")))?;
            let ctx = PolicyEvaluationContext {
                intent_type: &canonical_intent_type,
                metadata: intent.metadata(),
                mode: mode.mode(),
                schedule: &schedule,
                now: Utc::now(),
                entity_is_active: matches!(entity.formation_status(), FormationStatus::Active),
                service_agreement_executed: entity.service_agreement_executed(),
            };
            let decision = evaluate_full_with_override(&ctx, Some(intent.authority_tier()));
            let allowed = decision.allowed();
            let blockers = decision.blockers().to_vec();
            intent.update_authority_tier(decision.tier());
            intent.set_policy_decision(decision);

            if allowed {
                intent.evaluate()?;
            } else {
                intent.mark_failed(format!("policy blocked: {}", blockers.join("; ")))?;
            }

            let path = format!("execution/intents/{}.json", intent_id);
            store
                .write_json(
                    "main",
                    &path,
                    &intent,
                    &format!("EXECUTION: evaluate intent {intent_id}"),
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

            Ok::<_, AppError>(intent)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(intent_to_response(&intent)))
}

#[utoipa::path(
    post,
    path = "/v1/intents/{intent_id}/authorize",
    tag = "execution",
    params(
        ("intent_id" = IntentId, Path, description = "Intent ID"),
        ("entity_id" = EntityId, Query, description = "Entity ID"),
    ),
    responses(
        (status = 200, body = IntentResponse),
        (status = 404, description = "Intent not found"),
        (status = 422, description = "Blocked by policy or missing prerequisites"),
    ),
)]
async fn authorize_intent(
    RequireExecutionWrite(auth): RequireExecutionWrite,
    State(state): State<AppState>,
    Path(intent_id): Path<IntentId>,
    Query(query): Query<super::EntityIdQuery>,
) -> Result<Json<IntentResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = query.entity_id;

    let intent = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let mut intent = store
                .read::<Intent>("main", intent_id)
                .map_err(|_| AppError::NotFound(format!("intent {} not found", intent_id)))?;

            let canonical_intent_type = canonicalize_intent_type(intent.intent_type());
            let mode = read_mode_or_default(&store, entity_id);
            let schedule = read_schedule_or_default(&store, entity_id);
            let entity = store
                .read_entity("main")
                .map_err(|e| AppError::Internal(format!("read entity: {e}")))?;
            let ctx = PolicyEvaluationContext {
                intent_type: &canonical_intent_type,
                metadata: intent.metadata(),
                mode: mode.mode(),
                schedule: &schedule,
                now: Utc::now(),
                entity_is_active: matches!(entity.formation_status(), FormationStatus::Active),
                service_agreement_executed: entity.service_agreement_executed(),
            };
            let decision = evaluate_full_with_override(&ctx, Some(intent.authority_tier()));
            if !decision.allowed() {
                return Err(AppError::UnprocessableEntity(format!(
                    "intent blocked by policy: {}",
                    decision.blockers().join("; ")
                )));
            }
            if let Err(violation) = enforce_execution_prerequisites(&store, &intent, &decision) {
                if violation.should_trigger_lockdown() {
                    trigger_policy_evidence_lockdown(
                        &store,
                        entity_id,
                        intent.intent_id(),
                        &violation.reason(),
                    )?;
                }
                return Err(AppError::UnprocessableEntity(violation.reason()));
            }
            intent.update_authority_tier(decision.tier());
            intent.set_policy_decision(decision);
            intent.authorize()?;

            let path = format!("execution/intents/{}.json", intent_id);
            store
                .write_json(
                    "main",
                    &path,
                    &intent,
                    &format!("EXECUTION: authorize intent {intent_id}"),
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

            Ok::<_, AppError>(intent)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(intent_to_response(&intent)))
}

#[utoipa::path(
    post,
    path = "/v1/intents/{intent_id}/execute",
    tag = "execution",
    params(
        ("intent_id" = IntentId, Path, description = "Intent ID"),
        ("entity_id" = EntityId, Query, description = "Entity ID"),
    ),
    responses(
        (status = 200, body = IntentResponse),
        (status = 404, description = "Intent not found"),
        (status = 422, description = "Blocked by policy or missing prerequisites"),
    ),
)]
async fn execute_intent(
    RequireExecutionWrite(auth): RequireExecutionWrite,
    State(state): State<AppState>,
    Path(intent_id): Path<IntentId>,
    Query(query): Query<super::EntityIdQuery>,
) -> Result<Json<IntentResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = query.entity_id;

    let intent = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let mut intent = store
                .read::<Intent>("main", intent_id)
                .map_err(|_| AppError::NotFound(format!("intent {} not found", intent_id)))?;

            let canonical_intent_type = canonicalize_intent_type(intent.intent_type());
            let mode = read_mode_or_default(&store, entity_id);
            let schedule = read_schedule_or_default(&store, entity_id);
            let entity = store
                .read_entity("main")
                .map_err(|e| AppError::Internal(format!("read entity: {e}")))?;
            let ctx = PolicyEvaluationContext {
                intent_type: &canonical_intent_type,
                metadata: intent.metadata(),
                mode: mode.mode(),
                schedule: &schedule,
                now: Utc::now(),
                entity_is_active: matches!(entity.formation_status(), FormationStatus::Active),
                service_agreement_executed: entity.service_agreement_executed(),
            };
            let decision = evaluate_full_with_override(&ctx, Some(intent.authority_tier()));
            if !decision.allowed() {
                return Err(AppError::UnprocessableEntity(format!(
                    "intent blocked by policy: {}",
                    decision.blockers().join("; ")
                )));
            }
            if let Err(violation) = enforce_execution_prerequisites(&store, &intent, &decision) {
                if violation.should_trigger_lockdown() {
                    trigger_policy_evidence_lockdown(
                        &store,
                        entity_id,
                        intent.intent_id(),
                        &violation.reason(),
                    )?;
                }
                return Err(AppError::UnprocessableEntity(violation.reason()));
            }
            intent.update_authority_tier(decision.tier());
            intent.set_policy_decision(decision);
            intent.mark_executed()?;

            let path = format!("execution/intents/{}.json", intent_id);
            store
                .write_json(
                    "main",
                    &path,
                    &intent,
                    &format!("EXECUTION: execute intent {intent_id}"),
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

            Ok::<_, AppError>(intent)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(intent_to_response(&intent)))
}

#[utoipa::path(
    post,
    path = "/v1/intents/{intent_id}/cancel",
    tag = "execution",
    params(
        ("intent_id" = IntentId, Path, description = "Intent ID"),
        ("entity_id" = EntityId, Query, description = "Entity ID"),
    ),
    responses(
        (status = 200, body = IntentResponse),
        (status = 404, description = "Intent not found"),
    ),
)]
async fn cancel_intent(
    RequireExecutionWrite(auth): RequireExecutionWrite,
    State(state): State<AppState>,
    Path(intent_id): Path<IntentId>,
    Query(query): Query<super::EntityIdQuery>,
) -> Result<Json<IntentResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = query.entity_id;

    let intent = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let mut intent = store
                .read::<Intent>("main", intent_id)
                .map_err(|_| AppError::NotFound(format!("intent {} not found", intent_id)))?;

            intent.cancel()?;

            let path = format!("execution/intents/{}.json", intent_id);
            store
                .write_json(
                    "main",
                    &path,
                    &intent,
                    &format!("EXECUTION: cancel intent {intent_id}"),
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

            Ok::<_, AppError>(intent)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(intent_to_response(&intent)))
}

#[utoipa::path(
    post,
    path = "/v1/execution/approval-artifacts",
    tag = "execution",
    request_body = CreateApprovalArtifactRequest,
    responses(
        (status = 200, body = ApprovalArtifactResponse),
        (status = 400, description = "Bad request"),
    ),
)]
async fn create_approval_artifact(
    RequireExecutionWrite(auth): RequireExecutionWrite,
    State(state): State<AppState>,
    Json(req): Json<CreateApprovalArtifactRequest>,
) -> Result<Json<ApprovalArtifactResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = req.entity_id;

    let artifact = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let canonical_intent_type = canonicalize_intent_type(&req.intent_type);
            let artifact = ApprovalArtifact::new(
                ApprovalArtifactId::new(),
                entity_id,
                canonical_intent_type,
                req.scope,
                req.approver_identity,
                req.explicit,
                req.approved_at.unwrap_or_else(Utc::now),
                req.expires_at,
                req.channel,
                req.max_amount_cents,
            );
            let path = format!(
                "execution/approval-artifacts/{}.json",
                artifact.approval_artifact_id()
            );
            store
                .write_json(
                    "main",
                    &path,
                    &artifact,
                    &format!(
                        "EXECUTION: create approval artifact {}",
                        artifact.approval_artifact_id()
                    ),
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;
            Ok::<_, AppError>(artifact)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(approval_to_response(&artifact)))
}

#[utoipa::path(
    get,
    path = "/v1/entities/{entity_id}/approval-artifacts",
    tag = "execution",
    params(("entity_id" = EntityId, Path, description = "Entity ID")),
    responses(
        (status = 200, body = Vec<ApprovalArtifactResponse>),
        (status = 404, description = "Entity not found"),
    ),
)]
async fn list_approval_artifacts(
    RequireExecutionRead(auth): RequireExecutionRead,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
) -> Result<Json<Vec<ApprovalArtifactResponse>>, AppError> {
    let workspace_id = auth.workspace_id();
    let artifacts = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let ids = store
                .list_ids::<ApprovalArtifact>("main")
                .map_err(|e| AppError::Internal(format!("list approval artifacts: {e}")))?;
            let mut out = Vec::new();
            for id in ids {
                let artifact = store
                    .read::<ApprovalArtifact>("main", id)
                    .map_err(|e| AppError::Internal(format!("read approval artifact {id}: {e}")))?;
                out.push(approval_to_response(&artifact));
            }
            out.sort_by(|a, b| b.created_at.cmp(&a.created_at));
            Ok::<_, AppError>(out)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;
    Ok(Json(artifacts))
}

#[utoipa::path(
    post,
    path = "/v1/intents/{intent_id}/bind-approval-artifact",
    tag = "execution",
    params(("intent_id" = IntentId, Path, description = "Intent ID")),
    request_body = BindApprovalArtifactRequest,
    responses(
        (status = 200, body = IntentResponse),
        (status = 404, description = "Intent or approval artifact not found"),
    ),
)]
async fn bind_approval_artifact_to_intent(
    RequireExecutionWrite(auth): RequireExecutionWrite,
    State(state): State<AppState>,
    Path(intent_id): Path<IntentId>,
    Json(req): Json<BindApprovalArtifactRequest>,
) -> Result<Json<IntentResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = req.entity_id;

    let intent = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            // Ensure the approval artifact exists before binding.
            store
                .read::<ApprovalArtifact>("main", req.approval_artifact_id)
                .map_err(|_| {
                    AppError::NotFound(format!(
                        "approval artifact {} not found",
                        req.approval_artifact_id
                    ))
                })?;

            let mut intent = store
                .read::<Intent>("main", intent_id)
                .map_err(|_| AppError::NotFound(format!("intent {} not found", intent_id)))?;
            intent.bind_approval_artifact(req.approval_artifact_id);
            let path = format!("execution/intents/{}.json", intent_id);
            store
                .write_json(
                    "main",
                    &path,
                    &intent,
                    &format!(
                        "EXECUTION: bind approval artifact {} to intent {}",
                        req.approval_artifact_id, intent_id
                    ),
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;
            Ok::<_, AppError>(intent)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(intent_to_response(&intent)))
}

#[utoipa::path(
    post,
    path = "/v1/intents/{intent_id}/bind-document-request",
    tag = "execution",
    params(("intent_id" = IntentId, Path, description = "Intent ID")),
    request_body = BindDocumentRequestRequest,
    responses(
        (status = 200, body = IntentResponse),
        (status = 404, description = "Intent or document request not found"),
        (status = 422, description = "Document request belongs to a different entity"),
    ),
)]
async fn bind_document_request_to_intent(
    RequireExecutionWrite(auth): RequireExecutionWrite,
    State(state): State<AppState>,
    Path(intent_id): Path<IntentId>,
    Json(req): Json<BindDocumentRequestRequest>,
) -> Result<Json<IntentResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = req.entity_id;

    let intent = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let request = store
                .read::<DocumentRequest>("main", req.request_id)
                .map_err(|_| {
                    AppError::NotFound(format!("document request {} not found", req.request_id))
                })?;

            if request.entity_id() != entity_id {
                return Err(AppError::UnprocessableEntity(format!(
                    "document request {} belongs to a different entity",
                    req.request_id
                )));
            }

            let mut intent = store
                .read::<Intent>("main", intent_id)
                .map_err(|_| AppError::NotFound(format!("intent {} not found", intent_id)))?;
            intent.bind_document_request(req.request_id);

            let path = format!("execution/intents/{}.json", intent_id);
            store
                .write_json(
                    "main",
                    &path,
                    &intent,
                    &format!(
                        "EXECUTION: bind document request {} to intent {}",
                        req.request_id, intent_id
                    ),
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;
            Ok::<_, AppError>(intent)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(intent_to_response(&intent)))
}

// ── Handlers: Obligations ────────────────────────────────────────────

#[utoipa::path(
    post,
    path = "/v1/execution/obligations",
    tag = "execution",
    request_body = CreateObligationRequest,
    responses(
        (status = 200, body = ObligationResponse),
        (status = 400, description = "Bad request"),
    ),
)]
async fn create_obligation(
    RequireExecutionWrite(auth): RequireExecutionWrite,
    State(state): State<AppState>,
    Json(req): Json<CreateObligationRequest>,
) -> Result<Json<ObligationResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = req.entity_id;

    let obligation = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;

            let obligation_id = ObligationId::new();
            let obligation = Obligation::new(
                obligation_id,
                entity_id,
                req.intent_id,
                ObligationType::new(req.obligation_type),
                req.assignee_type,
                req.assignee_id,
                req.description,
                req.due_date,
            );

            let path = format!("execution/obligations/{}.json", obligation_id);
            store
                .write_json(
                    "main",
                    &path,
                    &obligation,
                    &format!("Create obligation {obligation_id}"),
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

            Ok::<_, AppError>(obligation)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(obligation_to_response(&obligation)))
}

#[utoipa::path(
    get,
    path = "/v1/entities/{entity_id}/obligations",
    tag = "execution",
    params(("entity_id" = EntityId, Path, description = "Entity ID")),
    responses(
        (status = 200, body = Vec<ObligationResponse>),
        (status = 404, description = "Entity not found"),
    ),
)]
async fn list_obligations(
    RequireExecutionRead(auth): RequireExecutionRead,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
) -> Result<Json<Vec<ObligationResponse>>, AppError> {
    let workspace_id = auth.workspace_id();

    let obligations = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let ids = store
                .list_ids::<Obligation>("main")
                .map_err(|e| AppError::Internal(format!("list obligations: {e}")))?;

            let mut results = Vec::new();
            for id in ids {
                let o = store
                    .read::<Obligation>("main", id)
                    .map_err(|e| AppError::Internal(format!("read obligation {id}: {e}")))?;
                results.push(obligation_to_response(&o));
            }
            Ok::<_, AppError>(results)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(obligations))
}

#[utoipa::path(
    post,
    path = "/v1/obligations/{obligation_id}/fulfill",
    tag = "execution",
    params(
        ("obligation_id" = ObligationId, Path, description = "Obligation ID"),
        ("entity_id" = EntityId, Query, description = "Entity ID"),
    ),
    responses(
        (status = 200, body = ObligationResponse),
        (status = 404, description = "Obligation not found"),
    ),
)]
async fn fulfill_obligation(
    RequireExecutionWrite(auth): RequireExecutionWrite,
    State(state): State<AppState>,
    Path(obligation_id): Path<ObligationId>,
    Query(query): Query<super::EntityIdQuery>,
) -> Result<Json<ObligationResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = query.entity_id;

    let obligation = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let mut obligation = store
                .read::<Obligation>("main", obligation_id)
                .map_err(|_| {
                    AppError::NotFound(format!("obligation {} not found", obligation_id))
                })?;

            obligation.fulfill()?;

            let path = format!("execution/obligations/{}.json", obligation_id);
            store
                .write_json(
                    "main",
                    &path,
                    &obligation,
                    &format!("Fulfill obligation {obligation_id}"),
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

            Ok::<_, AppError>(obligation)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(obligation_to_response(&obligation)))
}

#[utoipa::path(
    post,
    path = "/v1/obligations/{obligation_id}/waive",
    tag = "execution",
    params(
        ("obligation_id" = ObligationId, Path, description = "Obligation ID"),
        ("entity_id" = EntityId, Query, description = "Entity ID"),
    ),
    responses(
        (status = 200, body = ObligationResponse),
        (status = 404, description = "Obligation not found"),
    ),
)]
async fn waive_obligation(
    RequireExecutionWrite(auth): RequireExecutionWrite,
    State(state): State<AppState>,
    Path(obligation_id): Path<ObligationId>,
    Query(query): Query<super::EntityIdQuery>,
) -> Result<Json<ObligationResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = query.entity_id;

    let obligation = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let mut obligation = store
                .read::<Obligation>("main", obligation_id)
                .map_err(|_| {
                    AppError::NotFound(format!("obligation {} not found", obligation_id))
                })?;

            obligation.waive()?;

            let path = format!("execution/obligations/{}.json", obligation_id);
            store
                .write_json(
                    "main",
                    &path,
                    &obligation,
                    &format!("Waive obligation {obligation_id}"),
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

            Ok::<_, AppError>(obligation)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(obligation_to_response(&obligation)))
}

#[utoipa::path(
    post,
    path = "/v1/obligations/{obligation_id}/expire",
    tag = "execution",
    params(
        ("obligation_id" = ObligationId, Path, description = "Obligation ID"),
        ("entity_id" = EntityId, Query, description = "Entity ID"),
    ),
    responses(
        (status = 200, body = ObligationResponse),
        (status = 404, description = "Obligation not found"),
    ),
)]
async fn expire_obligation(
    RequireExecutionWrite(auth): RequireExecutionWrite,
    State(state): State<AppState>,
    Path(obligation_id): Path<ObligationId>,
    Query(query): Query<super::EntityIdQuery>,
) -> Result<Json<ObligationResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = query.entity_id;

    let obligation = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let mut obligation = store
                .read::<Obligation>("main", obligation_id)
                .map_err(|_| {
                    AppError::NotFound(format!("obligation {} not found", obligation_id))
                })?;

            obligation.expire()?;

            let path = format!("execution/obligations/{}.json", obligation_id);
            store
                .write_json(
                    "main",
                    &path,
                    &obligation,
                    &format!("Expire obligation {obligation_id}"),
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

            Ok::<_, AppError>(obligation)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(obligation_to_response(&obligation)))
}

// ── Receipt handlers ────────────────────────────────────────────────

#[derive(Serialize, utoipa::ToSchema)]
pub struct ReceiptResponse {
    pub receipt_id: ReceiptId,
    pub intent_id: IntentId,
    pub idempotency_key: String,
    pub status: ReceiptStatus,
    pub request_hash: String,
    pub response_hash: Option<String>,
    pub executed_at: Option<String>,
    pub created_at: String,
}

fn receipt_to_response(r: &Receipt) -> ReceiptResponse {
    ReceiptResponse {
        receipt_id: r.receipt_id(),
        intent_id: r.intent_id(),
        idempotency_key: r.idempotency_key().to_owned(),
        status: r.status(),
        request_hash: r.request_hash().to_owned(),
        response_hash: r.response_hash().map(|s| s.to_owned()),
        executed_at: r.executed_at().map(|t| t.to_rfc3339()),
        created_at: r.created_at().to_rfc3339(),
    }
}

#[utoipa::path(
    get,
    path = "/v1/receipts/{receipt_id}",
    tag = "execution",
    params(
        ("receipt_id" = ReceiptId, Path, description = "Receipt ID"),
        ("entity_id" = EntityId, Query, description = "Entity ID"),
    ),
    responses(
        (status = 200, body = ReceiptResponse),
        (status = 404, description = "Receipt not found"),
    ),
)]
async fn get_receipt(
    RequireExecutionRead(auth): RequireExecutionRead,
    State(state): State<AppState>,
    Path(receipt_id): Path<ReceiptId>,
    Query(query): Query<super::EntityIdQuery>,
) -> Result<Json<ReceiptResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = query.entity_id;

    let receipt = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            store
                .read::<Receipt>("main", receipt_id)
                .map_err(|_| AppError::NotFound(format!("receipt {} not found", receipt_id)))
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(receipt_to_response(&receipt)))
}

#[utoipa::path(
    get,
    path = "/v1/intents/{intent_id}/receipts",
    tag = "execution",
    params(
        ("intent_id" = IntentId, Path, description = "Intent ID"),
        ("entity_id" = EntityId, Query, description = "Entity ID"),
    ),
    responses(
        (status = 200, body = Vec<ReceiptResponse>),
        (status = 404, description = "Entity not found"),
    ),
)]
async fn list_receipts_by_intent(
    RequireExecutionRead(auth): RequireExecutionRead,
    State(state): State<AppState>,
    Path(intent_id): Path<IntentId>,
    Query(query): Query<super::EntityIdQuery>,
) -> Result<Json<Vec<ReceiptResponse>>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = query.entity_id;

    let receipts = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let ids = store
                .list_ids::<Receipt>("main")
                .map_err(|e| AppError::Internal(format!("list receipts: {e}")))?;

            let mut results = Vec::new();
            for id in ids {
                if let Ok(r) = store.read::<Receipt>("main", id) {
                    if r.intent_id() == intent_id {
                        results.push(receipt_to_response(&r));
                    }
                }
            }
            Ok::<_, AppError>(results)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(receipts))
}

// ── Packet handlers ─────────────────────────────────────────────────

#[utoipa::path(
    get,
    path = "/v1/execution/packets/{packet_id}",
    tag = "execution",
    params(
        ("packet_id" = PacketId, Path, description = "Packet ID"),
        ("entity_id" = EntityId, Query, description = "Entity ID"),
    ),
    responses(
        (status = 200, body = TransactionPacketResponse),
        (status = 403, description = "Packet belongs to a different entity"),
        (status = 404, description = "Packet not found"),
    ),
)]
async fn get_packet(
    RequireExecutionRead(auth): RequireExecutionRead,
    State(state): State<AppState>,
    Path(packet_id): Path<PacketId>,
    Query(query): Query<super::EntityIdQuery>,
) -> Result<Json<TransactionPacketResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = query.entity_id;

    let packet = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let packet = store
                .read::<TransactionPacket>("main", packet_id)
                .map_err(|_| AppError::NotFound(format!("packet {} not found", packet_id)))?;
            if packet.entity_id() != entity_id {
                return Err(AppError::Forbidden(
                    "packet belongs to a different entity".to_owned(),
                ));
            }
            Ok::<_, AppError>(packet)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(packet_to_response(&packet)))
}

#[utoipa::path(
    get,
    path = "/v1/entities/{entity_id}/packets",
    tag = "execution",
    params(("entity_id" = EntityId, Path, description = "Entity ID")),
    responses(
        (status = 200, body = Vec<TransactionPacketResponse>),
        (status = 404, description = "Entity not found"),
    ),
)]
async fn list_entity_packets(
    RequireExecutionRead(auth): RequireExecutionRead,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
) -> Result<Json<Vec<TransactionPacketResponse>>, AppError> {
    let workspace_id = auth.workspace_id();

    let packets = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let ids = store
                .list_ids::<TransactionPacket>("main")
                .map_err(|e| AppError::Internal(format!("list packets: {e}")))?;
            let mut out = Vec::new();
            for id in ids {
                if let Ok(packet) = store.read::<TransactionPacket>("main", id)
                    && packet.entity_id() == entity_id
                {
                    out.push(packet_to_response(&packet));
                }
            }
            out.sort_by(|a, b| b.created_at.cmp(&a.created_at));
            Ok::<_, AppError>(out)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(packets))
}

// ── Obligation extended ─────────────────────────────────────────────

#[derive(Deserialize, utoipa::ToSchema)]
pub struct AssignObligationRequest {
    pub entity_id: EntityId,
    pub assignee_id: ContactId,
}

#[utoipa::path(
    post,
    path = "/v1/obligations/{obligation_id}/assign",
    tag = "execution",
    params(("obligation_id" = ObligationId, Path, description = "Obligation ID")),
    request_body = AssignObligationRequest,
    responses(
        (status = 200, body = ObligationResponse),
        (status = 404, description = "Obligation not found"),
    ),
)]
async fn assign_obligation(
    RequireExecutionWrite(auth): RequireExecutionWrite,
    State(state): State<AppState>,
    Path(obligation_id): Path<ObligationId>,
    Json(req): Json<AssignObligationRequest>,
) -> Result<Json<ObligationResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = req.entity_id;

    let obligation = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let mut obligation = store
                .read::<Obligation>("main", obligation_id)
                .map_err(|_| {
                    AppError::NotFound(format!("obligation {} not found", obligation_id))
                })?;

            obligation.assign(req.assignee_id)?;

            let path = format!("execution/obligations/{}.json", obligation_id);
            store
                .write_json(
                    "main",
                    &path,
                    &obligation,
                    &format!("Assign obligation {obligation_id}"),
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

            Ok::<_, AppError>(obligation)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(obligation_to_response(&obligation)))
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct ObligationsSummaryResponse {
    pub total: usize,
    pub pending: usize,
    pub fulfilled: usize,
    pub waived: usize,
    pub expired: usize,
}

#[utoipa::path(
    get,
    path = "/v1/entities/{entity_id}/obligations/summary",
    tag = "execution",
    params(("entity_id" = EntityId, Path, description = "Entity ID")),
    responses(
        (status = 200, body = ObligationsSummaryResponse),
        (status = 404, description = "Entity not found"),
    ),
)]
async fn obligations_summary(
    RequireExecutionRead(auth): RequireExecutionRead,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
) -> Result<Json<ObligationsSummaryResponse>, AppError> {
    let workspace_id = auth.workspace_id();

    let summary = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let ids = store
                .list_ids::<Obligation>("main")
                .map_err(|e| AppError::Internal(format!("list obligations: {e}")))?;

            let mut total = 0;
            let mut pending = 0;
            let mut fulfilled = 0;
            let mut waived = 0;
            let mut expired = 0;

            for id in ids {
                if let Ok(o) = store.read::<Obligation>("main", id) {
                    total += 1;
                    match o.status() {
                        ObligationStatus::Required | ObligationStatus::InProgress => pending += 1,
                        ObligationStatus::Fulfilled => fulfilled += 1,
                        ObligationStatus::Waived => waived += 1,
                        ObligationStatus::Expired => expired += 1,
                    }
                }
            }

            Ok::<_, AppError>(ObligationsSummaryResponse {
                total,
                pending,
                fulfilled,
                waived,
                expired,
            })
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(summary))
}

#[utoipa::path(
    get,
    path = "/v1/entities/{entity_id}/obligations/human",
    tag = "execution",
    params(("entity_id" = EntityId, Path, description = "Entity ID")),
    responses(
        (status = 200, body = Vec<ObligationResponse>),
        (status = 404, description = "Entity not found"),
    ),
)]
async fn list_human_obligations(
    RequireExecutionRead(auth): RequireExecutionRead,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
) -> Result<Json<Vec<ObligationResponse>>, AppError> {
    let workspace_id = auth.workspace_id();

    let obligations = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let ids = store
                .list_ids::<Obligation>("main")
                .map_err(|e| AppError::Internal(format!("list obligations: {e}")))?;

            let mut results = Vec::new();
            for id in ids {
                if let Ok(o) = store.read::<Obligation>("main", id) {
                    if o.assignee_type() == AssigneeType::Human {
                        results.push(obligation_to_response(&o));
                    }
                }
            }
            Ok::<_, AppError>(results)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(obligations))
}

// ── Handlers: Global human obligations ──────────────────────────────

#[utoipa::path(
    get,
    path = "/v1/human-obligations",
    tag = "execution",
    responses(
        (status = 200, body = Vec<ObligationResponse>),
    ),
)]
async fn list_global_human_obligations(
    RequireExecutionRead(auth): RequireExecutionRead,
    State(state): State<AppState>,
) -> Result<Json<Vec<ObligationResponse>>, AppError> {
    let workspace_id = auth.workspace_id();

    let obligations = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let entity_ids = layout.list_entity_ids(workspace_id);
            let mut results = Vec::new();

            for entity_id in entity_ids {
                if let Ok(store) = EntityStore::open(&layout, workspace_id, entity_id) {
                    if let Ok(ids) = store.list_ids::<Obligation>("main") {
                        for id in ids {
                            if let Ok(o) = store.read::<Obligation>("main", id) {
                                if o.assignee_type() == AssigneeType::Human
                                    && o.status() != ObligationStatus::Fulfilled
                                    && o.status() != ObligationStatus::Waived
                                {
                                    results.push(obligation_to_response(&o));
                                }
                            }
                        }
                    }
                }
            }
            Ok::<_, AppError>(results)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(obligations))
}

// ── Handlers: Signer token ──────────────────────────────────────────

#[derive(Serialize, utoipa::ToSchema)]
pub struct SignerTokenResponse {
    pub obligation_id: ObligationId,
    pub token: String,
    pub expires_at: String,
}

#[utoipa::path(
    post,
    path = "/v1/human-obligations/{obligation_id}/signer-token",
    tag = "execution",
    params(("obligation_id" = ObligationId, Path, description = "Obligation ID")),
    responses(
        (status = 200, body = SignerTokenResponse),
    ),
)]
async fn generate_signer_token(
    RequireExecutionWrite(_auth): RequireExecutionWrite,
    Path(obligation_id): Path<ObligationId>,
) -> Json<SignerTokenResponse> {
    let token = format!("signer_{}", uuid::Uuid::new_v4().simple());
    let expires_at = (chrono::Utc::now() + chrono::Duration::hours(24)).to_rfc3339();
    Json(SignerTokenResponse {
        obligation_id,
        token,
        expires_at,
    })
}

// ── Handlers: Human obligation fulfill ──────────────────────────────

#[utoipa::path(
    post,
    path = "/v1/human-obligations/{obligation_id}/fulfill",
    tag = "execution",
    params(
        ("obligation_id" = ObligationId, Path, description = "Obligation ID"),
        ("entity_id" = EntityId, Query, description = "Entity ID"),
    ),
    responses(
        (status = 200, body = ObligationResponse),
        (status = 404, description = "Obligation not found"),
    ),
)]
async fn fulfill_human_obligation(
    RequireExecutionWrite(auth): RequireExecutionWrite,
    State(state): State<AppState>,
    Path(obligation_id): Path<ObligationId>,
    Query(query): Query<super::EntityIdQuery>,
) -> Result<Json<ObligationResponse>, AppError> {
    fulfill_obligation(
        RequireExecutionWrite(auth),
        State(state),
        Path(obligation_id),
        Query(query),
    )
    .await
}

// ── Handlers: Document requests ─────────────────────────────────────

#[derive(Deserialize, utoipa::ToSchema)]
pub struct CreateDocumentRequestPayload {
    pub description: String,
    pub document_type: String,
    pub entity_id: EntityId,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct DocumentRequestResponse {
    pub request_id: DocumentRequestId,
    pub obligation_id: ObligationId,
    pub entity_id: EntityId,
    pub description: String,
    pub document_type: String,
    pub status: DocumentRequestStatus,
    pub fulfilled_at: Option<String>,
    pub not_applicable_at: Option<String>,
    pub created_at: String,
}

fn document_request_to_response(r: &DocumentRequest) -> DocumentRequestResponse {
    DocumentRequestResponse {
        request_id: r.request_id(),
        obligation_id: r.obligation_id(),
        entity_id: r.entity_id(),
        description: r.description().to_owned(),
        document_type: r.document_type().to_owned(),
        status: r.status(),
        fulfilled_at: r.fulfilled_at().map(|t| t.to_rfc3339()),
        not_applicable_at: r.not_applicable_at().map(|t| t.to_rfc3339()),
        created_at: r.created_at().to_rfc3339(),
    }
}

#[utoipa::path(
    post,
    path = "/v1/obligations/{obligation_id}/document-requests",
    tag = "execution",
    params(("obligation_id" = ObligationId, Path, description = "Obligation ID")),
    request_body = CreateDocumentRequestPayload,
    responses(
        (status = 200, body = DocumentRequestResponse),
        (status = 404, description = "Obligation not found"),
    ),
)]
async fn create_document_request(
    RequireExecutionWrite(auth): RequireExecutionWrite,
    State(state): State<AppState>,
    Path(obligation_id): Path<ObligationId>,
    Json(req): Json<CreateDocumentRequestPayload>,
) -> Result<Json<DocumentRequestResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = req.entity_id;
    let request_id = DocumentRequestId::new();

    let request = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;

            // Verify obligation exists
            store
                .read::<Obligation>("main", obligation_id)
                .map_err(|_| {
                    AppError::NotFound(format!("obligation {} not found", obligation_id))
                })?;

            let request = DocumentRequest::new(
                request_id,
                entity_id,
                obligation_id,
                req.description,
                req.document_type,
            );

            store
                .write_json(
                    "main",
                    &format!("execution/document-requests/{}.json", request_id),
                    &request,
                    &format!("EXECUTION: create document request {request_id}"),
                )
                .map_err(|e| AppError::Internal(format!("commit: {e}")))?;

            Ok::<_, AppError>(request)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(document_request_to_response(&request)))
}

#[utoipa::path(
    get,
    path = "/v1/obligations/{obligation_id}/document-requests",
    tag = "execution",
    params(
        ("obligation_id" = ObligationId, Path, description = "Obligation ID"),
        ("entity_id" = EntityId, Query, description = "Entity ID"),
    ),
    responses(
        (status = 200, body = Vec<DocumentRequestResponse>),
        (status = 404, description = "Obligation not found"),
    ),
)]
async fn list_document_requests(
    RequireExecutionRead(auth): RequireExecutionRead,
    State(state): State<AppState>,
    Path(obligation_id): Path<ObligationId>,
    Query(query): Query<super::EntityIdQuery>,
) -> Result<Json<Vec<DocumentRequestResponse>>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = query.entity_id;

    let requests = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;

            let ids = store
                .list_ids::<DocumentRequest>("main")
                .map_err(|e| AppError::Internal(format!("list document requests: {e}")))?;

            let mut results = Vec::new();
            for id in ids {
                if let Ok(request) = store.read::<DocumentRequest>("main", id)
                    && request.obligation_id() == obligation_id
                {
                    results.push(document_request_to_response(&request));
                }
            }
            Ok::<_, AppError>(results)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(requests))
}

#[utoipa::path(
    patch,
    path = "/v1/document-requests/{request_id}/fulfill",
    tag = "execution",
    params(
        ("request_id" = DocumentRequestId, Path, description = "Document request ID"),
        ("entity_id" = EntityId, Query, description = "Entity ID"),
    ),
    responses(
        (status = 200, body = DocumentRequestResponse),
        (status = 404, description = "Document request not found"),
    ),
)]
async fn fulfill_document_request(
    RequireExecutionWrite(auth): RequireExecutionWrite,
    State(state): State<AppState>,
    Path(request_id): Path<DocumentRequestId>,
    Query(query): Query<super::EntityIdQuery>,
) -> Result<Json<DocumentRequestResponse>, AppError> {
    update_document_request_status(
        state,
        request_id,
        auth.workspace_id(),
        query.entity_id,
        DocumentRequestStatus::Provided,
    )
    .await
}

#[utoipa::path(
    patch,
    path = "/v1/document-requests/{request_id}/not-applicable",
    tag = "execution",
    params(
        ("request_id" = DocumentRequestId, Path, description = "Document request ID"),
        ("entity_id" = EntityId, Query, description = "Entity ID"),
    ),
    responses(
        (status = 200, body = DocumentRequestResponse),
        (status = 404, description = "Document request not found"),
    ),
)]
async fn mark_document_request_na(
    RequireExecutionWrite(auth): RequireExecutionWrite,
    State(state): State<AppState>,
    Path(request_id): Path<DocumentRequestId>,
    Query(query): Query<super::EntityIdQuery>,
) -> Result<Json<DocumentRequestResponse>, AppError> {
    update_document_request_status(
        state,
        request_id,
        auth.workspace_id(),
        query.entity_id,
        DocumentRequestStatus::NotApplicable,
    )
    .await
}

async fn update_document_request_status(
    state: AppState,
    request_id: DocumentRequestId,
    workspace_id: WorkspaceId,
    entity_id: EntityId,
    new_status: DocumentRequestStatus,
) -> Result<Json<DocumentRequestResponse>, AppError> {
    let result = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let path = format!("execution/document-requests/{}.json", request_id);
            let mut request: DocumentRequest = store.read_json("main", &path).map_err(|_| {
                AppError::NotFound(format!("document request {} not found", request_id))
            })?;

            match new_status {
                DocumentRequestStatus::Provided => request.fulfill()?,
                DocumentRequestStatus::NotApplicable => request.mark_not_applicable()?,
                DocumentRequestStatus::Waived => request.waive()?,
                DocumentRequestStatus::Requested => {
                    return Err(AppError::BadRequest(
                        "cannot transition document request back to requested".to_owned(),
                    ));
                }
            }

            store
                .write_json(
                    "main",
                    &path,
                    &request,
                    &format!("EXECUTION: update document request {request_id} to {new_status:?}"),
                )
                .map_err(|e| AppError::Internal(format!("commit: {e}")))?;

            Ok::<_, AppError>(request)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(document_request_to_response(&result)))
}

// ── Handlers: Global obligations summary ────────────────────────────

#[utoipa::path(
    get,
    path = "/v1/obligations/summary",
    tag = "execution",
    responses(
        (status = 200, body = ObligationsSummaryResponse),
    ),
)]
async fn global_obligations_summary(
    RequireExecutionRead(auth): RequireExecutionRead,
    State(state): State<AppState>,
) -> Result<Json<ObligationsSummaryResponse>, AppError> {
    let workspace_id = auth.workspace_id();

    let summary = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let entity_ids = layout.list_entity_ids(workspace_id);
            let mut total = 0;
            let mut pending = 0;
            let mut fulfilled = 0;
            let mut waived = 0;
            let mut expired = 0;

            for entity_id in entity_ids {
                if let Ok(store) = EntityStore::open(&layout, workspace_id, entity_id) {
                    if let Ok(ids) = store.list_ids::<Obligation>("main") {
                        for id in ids {
                            if let Ok(o) = store.read::<Obligation>("main", id) {
                                total += 1;
                                match o.status() {
                                    ObligationStatus::Required | ObligationStatus::InProgress => {
                                        pending += 1
                                    }
                                    ObligationStatus::Fulfilled => fulfilled += 1,
                                    ObligationStatus::Waived => waived += 1,
                                    ObligationStatus::Expired => expired += 1,
                                }
                            }
                        }
                    }
                }
            }

            Ok::<_, AppError>(ObligationsSummaryResponse {
                total,
                pending,
                fulfilled,
                waived,
                expired,
            })
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(summary))
}

// ── Router ───────────────────────────────────────────────────────────

pub fn execution_routes() -> Router<AppState> {
    Router::new()
        // Intents
        .route("/v1/execution/intents", post(create_intent))
        .route("/v1/entities/{entity_id}/intents", get(list_intents))
        .route("/v1/intents/{intent_id}/evaluate", post(evaluate_intent))
        .route("/v1/intents/{intent_id}/authorize", post(authorize_intent))
        .route("/v1/intents/{intent_id}/execute", post(execute_intent))
        .route("/v1/intents/{intent_id}/cancel", post(cancel_intent))
        .route(
            "/v1/intents/{intent_id}/bind-approval-artifact",
            post(bind_approval_artifact_to_intent),
        )
        .route(
            "/v1/intents/{intent_id}/bind-document-request",
            post(bind_document_request_to_intent),
        )
        .route(
            "/v1/execution/approval-artifacts",
            post(create_approval_artifact),
        )
        .route(
            "/v1/entities/{entity_id}/approval-artifacts",
            get(list_approval_artifacts),
        )
        // Obligations
        .route("/v1/execution/obligations", post(create_obligation))
        .route(
            "/v1/entities/{entity_id}/obligations",
            get(list_obligations),
        )
        .route(
            "/v1/obligations/{obligation_id}/fulfill",
            post(fulfill_obligation),
        )
        .route(
            "/v1/obligations/{obligation_id}/waive",
            post(waive_obligation),
        )
        .route(
            "/v1/obligations/{obligation_id}/expire",
            post(expire_obligation),
        )
        .route(
            "/v1/obligations/{obligation_id}/assign",
            post(assign_obligation),
        )
        .route(
            "/v1/entities/{entity_id}/obligations/summary",
            get(obligations_summary),
        )
        // Receipts
        .route("/v1/receipts/{receipt_id}", get(get_receipt))
        .route(
            "/v1/intents/{intent_id}/receipts",
            get(list_receipts_by_intent),
        )
        .route("/v1/execution/packets/{packet_id}", get(get_packet))
        .route("/v1/entities/{entity_id}/packets", get(list_entity_packets))
        // Human obligations
        .route(
            "/v1/entities/{entity_id}/obligations/human",
            get(list_human_obligations),
        )
        .route("/v1/human-obligations", get(list_global_human_obligations))
        .route(
            "/v1/human-obligations/{obligation_id}/signer-token",
            post(generate_signer_token),
        )
        .route(
            "/v1/human-obligations/{obligation_id}/fulfill",
            post(fulfill_human_obligation),
        )
        // Document requests
        .route(
            "/v1/obligations/{obligation_id}/document-requests",
            post(create_document_request).get(list_document_requests),
        )
        .route(
            "/v1/document-requests/{request_id}/fulfill",
            patch(fulfill_document_request),
        )
        .route(
            "/v1/document-requests/{request_id}/not-applicable",
            patch(mark_document_request_na),
        )
        // Global obligations summary
        .route("/v1/obligations/summary", get(global_obligations_summary))
}

#[derive(utoipa::OpenApi)]
#[openapi(
    paths(
        create_intent,
        list_intents,
        evaluate_intent,
        authorize_intent,
        execute_intent,
        cancel_intent,
        bind_approval_artifact_to_intent,
        bind_document_request_to_intent,
        create_approval_artifact,
        list_approval_artifacts,
        create_obligation,
        list_obligations,
        fulfill_obligation,
        waive_obligation,
        expire_obligation,
        assign_obligation,
        obligations_summary,
        list_human_obligations,
        list_global_human_obligations,
        generate_signer_token,
        fulfill_human_obligation,
        create_document_request,
        list_document_requests,
        fulfill_document_request,
        mark_document_request_na,
        global_obligations_summary,
        get_receipt,
        list_receipts_by_intent,
        get_packet,
        list_entity_packets,
    ),
    components(schemas(
        CreateIntentRequest,
        CreateObligationRequest,
        CreateApprovalArtifactRequest,
        BindApprovalArtifactRequest,
        BindDocumentRequestRequest,
        AssignObligationRequest,
        CreateDocumentRequestPayload,
        IntentResponse,
        ObligationResponse,
        ApprovalArtifactResponse,
        TransactionPacketResponse,
        PacketSignatureResponse,
        ReceiptResponse,
        ObligationsSummaryResponse,
        SignerTokenResponse,
        DocumentRequestResponse,
        IntentStatus,
        ObligationStatus,
        ObligationType,
        AssigneeType,
        AuthorityTier,
        ReceiptStatus,
        DocumentRequestStatus,
        PolicyDecision,
        WorkflowType,
        TransactionPacketStatus,
        PacketItem,
    )),
    tags((name = "execution", description = "Execution intents, obligations, and receipts")),
)]
pub struct ExecutionApi;
