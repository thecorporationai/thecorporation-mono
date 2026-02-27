//! OpenAPI specification generation.
//!
//! Builds an OpenAPI 3.1 JSON spec from route metadata.
//! Served at `GET /v1/openapi.json`.

use serde_json::{json, Map, Value};

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
            { "name": "equity", "description": "Cap table, grants, SAFEs, and transfers" },
            { "name": "governance", "description": "Bodies, seats, meetings, and votes" },
            { "name": "treasury", "description": "Accounts, journal entries, invoices, and banking" },
            { "name": "contacts", "description": "Contact management" },
            { "name": "execution", "description": "Intents, obligations, and receipts" },
            { "name": "branches", "description": "Git branch management" },
            { "name": "auth", "description": "Authentication and API keys" },
            { "name": "agents", "description": "Agent management" },
            { "name": "compliance", "description": "Tax filings and deadlines" },
            { "name": "billing", "description": "Billing and subscriptions" },
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
    add_path(paths, "/v1/formations", "post", op("formation", "create_formation", "Create a new entity formation"));
    add_path(paths, "/v1/formations/{entity_id}", "get", op("formation", "get_formation", "Get entity formation details"));
    add_path(paths, "/v1/formations/{entity_id}/documents", "get", op("formation", "list_documents", "List formation documents"));
    add_path(paths, "/v1/formations/{entity_id}/filing-confirmation", "post", op("formation", "confirm_filing", "Confirm entity filing"));
    add_path(paths, "/v1/formations/{entity_id}/ein-confirmation", "post", op("formation", "confirm_ein", "Confirm EIN assignment"));
    add_path(paths, "/v1/documents/{document_id}", "get", op("formation", "get_document", "Get a document"));
    add_path(paths, "/v1/documents/{document_id}/sign", "post", op("formation", "sign_document", "Sign a document"));
    add_path(paths, "/v1/documents/{document_id}/pdf", "get", op("formation", "get_document_pdf", "Get document PDF"));
    add_path(paths, "/v1/documents/{document_id}/request-copy", "post", op("formation", "request_document_copy", "Request document copy"));
    add_path(paths, "/v1/documents/{document_id}/amendment-history", "get", op("formation", "get_amendment_history", "Get amendment history"));
    add_path(paths, "/v1/contracts", "post", op("formation", "generate_contract", "Generate a contract"));
    add_path(paths, "/v1/sign/{document_id}", "get", op("formation", "get_signing_link", "Get signing link"));
    add_path(paths, "/v1/entities", "get", op("formation", "list_entities", "List entities"));
    add_path(paths, "/v1/entities/{entity_id}/convert", "post", op("formation", "convert_entity", "Convert entity type"));
    add_path(paths, "/v1/entities/{entity_id}/dissolve", "post", op("formation", "dissolve_entity", "Dissolve entity"));
    add_path(paths, "/v1/entities/{entity_id}/governance-documents", "get", op("formation", "list_governance_documents", "List governance documents"));
    add_path(paths, "/v1/entities/{entity_id}/governance-documents/current", "get", op("formation", "get_current_governance_document", "Get current governance document"));
}

fn add_equity_paths(paths: &mut Map<String, Value>) {
    add_path(paths, "/v1/entities/{entity_id}/cap-table", "get", op("equity", "get_cap_table", "Get cap table"));
    add_path(paths, "/v1/equity/grants", "post", op("equity", "issue_grant", "Issue equity grant"));
    add_path(paths, "/v1/safe-notes", "post", op("equity", "create_safe_note", "Create SAFE note"));
    add_path(paths, "/v1/safe-notes/{safe_note_id}", "get", op("equity", "get_safe_note", "Get SAFE note"));
    add_path(paths, "/v1/entities/{entity_id}/safe-notes", "get", op("equity", "list_safe_notes", "List SAFE notes"));
    add_path(paths, "/v1/valuations", "post", op("equity", "create_valuation", "Create valuation"));
    add_path(paths, "/v1/valuations/{valuation_id}", "get", op("equity", "get_valuation", "Get valuation"));
    add_path(paths, "/v1/entities/{entity_id}/valuations", "get", op("equity", "list_valuations", "List valuations"));
    add_path(paths, "/v1/entities/{entity_id}/current-409a", "get", op("equity", "get_current_409a", "Get current 409A valuation"));
    add_path(paths, "/v1/valuations/{valuation_id}/approve", "post", op("equity", "approve_valuation", "Approve valuation"));
    add_path(paths, "/v1/valuations/{valuation_id}/expire", "post", op("equity", "expire_valuation", "Expire valuation"));
    add_path(paths, "/v1/entities/{entity_id}/check-exercise-price", "post", op("equity", "check_exercise_price", "Check exercise price"));
    add_path(paths, "/v1/share-transfers", "post", op("equity", "create_transfer", "Create share transfer"));
    add_path(paths, "/v1/share-transfers/{transfer_id}", "get", op("equity", "get_transfer", "Get transfer"));
    add_path(paths, "/v1/entities/{entity_id}/share-transfers", "get", op("equity", "list_transfers", "List transfers"));
    add_path(paths, "/v1/share-transfers/{transfer_id}/submit-review", "post", op("equity", "submit_transfer_review", "Submit transfer for review"));
    add_path(paths, "/v1/share-transfers/{transfer_id}/bylaws-review", "post", op("equity", "record_bylaws_review", "Record bylaws review"));
    add_path(paths, "/v1/share-transfers/{transfer_id}/rofr-decision", "post", op("equity", "record_rofr_decision", "Record ROFR decision"));
    add_path(paths, "/v1/share-transfers/{transfer_id}/approve", "post", op("equity", "approve_transfer", "Approve transfer"));
    add_path(paths, "/v1/share-transfers/{transfer_id}/execute", "post", op("equity", "execute_transfer", "Execute transfer"));
    add_path(paths, "/v1/funding-rounds", "post", op("equity", "create_funding_round", "Create funding round"));
    add_path(paths, "/v1/entities/{entity_id}/funding-rounds", "get", op("equity", "list_funding_rounds", "List funding rounds"));
}

fn add_governance_paths(paths: &mut Map<String, Value>) {
    add_path(paths, "/v1/governance-bodies", "post", op("governance", "create_governance_body", "Create governing body"));
    add_path(paths, "/v1/governance-bodies", "get", op("governance", "list_all_governance_bodies", "List all governance bodies"));
    add_path(paths, "/v1/entities/{entity_id}/governance-bodies", "get", op("governance", "list_governance_bodies", "List entity governance bodies"));
    add_path(paths, "/v1/governance-bodies/{body_id}/seats", "post", op("governance", "create_seat", "Create seat"));
    add_path(paths, "/v1/governance-bodies/{body_id}/seats", "get", op("governance", "list_seats", "List seats"));
    add_path(paths, "/v1/governance-seats/{seat_id}/resign", "post", op("governance", "resign_seat", "Resign seat"));
    add_path(paths, "/v1/governance-seats/scan-expired", "post", op("governance", "scan_expired_seats", "Scan expired seats"));
    add_path(paths, "/v1/meetings", "post", op("governance", "schedule_meeting", "Schedule meeting"));
    add_path(paths, "/v1/meetings", "get", op("governance", "list_all_meetings", "List all meetings"));
    add_path(paths, "/v1/governance-bodies/{body_id}/meetings", "get", op("governance", "list_meetings", "List meetings for body"));
    add_path(paths, "/v1/meetings/{meeting_id}/notice", "post", op("governance", "send_notice", "Send meeting notice"));
    add_path(paths, "/v1/meetings/{meeting_id}/convene", "post", op("governance", "convene_meeting", "Convene meeting"));
    add_path(paths, "/v1/meetings/{meeting_id}/adjourn", "post", op("governance", "adjourn_meeting", "Adjourn meeting"));
    add_path(paths, "/v1/meetings/{meeting_id}/cancel", "post", op("governance", "cancel_meeting", "Cancel meeting"));
    add_path(paths, "/v1/meetings/{meeting_id}/agenda-items/{item_id}/vote", "post", op("governance", "cast_vote", "Cast vote"));
    add_path(paths, "/v1/meetings/{meeting_id}/agenda-items/{item_id}/votes", "get", op("governance", "list_votes", "List votes"));
    add_path(paths, "/v1/meetings/{meeting_id}/agenda-items/{item_id}/resolution", "post", op("governance", "compute_resolution", "Compute resolution"));
    add_path(paths, "/v1/meetings/{meeting_id}/resolutions", "get", op("governance", "list_resolutions", "List resolutions"));
    add_path(paths, "/v1/meetings/written-consent", "post", op("governance", "written_consent", "Create written consent"));
}

fn add_treasury_paths(paths: &mut Map<String, Value>) {
    add_path(paths, "/v1/treasury/accounts", "post", op("treasury", "create_account", "Create GL account"));
    add_path(paths, "/v1/entities/{entity_id}/accounts", "get", op("treasury", "list_accounts", "List accounts"));
    add_path(paths, "/v1/treasury/journal-entries", "post", op("treasury", "create_journal_entry", "Create journal entry"));
    add_path(paths, "/v1/entities/{entity_id}/journal-entries", "get", op("treasury", "list_journal_entries", "List journal entries"));
    add_path(paths, "/v1/journal-entries/{entry_id}/post", "post", op("treasury", "post_journal_entry", "Post journal entry"));
    add_path(paths, "/v1/journal-entries/{entry_id}/void", "post", op("treasury", "void_journal_entry", "Void journal entry"));
    add_path(paths, "/v1/treasury/invoices", "post", op("treasury", "create_invoice", "Create invoice"));
    add_path(paths, "/v1/entities/{entity_id}/invoices", "get", op("treasury", "list_invoices", "List invoices"));
    add_path(paths, "/v1/invoices/{invoice_id}/send", "post", op("treasury", "send_invoice", "Send invoice"));
    add_path(paths, "/v1/invoices/{invoice_id}/mark-paid", "post", op("treasury", "mark_invoice_paid", "Mark invoice paid"));
    add_path(paths, "/v1/treasury/bank-accounts", "post", op("treasury", "create_bank_account", "Create bank account"));
    add_path(paths, "/v1/entities/{entity_id}/bank-accounts", "get", op("treasury", "list_bank_accounts", "List bank accounts"));
    add_path(paths, "/v1/bank-accounts/{bank_account_id}/activate", "post", op("treasury", "activate_bank_account", "Activate bank account"));
    add_path(paths, "/v1/bank-accounts/{bank_account_id}/close", "post", op("treasury", "close_bank_account", "Close bank account"));
    add_path(paths, "/v1/payments", "post", op("treasury", "submit_payment", "Submit payment"));
    add_path(paths, "/v1/payments/execute", "post", op("treasury", "execute_payment", "Execute payment"));
    add_path(paths, "/v1/payroll/runs", "post", op("treasury", "create_payroll_run", "Create payroll run"));
    add_path(paths, "/v1/distributions", "post", op("treasury", "create_distribution", "Create distribution"));
    add_path(paths, "/v1/ledger/reconcile", "post", op("treasury", "reconcile_ledger", "Reconcile ledger"));
    add_path(paths, "/v1/entities/{entity_id}/stripe-account", "get", op("treasury", "get_stripe_account", "Get Stripe account"));
    add_path(paths, "/v1/spending-limits", "post", op("treasury", "create_spending_limit", "Create spending limit"));
    add_path(paths, "/v1/entities/{entity_id}/spending-limits", "get", op("treasury", "list_spending_limits", "List spending limits"));
    add_path(paths, "/v1/entities/{entity_id}/financial-statements", "get", op("treasury", "get_financial_statements", "Get financial statements"));
    add_path(paths, "/v1/treasury/seed-chart-of-accounts", "post", op("treasury", "seed_chart_of_accounts", "Seed chart of accounts"));
    add_path(paths, "/v1/invoices/{invoice_id}", "get", op("treasury", "get_invoice", "Get invoice"));
    add_path(paths, "/v1/invoices/{invoice_id}/status", "get", op("treasury", "get_invoice_status", "Get invoice status"));
    add_path(paths, "/v1/invoices/{invoice_id}/pay-instructions", "get", op("treasury", "get_pay_instructions", "Get pay instructions"));
    add_path(paths, "/v1/invoices/from-agent-request", "post", op("treasury", "from_agent_request", "Create invoice from agent request"));
    add_path(paths, "/v1/treasury/stripe-accounts", "post", op("treasury", "create_stripe_account", "Create Stripe account"));
    add_path(paths, "/v1/treasury/chart-of-accounts/{entity_id}", "get", op("treasury", "get_chart_of_accounts", "Get chart of accounts"));
    add_path(paths, "/v1/treasury/payouts", "post", op("treasury", "create_payout", "Create payout"));
    add_path(paths, "/v1/treasury/payment-intents", "post", op("treasury", "create_payment_intent", "Create payment intent"));
    add_path(paths, "/v1/treasury/webhooks/stripe", "post", op("treasury", "treasury_stripe_webhook", "Treasury Stripe webhook"));
    add_path(paths, "/v1/bank-accounts", "post", op("treasury", "create_bank_account_alias", "Create bank account (alias)"));
}

fn add_contacts_paths(paths: &mut Map<String, Value>) {
    add_path(paths, "/v1/contacts", "post", op("contacts", "create_contact", "Create contact"));
    add_path(paths, "/v1/entities/{entity_id}/contacts", "get", op("contacts", "list_contacts", "List contacts"));
    add_path(paths, "/v1/contacts/{contact_id}", "get", op("contacts", "get_contact", "Get contact"));
    add_path(paths, "/v1/contacts/{contact_id}", "patch", op("contacts", "update_contact", "Update contact"));
    add_path(paths, "/v1/contacts/{contact_id}/profile", "get", op("contacts", "get_contact_profile", "Get contact profile"));
    add_path(paths, "/v1/contacts/{contact_id}/notification-prefs", "get", op("contacts", "get_notification_prefs", "Get notification prefs"));
    add_path(paths, "/v1/contacts/{contact_id}/notification-prefs", "patch", op("contacts", "update_notification_prefs", "Update notification prefs"));
}

fn add_execution_paths(paths: &mut Map<String, Value>) {
    add_path(paths, "/v1/execution/intents", "post", op("execution", "create_intent", "Create intent"));
    add_path(paths, "/v1/entities/{entity_id}/intents", "get", op("execution", "list_intents", "List intents"));
    add_path(paths, "/v1/intents/{intent_id}/evaluate", "post", op("execution", "evaluate_intent", "Evaluate intent"));
    add_path(paths, "/v1/intents/{intent_id}/authorize", "post", op("execution", "authorize_intent", "Authorize intent"));
    add_path(paths, "/v1/intents/{intent_id}/execute", "post", op("execution", "execute_intent", "Execute intent"));
    add_path(paths, "/v1/execution/obligations", "post", op("execution", "create_obligation", "Create obligation"));
    add_path(paths, "/v1/entities/{entity_id}/obligations", "get", op("execution", "list_obligations", "List obligations"));
    add_path(paths, "/v1/obligations/{obligation_id}/fulfill", "post", op("execution", "fulfill_obligation", "Fulfill obligation"));
    add_path(paths, "/v1/obligations/{obligation_id}/waive", "post", op("execution", "waive_obligation", "Waive obligation"));
    add_path(paths, "/v1/obligations/{obligation_id}/assign", "post", op("execution", "assign_obligation", "Assign obligation"));
    add_path(paths, "/v1/entities/{entity_id}/obligations/summary", "get", op("execution", "obligations_summary", "Obligations summary"));
    add_path(paths, "/v1/receipts/{receipt_id}", "get", op("execution", "get_receipt", "Get receipt"));
    add_path(paths, "/v1/intents/{intent_id}/receipts", "get", op("execution", "list_receipts_by_intent", "List receipts by intent"));
    add_path(paths, "/v1/entities/{entity_id}/obligations/human", "get", op("execution", "list_human_obligations", "List human obligations"));
    add_path(paths, "/v1/human-obligations", "get", op("execution", "list_global_human_obligations", "List all human obligations"));
    add_path(paths, "/v1/human-obligations/{obligation_id}/signer-token", "post", op("execution", "generate_signer_token", "Generate signer token"));
    add_path(paths, "/v1/human-obligations/{obligation_id}/fulfill", "post", op("execution", "fulfill_human_obligation", "Fulfill human obligation"));
    add_path(paths, "/v1/obligations/{obligation_id}/document-requests", "post", op("execution", "create_document_request", "Create document request"));
    add_path(paths, "/v1/obligations/{obligation_id}/document-requests", "get", op("execution", "list_document_requests", "List document requests"));
    add_path(paths, "/v1/document-requests/{request_id}/fulfill", "patch", op("execution", "fulfill_document_request", "Fulfill document request"));
    add_path(paths, "/v1/document-requests/{request_id}/not-applicable", "patch", op("execution", "mark_document_request_na", "Mark request not applicable"));
    add_path(paths, "/v1/obligations/summary", "get", op("execution", "global_obligations_summary", "Global obligations summary"));
}

fn add_branches_paths(paths: &mut Map<String, Value>) {
    add_path(paths, "/v1/branches", "post", op("branches", "create_branch", "Create branch"));
    add_path(paths, "/v1/branches", "get", op("branches", "list_branches", "List branches"));
    add_path(paths, "/v1/branches/{name}/merge", "post", op("branches", "merge_branch", "Merge branch"));
    add_path(paths, "/v1/branches/{name}", "delete", op("branches", "delete_branch", "Delete branch"));
    add_path(paths, "/v1/branches/{name}/prune", "post", op("branches", "prune_branch", "Prune branch (POST alias for DELETE)"));
}

fn add_auth_paths(paths: &mut Map<String, Value>) {
    add_path(paths, "/v1/workspaces/provision", "post", op("auth", "provision_workspace", "Provision workspace"));
    add_path(paths, "/v1/api-keys", "post", op("auth", "create_api_key", "Create API key"));
    add_path(paths, "/v1/api-keys/{workspace_id}", "get", op("auth", "list_api_keys", "List API keys"));
    add_path(paths, "/v1/api-keys/{workspace_id}/{key_id}", "delete", op("auth", "revoke_api_key", "Revoke API key"));
    add_path(paths, "/v1/api-keys/{workspace_id}/{key_id}/rotate", "post", op("auth", "rotate_api_key", "Rotate API key"));
    add_path(paths, "/v1/auth/token-exchange", "post", op("auth", "token_exchange", "Token exchange"));
}

fn add_agents_paths(paths: &mut Map<String, Value>) {
    add_path(paths, "/v1/agents", "post", op("agents", "create_agent", "Create agent"));
    add_path(paths, "/v1/agents", "get", op("agents", "list_agents", "List agents"));
    add_path(paths, "/v1/agents/{agent_id}", "patch", op("agents", "update_agent", "Update agent"));
    add_path(paths, "/v1/agents/{agent_id}/skills", "post", op("agents", "add_agent_skill", "Add agent skill"));
    add_path(paths, "/v1/agents/{agent_id}/messages", "post", op("agents", "send_agent_message", "Send agent message"));
}

fn add_compliance_paths(paths: &mut Map<String, Value>) {
    add_path(paths, "/v1/tax/filings", "post", op("compliance", "file_tax_document", "File tax document"));
    add_path(paths, "/v1/deadlines", "post", op("compliance", "create_deadline", "Create deadline"));
    add_path(paths, "/v1/contractors/classify", "post", op("compliance", "classify_contractor", "Classify contractor"));
}

fn add_billing_paths(paths: &mut Map<String, Value>) {
    add_path(paths, "/v1/billing/checkout", "post", op("billing", "checkout", "Create checkout session"));
    add_path(paths, "/v1/billing/portal", "post", op("billing", "portal", "Create portal session"));
    add_path(paths, "/v1/billing/status", "get", op("billing", "billing_status", "Get billing status"));
    add_path(paths, "/v1/billing/plans", "get", op("billing", "list_plans", "List plans"));
    add_path(paths, "/v1/subscriptions", "post", op("billing", "create_subscription", "Create subscription"));
    add_path(paths, "/v1/subscriptions/{subscription_id}", "get", op("billing", "get_subscription", "Get subscription"));
    add_path(paths, "/v1/subscriptions/tick", "post", op("billing", "tick_subscriptions", "Tick subscriptions"));
}

fn add_admin_paths(paths: &mut Map<String, Value>) {
    add_path(paths, "/v1/admin/workspaces", "get", op("admin", "list_workspaces", "List workspaces"));
    add_path(paths, "/v1/admin/audit-events", "get", op("admin", "list_audit_events", "List audit events"));
    add_path(paths, "/v1/admin/system-health", "get", op("admin", "system_health", "System health"));
    add_path(paths, "/v1/workspace/status", "get", op("admin", "workspace_status", "Workspace status"));
    add_path(paths, "/v1/workspace/entities", "get", op("admin", "list_workspace_entities", "List workspace entities"));
    add_path(paths, "/v1/demo/seed", "post", op("admin", "demo_seed", "Seed demo data"));
    add_path(paths, "/v1/subscription", "get", op("admin", "get_subscription", "Get subscription"));
    add_path(paths, "/v1/config", "get", op("admin", "get_config", "Get config"));
    add_path(paths, "/v1/workspaces/link", "post", op("admin", "link_workspace", "Link workspace"));
    add_path(paths, "/v1/workspaces/claim", "post", op("admin", "claim_workspace", "Claim workspace"));
    add_path(paths, "/v1/workspaces/{workspace_id}/status", "get", op("admin", "workspace_status_by_path", "Workspace status by path"));
    add_path(paths, "/v1/workspaces/{workspace_id}/entities", "get", op("admin", "workspace_entities_by_path", "Workspace entities by path"));
    add_path(paths, "/v1/workspaces/{workspace_id}/contacts", "get", op("admin", "workspace_contacts", "Workspace contacts"));
    add_path(paths, "/v1/digests", "get", op("admin", "list_digests", "List digests"));
    add_path(paths, "/v1/digests/trigger", "post", op("admin", "trigger_digests", "Trigger digests"));
    add_path(paths, "/v1/digests/{digest_key}", "get", op("admin", "get_digest", "Get digest"));
    add_path(paths, "/v1/service-token", "get", op("admin", "get_service_token", "Get service token"));
    add_path(paths, "/v1/jwks", "get", op("admin", "get_jwks", "Get JWKS"));
}

fn add_webhooks_paths(paths: &mut Map<String, Value>) {
    add_path(paths, "/v1/webhooks/stripe", "post", op("webhooks", "stripe_webhook", "Stripe webhook"));
    add_path(paths, "/v1/webhooks/stripe-billing", "post", op("webhooks", "stripe_billing_webhook", "Stripe billing webhook"));
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
        assert!(paths.len() >= 120, "expected >= 120 paths, got {}", paths.len());

        // Spot-check key endpoints exist
        assert!(paths.contains_key("/v1/formations"));
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
