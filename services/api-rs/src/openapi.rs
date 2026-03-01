//! OpenAPI specification generation.
//!
//! Builds an OpenAPI 3.1 JSON spec from route metadata.
//! Served at `GET /v1/openapi.json`.

use serde_json::{Map, Value, json};

/// Build the complete OpenAPI spec as a JSON value.
pub fn openapi_spec() -> Value {
    let mut paths = Map::new();

    add_formation_paths(&mut paths);
    add_equity_paths(&mut paths);
    add_governance_paths(&mut paths);
    add_treasury_paths(&mut paths);
    add_contacts_paths(&mut paths);
    add_execution_paths(&mut paths);
    add_branches_paths(&mut paths);
    add_auth_paths(&mut paths);
    add_agents_paths(&mut paths);
    add_compliance_paths(&mut paths);
    add_billing_paths(&mut paths);
    add_services_paths(&mut paths);
    add_admin_paths(&mut paths);
    add_webhooks_paths(&mut paths);

    json!({
        "openapi": "3.1.0",
        "info": {
            "title": "The Corporation API",
            "version": "1.0.0",
            "description": "Git-backed corporate operations platform API"
        },
        "paths": Value::Object(paths),
        "tags": [
            { "name": "formation", "description": "Entity formation and document management" },
            { "name": "equity", "description": "Canonical cap table, instruments, rounds, and conversions" },
            { "name": "governance", "description": "Bodies, seats, meetings, and votes" },
            { "name": "treasury", "description": "Accounts, journal entries, invoices, and banking" },
            { "name": "contacts", "description": "Contact management" },
            { "name": "execution", "description": "Intents, obligations, and receipts" },
            { "name": "branches", "description": "Git branch management" },
            { "name": "auth", "description": "Authentication and API keys" },
            { "name": "agents", "description": "Agent management" },
            { "name": "compliance", "description": "Tax filings and deadlines" },
            { "name": "billing", "description": "Billing and subscriptions" },
            { "name": "services", "description": "Fulfillment service catalog and requests" },
            { "name": "admin", "description": "Administration and system health" }
        ]
    })
}

fn op(tag: &str, operation_id: &str, summary: &str) -> Value {
    json!({
        "tags": [tag],
        "operationId": operation_id,
        "summary": summary,
        "responses": {
            "200": { "description": "Successful response" },
            "400": { "description": "Bad request" },
            "404": { "description": "Not found" },
            "500": { "description": "Internal server error" }
        }
    })
}

fn add_path(paths: &mut Map<String, Value>, path: &str, method: &str, val: Value) {
    let entry = paths
        .entry(path.to_owned())
        .or_insert_with(|| Value::Object(Map::new()));
    if let Value::Object(m) = entry {
        m.insert(method.to_owned(), val);
    }
}

fn add_formation_paths(paths: &mut Map<String, Value>) {
    add_path(
        paths,
        "/v1/formations",
        "post",
        op(
            "formation",
            "create_formation",
            "Create a new entity formation",
        ),
    );
    add_path(
        paths,
        "/v1/formations/{entity_id}",
        "get",
        op("formation", "get_formation", "Get entity formation details"),
    );
    add_path(
        paths,
        "/v1/formations/{entity_id}/documents",
        "get",
        op("formation", "list_documents", "List formation documents"),
    );
    add_path(
        paths,
        "/v1/formations/{entity_id}/mark-documents-signed",
        "post",
        op(
            "formation",
            "mark_documents_signed",
            "Mark formation documents as fully signed",
        ),
    );
    add_path(
        paths,
        "/v1/formations/{entity_id}/filing-attestation",
        "post",
        op(
            "formation",
            "record_filing_attestation",
            "Record filing attestation by designated natural-person signer",
        ),
    );
    add_path(
        paths,
        "/v1/formations/{entity_id}/registered-agent-consent-evidence",
        "post",
        op(
            "formation",
            "add_registered_agent_consent_evidence",
            "Attach registered-agent consent evidence to filing",
        ),
    );
    add_path(
        paths,
        "/v1/formations/{entity_id}/service-agreement/execute",
        "post",
        op(
            "formation",
            "execute_service_agreement",
            "Record executed service agreement for autonomy precondition",
        ),
    );
    add_path(
        paths,
        "/v1/formations/{entity_id}/gates",
        "get",
        op(
            "formation",
            "get_formation_gates",
            "Get filing and service-agreement gate status",
        ),
    );
    add_path(
        paths,
        "/v1/formations/{entity_id}/submit-filing",
        "post",
        op(
            "formation",
            "submit_filing",
            "Submit entity filing to state",
        ),
    );
    add_path(
        paths,
        "/v1/formations/{entity_id}/filing-confirmation",
        "post",
        op("formation", "confirm_filing", "Confirm entity filing"),
    );
    add_path(
        paths,
        "/v1/formations/{entity_id}/apply-ein",
        "post",
        op("formation", "apply_ein", "Submit EIN application"),
    );
    add_path(
        paths,
        "/v1/formations/{entity_id}/ein-confirmation",
        "post",
        op("formation", "confirm_ein", "Confirm EIN assignment"),
    );
    add_path(
        paths,
        "/v1/documents/{document_id}",
        "get",
        op("formation", "get_document", "Get a document"),
    );
    add_path(
        paths,
        "/v1/documents/{document_id}/sign",
        "post",
        op("formation", "sign_document", "Sign a document"),
    );
    add_path(
        paths,
        "/v1/documents/{document_id}/pdf",
        "get",
        op("formation", "get_document_pdf", "Get document PDF"),
    );
    add_path(
        paths,
        "/v1/documents/{document_id}/request-copy",
        "post",
        op(
            "formation",
            "request_document_copy",
            "Request document copy",
        ),
    );
    add_path(
        paths,
        "/v1/documents/{document_id}/amendment-history",
        "get",
        op(
            "formation",
            "get_amendment_history",
            "Get amendment history",
        ),
    );
    add_path(
        paths,
        "/v1/contracts",
        "post",
        op("formation", "generate_contract", "Generate a contract"),
    );
    add_path(
        paths,
        "/v1/sign/{document_id}",
        "get",
        op("formation", "get_signing_link", "Get signing link"),
    );
    add_path(
        paths,
        "/v1/entities",
        "get",
        op("formation", "list_entities", "List entities"),
    );
    add_path(
        paths,
        "/v1/entities/{entity_id}/convert",
        "post",
        op("formation", "convert_entity", "Convert entity type"),
    );
    add_path(
        paths,
        "/v1/entities/{entity_id}/dissolve",
        "post",
        op("formation", "dissolve_entity", "Dissolve entity"),
    );
    add_path(
        paths,
        "/v1/entities/{entity_id}/governance-documents",
        "get",
        op(
            "formation",
            "list_governance_documents",
            "List governance documents",
        ),
    );
    add_path(
        paths,
        "/v1/entities/{entity_id}/governance-documents/current",
        "get",
        op(
            "formation",
            "get_current_governance_document",
            "Get current governance document",
        ),
    );
}

fn add_equity_paths(paths: &mut Map<String, Value>) {
    add_path(
        paths,
        "/v1/entities/{entity_id}/cap-table",
        "get",
        op("equity", "get_cap_table", "Get cap table"),
    );
    add_path(
        paths,
        "/v1/equity/holders",
        "post",
        op("equity", "create_holder", "Create holder"),
    );
    add_path(
        paths,
        "/v1/equity/entities",
        "post",
        op("equity", "create_legal_entity", "Create legal entity"),
    );
    add_path(
        paths,
        "/v1/equity/control-links",
        "post",
        op("equity", "create_control_link", "Create control link"),
    );
    add_path(
        paths,
        "/v1/equity/instruments",
        "post",
        op("equity", "create_instrument", "Create instrument"),
    );
    add_path(
        paths,
        "/v1/equity/positions/adjust",
        "post",
        op("equity", "adjust_position", "Adjust position"),
    );
    add_path(
        paths,
        "/v1/equity/rounds",
        "post",
        op("equity", "create_round", "Create round"),
    );
    add_path(
        paths,
        "/v1/equity/rounds/{round_id}/apply-terms",
        "post",
        op("equity", "apply_round_terms", "Apply round terms"),
    );
    add_path(
        paths,
        "/v1/equity/rounds/{round_id}/board-approve",
        "post",
        op(
            "equity",
            "board_approve_round",
            "Record board approval for round",
        ),
    );
    add_path(
        paths,
        "/v1/equity/rounds/{round_id}/accept",
        "post",
        op(
            "equity",
            "accept_round",
            "Accept round after board approval",
        ),
    );
    add_path(
        paths,
        "/v1/equity/transfer-workflows",
        "post",
        op(
            "equity",
            "create_transfer_workflow",
            "Create transfer workflow",
        ),
    );
    add_path(
        paths,
        "/v1/equity/transfer-workflows/{workflow_id}",
        "get",
        op("equity", "get_transfer_workflow", "Get transfer workflow"),
    );
    add_path(
        paths,
        "/v1/equity/transfer-workflows/{workflow_id}/generate-docs",
        "post",
        op(
            "equity",
            "generate_transfer_workflow_docs",
            "Generate transfer workflow docs",
        ),
    );
    add_path(
        paths,
        "/v1/equity/transfer-workflows/{workflow_id}/submit-review",
        "post",
        op(
            "equity",
            "submit_transfer_workflow_for_review",
            "Submit transfer workflow for review",
        ),
    );
    add_path(
        paths,
        "/v1/equity/transfer-workflows/{workflow_id}/record-review",
        "post",
        op(
            "equity",
            "record_transfer_workflow_review",
            "Record transfer workflow review",
        ),
    );
    add_path(
        paths,
        "/v1/equity/transfer-workflows/{workflow_id}/record-rofr",
        "post",
        op(
            "equity",
            "record_transfer_workflow_rofr",
            "Record transfer workflow ROFR",
        ),
    );
    add_path(
        paths,
        "/v1/equity/transfer-workflows/{workflow_id}/record-board-approval",
        "post",
        op(
            "equity",
            "record_transfer_workflow_board_approval",
            "Record transfer workflow board approval",
        ),
    );
    add_path(
        paths,
        "/v1/equity/transfer-workflows/{workflow_id}/record-execution",
        "post",
        op(
            "equity",
            "record_transfer_workflow_execution",
            "Record transfer workflow execution",
        ),
    );
    add_path(
        paths,
        "/v1/equity/transfer-workflows/{workflow_id}/prepare-execution",
        "post",
        op(
            "equity",
            "prepare_transfer_workflow_execution",
            "Bind execution prerequisites for transfer workflow",
        ),
    );
    add_path(
        paths,
        "/v1/equity/transfer-workflows/{workflow_id}/compile-packet",
        "post",
        op(
            "equity",
            "compile_transfer_workflow_packet",
            "Compile transfer transaction packet",
        ),
    );
    add_path(
        paths,
        "/v1/equity/transfer-workflows/{workflow_id}/start-signatures",
        "post",
        op(
            "equity",
            "start_transfer_workflow_signatures",
            "Start transfer workflow signature collection",
        ),
    );
    add_path(
        paths,
        "/v1/equity/transfer-workflows/{workflow_id}/record-signature",
        "post",
        op(
            "equity",
            "record_transfer_workflow_signature",
            "Record transfer workflow signature",
        ),
    );
    add_path(
        paths,
        "/v1/equity/transfer-workflows/{workflow_id}/finalize",
        "post",
        op(
            "equity",
            "finalize_transfer_workflow",
            "Finalize transfer workflow execution",
        ),
    );
    add_path(
        paths,
        "/v1/equity/fundraising-workflows",
        "post",
        op(
            "equity",
            "create_fundraising_workflow",
            "Create fundraising workflow",
        ),
    );
    add_path(
        paths,
        "/v1/equity/fundraising-workflows/{workflow_id}",
        "get",
        op(
            "equity",
            "get_fundraising_workflow",
            "Get fundraising workflow",
        ),
    );
    add_path(
        paths,
        "/v1/equity/fundraising-workflows/{workflow_id}/apply-terms",
        "post",
        op(
            "equity",
            "apply_fundraising_workflow_terms",
            "Apply fundraising workflow terms",
        ),
    );
    add_path(
        paths,
        "/v1/equity/fundraising-workflows/{workflow_id}/generate-board-packet",
        "post",
        op(
            "equity",
            "generate_fundraising_board_packet",
            "Generate fundraising board packet",
        ),
    );
    add_path(
        paths,
        "/v1/equity/fundraising-workflows/{workflow_id}/record-board-approval",
        "post",
        op(
            "equity",
            "record_fundraising_workflow_board_approval",
            "Record fundraising workflow board approval",
        ),
    );
    add_path(
        paths,
        "/v1/equity/fundraising-workflows/{workflow_id}/record-investor-acceptance",
        "post",
        op(
            "equity",
            "record_fundraising_workflow_acceptance",
            "Record fundraising workflow investor acceptance",
        ),
    );
    add_path(
        paths,
        "/v1/equity/fundraising-workflows/{workflow_id}/generate-closing-packet",
        "post",
        op(
            "equity",
            "generate_fundraising_closing_packet",
            "Generate fundraising closing packet",
        ),
    );
    add_path(
        paths,
        "/v1/equity/fundraising-workflows/{workflow_id}/record-close",
        "post",
        op(
            "equity",
            "record_fundraising_workflow_close",
            "Record fundraising workflow close",
        ),
    );
    add_path(
        paths,
        "/v1/equity/fundraising-workflows/{workflow_id}/prepare-execution",
        "post",
        op(
            "equity",
            "prepare_fundraising_workflow_execution",
            "Bind execution prerequisites for fundraising workflow",
        ),
    );
    add_path(
        paths,
        "/v1/equity/fundraising-workflows/{workflow_id}/compile-packet",
        "post",
        op(
            "equity",
            "compile_fundraising_workflow_packet",
            "Compile fundraising transaction packet",
        ),
    );
    add_path(
        paths,
        "/v1/equity/fundraising-workflows/{workflow_id}/start-signatures",
        "post",
        op(
            "equity",
            "start_fundraising_workflow_signatures",
            "Start fundraising workflow signature collection",
        ),
    );
    add_path(
        paths,
        "/v1/equity/fundraising-workflows/{workflow_id}/record-signature",
        "post",
        op(
            "equity",
            "record_fundraising_workflow_signature",
            "Record fundraising workflow signature",
        ),
    );
    add_path(
        paths,
        "/v1/equity/fundraising-workflows/{workflow_id}/finalize",
        "post",
        op(
            "equity",
            "finalize_fundraising_workflow",
            "Finalize fundraising workflow execution",
        ),
    );
    add_path(
        paths,
        "/v1/equity/workflows/{workflow_type}/{workflow_id}/status",
        "get",
        op(
            "equity",
            "get_workflow_status",
            "Get workflow orchestration status",
        ),
    );
    add_path(
        paths,
        "/v1/equity/conversions/preview",
        "post",
        op("equity", "preview_conversion", "Preview conversion"),
    );
    add_path(
        paths,
        "/v1/equity/conversions/execute",
        "post",
        json!({
            "tags": ["equity"],
            "operationId": "execute_conversion",
            "summary": "Execute conversion (requires accepted round + authorized execute intent)",
            "description": "Requires `intent_id` for an authorized `equity.round.execute_conversion` intent whose metadata round_id matches the request round_id. Returns 422 if the round has not been accepted.",
            "requestBody": {
                "required": true,
                "content": {
                    "application/json": {
                        "schema": {
                            "type": "object",
                            "required": ["entity_id", "round_id", "intent_id"],
                            "properties": {
                                "entity_id": { "type": "string", "format": "uuid" },
                                "round_id": { "type": "string", "format": "uuid" },
                                "intent_id": { "type": "string", "format": "uuid" },
                                "source_reference": { "type": "string" }
                            }
                        }
                    }
                }
            },
            "responses": {
                "200": { "description": "Successful response" },
                "400": { "description": "Bad request" },
                "404": { "description": "Not found" },
                "422": { "description": "Round/intent validation failed" },
                "500": { "description": "Internal server error" }
            }
        }),
    );
    add_path(
        paths,
        "/v1/equity/control-map",
        "get",
        op("equity", "get_control_map", "Get control map"),
    );
    add_path(
        paths,
        "/v1/equity/dilution/preview",
        "get",
        op("equity", "get_dilution_preview", "Get dilution preview"),
    );
}

fn add_governance_paths(paths: &mut Map<String, Value>) {
    add_path(
        paths,
        "/v1/entities/{entity_id}/governance/profile",
        "get",
        op(
            "governance",
            "get_governance_profile",
            "Get governance profile",
        ),
    );
    add_path(
        paths,
        "/v1/entities/{entity_id}/governance/profile",
        "put",
        op(
            "governance",
            "update_governance_profile",
            "Update governance profile",
        ),
    );
    add_path(
        paths,
        "/v1/entities/{entity_id}/governance/doc-bundles/generate",
        "post",
        op(
            "governance",
            "generate_governance_doc_bundle",
            "Generate governance doc bundle",
        ),
    );
    add_path(
        paths,
        "/v1/entities/{entity_id}/governance/doc-bundles/current",
        "get",
        op(
            "governance",
            "get_current_governance_doc_bundle",
            "Get current governance doc bundle",
        ),
    );
    add_path(
        paths,
        "/v1/entities/{entity_id}/governance/doc-bundles",
        "get",
        op(
            "governance",
            "list_governance_doc_bundles",
            "List governance doc bundles",
        ),
    );
    add_path(
        paths,
        "/v1/entities/{entity_id}/governance/doc-bundles/{bundle_id}",
        "get",
        op(
            "governance",
            "get_governance_doc_bundle",
            "Get governance doc bundle manifest",
        ),
    );
    add_path(
        paths,
        "/v1/entities/{entity_id}/governance/triggers",
        "get",
        op(
            "governance",
            "list_governance_triggers",
            "List governance triggers",
        ),
    );
    add_path(
        paths,
        "/v1/entities/{entity_id}/governance/mode-history",
        "get",
        op(
            "governance",
            "list_governance_mode_history",
            "List governance mode history",
        ),
    );
    add_path(
        paths,
        "/v1/entities/{entity_id}/governance/audit/entries",
        "get",
        op(
            "governance",
            "list_governance_audit_entries",
            "List governance audit entries",
        ),
    );
    add_path(
        paths,
        "/v1/governance/audit/events",
        "post",
        op(
            "governance",
            "create_governance_audit_event",
            "Append governance audit entry",
        ),
    );
    add_path(
        paths,
        "/v1/governance/audit/checkpoints",
        "post",
        op(
            "governance",
            "write_governance_audit_checkpoint",
            "Write governance audit checkpoint",
        ),
    );
    add_path(
        paths,
        "/v1/entities/{entity_id}/governance/audit/checkpoints",
        "get",
        op(
            "governance",
            "list_governance_audit_checkpoints",
            "List governance audit checkpoints",
        ),
    );
    add_path(
        paths,
        "/v1/governance/audit/verify",
        "post",
        op(
            "governance",
            "verify_governance_audit_chain",
            "Verify governance audit chain",
        ),
    );
    add_path(
        paths,
        "/v1/entities/{entity_id}/governance/audit/verifications",
        "get",
        op(
            "governance",
            "list_governance_audit_verifications",
            "List governance audit verifications",
        ),
    );
    add_path(
        paths,
        "/v1/internal/workspaces/{workspace_id}/entities/{entity_id}/governance/triggers/lockdown",
        "post",
        op(
            "governance",
            "ingest_lockdown_trigger",
            "Ingest lockdown trigger (internal)",
        ),
    );

    add_path(
        paths,
        "/v1/governance/mode",
        "get",
        op("governance", "get_governance_mode", "Get governance mode"),
    );
    add_path(
        paths,
        "/v1/governance/mode",
        "post",
        op("governance", "set_governance_mode", "Set governance mode"),
    );
    add_path(
        paths,
        "/v1/governance/incidents",
        "post",
        op(
            "governance",
            "create_incident",
            "Create governance incident",
        ),
    );
    add_path(
        paths,
        "/v1/entities/{entity_id}/governance/incidents",
        "get",
        op("governance", "list_incidents", "List governance incidents"),
    );
    add_path(
        paths,
        "/v1/governance/incidents/{incident_id}/resolve",
        "post",
        op(
            "governance",
            "resolve_incident",
            "Resolve governance incident",
        ),
    );
    add_path(
        paths,
        "/v1/governance/delegation-schedule",
        "get",
        op(
            "governance",
            "get_delegation_schedule",
            "Get current delegation schedule",
        ),
    );
    add_path(
        paths,
        "/v1/governance/delegation-schedule/amend",
        "post",
        op(
            "governance",
            "amend_delegation_schedule",
            "Amend delegation schedule",
        ),
    );
    add_path(
        paths,
        "/v1/governance/delegation-schedule/reauthorize",
        "post",
        op(
            "governance",
            "reauthorize_delegation_schedule",
            "Reauthorize delegation schedule",
        ),
    );
    add_path(
        paths,
        "/v1/governance/delegation-schedule/history",
        "get",
        op(
            "governance",
            "list_delegation_schedule_history",
            "List delegation schedule amendments",
        ),
    );

    add_path(
        paths,
        "/v1/governance-bodies",
        "post",
        op(
            "governance",
            "create_governance_body",
            "Create governing body",
        ),
    );
    add_path(
        paths,
        "/v1/governance-bodies",
        "get",
        op(
            "governance",
            "list_all_governance_bodies",
            "List all governance bodies",
        ),
    );
    add_path(
        paths,
        "/v1/entities/{entity_id}/governance-bodies",
        "get",
        op(
            "governance",
            "list_governance_bodies",
            "List entity governance bodies",
        ),
    );
    add_path(
        paths,
        "/v1/governance-bodies/{body_id}/seats",
        "post",
        op("governance", "create_seat", "Create seat"),
    );
    add_path(
        paths,
        "/v1/governance-bodies/{body_id}/seats",
        "get",
        op("governance", "list_seats", "List seats"),
    );
    add_path(
        paths,
        "/v1/governance-seats/{seat_id}/resign",
        "post",
        op("governance", "resign_seat", "Resign seat"),
    );
    add_path(
        paths,
        "/v1/governance-seats/scan-expired",
        "post",
        op("governance", "scan_expired_seats", "Scan expired seats"),
    );
    add_path(
        paths,
        "/v1/meetings",
        "post",
        op("governance", "schedule_meeting", "Schedule meeting"),
    );
    add_path(
        paths,
        "/v1/meetings",
        "get",
        op("governance", "list_all_meetings", "List all meetings"),
    );
    add_path(
        paths,
        "/v1/meetings/{meeting_id}/agenda-items",
        "get",
        op(
            "governance",
            "list_agenda_items",
            "List agenda items for a meeting",
        ),
    );
    add_path(
        paths,
        "/v1/governance-bodies/{body_id}/meetings",
        "get",
        op("governance", "list_meetings", "List meetings for body"),
    );
    add_path(
        paths,
        "/v1/meetings/{meeting_id}/notice",
        "post",
        op("governance", "send_notice", "Send meeting notice"),
    );
    add_path(
        paths,
        "/v1/meetings/{meeting_id}/convene",
        "post",
        op("governance", "convene_meeting", "Convene meeting"),
    );
    add_path(
        paths,
        "/v1/meetings/{meeting_id}/adjourn",
        "post",
        op("governance", "adjourn_meeting", "Adjourn meeting"),
    );
    add_path(
        paths,
        "/v1/meetings/{meeting_id}/cancel",
        "post",
        op("governance", "cancel_meeting", "Cancel meeting"),
    );
    add_path(
        paths,
        "/v1/meetings/{meeting_id}/agenda-items/{item_id}/vote",
        "post",
        op("governance", "cast_vote", "Cast vote"),
    );
    add_path(
        paths,
        "/v1/meetings/{meeting_id}/agenda-items/{item_id}/votes",
        "get",
        op("governance", "list_votes", "List votes"),
    );
    add_path(
        paths,
        "/v1/meetings/{meeting_id}/agenda-items/{item_id}/finalize",
        "post",
        op("governance", "finalize_agenda_item", "Finalize agenda item"),
    );
    add_path(
        paths,
        "/v1/meetings/{meeting_id}/agenda-items/{item_id}/resolution",
        "post",
        op("governance", "compute_resolution", "Compute resolution"),
    );
    add_path(
        paths,
        "/v1/meetings/{meeting_id}/resolutions/{resolution_id}/attach-document",
        "post",
        op(
            "governance",
            "attach_resolution_document",
            "Attach document to resolution",
        ),
    );
    add_path(
        paths,
        "/v1/meetings/{meeting_id}/resolutions",
        "get",
        op("governance", "list_resolutions", "List resolutions"),
    );
    add_path(
        paths,
        "/v1/meetings/written-consent",
        "post",
        op("governance", "written_consent", "Create written consent"),
    );
}

fn add_treasury_paths(paths: &mut Map<String, Value>) {
    add_path(
        paths,
        "/v1/treasury/accounts",
        "post",
        op("treasury", "create_account", "Create GL account"),
    );
    add_path(
        paths,
        "/v1/entities/{entity_id}/accounts",
        "get",
        op("treasury", "list_accounts", "List accounts"),
    );
    add_path(
        paths,
        "/v1/treasury/journal-entries",
        "post",
        op("treasury", "create_journal_entry", "Create journal entry"),
    );
    add_path(
        paths,
        "/v1/entities/{entity_id}/journal-entries",
        "get",
        op("treasury", "list_journal_entries", "List journal entries"),
    );
    add_path(
        paths,
        "/v1/journal-entries/{entry_id}/post",
        "post",
        op("treasury", "post_journal_entry", "Post journal entry"),
    );
    add_path(
        paths,
        "/v1/journal-entries/{entry_id}/void",
        "post",
        op("treasury", "void_journal_entry", "Void journal entry"),
    );
    add_path(
        paths,
        "/v1/treasury/invoices",
        "post",
        op("treasury", "create_invoice", "Create invoice"),
    );
    add_path(
        paths,
        "/v1/entities/{entity_id}/invoices",
        "get",
        op("treasury", "list_invoices", "List invoices"),
    );
    add_path(
        paths,
        "/v1/invoices/{invoice_id}/send",
        "post",
        op("treasury", "send_invoice", "Send invoice"),
    );
    add_path(
        paths,
        "/v1/invoices/{invoice_id}/mark-paid",
        "post",
        op("treasury", "mark_invoice_paid", "Mark invoice paid"),
    );
    add_path(
        paths,
        "/v1/treasury/bank-accounts",
        "post",
        op("treasury", "create_bank_account", "Create bank account"),
    );
    add_path(
        paths,
        "/v1/entities/{entity_id}/bank-accounts",
        "get",
        op("treasury", "list_bank_accounts", "List bank accounts"),
    );
    add_path(
        paths,
        "/v1/bank-accounts/{bank_account_id}/activate",
        "post",
        op("treasury", "activate_bank_account", "Activate bank account"),
    );
    add_path(
        paths,
        "/v1/bank-accounts/{bank_account_id}/close",
        "post",
        op("treasury", "close_bank_account", "Close bank account"),
    );
    add_path(
        paths,
        "/v1/payments",
        "post",
        op("treasury", "submit_payment", "Submit payment"),
    );
    add_path(
        paths,
        "/v1/payments/execute",
        "post",
        op("treasury", "execute_payment", "Execute payment"),
    );
    add_path(
        paths,
        "/v1/payroll/runs",
        "post",
        op("treasury", "create_payroll_run", "Create payroll run"),
    );
    add_path(
        paths,
        "/v1/distributions",
        "post",
        op("treasury", "create_distribution", "Create distribution"),
    );
    add_path(
        paths,
        "/v1/ledger/reconcile",
        "post",
        op("treasury", "reconcile_ledger", "Reconcile ledger"),
    );
    add_path(
        paths,
        "/v1/entities/{entity_id}/stripe-account",
        "get",
        op("treasury", "get_stripe_account", "Get Stripe account"),
    );
    add_path(
        paths,
        "/v1/spending-limits",
        "post",
        op("treasury", "create_spending_limit", "Create spending limit"),
    );
    add_path(
        paths,
        "/v1/entities/{entity_id}/spending-limits",
        "get",
        op("treasury", "list_spending_limits", "List spending limits"),
    );
    add_path(
        paths,
        "/v1/entities/{entity_id}/financial-statements",
        "get",
        op(
            "treasury",
            "get_financial_statements",
            "Get financial statements",
        ),
    );
    add_path(
        paths,
        "/v1/treasury/seed-chart-of-accounts",
        "post",
        op(
            "treasury",
            "seed_chart_of_accounts",
            "Seed chart of accounts",
        ),
    );
    add_path(
        paths,
        "/v1/invoices/{invoice_id}",
        "get",
        op("treasury", "get_invoice", "Get invoice"),
    );
    add_path(
        paths,
        "/v1/invoices/{invoice_id}/status",
        "get",
        op("treasury", "get_invoice_status", "Get invoice status"),
    );
    add_path(
        paths,
        "/v1/invoices/{invoice_id}/pay-instructions",
        "get",
        op("treasury", "get_pay_instructions", "Get pay instructions"),
    );
    add_path(
        paths,
        "/v1/invoices/from-agent-request",
        "post",
        op(
            "treasury",
            "from_agent_request",
            "Create invoice from agent request",
        ),
    );
    add_path(
        paths,
        "/v1/treasury/stripe-accounts",
        "post",
        op("treasury", "create_stripe_account", "Create Stripe account"),
    );
    add_path(
        paths,
        "/v1/treasury/chart-of-accounts/{entity_id}",
        "get",
        op("treasury", "get_chart_of_accounts", "Get chart of accounts"),
    );
    add_path(
        paths,
        "/v1/treasury/payouts",
        "post",
        op("treasury", "create_payout", "Create payout"),
    );
    add_path(
        paths,
        "/v1/treasury/payment-intents",
        "post",
        op("treasury", "create_payment_intent", "Create payment intent"),
    );
    add_path(
        paths,
        "/v1/treasury/webhooks/stripe",
        "post",
        op(
            "treasury",
            "treasury_stripe_webhook",
            "Treasury Stripe webhook",
        ),
    );
    add_path(
        paths,
        "/v1/bank-accounts",
        "post",
        op(
            "treasury",
            "create_bank_account_alias",
            "Create bank account (alias)",
        ),
    );
}

fn add_contacts_paths(paths: &mut Map<String, Value>) {
    add_path(
        paths,
        "/v1/contacts",
        "post",
        op("contacts", "create_contact", "Create contact"),
    );
    add_path(
        paths,
        "/v1/entities/{entity_id}/contacts",
        "get",
        op("contacts", "list_contacts", "List contacts"),
    );
    add_path(
        paths,
        "/v1/contacts/{contact_id}",
        "get",
        op("contacts", "get_contact", "Get contact"),
    );
    add_path(
        paths,
        "/v1/contacts/{contact_id}",
        "patch",
        op("contacts", "update_contact", "Update contact"),
    );
    add_path(
        paths,
        "/v1/contacts/{contact_id}/profile",
        "get",
        op("contacts", "get_contact_profile", "Get contact profile"),
    );
    add_path(
        paths,
        "/v1/contacts/{contact_id}/notification-prefs",
        "get",
        op(
            "contacts",
            "get_notification_prefs",
            "Get notification prefs",
        ),
    );
    add_path(
        paths,
        "/v1/contacts/{contact_id}/notification-prefs",
        "patch",
        op(
            "contacts",
            "update_notification_prefs",
            "Update notification prefs",
        ),
    );
}

fn add_execution_paths(paths: &mut Map<String, Value>) {
    add_path(
        paths,
        "/v1/execution/intents",
        "post",
        op("execution", "create_intent", "Create intent"),
    );
    add_path(
        paths,
        "/v1/entities/{entity_id}/intents",
        "get",
        op("execution", "list_intents", "List intents"),
    );
    add_path(
        paths,
        "/v1/intents/{intent_id}/evaluate",
        "post",
        op("execution", "evaluate_intent", "Evaluate intent"),
    );
    add_path(
        paths,
        "/v1/intents/{intent_id}/authorize",
        "post",
        op("execution", "authorize_intent", "Authorize intent"),
    );
    add_path(
        paths,
        "/v1/intents/{intent_id}/execute",
        "post",
        op("execution", "execute_intent", "Execute intent"),
    );
    add_path(
        paths,
        "/v1/intents/{intent_id}/bind-approval-artifact",
        "post",
        op(
            "execution",
            "bind_approval_artifact_to_intent",
            "Bind approval artifact to intent",
        ),
    );
    add_path(
        paths,
        "/v1/intents/{intent_id}/bind-document-request",
        "post",
        op(
            "execution",
            "bind_document_request_to_intent",
            "Bind document request to intent",
        ),
    );
    add_path(
        paths,
        "/v1/execution/approval-artifacts",
        "post",
        op(
            "execution",
            "create_approval_artifact",
            "Create approval artifact",
        ),
    );
    add_path(
        paths,
        "/v1/entities/{entity_id}/approval-artifacts",
        "get",
        op(
            "execution",
            "list_approval_artifacts",
            "List approval artifacts",
        ),
    );
    add_path(
        paths,
        "/v1/execution/obligations",
        "post",
        op("execution", "create_obligation", "Create obligation"),
    );
    add_path(
        paths,
        "/v1/entities/{entity_id}/obligations",
        "get",
        op("execution", "list_obligations", "List obligations"),
    );
    add_path(
        paths,
        "/v1/obligations/{obligation_id}/fulfill",
        "post",
        op("execution", "fulfill_obligation", "Fulfill obligation"),
    );
    add_path(
        paths,
        "/v1/obligations/{obligation_id}/waive",
        "post",
        op("execution", "waive_obligation", "Waive obligation"),
    );
    add_path(
        paths,
        "/v1/obligations/{obligation_id}/assign",
        "post",
        op("execution", "assign_obligation", "Assign obligation"),
    );
    add_path(
        paths,
        "/v1/entities/{entity_id}/obligations/summary",
        "get",
        op("execution", "obligations_summary", "Obligations summary"),
    );
    add_path(
        paths,
        "/v1/receipts/{receipt_id}",
        "get",
        op("execution", "get_receipt", "Get receipt"),
    );
    add_path(
        paths,
        "/v1/intents/{intent_id}/receipts",
        "get",
        op(
            "execution",
            "list_receipts_by_intent",
            "List receipts by intent",
        ),
    );
    add_path(
        paths,
        "/v1/execution/packets/{packet_id}",
        "get",
        op("execution", "get_packet", "Get transaction packet"),
    );
    add_path(
        paths,
        "/v1/entities/{entity_id}/packets",
        "get",
        op(
            "execution",
            "list_entity_packets",
            "List entity transaction packets",
        ),
    );
    add_path(
        paths,
        "/v1/entities/{entity_id}/obligations/human",
        "get",
        op(
            "execution",
            "list_human_obligations",
            "List human obligations",
        ),
    );
    add_path(
        paths,
        "/v1/human-obligations",
        "get",
        op(
            "execution",
            "list_global_human_obligations",
            "List all human obligations",
        ),
    );
    add_path(
        paths,
        "/v1/human-obligations/{obligation_id}/signer-token",
        "post",
        op(
            "execution",
            "generate_signer_token",
            "Generate signer token",
        ),
    );
    add_path(
        paths,
        "/v1/human-obligations/{obligation_id}/fulfill",
        "post",
        op(
            "execution",
            "fulfill_human_obligation",
            "Fulfill human obligation",
        ),
    );
    add_path(
        paths,
        "/v1/obligations/{obligation_id}/document-requests",
        "post",
        op(
            "execution",
            "create_document_request",
            "Create document request",
        ),
    );
    add_path(
        paths,
        "/v1/obligations/{obligation_id}/document-requests",
        "get",
        op(
            "execution",
            "list_document_requests",
            "List document requests",
        ),
    );
    add_path(
        paths,
        "/v1/document-requests/{request_id}/fulfill",
        "patch",
        op(
            "execution",
            "fulfill_document_request",
            "Fulfill document request",
        ),
    );
    add_path(
        paths,
        "/v1/document-requests/{request_id}/not-applicable",
        "patch",
        op(
            "execution",
            "mark_document_request_na",
            "Mark request not applicable",
        ),
    );
    add_path(
        paths,
        "/v1/obligations/summary",
        "get",
        op(
            "execution",
            "global_obligations_summary",
            "Global obligations summary",
        ),
    );
}

fn add_branches_paths(paths: &mut Map<String, Value>) {
    add_path(
        paths,
        "/v1/branches",
        "post",
        op("branches", "create_branch", "Create branch"),
    );
    add_path(
        paths,
        "/v1/branches",
        "get",
        op("branches", "list_branches", "List branches"),
    );
    add_path(
        paths,
        "/v1/branches/{name}/merge",
        "post",
        op("branches", "merge_branch", "Merge branch"),
    );
    add_path(
        paths,
        "/v1/branches/{name}",
        "delete",
        op("branches", "delete_branch", "Delete branch"),
    );
    add_path(
        paths,
        "/v1/branches/{name}/prune",
        "post",
        op(
            "branches",
            "prune_branch",
            "Prune branch (POST alias for DELETE)",
        ),
    );
}

fn add_auth_paths(paths: &mut Map<String, Value>) {
    add_path(
        paths,
        "/v1/chat/session",
        "post",
        op("auth", "create_chat_session", "Create public chat session"),
    );
    add_path(
        paths,
        "/v1/workspaces/provision",
        "post",
        op("auth", "provision_workspace", "Provision workspace"),
    );
    add_path(
        paths,
        "/v1/api-keys",
        "post",
        op("auth", "create_api_key", "Create API key"),
    );
    add_path(
        paths,
        "/v1/api-keys",
        "get",
        op("auth", "list_api_keys", "List API keys"),
    );
    add_path(
        paths,
        "/v1/api-keys/{key_id}",
        "delete",
        op("auth", "revoke_api_key", "Revoke API key"),
    );
    add_path(
        paths,
        "/v1/api-keys/{key_id}/rotate",
        "post",
        op("auth", "rotate_api_key", "Rotate API key"),
    );
    add_path(
        paths,
        "/v1/auth/token-exchange",
        "post",
        op("auth", "token_exchange", "Token exchange"),
    );
}

fn add_agents_paths(paths: &mut Map<String, Value>) {
    add_path(
        paths,
        "/v1/agents",
        "post",
        op("agents", "create_agent", "Create agent"),
    );
    add_path(
        paths,
        "/v1/agents",
        "get",
        op("agents", "list_agents", "List agents"),
    );
    add_path(
        paths,
        "/v1/agents/{agent_id}",
        "patch",
        op("agents", "update_agent", "Update agent"),
    );
    add_path(
        paths,
        "/v1/agents/{agent_id}/resolved",
        "get",
        op("agents", "get_resolved_agent", "Get resolved agent"),
    );
    add_path(
        paths,
        "/v1/agents/{agent_id}/skills",
        "post",
        op("agents", "add_agent_skill", "Add agent skill"),
    );
    add_path(
        paths,
        "/v1/agents/{agent_id}/messages",
        "post",
        op("agents", "send_agent_message", "Send agent message"),
    );
    add_path(
        paths,
        "/v1/agents/{agent_id}/executions/{execution_id}",
        "get",
        op("agents", "get_execution", "Get agent execution status"),
    );
    add_path(
        paths,
        "/v1/agents/{agent_id}/executions/{execution_id}/result",
        "get",
        op(
            "agents",
            "get_execution_result",
            "Get agent execution result",
        ),
    );
    add_path(
        paths,
        "/v1/agents/{agent_id}/executions/{execution_id}/logs",
        "get",
        op("agents", "get_execution_logs", "Get agent execution logs"),
    );
    add_path(
        paths,
        "/v1/agents/{agent_id}/executions/{execution_id}/kill",
        "post",
        op("agents", "kill_execution", "Kill agent execution"),
    );
}

fn add_compliance_paths(paths: &mut Map<String, Value>) {
    add_path(
        paths,
        "/v1/tax/filings",
        "post",
        op("compliance", "file_tax_document", "File tax document"),
    );
    add_path(
        paths,
        "/v1/deadlines",
        "post",
        op("compliance", "create_deadline", "Create deadline"),
    );
    add_path(
        paths,
        "/v1/contractors/classify",
        "post",
        op("compliance", "classify_contractor", "Classify contractor"),
    );
    add_path(
        paths,
        "/v1/compliance/escalations/scan",
        "post",
        op(
            "compliance",
            "scan_compliance_escalations",
            "Scan compliance escalations",
        ),
    );
    add_path(
        paths,
        "/v1/entities/{entity_id}/compliance/escalations",
        "get",
        op(
            "compliance",
            "list_entity_escalations",
            "List compliance escalations",
        ),
    );
    add_path(
        paths,
        "/v1/compliance/escalations/{escalation_id}/resolve-with-evidence",
        "post",
        op(
            "compliance",
            "resolve_escalation_with_evidence",
            "Resolve compliance escalation and attach evidence",
        ),
    );
}

fn add_billing_paths(paths: &mut Map<String, Value>) {
    add_path(
        paths,
        "/v1/billing/checkout",
        "post",
        op("billing", "checkout", "Create checkout session"),
    );
    add_path(
        paths,
        "/v1/billing/portal",
        "post",
        op("billing", "portal", "Create portal session"),
    );
    add_path(
        paths,
        "/v1/billing/status",
        "get",
        op("billing", "billing_status", "Get billing status"),
    );
    add_path(
        paths,
        "/v1/billing/plans",
        "get",
        op("billing", "list_plans", "List plans"),
    );
    add_path(
        paths,
        "/v1/subscriptions",
        "post",
        op("billing", "create_subscription", "Create subscription"),
    );
    add_path(
        paths,
        "/v1/subscriptions/{subscription_id}",
        "get",
        op("billing", "get_subscription", "Get subscription"),
    );
    add_path(
        paths,
        "/v1/subscriptions/tick",
        "post",
        op("billing", "tick_subscriptions", "Tick subscriptions"),
    );
}

fn add_services_paths(paths: &mut Map<String, Value>) {
    add_path(
        paths,
        "/v1/services/catalog",
        "get",
        op("services", "list_catalog", "List service catalog"),
    );
    add_path(
        paths,
        "/v1/services/catalog/{slug}",
        "get",
        op("services", "get_catalog_item", "Get catalog item by slug"),
    );
    add_path(
        paths,
        "/v1/services/requests",
        "post",
        op(
            "services",
            "create_service_request",
            "Create service request",
        ),
    );
    add_path(
        paths,
        "/v1/entities/{entity_id}/services/requests",
        "get",
        op(
            "services",
            "list_service_requests",
            "List service requests for entity",
        ),
    );
    add_path(
        paths,
        "/v1/entities/{entity_id}/services/requests/{request_id}",
        "get",
        op("services", "get_service_request", "Get service request"),
    );
    add_path(
        paths,
        "/v1/services/requests/{entity_id}/{request_id}/checkout",
        "post",
        op("services", "initiate_checkout", "Initiate Stripe checkout"),
    );
    add_path(
        paths,
        "/v1/services/requests/{entity_id}/{request_id}/begin-fulfillment",
        "post",
        op("services", "begin_fulfillment", "Begin fulfillment"),
    );
    add_path(
        paths,
        "/v1/services/requests/{entity_id}/{request_id}/fulfill",
        "post",
        op(
            "services",
            "fulfill_service_request",
            "Fulfill service request",
        ),
    );
    add_path(
        paths,
        "/v1/services/requests/{entity_id}/{request_id}/fail",
        "post",
        op("services", "fail_service_request", "Fail service request"),
    );
    add_path(
        paths,
        "/v1/services/pending",
        "get",
        op(
            "services",
            "list_pending_fulfillment",
            "List pending fulfillment requests",
        ),
    );
    add_path(
        paths,
        "/v1/services/webhooks/stripe",
        "post",
        op(
            "services",
            "stripe_webhook",
            "Stripe webhook for service payments",
        ),
    );
}

fn add_admin_paths(paths: &mut Map<String, Value>) {
    add_path(
        paths,
        "/v1/admin/workspaces",
        "get",
        op("admin", "list_workspaces", "List workspaces"),
    );
    add_path(
        paths,
        "/v1/admin/audit-events",
        "get",
        op("admin", "list_audit_events", "List audit events"),
    );
    add_path(
        paths,
        "/v1/admin/system-health",
        "get",
        op("admin", "system_health", "System health"),
    );
    add_path(
        paths,
        "/v1/workspace/status",
        "get",
        op("admin", "workspace_status", "Workspace status"),
    );
    add_path(
        paths,
        "/v1/workspace/entities",
        "get",
        op(
            "admin",
            "list_workspace_entities",
            "List workspace entities",
        ),
    );
    add_path(
        paths,
        "/v1/demo/seed",
        "post",
        op("admin", "demo_seed", "Seed demo data"),
    );
    add_path(
        paths,
        "/v1/subscription",
        "get",
        op("admin", "get_subscription", "Get subscription"),
    );
    add_path(
        paths,
        "/v1/config",
        "get",
        op("admin", "get_config", "Get config"),
    );
    add_path(
        paths,
        "/v1/workspaces/link",
        "post",
        op("admin", "link_workspace", "Link workspace"),
    );
    add_path(
        paths,
        "/v1/workspaces/claim",
        "post",
        op("admin", "claim_workspace", "Claim workspace"),
    );
    add_path(
        paths,
        "/v1/workspaces/{workspace_id}/status",
        "get",
        op(
            "admin",
            "workspace_status_by_path",
            "Workspace status by path",
        ),
    );
    add_path(
        paths,
        "/v1/workspaces/{workspace_id}/entities",
        "get",
        op(
            "admin",
            "workspace_entities_by_path",
            "Workspace entities by path",
        ),
    );
    add_path(
        paths,
        "/v1/workspaces/{workspace_id}/contacts",
        "get",
        op("admin", "workspace_contacts", "Workspace contacts"),
    );
    add_path(
        paths,
        "/v1/digests",
        "get",
        op("admin", "list_digests", "List digests"),
    );
    add_path(
        paths,
        "/v1/digests/trigger",
        "post",
        op("admin", "trigger_digests", "Trigger digests"),
    );
    add_path(
        paths,
        "/v1/digests/{digest_key}",
        "get",
        op("admin", "get_digest", "Get digest"),
    );
    add_path(
        paths,
        "/v1/service-token",
        "get",
        op("admin", "get_service_token", "Get service token"),
    );
    add_path(
        paths,
        "/v1/jwks",
        "get",
        op("admin", "get_jwks", "Get JWKS"),
    );
}

fn add_webhooks_paths(paths: &mut Map<String, Value>) {
    add_path(
        paths,
        "/v1/webhooks/stripe",
        "post",
        op("webhooks", "stripe_webhook", "Stripe webhook"),
    );
    add_path(
        paths,
        "/v1/webhooks/stripe-billing",
        "post",
        op(
            "webhooks",
            "stripe_billing_webhook",
            "Stripe billing webhook",
        ),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spec_is_valid_json_with_expected_paths() {
        let spec = openapi_spec();
        assert_eq!(spec["openapi"], "3.1.0");
        assert_eq!(spec["info"]["title"], "The Corporation API");

        let paths = spec["paths"].as_object().unwrap();
        // Verify we have a substantial number of paths
        assert!(
            paths.len() >= 120,
            "expected >= 120 paths, got {}",
            paths.len()
        );

        // Spot-check key endpoints exist
        assert!(paths.contains_key("/v1/formations"));
        assert!(paths.contains_key("/v1/formations/{entity_id}/gates"));
        assert!(paths.contains_key("/v1/entities/{entity_id}/cap-table"));
        assert!(paths.contains_key("/v1/governance-bodies"));
        assert!(paths.contains_key("/v1/treasury/accounts"));
        assert!(paths.contains_key("/v1/contacts"));
        assert!(paths.contains_key("/v1/execution/intents"));
        assert!(paths.contains_key("/v1/branches"));
        assert!(paths.contains_key("/v1/api-keys"));
        assert!(paths.contains_key("/v1/agents"));
        assert!(paths.contains_key("/v1/tax/filings"));
        assert!(paths.contains_key("/v1/billing/checkout"));
        assert!(paths.contains_key("/v1/admin/system-health"));
    }

    #[test]
    fn each_operation_has_tags_and_responses() {
        let spec = openapi_spec();
        let paths = spec["paths"].as_object().unwrap();

        for (path, methods) in paths {
            let methods = methods.as_object().unwrap();
            for (method, op) in methods {
                assert!(
                    op["tags"].is_array(),
                    "missing tags on {} {}",
                    method.to_uppercase(),
                    path
                );
                assert!(
                    op["responses"].is_object(),
                    "missing responses on {} {}",
                    method.to_uppercase(),
                    path
                );
                assert!(
                    op["operationId"].is_string(),
                    "missing operationId on {} {}",
                    method.to_uppercase(),
                    path
                );
            }
        }
    }
}
