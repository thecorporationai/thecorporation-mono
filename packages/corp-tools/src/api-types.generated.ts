export type paths = {
    "/v1/admin/audit-events": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["list_audit_events"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/admin/system-health": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["system_health"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/admin/workspaces": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["list_workspaces"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/agents": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["list_agents"];
        put?: never;
        post: operations["create_agent"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/agents/{agent_id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch: operations["update_agent"];
        trace?: never;
    };
    "/v1/agents/{agent_id}/executions/{execution_id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["get_execution"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/agents/{agent_id}/executions/{execution_id}/kill": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["kill_execution"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/agents/{agent_id}/executions/{execution_id}/logs": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["get_execution_logs"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/agents/{agent_id}/executions/{execution_id}/result": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["get_execution_result"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/agents/{agent_id}/messages": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["send_agent_message"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/agents/{agent_id}/messages/{message_id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["get_agent_message_internal"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/agents/{agent_id}/resolved": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["get_resolved_agent"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/agents/{agent_id}/skills": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["add_agent_skill"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/api-keys": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["list_api_keys"];
        put?: never;
        post: operations["create_api_key"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/api-keys/{key_id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post?: never;
        delete: operations["revoke_api_key"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/api-keys/{key_id}/rotate": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["rotate_api_key"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/auth/token-exchange": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["token_exchange"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/bank-accounts/{bank_account_id}/activate": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["activate_bank_account"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/bank-accounts/{bank_account_id}/close": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["close_bank_account"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/branches": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["list_branches"];
        put?: never;
        post: operations["create_branch"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/branches/{name}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post?: never;
        delete: operations["delete_branch_handler"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/branches/{name}/merge": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["merge_branch"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/branches/{name}/prune": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Prune a branch (POST alternative to DELETE for clients that don't support DELETE). */
        post: operations["prune_branch"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/compliance/escalations/scan": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["scan_compliance_escalations"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/compliance/escalations/{escalation_id}/resolve-with-evidence": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["resolve_escalation_with_evidence"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/config": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["get_config"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/contacts": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["create_contact"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/contacts/{contact_id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["get_contact"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch: operations["update_contact"];
        trace?: never;
    };
    "/v1/contacts/{contact_id}/notification-prefs": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["get_notification_prefs"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch: operations["update_notification_prefs"];
        trace?: never;
    };
    "/v1/contacts/{contact_id}/profile": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["get_contact_profile"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/contractors/classify": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["classify_contractor"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/contracts": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["generate_contract"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/deadlines": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["create_deadline"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/demo/seed": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["demo_seed"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/digests": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["list_digests"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/digests/trigger": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["trigger_digests"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/digests/{digest_key}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["get_digest"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/distributions": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["create_distribution"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/document-requests/{request_id}/fulfill": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch: operations["fulfill_document_request"];
        trace?: never;
    };
    "/v1/document-requests/{request_id}/not-applicable": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch: operations["mark_document_request_na"];
        trace?: never;
    };
    "/v1/documents/preview/pdf": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Preview a governance document as PDF without requiring a saved Document record. */
        get: operations["preview_document_pdf"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/documents/{document_id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["get_document"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/documents/{document_id}/amendment-history": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["get_amendment_history"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/documents/{document_id}/pdf": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["get_document_pdf"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/documents/{document_id}/request-copy": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["request_document_copy"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/documents/{document_id}/sign": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["sign_document"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/entities": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["list_entities"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/entities/{entity_id}/accounts": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["list_accounts"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/entities/{entity_id}/approval-artifacts": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["list_approval_artifacts"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/entities/{entity_id}/bank-accounts": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["list_bank_accounts"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/entities/{entity_id}/cap-table": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["get_cap_table"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/entities/{entity_id}/compliance/escalations": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["list_entity_escalations"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/entities/{entity_id}/contacts": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["list_contacts"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/entities/{entity_id}/convert": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["convert_entity"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/entities/{entity_id}/current-409a": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["get_current_409a"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/entities/{entity_id}/dissolve": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["dissolve_entity"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/entities/{entity_id}/financial-statements": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["get_financial_statements"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/entities/{entity_id}/governance-bodies": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["list_governance_bodies"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/entities/{entity_id}/governance-documents": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["list_governance_documents"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/entities/{entity_id}/governance-documents/current": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["get_current_governance_document"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/entities/{entity_id}/governance/audit/checkpoints": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["list_governance_audit_checkpoints"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/entities/{entity_id}/governance/audit/entries": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["list_governance_audit_entries"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/entities/{entity_id}/governance/audit/verifications": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["list_governance_audit_verifications"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/entities/{entity_id}/governance/doc-bundles": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["list_governance_doc_bundles"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/entities/{entity_id}/governance/doc-bundles/current": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["get_current_governance_doc_bundle"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/entities/{entity_id}/governance/doc-bundles/generate": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["generate_governance_doc_bundle"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/entities/{entity_id}/governance/doc-bundles/{bundle_id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["get_governance_doc_bundle"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/entities/{entity_id}/governance/incidents": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["list_incidents"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/entities/{entity_id}/governance/mode-history": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["list_governance_mode_history"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/entities/{entity_id}/governance/profile": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["get_governance_profile"];
        put: operations["update_governance_profile"];
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/entities/{entity_id}/governance/triggers": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["list_governance_triggers"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/entities/{entity_id}/intents": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["list_intents"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/entities/{entity_id}/invoices": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["list_invoices"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/entities/{entity_id}/journal-entries": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["list_journal_entries"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/entities/{entity_id}/obligations": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["list_obligations"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/entities/{entity_id}/obligations/human": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["list_human_obligations"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/entities/{entity_id}/obligations/summary": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["obligations_summary"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/entities/{entity_id}/packets": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["list_entity_packets"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/entities/{entity_id}/share-transfers": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["list_legacy_share_transfers"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/entities/{entity_id}/spending-limits": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["list_spending_limits"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/entities/{entity_id}/stripe-account": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["get_stripe_account"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/entities/{entity_id}/valuations": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["list_valuations"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/entities/{entity_id}/work-items": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["list_work_items"];
        put?: never;
        post: operations["create_work_item"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/entities/{entity_id}/work-items/{work_item_id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["get_work_item"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/entities/{entity_id}/work-items/{work_item_id}/cancel": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["cancel_work_item"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/entities/{entity_id}/work-items/{work_item_id}/claim": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["claim_work_item"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/entities/{entity_id}/work-items/{work_item_id}/complete": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["complete_work_item"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/entities/{entity_id}/work-items/{work_item_id}/release": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["release_work_item"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/equity/control-links": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["create_control_link"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/equity/control-map": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["get_control_map"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/equity/conversions/execute": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["execute_conversion"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/equity/conversions/preview": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["preview_conversion"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/equity/dilution/preview": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["get_dilution_preview"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/equity/entities": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["create_legal_entity"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/equity/fundraising-workflows": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["create_fundraising_workflow"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/equity/fundraising-workflows/{workflow_id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["get_fundraising_workflow"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/equity/fundraising-workflows/{workflow_id}/apply-terms": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["apply_fundraising_workflow_terms"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/equity/fundraising-workflows/{workflow_id}/compile-packet": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["compile_fundraising_workflow_packet"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/equity/fundraising-workflows/{workflow_id}/finalize": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["finalize_fundraising_workflow"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/equity/fundraising-workflows/{workflow_id}/generate-board-packet": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["generate_fundraising_board_packet"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/equity/fundraising-workflows/{workflow_id}/generate-closing-packet": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["generate_fundraising_closing_packet"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/equity/fundraising-workflows/{workflow_id}/prepare-execution": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["prepare_fundraising_workflow_execution"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/equity/fundraising-workflows/{workflow_id}/record-board-approval": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["record_fundraising_workflow_board_approval"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/equity/fundraising-workflows/{workflow_id}/record-close": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["record_fundraising_workflow_close"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/equity/fundraising-workflows/{workflow_id}/record-investor-acceptance": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["record_fundraising_workflow_acceptance"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/equity/fundraising-workflows/{workflow_id}/record-signature": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["record_fundraising_workflow_signature"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/equity/fundraising-workflows/{workflow_id}/start-signatures": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["start_fundraising_workflow_signatures"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/equity/grants": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["create_legacy_grant"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/equity/holders": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["create_holder"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/equity/instruments": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["create_instrument"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/equity/positions/adjust": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["adjust_position"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/equity/rounds": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["create_round"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/equity/rounds/staged": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["start_staged_round"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/equity/rounds/{round_id}/accept": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["accept_round"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/equity/rounds/{round_id}/apply-terms": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["apply_round_terms"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/equity/rounds/{round_id}/board-approve": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["board_approve_round"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/equity/rounds/{round_id}/issue": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["issue_staged_round"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/equity/rounds/{round_id}/securities": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["add_round_security"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/equity/transfer-workflows": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["create_transfer_workflow"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/equity/transfer-workflows/{workflow_id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["get_transfer_workflow"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/equity/transfer-workflows/{workflow_id}/compile-packet": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["compile_transfer_workflow_packet"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/equity/transfer-workflows/{workflow_id}/finalize": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["finalize_transfer_workflow"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/equity/transfer-workflows/{workflow_id}/generate-docs": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["generate_transfer_workflow_docs"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/equity/transfer-workflows/{workflow_id}/prepare-execution": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["prepare_transfer_workflow_execution"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/equity/transfer-workflows/{workflow_id}/record-board-approval": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["record_transfer_workflow_board_approval"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/equity/transfer-workflows/{workflow_id}/record-execution": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["record_transfer_workflow_execution"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/equity/transfer-workflows/{workflow_id}/record-review": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["record_transfer_workflow_review"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/equity/transfer-workflows/{workflow_id}/record-rofr": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["record_transfer_workflow_rofr"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/equity/transfer-workflows/{workflow_id}/record-signature": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["record_transfer_workflow_signature"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/equity/transfer-workflows/{workflow_id}/start-signatures": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["start_transfer_workflow_signatures"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/equity/transfer-workflows/{workflow_id}/submit-review": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["submit_transfer_workflow_for_review"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/equity/workflows/{workflow_type}/{workflow_id}/status": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["get_workflow_status"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/execution/approval-artifacts": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["create_approval_artifact"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/execution/intents": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["create_intent"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/execution/obligations": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["create_obligation"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/execution/packets/{packet_id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["get_packet"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/formations": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["create_formation"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/formations/pending": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["create_pending_formation"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/formations/with-cap-table": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["create_formation_with_cap_table"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/formations/{entity_id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["get_formation"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/formations/{entity_id}/apply-ein": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["apply_ein"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/formations/{entity_id}/documents": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["list_documents"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/formations/{entity_id}/ein-confirmation": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["confirm_ein"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/formations/{entity_id}/filing-attestation": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["record_filing_attestation"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/formations/{entity_id}/filing-confirmation": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["confirm_filing"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/formations/{entity_id}/finalize": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["finalize_pending_formation"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/formations/{entity_id}/founders": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["add_founder"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/formations/{entity_id}/gates": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["get_formation_gates"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/formations/{entity_id}/mark-documents-signed": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["mark_documents_signed"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/formations/{entity_id}/registered-agent-consent-evidence": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["add_registered_agent_consent_evidence"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/formations/{entity_id}/service-agreement/execute": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["execute_service_agreement"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/formations/{entity_id}/submit-filing": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["submit_filing"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/governance-bodies": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["list_all_governance_bodies"];
        put?: never;
        post: operations["create_governance_body"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/governance-bodies/{body_id}/meetings": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["list_meetings"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/governance-bodies/{body_id}/seats": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["list_seats"];
        put?: never;
        post: operations["create_seat"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/governance-seats/scan-expired": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["scan_expired_seats"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/governance-seats/{seat_id}/resign": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["resign_seat"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/governance/audit/checkpoints": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["write_governance_audit_checkpoint"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/governance/audit/events": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["create_governance_audit_event"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/governance/audit/verify": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["verify_governance_audit_chain"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/governance/delegation-schedule": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["get_delegation_schedule"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/governance/delegation-schedule/amend": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["amend_delegation_schedule"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/governance/delegation-schedule/history": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["list_delegation_schedule_history"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/governance/delegation-schedule/reauthorize": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["reauthorize_delegation_schedule"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/governance/evaluate": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["evaluate_governance"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/governance/incidents": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["create_incident"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/governance/incidents/{incident_id}/resolve": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["resolve_incident"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/governance/mode": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["get_governance_mode"];
        put?: never;
        post: operations["set_governance_mode"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/human-obligations": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["list_global_human_obligations"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/human-obligations/{obligation_id}/fulfill": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["fulfill_human_obligation"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/human-obligations/{obligation_id}/signer-token": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["generate_signer_token"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/human/sign/{document_id}/pdf": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["get_signing_pdf"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/human/sign/{document_id}/resolve": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["resolve_signing_link"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/human/sign/{document_id}/submit": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["submit_signing"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/intents/{intent_id}/authorize": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["authorize_intent"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/intents/{intent_id}/bind-approval-artifact": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["bind_approval_artifact_to_intent"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/intents/{intent_id}/bind-document-request": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["bind_document_request_to_intent"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/intents/{intent_id}/cancel": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["cancel_intent"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/intents/{intent_id}/evaluate": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["evaluate_intent"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/intents/{intent_id}/execute": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["execute_intent"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/intents/{intent_id}/receipts": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["list_receipts_by_intent"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/internal/agent-token": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["mint_agent_token"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/internal/agents/active": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["list_active_agents_internal"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/internal/agents/{agent_id}/resolved": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["get_resolved_agent_internal"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/internal/resolve-secrets": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Internal endpoint: resolve encrypted secrets for agent execution.
         * @description Called by the worker to get plaintext secret values for opaque token creation.
         *     For `"self"` proxies, decrypts from git. For external proxies, returns the URL
         *     so the worker can forward requests there.
         */
        post: operations["resolve_secrets"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/internal/workspaces/{workspace_id}/entities/{entity_id}/governance/triggers/lockdown": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["ingest_lockdown_trigger"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/invoices/from-agent-request": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["from_agent_request"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/invoices/{invoice_id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["get_invoice"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/invoices/{invoice_id}/mark-paid": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["mark_invoice_paid"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/invoices/{invoice_id}/pay-instructions": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["get_pay_instructions"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/invoices/{invoice_id}/send": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["send_invoice"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/invoices/{invoice_id}/status": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["get_invoice_status"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/journal-entries/{entry_id}/post": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["post_journal_entry"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/journal-entries/{entry_id}/void": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["void_journal_entry"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/jwks": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["get_jwks"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/ledger/reconcile": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["reconcile_ledger"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/llm/proxy/{path}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["proxy_handler"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/meetings": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["list_all_meetings"];
        put?: never;
        post: operations["schedule_meeting"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/meetings/written-consent": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["written_consent"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/meetings/{meeting_id}/adjourn": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["adjourn_meeting"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/meetings/{meeting_id}/agenda-items": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["list_agenda_items"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/meetings/{meeting_id}/agenda-items/{item_id}/finalize": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["finalize_agenda_item"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/meetings/{meeting_id}/agenda-items/{item_id}/resolution": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["compute_resolution"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/meetings/{meeting_id}/agenda-items/{item_id}/vote": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["cast_vote"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/meetings/{meeting_id}/agenda-items/{item_id}/votes": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["list_votes"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/meetings/{meeting_id}/cancel": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["cancel_meeting"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/meetings/{meeting_id}/convene": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["convene_meeting"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/meetings/{meeting_id}/notice": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["send_notice"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/meetings/{meeting_id}/resolutions": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["list_resolutions"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/meetings/{meeting_id}/resolutions/{resolution_id}/attach-document": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["attach_resolution_document"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/obligations/summary": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["global_obligations_summary"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/obligations/{obligation_id}/assign": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["assign_obligation"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/obligations/{obligation_id}/document-requests": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["list_document_requests"];
        put?: never;
        post: operations["create_document_request"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/obligations/{obligation_id}/expire": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["expire_obligation"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/obligations/{obligation_id}/fulfill": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["fulfill_obligation"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/obligations/{obligation_id}/waive": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["waive_obligation"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/payments": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["submit_payment"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/payments/execute": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["execute_payment"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/payroll/runs": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["create_payroll_run"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/receipts/{receipt_id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["get_receipt"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/secrets/interpolate": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["interpolate_template"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/secrets/resolve": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["resolve_token"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/service-token": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["get_service_token"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/share-transfers": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["create_legacy_share_transfer"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/sign/{document_id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["get_signing_link"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/spending-limits": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["create_spending_limit"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/tax/filings": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["file_tax_document"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/treasury/accounts": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["create_account"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/treasury/bank-accounts": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["create_bank_account"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/treasury/chart-of-accounts/{entity_id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["get_chart_of_accounts"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/treasury/invoices": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["create_invoice"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/treasury/journal-entries": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["create_journal_entry"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/treasury/payment-intents": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["create_payment_intent"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/treasury/payouts": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["create_payout"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/treasury/seed-chart-of-accounts": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["seed_chart_of_accounts"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/treasury/stripe-accounts": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["create_stripe_account"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/treasury/webhooks/stripe": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["treasury_stripe_webhook"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/valuations": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["create_valuation"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/valuations/{valuation_id}/approve": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["approve_valuation"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/valuations/{valuation_id}/submit-for-approval": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["submit_valuation_for_approval"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/workspace/entities": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["list_workspace_entities"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/workspace/status": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["workspace_status"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/workspaces/claim": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["claim_workspace"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/workspaces/link": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["link_workspace"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/workspaces/provision": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["provision_workspace"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/workspaces/{workspace_id}/contacts": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["workspace_contacts"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/workspaces/{workspace_id}/entities": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["workspace_entities_by_path"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/workspaces/{workspace_id}/secret-proxies": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["list_proxies"];
        put?: never;
        post: operations["create_proxy"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/workspaces/{workspace_id}/secret-proxies/{proxy_name}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["get_proxy"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/workspaces/{workspace_id}/secret-proxies/{proxy_name}/secrets": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["list_secret_names"];
        put: operations["set_secrets"];
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/workspaces/{workspace_id}/status": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["workspace_status_by_path"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
};
export type webhooks = Record<string, never>;
export type components = {
    schemas: {
        AcceptRoundRequest: {
            accepted_by_contact_id?: null | components["schemas"]["ContactId"];
            entity_id: components["schemas"]["EntityId"];
            intent_id: components["schemas"]["IntentId"];
        };
        /** Format: uuid */
        AccountId: string;
        AccountResponse: {
            account_code: components["schemas"]["GlAccountCode"];
            account_id: components["schemas"]["AccountId"];
            account_name: string;
            account_type: components["schemas"]["AccountType"];
            created_at: string;
            currency: components["schemas"]["Currency"];
            entity_id: components["schemas"]["EntityId"];
            is_active: boolean;
            normal_balance: components["schemas"]["Side"];
        };
        /**
         * @description The five fundamental accounting categories.
         * @enum {string}
         */
        AccountType: "asset" | "liability" | "equity" | "revenue" | "expense";
        AddFounderRequest: {
            address?: null | components["schemas"]["Address"];
            email?: string | null;
            is_incorporator?: boolean | null;
            name: string;
            officer_title?: null | components["schemas"]["OfficerTitle"];
            /** Format: double */
            ownership_pct?: number | null;
            role?: null | components["schemas"]["MemberRole"];
        };
        AddFounderResponse: {
            entity_id: components["schemas"]["EntityId"];
            member_count: number;
            members: components["schemas"]["FounderSummary"][];
        };
        AddSecurityRequest: {
            email?: string | null;
            entity_id: components["schemas"]["EntityId"];
            grant_type?: string | null;
            holder_id?: null | components["schemas"]["HolderId"];
            instrument_id: components["schemas"]["InstrumentId"];
            /** Format: int64 */
            principal_cents?: number;
            /** Format: int64 */
            quantity: number;
            recipient_name: string;
        };
        AddSkillRequest: {
            description: string;
            /** @description Parsed at deserialization — empty names are rejected by `NonEmpty`. */
            name: components["schemas"]["NonEmpty"];
            parameters?: Record<string, never>;
        };
        /** @description A mailing address. */
        Address: {
            city: string;
            state: string;
            street: string;
            street2?: string | null;
            zip: string;
        };
        AdjustPositionRequest: {
            entity_id: components["schemas"]["EntityId"];
            holder_id: components["schemas"]["HolderId"];
            instrument_id: components["schemas"]["InstrumentId"];
            issuer_legal_entity_id: components["schemas"]["LegalEntityId"];
            /** Format: int64 */
            principal_delta_cents?: number;
            /** Format: int64 */
            quantity_delta: number;
            source_reference?: string | null;
        };
        /** Format: uuid */
        AgendaItemId: string;
        AgendaItemResponse: {
            agenda_item_id: components["schemas"]["AgendaItemId"];
            created_at: string;
            description?: string | null;
            item_type: components["schemas"]["AgendaItemType"];
            meeting_id: components["schemas"]["MeetingId"];
            /** Format: int32 */
            sequence_number: number;
            status: components["schemas"]["AgendaItemStatus"];
            title: string;
        };
        /**
         * @description Status of an agenda item.
         * @enum {string}
         */
        AgendaItemStatus: "pending" | "discussed" | "voted" | "tabled" | "withdrawn";
        /**
         * @description Type of item on a meeting agenda.
         * @enum {string}
         */
        AgendaItemType: "resolution" | "discussion" | "report" | "election";
        /** Format: uuid */
        AgentId: string;
        AgentInvoiceRequest: {
            /** Format: int64 */
            amount_cents: number;
            customer_name: string;
            description: string;
            /** Format: date */
            due_date: string;
            entity_id: components["schemas"]["EntityId"];
        };
        AgentResponse: {
            agent_id: components["schemas"]["AgentId"];
            budget?: null | components["schemas"]["BudgetConfig"];
            channels: components["schemas"]["ChannelConfig"][];
            created_at: string;
            email_address?: string | null;
            entity_id?: null | components["schemas"]["EntityId"];
            mcp_servers: components["schemas"]["MCPServerSpec"][];
            model?: string | null;
            name: string;
            parent_agent_id?: null | components["schemas"]["AgentId"];
            sandbox?: null | components["schemas"]["SandboxConfig"];
            scopes: components["schemas"]["Scope"][];
            skills: components["schemas"]["AgentSkill"][];
            status: components["schemas"]["AgentStatus"];
            system_prompt?: string | null;
            tools: components["schemas"]["ToolSpec"][];
            webhook_url?: string | null;
            workspace_id: components["schemas"]["WorkspaceId"];
        };
        /** @description A skill that an agent can perform (used in api-rs Agent struct). */
        AgentSkill: {
            description: string;
            name: components["schemas"]["NonEmpty"];
            parameters?: Record<string, never>;
        };
        /** @enum {string} */
        AgentStatus: "active" | "paused" | "disabled";
        AmendDelegationScheduleRequest: {
            adopted_resolution_id?: null | components["schemas"]["ResolutionId"];
            allowed_tier1_intent_types?: string[] | null;
            entity_id: components["schemas"]["EntityId"];
            meeting_id?: null | components["schemas"]["MeetingId"];
            /** Format: date */
            next_mandatory_review_at?: string | null;
            rationale?: string | null;
            /** Format: int64 */
            tier1_max_amount_cents?: number | null;
        };
        AmendmentHistoryEntry: {
            amended_at: string;
            description: string;
            /** Format: int32 */
            version: number;
        };
        /** @enum {string} */
        AntiDilutionMethod: "none" | "broad_based_weighted_average" | "narrow_based_weighted_average" | "full_ratchet";
        /** Format: uuid */
        ApiKeyId: string;
        ApiKeyResponse: {
            contact_id?: null | components["schemas"]["ContactId"];
            created_at: string;
            entity_ids?: components["schemas"]["EntityId"][] | null;
            key_id: components["schemas"]["ApiKeyId"];
            name: string;
            raw_key?: string | null;
            scopes: components["schemas"]["Scope"][];
            workspace_id: components["schemas"]["WorkspaceId"];
        };
        ApplyFundraisingTermsRequest: {
            anti_dilution_method: components["schemas"]["AntiDilutionMethod"];
            conversion_precedence?: components["schemas"]["InstrumentKind"][];
            entity_id: components["schemas"]["EntityId"];
            protective_provisions?: Record<string, never>;
        };
        ApplyRoundTermsRequest: {
            anti_dilution_method: components["schemas"]["AntiDilutionMethod"];
            conversion_precedence?: components["schemas"]["InstrumentKind"][];
            entity_id: components["schemas"]["EntityId"];
            protective_provisions?: Record<string, never>;
        };
        /** Format: uuid */
        ApprovalArtifactId: string;
        ApprovalArtifactResponse: {
            approval_artifact_id: components["schemas"]["ApprovalArtifactId"];
            approved_at: string;
            approver_identity: string;
            channel: string;
            created_at: string;
            entity_id: components["schemas"]["EntityId"];
            expires_at?: string | null;
            explicit: boolean;
            intent_type: string;
            /** Format: int64 */
            max_amount_cents?: number | null;
            revoked_at?: string | null;
            scope: string;
        };
        ApproveValuationRequest: {
            entity_id: components["schemas"]["EntityId"];
            resolution_id?: null | components["schemas"]["ResolutionId"];
        };
        AssignObligationRequest: {
            assignee_id: components["schemas"]["ContactId"];
            entity_id: components["schemas"]["EntityId"];
        };
        /**
         * @description Who is responsible for fulfilling an obligation.
         * @enum {string}
         */
        AssigneeType: "internal" | "third_party" | "human";
        AttachResolutionDocumentRequest: {
            document_id: components["schemas"]["DocumentId"];
            entity_id: components["schemas"]["EntityId"];
        };
        AuditEvent: {
            details: Record<string, never>;
            event_id: string;
            event_type: string;
            timestamp: string;
        };
        /** @enum {string} */
        AuthoritySource: "law" | "charter" | "governance_docs" | "resolution" | "directive" | "standing_instruction" | "delegation_schedule" | "heuristic";
        /**
         * @description The authority level required to approve an action.
         * @enum {string}
         */
        AuthorityTier: "tier_1" | "tier_2" | "tier_3";
        /** Format: uuid */
        BankAccountId: string;
        BankAccountResponse: {
            account_type: components["schemas"]["BankAccountType"];
            bank_account_id: components["schemas"]["BankAccountId"];
            bank_name: string;
            created_at: string;
            currency: components["schemas"]["Currency"];
            entity_id: components["schemas"]["EntityId"];
            status: components["schemas"]["BankAccountStatus"];
        };
        /**
         * @description Lifecycle status of a bank account connection.
         * @enum {string}
         */
        BankAccountStatus: "pending_review" | "active" | "closed";
        /**
         * @description Type of bank account held by the entity.
         * @enum {string}
         */
        BankAccountType: "checking" | "savings";
        BindApprovalArtifactRequest: {
            approval_artifact_id: components["schemas"]["ApprovalArtifactId"];
            entity_id: components["schemas"]["EntityId"];
        };
        BindDocumentRequestRequest: {
            entity_id: components["schemas"]["EntityId"];
            request_id: components["schemas"]["DocumentRequestId"];
        };
        BoardApproveRoundRequest: {
            entity_id: components["schemas"]["EntityId"];
            meeting_id: components["schemas"]["MeetingId"];
            resolution_id: components["schemas"]["ResolutionId"];
        };
        BodyQuery: {
            entity_id: components["schemas"]["EntityId"];
        };
        /**
         * @description Whether a governance body is active.
         * @enum {string}
         */
        BodyStatus: "active" | "inactive";
        /**
         * @description The type of governance body.
         * @enum {string}
         */
        BodyType: "board_of_directors" | "llc_member_vote";
        BranchListEntry: {
            head_oid: string;
            name: string;
        };
        /**
         * @description A validated git branch name.
         *
         *     Guarantees: non-empty, no `..`, no spaces, no leading `-`, no null bytes.
         *     Implements `Deref<Target=str>` for transparent use with `&str` parameters.
         */
        BranchName: string;
        /**
         * @description Execution budget limits.
         *
         *     All limits are validated positive on deserialization — a budget of
         *     zero turns or zero tokens is nonsensical and rejected at parse time.
         */
        BudgetConfig: {
            /** Format: int64 */
            max_monthly_cost_cents?: number;
            /** Format: int64 */
            max_tokens?: number;
            /** Format: int32 */
            max_turns?: number;
        };
        /**
         * @description Level of cap table visibility granted to a contact.
         * @enum {string}
         */
        CapTableAccess: "none" | "summary" | "detailed";
        /** @enum {string} */
        CapTableBasis: "outstanding" | "as_converted" | "fully_diluted";
        CapTableHolderSummary: {
            /** Format: int32 */
            as_converted_bps: number;
            /** Format: int64 */
            as_converted_units: number;
            /** Format: int32 */
            fully_diluted_bps: number;
            /** Format: int64 */
            fully_diluted_units: number;
            holder_id: components["schemas"]["HolderId"];
            name: string;
            /** Format: int32 */
            outstanding_bps: number;
            /** Format: int64 */
            outstanding_units: number;
        };
        CapTableInstrumentSummary: {
            /** Format: int64 */
            authorized_units?: number | null;
            /** Format: int64 */
            diluted_units: number;
            instrument_id: components["schemas"]["InstrumentId"];
            /** Format: int64 */
            issued_units: number;
            kind: components["schemas"]["InstrumentKind"];
            symbol: string;
        };
        CapTableQuery: {
            basis?: components["schemas"]["CapTableBasis"];
            issuer_legal_entity_id?: null | components["schemas"]["LegalEntityId"];
        };
        CapTableResponse: {
            basis: components["schemas"]["CapTableBasis"];
            entity_id: components["schemas"]["EntityId"];
            generated_at: string;
            holders: components["schemas"]["CapTableHolderSummary"][];
            instruments: components["schemas"]["CapTableInstrumentSummary"][];
            issuer_legal_entity_id: components["schemas"]["LegalEntityId"];
            /** Format: int64 */
            total_units: number;
        };
        CastVoteRequest: {
            vote_value: components["schemas"]["VoteValue"];
            voter_id: components["schemas"]["ContactId"];
        };
        /**
         * @description Inbound channel configuration — a tagged enum so that each variant
         *     carries only the fields it needs.
         *
         *     The key invariant: a `Cron` channel *always* has a valid schedule.
         *     This is unrepresentable with the old struct approach (schedule was
         *     `Option<String>`, allowing a cron channel with no schedule).
         *
         *     Wire format is the same as before — internally tagged on `"type"`:
         *     ```json
         *     {"type": "cron", "schedule": "*\/5 * * * *"}
         *     {"type": "email", "address": "bot@acme.com"}
         *     ```
         */
        ChannelConfig: {
            address?: string | null;
            /** @enum {string} */
            type: "email";
            webhook_secret?: string | null;
        } | {
            address?: string | null;
            /** @enum {string} */
            type: "webhook";
            webhook_secret?: string | null;
        } | {
            schedule: components["schemas"]["CronExpr"];
            /** @enum {string} */
            type: "cron";
        } | {
            /** @enum {string} */
            type: "manual";
        };
        ChartOfAccountsResponse: {
            accounts: components["schemas"]["AccountResponse"][];
            entity_id: components["schemas"]["EntityId"];
        };
        ClaimWorkItemRequest: {
            claimed_by: string;
            /** Format: int64 */
            ttl_seconds?: number | null;
        };
        /** Format: uuid */
        ClassificationId: string;
        ClassificationResponse: {
            classification: components["schemas"]["ClassificationResult"];
            classification_id: components["schemas"]["ClassificationId"];
            contractor_name: string;
            created_at: string;
            entity_id: components["schemas"]["EntityId"];
            flags: string[];
            risk_level: components["schemas"]["RiskLevel"];
            state: string;
        };
        /**
         * @description Classification result.
         * @enum {string}
         */
        ClassificationResult: "independent" | "employee" | "uncertain";
        ClassifyContractorRequest: {
            contractor_name: string;
            entity_id: components["schemas"]["EntityId"];
            factors?: Record<string, never>;
            state?: string;
        };
        CompanyAddress: {
            city: string;
            county?: string | null;
            state: string;
            street: string;
            zip: string;
        };
        CompileWorkflowPacketRequest: {
            entity_id: components["schemas"]["EntityId"];
            phase?: string | null;
            required_signers?: string[];
        };
        CompleteWorkItemRequest: {
            completed_by: string;
            result?: string | null;
        };
        /** Format: uuid */
        ComplianceEscalationId: string;
        ComplianceEscalationResponse: {
            action: string;
            authority: string;
            created_at: string;
            deadline_id: components["schemas"]["DeadlineId"];
            entity_id: components["schemas"]["EntityId"];
            escalation_id: components["schemas"]["ComplianceEscalationId"];
            incident_id?: null | components["schemas"]["IncidentId"];
            milestone: string;
            obligation_id?: null | components["schemas"]["ObligationId"];
            status: components["schemas"]["EscalationStatus"];
        };
        /** Format: uuid */
        ComplianceEvidenceLinkId: string;
        ComplianceScanResponse: {
            escalations_created: number;
            incidents_created: number;
            scanned_deadlines: number;
        };
        ComputeResolutionRequest: {
            /** Format: date */
            effective_date?: string | null;
            resolution_text: string;
        };
        ConfigResponse: {
            environment: string;
            features: string[];
            version: string;
        };
        ConfirmEinRequest: {
            ein: string;
        };
        ConfirmFilingRequest: {
            external_filing_id: string;
            receipt_reference?: string | null;
        };
        /**
         * @description The role or relationship a contact has with the entity.
         * @enum {string}
         */
        ContactCategory: "employee" | "contractor" | "board_member" | "law_firm" | "valuation_firm" | "accounting_firm" | "investor" | "officer" | "founder" | "member" | "other";
        /** Format: uuid */
        ContactId: string;
        ContactProfileResponse: {
            category: components["schemas"]["ContactCategory"];
            contact_id: components["schemas"]["ContactId"];
            email?: string | null;
            entities: components["schemas"]["EntityId"][];
            mailing_address?: string | null;
            name: string;
            notes?: string | null;
            phone?: string | null;
        };
        ContactResponse: {
            cap_table_access: components["schemas"]["CapTableAccess"];
            category: components["schemas"]["ContactCategory"];
            contact_id: components["schemas"]["ContactId"];
            contact_type: components["schemas"]["ContactType"];
            created_at: string;
            email?: string | null;
            entity_id: components["schemas"]["EntityId"];
            mailing_address?: string | null;
            name: string;
            notes?: string | null;
            phone?: string | null;
            status: components["schemas"]["ContactStatus"];
        };
        /**
         * @description Whether a contact record is active.
         * @enum {string}
         */
        ContactStatus: "active" | "inactive";
        /**
         * @description Whether a contact is a person or an organization.
         * @enum {string}
         */
        ContactType: "individual" | "organization";
        /** Format: uuid */
        ContractId: string;
        ContractResponse: {
            contract_id: components["schemas"]["ContractId"];
            counterparty_name: string;
            created_at: string;
            document_id: components["schemas"]["DocumentId"];
            effective_date: string;
            entity_id: components["schemas"]["EntityId"];
            status: components["schemas"]["ContractStatus"];
            template_type: components["schemas"]["ContractTemplateType"];
        };
        /**
         * @description Lifecycle status of a contract.
         * @enum {string}
         */
        ContractStatus: "draft" | "active" | "expired" | "terminated";
        /**
         * @description Template type for generated contracts.
         * @enum {string}
         */
        ContractTemplateType: "consulting_agreement" | "employment_offer" | "contractor_agreement" | "nda" | "safe_agreement" | "custom";
        /** Format: uuid */
        ControlLinkId: string;
        ControlLinkResponse: {
            child_legal_entity_id: components["schemas"]["LegalEntityId"];
            control_link_id: components["schemas"]["ControlLinkId"];
            control_type: components["schemas"]["ControlType"];
            created_at: string;
            parent_legal_entity_id: components["schemas"]["LegalEntityId"];
            /** Format: int32 */
            voting_power_bps?: number | null;
        };
        ControlMapEdge: {
            child_legal_entity_id: components["schemas"]["LegalEntityId"];
            control_type: components["schemas"]["ControlType"];
            parent_legal_entity_id: components["schemas"]["LegalEntityId"];
            /** Format: int32 */
            voting_power_bps?: number | null;
        };
        ControlMapQuery: {
            entity_id: components["schemas"]["EntityId"];
            root_entity_id: components["schemas"]["LegalEntityId"];
        };
        ControlMapResponse: {
            edges: components["schemas"]["ControlMapEdge"][];
            root_entity_id: components["schemas"]["LegalEntityId"];
            traversed_entities: components["schemas"]["LegalEntityId"][];
        };
        /**
         * @description Type of control relationship.
         * @enum {string}
         */
        ControlType: "voting" | "board" | "economic" | "contractual";
        ConveneMeetingRequest: {
            present_seat_ids: components["schemas"]["GovernanceSeatId"][];
        };
        ConversionExecuteResponse: {
            conversion_execution_id: components["schemas"]["ConversionExecutionId"];
            converted_positions: number;
            round_id: components["schemas"]["EquityRoundId"];
            target_positions_touched: number;
            /** Format: int64 */
            total_new_units: number;
        };
        /** Format: uuid */
        ConversionExecutionId: string;
        ConversionPreviewLine: {
            basis: string;
            /** Format: int64 */
            conversion_price_cents: number;
            holder_id: components["schemas"]["HolderId"];
            instrument_id: components["schemas"]["InstrumentId"];
            /** Format: int64 */
            new_units: number;
            /** Format: int64 */
            principal_cents: number;
            source_position_id: components["schemas"]["PositionId"];
        };
        ConversionPreviewResponse: {
            /** Format: int64 */
            anti_dilution_adjustment_units: number;
            entity_id: components["schemas"]["EntityId"];
            lines: components["schemas"]["ConversionPreviewLine"][];
            round_id: components["schemas"]["EquityRoundId"];
            target_instrument_id: components["schemas"]["InstrumentId"];
            /** Format: int64 */
            total_new_units: number;
        };
        ConvertEntityRequest: {
            jurisdiction?: null | components["schemas"]["Jurisdiction"];
            target_type: components["schemas"]["EntityType"];
        };
        CreateAccountRequest: {
            account_code: components["schemas"]["GlAccountCode"];
            entity_id: components["schemas"]["EntityId"];
        };
        CreateAgentRequest: {
            entity_id?: null | components["schemas"]["EntityId"];
            model?: string | null;
            name: string;
            parent_agent_id?: null | components["schemas"]["AgentId"];
            scopes?: components["schemas"]["Scope"][];
            system_prompt?: string | null;
        };
        CreateApiKeyRequest: {
            contact_id?: null | components["schemas"]["ContactId"];
            /** @description Restrict this key to specific entities. `null` = all entities. */
            entity_ids?: components["schemas"]["EntityId"][] | null;
            name: string;
            scopes?: components["schemas"]["Scope"][];
        };
        CreateApprovalArtifactRequest: {
            /** Format: date-time */
            approved_at?: string | null;
            approver_identity: string;
            channel: string;
            entity_id: components["schemas"]["EntityId"];
            /** Format: date-time */
            expires_at?: string | null;
            explicit?: boolean;
            intent_type: string;
            /** Format: int64 */
            max_amount_cents?: number | null;
            scope: string;
        };
        CreateBankAccountRequest: {
            account_type?: null | components["schemas"]["BankAccountType"];
            bank_name: string;
            entity_id: components["schemas"]["EntityId"];
        };
        CreateBranchRequest: {
            from?: components["schemas"]["BranchName"];
            name: components["schemas"]["BranchName"];
        };
        CreateBranchResponse: {
            base_commit: string;
            branch: string;
        };
        CreateContactRequest: {
            category: components["schemas"]["ContactCategory"];
            contact_type: components["schemas"]["ContactType"];
            email?: string | null;
            entity_id: components["schemas"]["EntityId"];
            mailing_address?: string | null;
            name: string;
            notes?: string | null;
        };
        CreateControlLinkRequest: {
            child_legal_entity_id: components["schemas"]["LegalEntityId"];
            control_type: components["schemas"]["ControlType"];
            entity_id: components["schemas"]["EntityId"];
            notes?: string | null;
            parent_legal_entity_id: components["schemas"]["LegalEntityId"];
            /** Format: int32 */
            voting_power_bps?: number | null;
        };
        CreateDeadlineRequest: {
            deadline_type: string;
            description: string;
            /** Format: date */
            due_date: string;
            entity_id: components["schemas"]["EntityId"];
            recurrence?: components["schemas"]["Recurrence"];
            severity?: components["schemas"]["DeadlineSeverity"];
        };
        CreateDistributionRequest: {
            description: string;
            distribution_type?: components["schemas"]["DistributionType"];
            entity_id: components["schemas"]["EntityId"];
            /** Format: int64 */
            total_amount_cents: number;
        };
        CreateDocumentRequestPayload: {
            description: string;
            document_type: string;
            entity_id: components["schemas"]["EntityId"];
        };
        CreateFormationRequest: {
            /** Format: int64 */
            authorized_shares?: number | null;
            company_address?: null | components["schemas"]["Address"];
            entity_type: components["schemas"]["EntityType"];
            /** @description Fiscal year end, e.g. "12-31". Defaults to "12-31". */
            fiscal_year_end?: string | null;
            /** @description Optional formation date for importing pre-formed entities. */
            formation_date?: string | null;
            jurisdiction: components["schemas"]["Jurisdiction"];
            legal_name: string;
            members: components["schemas"]["MemberInput"][];
            par_value?: string | null;
            registered_agent_address?: string | null;
            registered_agent_name?: string | null;
            /** @description Include right of first refusal in bylaws (corp). Default true. */
            right_of_first_refusal?: boolean | null;
            /** @description Whether the company will elect S-Corp tax treatment. */
            s_corp_election?: boolean | null;
            /** @description Include transfer restrictions in bylaws (corp). Default true. */
            transfer_restrictions?: boolean | null;
        };
        CreateFundraisingWorkflowRequest: {
            conversion_target_instrument_id?: null | components["schemas"]["InstrumentId"];
            entity_id: components["schemas"]["EntityId"];
            issuer_legal_entity_id: components["schemas"]["LegalEntityId"];
            metadata?: Record<string, never>;
            name: string;
            /** Format: int64 */
            pre_money_cents?: number | null;
            prepare_intent_id: components["schemas"]["IntentId"];
            /** Format: int64 */
            round_price_cents?: number | null;
            /** Format: int64 */
            target_raise_cents?: number | null;
        };
        CreateGovernanceAuditEventRequest: {
            action: string;
            details?: Record<string, never>;
            entity_id: components["schemas"]["EntityId"];
            event_type: components["schemas"]["GovernanceAuditEventType"];
            evidence_refs?: string[];
            linked_incident_id?: null | components["schemas"]["IncidentId"];
            linked_intent_id?: null | components["schemas"]["IntentId"];
            linked_mode_event_id?: null | components["schemas"]["GovernanceModeEventId"];
            linked_trigger_id?: null | components["schemas"]["GovernanceTriggerId"];
        };
        CreateGovernanceBodyRequest: {
            body_type: components["schemas"]["BodyType"];
            entity_id: components["schemas"]["EntityId"];
            name: string;
            quorum_rule: components["schemas"]["QuorumThreshold"];
            voting_method: components["schemas"]["VotingMethod"];
        };
        CreateHolderRequest: {
            contact_id: components["schemas"]["ContactId"];
            entity_id: components["schemas"]["EntityId"];
            external_reference?: string | null;
            holder_type: components["schemas"]["HolderType"];
            linked_entity_id?: null | components["schemas"]["EntityId"];
            name: string;
        };
        CreateIncidentRequest: {
            description: string;
            entity_id: components["schemas"]["EntityId"];
            severity: components["schemas"]["IncidentSeverity"];
            title: string;
        };
        CreateInstrumentRequest: {
            /** Format: int64 */
            authorized_units?: number | null;
            entity_id: components["schemas"]["EntityId"];
            /** Format: int64 */
            issue_price_cents?: number | null;
            issuer_legal_entity_id: components["schemas"]["LegalEntityId"];
            kind: components["schemas"]["InstrumentKind"];
            symbol: string;
            terms?: Record<string, never>;
        };
        CreateIntentRequest: {
            authority_tier?: null | components["schemas"]["AuthorityTier"];
            description: string;
            entity_id: components["schemas"]["EntityId"];
            intent_type: string;
            metadata?: Record<string, never>;
        };
        CreateInvoiceRequest: {
            /** Format: int64 */
            amount_cents: number;
            customer_name: string;
            description: string;
            /** Format: date */
            due_date: string;
            entity_id: components["schemas"]["EntityId"];
        };
        CreateJournalEntryRequest: {
            description: string;
            /** Format: date */
            effective_date: string;
            entity_id: components["schemas"]["EntityId"];
            lines: components["schemas"]["LedgerLineRequest"][];
        };
        CreateLegacyGrantRequest: {
            entity_id: components["schemas"]["EntityId"];
            grant_type: components["schemas"]["GrantType"];
            recipient_name: string;
            /** Format: int64 */
            shares: number;
        };
        CreateLegacyShareTransferRequest: {
            entity_id: components["schemas"]["EntityId"];
            from_holder: string;
            governing_doc_type?: null | components["schemas"]["GoverningDocType"];
            share_class_id: components["schemas"]["ShareClassId"];
            /** Format: int64 */
            shares: number;
            to_holder: string;
            transfer_type: components["schemas"]["TransferType"];
            transferee_rights?: null | components["schemas"]["TransfereeRights"];
        };
        CreateLegalEntityRequest: {
            entity_id: components["schemas"]["EntityId"];
            linked_entity_id?: null | components["schemas"]["EntityId"];
            name: string;
            role: components["schemas"]["LegalEntityRole"];
        };
        CreateObligationRequest: {
            assignee_id?: null | components["schemas"]["ContactId"];
            assignee_type: components["schemas"]["AssigneeType"];
            description: string;
            /** Format: date */
            due_date?: string | null;
            entity_id: components["schemas"]["EntityId"];
            intent_id?: null | components["schemas"]["IntentId"];
            obligation_type: string;
        };
        CreatePaymentIntentRequest: {
            /** Format: int64 */
            amount_cents: number;
            currency?: string | null;
            description?: string | null;
            entity_id: components["schemas"]["EntityId"];
        };
        CreatePayoutRequest: {
            /** Format: int64 */
            amount_cents: number;
            description?: string | null;
            destination: string;
            entity_id: components["schemas"]["EntityId"];
        };
        CreatePayrollRunRequest: {
            entity_id: components["schemas"]["EntityId"];
            /** Format: date */
            pay_period_end: string;
            /** Format: date */
            pay_period_start: string;
        };
        CreatePendingFormationRequest: {
            company_address?: null | components["schemas"]["Address"];
            entity_type: components["schemas"]["EntityType"];
            fiscal_year_end?: string | null;
            formation_date?: string | null;
            jurisdiction?: null | components["schemas"]["Jurisdiction"];
            legal_name: string;
            registered_agent_address?: string | null;
            registered_agent_name?: string | null;
            right_of_first_refusal?: boolean | null;
            s_corp_election?: boolean | null;
            transfer_restrictions?: boolean | null;
        };
        CreateProxyRequest: {
            description?: string | null;
            name: string;
            /** @description `"self"` for local encrypted secrets, or an external URL. */
            url: string;
        };
        CreateRoundRequest: {
            conversion_target_instrument_id?: null | components["schemas"]["InstrumentId"];
            entity_id: components["schemas"]["EntityId"];
            issuer_legal_entity_id: components["schemas"]["LegalEntityId"];
            metadata?: Record<string, never>;
            name: string;
            /** Format: int64 */
            pre_money_cents?: number | null;
            /** Format: int64 */
            round_price_cents?: number | null;
            /** Format: int64 */
            target_raise_cents?: number | null;
        };
        CreateSeatRequest: {
            /** Format: date */
            appointed_date?: string | null;
            holder_id: components["schemas"]["ContactId"];
            role: components["schemas"]["SeatRole"];
            /** Format: date */
            term_expiration?: string | null;
            /** Format: int32 */
            voting_power?: number | null;
        };
        CreateSpendingLimitRequest: {
            /** Format: int64 */
            amount_cents: number;
            category: string;
            entity_id: components["schemas"]["EntityId"];
            period: string;
        };
        CreateStripeAccountRequest: {
            entity_id: components["schemas"]["EntityId"];
        };
        CreateTransferWorkflowRequest: {
            entity_id: components["schemas"]["EntityId"];
            from_contact_id: components["schemas"]["ContactId"];
            governing_doc_type: components["schemas"]["GoverningDocType"];
            prepare_intent_id: components["schemas"]["IntentId"];
            /** Format: int64 */
            price_per_share_cents?: number | null;
            relationship_to_holder?: string | null;
            share_class_id: components["schemas"]["ShareClassId"];
            /** Format: int64 */
            share_count: number;
            to_contact_id: components["schemas"]["ContactId"];
            transfer_type: components["schemas"]["TransferType"];
            transferee_rights: components["schemas"]["TransfereeRights"];
        };
        CreateValuationRequest: {
            dlom?: string | null;
            /** Format: date */
            effective_date: string;
            /** Format: int64 */
            enterprise_value_cents?: number | null;
            entity_id: components["schemas"]["EntityId"];
            /** Format: int64 */
            fmv_per_share_cents?: number | null;
            /** Format: int64 */
            hurdle_amount_cents?: number | null;
            methodology: components["schemas"]["ValuationMethodology"];
            provider_contact_id?: null | components["schemas"]["ContactId"];
            report_date?: string | null;
            report_document_id?: null | components["schemas"]["DocumentId"];
            valuation_type: components["schemas"]["ValuationType"];
        };
        CreateWorkItemRequest: {
            asap?: boolean;
            category: string;
            created_by?: string | null;
            /** Format: date */
            deadline?: string | null;
            description?: string | null;
            metadata?: unknown;
            title: string;
        };
        /**
         * @description A cron expression validated to have at least 5 whitespace-separated fields.
         *
         *     This is a lightweight parse — it doesn't validate each field's range,
         *     but it rejects obviously malformed expressions at the system boundary.
         *     The full matching logic lives in the worker's cron module.
         */
        CronExpr: string;
        /**
         * @description Supported currencies. Currently USD only.
         * @enum {string}
         */
        Currency: "usd";
        /** Format: uuid */
        DeadlineId: string;
        DeadlineResponse: {
            completed_at?: string | null;
            created_at: string;
            deadline_id: components["schemas"]["DeadlineId"];
            deadline_type: string;
            description: string;
            /** Format: date */
            due_date: string;
            entity_id: components["schemas"]["EntityId"];
            recurrence: components["schemas"]["Recurrence"];
            severity: components["schemas"]["DeadlineSeverity"];
            status: components["schemas"]["DeadlineStatus"];
        };
        /**
         * @description Risk severity of missing a deadline.
         * @enum {string}
         */
        DeadlineSeverity: "low" | "medium" | "high" | "critical";
        /**
         * @description Status of a deadline.
         * @enum {string}
         */
        DeadlineStatus: "upcoming" | "due" | "completed" | "overdue";
        DelegationSchedule: {
            adopted_resolution_id?: null | components["schemas"]["ResolutionId"];
            allowed_tier1_intent_types?: string[];
            /** Format: date-time */
            created_at: string;
            entity_id: components["schemas"]["EntityId"];
            /** Format: date-time */
            last_reauthorized_at: string;
            /** Format: date */
            next_mandatory_review_at: string;
            /** Format: int64 */
            reauth_full_suspension_at_days: number;
            /** Format: int64 */
            reauth_reduced_limits_at_days: number;
            /** Format: int32 */
            reauth_reduced_limits_percent: number;
            /** Format: int64 */
            tier1_max_amount_cents: number;
            /** Format: date-time */
            updated_at: string;
            /** Format: int32 */
            version: number;
        };
        DelegationScheduleChangeResponse: {
            amendment: components["schemas"]["ScheduleAmendment"];
            schedule: components["schemas"]["DelegationSchedule"];
        };
        DemoSeedRequest: {
            scenario?: string;
        };
        DemoSeedResponse: {
            entities_created: number;
            message: string;
            scenario: string;
            workspace_id: components["schemas"]["WorkspaceId"];
        };
        DigestSummary: {
            digest_key: string;
            generated_at: string;
        };
        DigestTriggerResponse: {
            digest_count: number;
            message: string;
            triggered: boolean;
        };
        DilutionPreviewQuery: {
            entity_id: components["schemas"]["EntityId"];
            round_id: components["schemas"]["EquityRoundId"];
        };
        DilutionPreviewResponse: {
            issuer_legal_entity_id: components["schemas"]["LegalEntityId"];
            /** Format: int64 */
            pre_round_outstanding_units: number;
            /** Format: int32 */
            projected_dilution_bps: number;
            /** Format: int64 */
            projected_new_units: number;
            /** Format: int64 */
            projected_post_outstanding_units: number;
            round_id: components["schemas"]["EquityRoundId"];
        };
        DirectorInfo: {
            address?: null | components["schemas"]["CompanyAddress"];
            name: string;
        };
        DissolveEntityRequest: {
            reason?: string | null;
        };
        /** Format: uuid */
        DistributionId: string;
        DistributionResponse: {
            created_at: string;
            description: string;
            distribution_id: components["schemas"]["DistributionId"];
            distribution_type: components["schemas"]["DistributionType"];
            entity_id: components["schemas"]["EntityId"];
            status: components["schemas"]["DistributionStatus"];
            /** Format: int64 */
            total_amount_cents: number;
        };
        /**
         * @description Status of a distribution.
         * @enum {string}
         */
        DistributionStatus: "pending" | "approved" | "distributed";
        /**
         * @description Type of distribution.
         * @enum {string}
         */
        DistributionType: "dividend" | "return" | "liquidation";
        DocumentCopyRequest: {
            entity_id: components["schemas"]["EntityId"];
            recipient_email?: string | null;
        };
        DocumentCopyResponse: {
            created_at: string;
            document_id: components["schemas"]["DocumentId"];
            recipient_email?: string | null;
            request_id: string;
            status: string;
            title: string;
        };
        /** Format: uuid */
        DocumentId: string;
        DocumentOptions: {
            dating_format?: string;
            right_of_first_refusal?: boolean;
            s_corp_election?: boolean;
            transfer_restrictions?: boolean;
        };
        /** Format: uuid */
        DocumentRequestId: string;
        DocumentRequestResponse: {
            created_at: string;
            description: string;
            document_type: string;
            entity_id: components["schemas"]["EntityId"];
            fulfilled_at?: string | null;
            not_applicable_at?: string | null;
            obligation_id: components["schemas"]["ObligationId"];
            request_id: components["schemas"]["DocumentRequestId"];
            status: components["schemas"]["DocumentRequestStatus"];
        };
        /**
         * @description Status of a request for a document from a stakeholder.
         * @enum {string}
         */
        DocumentRequestStatus: "requested" | "provided" | "not_applicable" | "waived";
        DocumentResponse: {
            content: Record<string, never>;
            content_hash: string;
            created_at: string;
            document_id: components["schemas"]["DocumentId"];
            document_type: components["schemas"]["DocumentType"];
            entity_id: components["schemas"]["EntityId"];
            signatures: components["schemas"]["SignatureSummary"][];
            status: components["schemas"]["DocumentStatus"];
            title: string;
            /** Format: int32 */
            version: number;
        };
        /**
         * @description Status of a document in the signing workflow.
         * @enum {string}
         */
        DocumentStatus: "draft" | "signed" | "amended" | "filed";
        DocumentSummary: {
            created_at: string;
            document_id: components["schemas"]["DocumentId"];
            document_type: components["schemas"]["DocumentType"];
            signature_count: number;
            status: components["schemas"]["DocumentStatus"];
            title: string;
        };
        /**
         * @description Type of legal document.
         * @enum {string}
         */
        DocumentType: "articles_of_incorporation" | "articles_of_organization" | "bylaws" | "incorporator_action" | "initial_board_consent" | "operating_agreement" | "initial_written_consent" | "ss4_application" | "meeting_notice" | "resolution" | "consulting_agreement" | "employment_offer_letter" | "contractor_services_agreement" | "mutual_nondisclosure_agreement" | "safe_agreement" | "four_oh_nine_a_valuation_report" | "stock_transfer_agreement" | "transfer_board_consent" | "financing_board_consent" | "equity_issuance_approval" | "subscription_agreement" | "investor_rights_agreement" | "restricted_stock_purchase_agreement" | "ip_assignment_agreement" | "contract";
        /** Format: uuid */
        EntityId: string;
        /**
         * @description The legal structure of a business entity.
         * @enum {string}
         */
        EntityType: "c_corp" | "llc";
        /** Format: uuid */
        EquityRoundId: string;
        /** @enum {string} */
        EquityRoundStatus: "draft" | "open" | "board_approved" | "accepted" | "closed" | "cancelled";
        /** Format: uuid */
        EquityRuleSetId: string;
        /** @enum {string} */
        EscalationStatus: "open" | "resolved";
        EvaluateGovernanceRequest: {
            entity_id: components["schemas"]["EntityId"];
            intent_type: string;
            metadata?: Record<string, never>;
        };
        ExecuteConversionRequest: {
            entity_id: components["schemas"]["EntityId"];
            intent_id: components["schemas"]["IntentId"];
            round_id: components["schemas"]["EquityRoundId"];
            source_reference?: string | null;
        };
        ExecutePaymentRequest: {
            /** Format: int64 */
            amount_cents: number;
            description: string;
            entity_id: components["schemas"]["EntityId"];
            payment_method?: components["schemas"]["PaymentMethod"];
            recipient: string;
        };
        ExecuteServiceAgreementRequest: {
            contract_id?: null | components["schemas"]["ContractId"];
            document_id?: null | components["schemas"]["DocumentId"];
            notes?: string | null;
        };
        /** Format: uuid */
        ExecutionId: string;
        ExecutionResponse: {
            agent_id: string;
            completed_at?: string | null;
            container_id?: string | null;
            execution_id: components["schemas"]["ExecutionId"];
            reason?: string | null;
            started_at?: string | null;
            status: string;
        };
        FileTaxDocumentRequest: {
            document_type: string;
            entity_id: components["schemas"]["EntityId"];
            /** Format: int32 */
            tax_year: number;
        };
        FilingAttestationRequest: {
            consent_text?: string;
            notes?: string | null;
            signer_email: string;
            signer_name: string;
            signer_role: string;
        };
        FinalizeAgendaItemRequest: {
            entity_id: components["schemas"]["EntityId"];
            status: components["schemas"]["AgendaItemStatus"];
        };
        FinalizePendingFormationRequest: {
            /** Format: int64 */
            authorized_shares?: number | null;
            company_address?: null | components["schemas"]["Address"];
            fiscal_year_end?: string | null;
            formation_date?: string | null;
            incorporator_address?: string | null;
            incorporator_name?: string | null;
            par_value?: string | null;
            registered_agent_address?: string | null;
            registered_agent_name?: string | null;
            right_of_first_refusal?: boolean | null;
            s_corp_election?: boolean | null;
            transfer_restrictions?: boolean | null;
        };
        FinalizeWorkflowRequest: {
            entity_id: components["schemas"]["EntityId"];
            phase?: string | null;
        };
        FinancialStatementResponse: {
            entity_id: components["schemas"]["EntityId"];
            /** Format: int64 */
            net_income_cents: number;
            period_end: string;
            period_start: string;
            statement_type: string;
            /** Format: int64 */
            total_assets_cents: number;
            /** Format: int64 */
            total_equity_cents: number;
            /** Format: int64 */
            total_liabilities_cents: number;
        };
        FiscalYearEnd: {
            /** Format: int32 */
            day: number;
            /** Format: int32 */
            month: number;
        };
        FormationGatesResponse: {
            attestation_recorded: boolean;
            designated_attestor_email?: string | null;
            designated_attestor_name: string;
            designated_attestor_role: string;
            entity_id: components["schemas"]["EntityId"];
            filing_submission_blockers: string[];
            registered_agent_consent_evidence_count: number;
            requires_natural_person_attestation: boolean;
            requires_registered_agent_consent_evidence: boolean;
            service_agreement_contract_id?: null | components["schemas"]["ContractId"];
            service_agreement_document_id?: null | components["schemas"]["DocumentId"];
            service_agreement_executed: boolean;
            service_agreement_executed_at?: string | null;
            service_agreement_notes?: string | null;
            service_agreement_required_for_tier1_autonomy: boolean;
        };
        FormationResponse: {
            document_ids: components["schemas"]["DocumentId"][];
            entity_id: components["schemas"]["EntityId"];
            formation_id: components["schemas"]["EntityId"];
            formation_status: components["schemas"]["FormationStatus"];
            next_action?: string | null;
        };
        /**
         * @description High-level state of a forming entity.
         * @enum {string}
         */
        FormationState: "forming" | "active";
        /**
         * @description Detailed formation workflow status with valid state transitions.
         * @enum {string}
         */
        FormationStatus: "pending" | "documents_generated" | "documents_signed" | "filing_submitted" | "filed" | "ein_applied" | "active" | "rejected" | "dissolved";
        FormationStatusResponse: {
            entity_id: components["schemas"]["EntityId"];
            entity_type: components["schemas"]["EntityType"];
            formation_date?: string | null;
            formation_state: components["schemas"]["FormationState"];
            formation_status: components["schemas"]["FormationStatus"];
            jurisdiction: components["schemas"]["Jurisdiction"];
            legal_name: string;
            next_action?: string | null;
        };
        FormationWithCapTableResponse: {
            document_ids: components["schemas"]["DocumentId"][];
            entity_id: components["schemas"]["EntityId"];
            formation_id: components["schemas"]["EntityId"];
            formation_status: components["schemas"]["FormationStatus"];
            holders: components["schemas"]["HolderSummary"][];
            instrument_id?: null | components["schemas"]["InstrumentId"];
            legal_entity_id?: null | components["schemas"]["LegalEntityId"];
            next_action?: string | null;
        };
        FounderInfo: {
            address?: null | components["schemas"]["CompanyAddress"];
            email?: string | null;
            ip_contribution?: string | null;
            name: string;
            /** Format: int64 */
            shares?: number | null;
            vesting?: null | components["schemas"]["VestingSchedule"];
        };
        FounderSummary: {
            address?: null | components["schemas"]["Address"];
            email?: string | null;
            name: string;
            /** Format: double */
            ownership_pct?: number | null;
            role?: null | components["schemas"]["MemberRole"];
        };
        /** Format: uuid */
        FundraisingWorkflowId: string;
        FundraisingWorkflowResponse: {
            accept_intent_id?: null | components["schemas"]["IntentId"];
            active_packet_id?: null | components["schemas"]["PacketId"];
            board_approval_meeting_id?: null | components["schemas"]["MeetingId"];
            board_approval_resolution_id?: null | components["schemas"]["ResolutionId"];
            board_packet_documents: string[];
            close_intent_id?: null | components["schemas"]["IntentId"];
            closing_packet_documents: string[];
            created_at: string;
            execution_status: string;
            fundraising_workflow_id: components["schemas"]["FundraisingWorkflowId"];
            last_packet_hash?: string | null;
            prepare_intent_id: components["schemas"]["IntentId"];
            round_id: components["schemas"]["EquityRoundId"];
            round_status: components["schemas"]["EquityRoundStatus"];
            rule_set_id?: null | components["schemas"]["EquityRuleSetId"];
            updated_at: string;
        };
        GenerateContractRequest: {
            counterparty_name: string;
            effective_date: string;
            entity_id: components["schemas"]["EntityId"];
            parameters?: Record<string, never>;
            template_type: components["schemas"]["ContractTemplateType"];
        };
        GenerateGovernanceDocBundleRequest: {
            template_version?: string | null;
        };
        GenerateGovernanceDocBundleResponse: {
            current: components["schemas"]["GovernanceDocBundleCurrent"];
            manifest: components["schemas"]["GovernanceDocBundleManifest"];
            summary: components["schemas"]["GovernanceDocBundleSummary"];
        };
        GenerateWorkflowDocsRequest: {
            documents?: string[];
            entity_id: components["schemas"]["EntityId"];
        };
        GeneratedGovernanceDocument: {
            bytes: number;
            path: string;
            sha256: string;
            source_path: string;
        };
        /**
         * @description Standard GL account codes with integer discriminants matching the code number.
         * @enum {string}
         */
        GlAccountCode: "Cash" | "AccountsReceivable" | "AccountsPayable" | "AccruedExpenses" | "FounderCapital" | "Revenue" | "OperatingExpenses" | "Cogs";
        GovernanceAuditCheckpoint: {
            checkpoint_id: components["schemas"]["GovernanceAuditCheckpointId"];
            /** Format: date-time */
            created_at: string;
            entity_id: components["schemas"]["EntityId"];
            latest_entry_hash: string;
            latest_entry_id: components["schemas"]["GovernanceAuditEntryId"];
            /** Format: int64 */
            total_entries: number;
        };
        /** Format: uuid */
        GovernanceAuditCheckpointId: string;
        GovernanceAuditEntry: {
            action: string;
            audit_entry_id: components["schemas"]["GovernanceAuditEntryId"];
            /** Format: date-time */
            created_at: string;
            details?: Record<string, never>;
            entity_id: components["schemas"]["EntityId"];
            entry_hash: string;
            event_type: components["schemas"]["GovernanceAuditEventType"];
            evidence_refs?: string[];
            linked_incident_id?: null | components["schemas"]["IncidentId"];
            linked_intent_id?: null | components["schemas"]["IntentId"];
            linked_mode_event_id?: null | components["schemas"]["GovernanceModeEventId"];
            linked_trigger_id?: null | components["schemas"]["GovernanceTriggerId"];
            previous_entry_hash?: string | null;
        };
        /** Format: uuid */
        GovernanceAuditEntryId: string;
        /** @enum {string} */
        GovernanceAuditEventType: "mode_changed" | "lockdown_trigger_applied" | "manual_event" | "checkpoint_written" | "chain_verified" | "chain_verification_failed";
        /** Format: uuid */
        GovernanceAuditVerificationId: string;
        GovernanceAuditVerificationReport: {
            anomalies?: string[];
            /** Format: date-time */
            created_at: string;
            entity_id: components["schemas"]["EntityId"];
            incident_id?: null | components["schemas"]["IncidentId"];
            latest_entry_hash?: string | null;
            ok: boolean;
            /** Format: int64 */
            total_entries: number;
            trigger_id?: null | components["schemas"]["GovernanceTriggerId"];
            triggered_lockdown: boolean;
            verification_id: components["schemas"]["GovernanceAuditVerificationId"];
        };
        /** Format: uuid */
        GovernanceBodyId: string;
        GovernanceBodyResponse: {
            body_id: components["schemas"]["GovernanceBodyId"];
            body_type: components["schemas"]["BodyType"];
            created_at: string;
            entity_id: components["schemas"]["EntityId"];
            name: string;
            quorum_rule: components["schemas"]["QuorumThreshold"];
            status: components["schemas"]["BodyStatus"];
            voting_method: components["schemas"]["VotingMethod"];
        };
        GovernanceDocBundleCurrent: {
            bundle_id: components["schemas"]["GovernanceDocBundleId"];
            entity_id: components["schemas"]["EntityId"];
            generated_at: string;
            manifest_path: string;
            template_version: string;
        };
        /** Format: uuid */
        GovernanceDocBundleId: string;
        GovernanceDocBundleManifest: {
            bundle_id: components["schemas"]["GovernanceDocBundleId"];
            documents: components["schemas"]["GeneratedGovernanceDocument"][];
            entity_id: components["schemas"]["EntityId"];
            entity_type: string;
            generated_at: string;
            /** Format: int32 */
            profile_version: number;
            source_root: string;
            template_version: string;
            warnings?: string[];
        };
        GovernanceDocBundleSummary: {
            bundle_id: components["schemas"]["GovernanceDocBundleId"];
            document_count: number;
            entity_id: components["schemas"]["EntityId"];
            entity_type: string;
            generated_at: string;
            /** Format: int32 */
            profile_version: number;
            template_version: string;
        };
        GovernanceIncident: {
            /** Format: date-time */
            created_at: string;
            description: string;
            entity_id: components["schemas"]["EntityId"];
            incident_id: components["schemas"]["IncidentId"];
            /** Format: date-time */
            resolved_at?: string | null;
            severity: components["schemas"]["IncidentSeverity"];
            status: components["schemas"]["IncidentStatus"];
            title: string;
        };
        /**
         * @description Governance mode for policy enforcement.
         * @enum {string}
         */
        GovernanceMode: "normal" | "principal_unavailable" | "incident_lockdown";
        GovernanceModeChangeEvent: {
            /** Format: date-time */
            created_at: string;
            entity_id: components["schemas"]["EntityId"];
            evidence_refs?: string[];
            from_mode: components["schemas"]["GovernanceMode"];
            incident_ids?: components["schemas"]["IncidentId"][];
            mode_event_id: components["schemas"]["GovernanceModeEventId"];
            reason?: string | null;
            to_mode: components["schemas"]["GovernanceMode"];
            trigger_id?: null | components["schemas"]["GovernanceTriggerId"];
            updated_by?: null | components["schemas"]["ContactId"];
        };
        /** Format: uuid */
        GovernanceModeEventId: string;
        GovernanceModeResponse: {
            created_at: string;
            entity_id: components["schemas"]["EntityId"];
            mode: components["schemas"]["GovernanceMode"];
            reason?: string | null;
            updated_at: string;
        };
        GovernanceProfile: {
            adopted_by: string;
            /** Format: int32 */
            board_size?: number | null;
            company_address?: null | components["schemas"]["CompanyAddress"];
            /** Format: date-time */
            created_at: string;
            directors?: components["schemas"]["DirectorInfo"][];
            document_options?: null | components["schemas"]["DocumentOptions"];
            /** Format: date */
            effective_date: string;
            entity_id: components["schemas"]["EntityId"];
            entity_type: components["schemas"]["EntityType"];
            fiscal_year_end?: null | components["schemas"]["FiscalYearEnd"];
            founders?: components["schemas"]["FounderInfo"][];
            incomplete_profile?: boolean;
            incorporator_address?: string | null;
            incorporator_name?: string | null;
            jurisdiction: string;
            /** Format: date */
            last_reviewed: string;
            legal_name: string;
            /** Format: date */
            next_mandatory_review: string;
            officers?: components["schemas"]["OfficerInfo"][];
            principal_name?: string | null;
            principal_title?: string | null;
            registered_agent_address?: string | null;
            registered_agent_name?: string | null;
            stock_details?: null | components["schemas"]["StockDetails"];
            /** Format: date-time */
            updated_at: string;
            /** Format: int32 */
            version: number;
        };
        /** Format: uuid */
        GovernanceSeatId: string;
        GovernanceSeatResponse: {
            /** Format: date */
            appointed_date?: string | null;
            body_id: components["schemas"]["GovernanceBodyId"];
            created_at: string;
            holder_id: components["schemas"]["ContactId"];
            role: components["schemas"]["SeatRole"];
            seat_id: components["schemas"]["GovernanceSeatId"];
            status: components["schemas"]["SeatStatus"];
            /** Format: date */
            term_expiration?: string | null;
            /** Format: int32 */
            voting_power: number;
        };
        GovernanceTriggerEvent: {
            /** Format: date-time */
            created_at: string;
            description: string;
            entity_id: components["schemas"]["EntityId"];
            evidence_refs?: string[];
            idempotency_key_hash?: string | null;
            incident_id: components["schemas"]["IncidentId"];
            linked_escalation_id?: null | components["schemas"]["ComplianceEscalationId"];
            linked_intent_id?: null | components["schemas"]["IntentId"];
            mode_event_id: components["schemas"]["GovernanceModeEventId"];
            severity: components["schemas"]["IncidentSeverity"];
            source: components["schemas"]["GovernanceTriggerSource"];
            title: string;
            trigger_id: components["schemas"]["GovernanceTriggerId"];
            trigger_type: components["schemas"]["GovernanceTriggerType"];
        };
        /** Format: uuid */
        GovernanceTriggerId: string;
        /** @enum {string} */
        GovernanceTriggerSource: "compliance_scanner" | "execution_gate" | "external_ingestion";
        /** @enum {string} */
        GovernanceTriggerType: "external_signal" | "policy_evidence_mismatch" | "compliance_deadline_missed_d_plus_1" | "audit_chain_verification_failed";
        /**
         * @description The type of governing document for a share transfer.
         * @enum {string}
         */
        GoverningDocType: "bylaws" | "operating_agreement" | "shareholder_agreement" | "other";
        /**
         * @description The type of equity grant.
         * @enum {string}
         */
        GrantType: "common_stock" | "preferred_stock" | "membership_unit" | "stock_option" | "iso" | "nso" | "rsa" | "svu";
        /** Format: uuid */
        HolderId: string;
        HolderResponse: {
            contact_id: components["schemas"]["ContactId"];
            created_at: string;
            holder_id: components["schemas"]["HolderId"];
            holder_type: components["schemas"]["HolderType"];
            linked_entity_id?: null | components["schemas"]["EntityId"];
            name: string;
        };
        /** @description Summary of a holder created during cap table setup. */
        HolderSummary: {
            holder_id: components["schemas"]["HolderId"];
            name: string;
            /** Format: double */
            ownership_pct: number;
            /** Format: int64 */
            shares: number;
        };
        /**
         * @description Type of holder represented in the cap table.
         * @enum {string}
         */
        HolderType: "individual" | "organization" | "fund" | "nonprofit" | "trust" | "other";
        /** @enum {string} */
        HttpMethod: "GET" | "POST" | "PUT" | "PATCH" | "DELETE" | "HEAD" | "OPTIONS";
        /** Format: uuid */
        IncidentId: string;
        IncidentResponse: {
            created_at: string;
            description: string;
            entity_id: components["schemas"]["EntityId"];
            incident_id: components["schemas"]["IncidentId"];
            resolved_at?: string | null;
            severity: components["schemas"]["IncidentSeverity"];
            status: components["schemas"]["IncidentStatus"];
            title: string;
        };
        /** @enum {string} */
        IncidentSeverity: "low" | "medium" | "high" | "critical";
        /** @enum {string} */
        IncidentStatus: "open" | "resolved";
        /** Format: uuid */
        InstrumentId: string;
        /**
         * @description Instrument kind in the ownership model.
         * @enum {string}
         */
        InstrumentKind: "common_equity" | "preferred_equity" | "membership_unit" | "option_grant" | "safe" | "convertible_note" | "warrant";
        InstrumentResponse: {
            /** Format: int64 */
            authorized_units?: number | null;
            created_at: string;
            instrument_id: components["schemas"]["InstrumentId"];
            /** Format: int64 */
            issue_price_cents?: number | null;
            issuer_legal_entity_id: components["schemas"]["LegalEntityId"];
            kind: components["schemas"]["InstrumentKind"];
            status: components["schemas"]["InstrumentStatus"];
            symbol: string;
        };
        /**
         * @description Lifecycle status of the instrument.
         * @enum {string}
         */
        InstrumentStatus: "active" | "closed" | "cancelled";
        /** Format: uuid */
        IntentId: string;
        IntentResponse: {
            authority_tier: components["schemas"]["AuthorityTier"];
            authorized_at?: string | null;
            bound_approval_artifact_id?: null | components["schemas"]["ApprovalArtifactId"];
            bound_document_request_ids: components["schemas"]["DocumentRequestId"][];
            cancelled_at?: string | null;
            created_at: string;
            description: string;
            entity_id: components["schemas"]["EntityId"];
            evaluated_at?: string | null;
            executed_at?: string | null;
            failed_at?: string | null;
            failure_reason?: string | null;
            intent_id: components["schemas"]["IntentId"];
            intent_type: string;
            policy_decision?: null | components["schemas"]["PolicyDecision"];
            status: components["schemas"]["IntentStatus"];
        };
        /**
         * @description Lifecycle status of an execution intent.
         * @enum {string}
         */
        IntentStatus: "pending" | "evaluated" | "authorized" | "executed" | "failed" | "cancelled";
        InternalChannelResponse: {
            schedule?: string | null;
            type: string;
        };
        InternalCronAgentResponse: {
            agent_id: string;
            channels: components["schemas"]["InternalChannelResponse"][];
            status: string;
            workspace_id: string;
        };
        InternalLockdownTriggerRequest: {
            description: string;
            evidence_refs?: string[];
            idempotency_key: string;
            linked_escalation_id?: null | components["schemas"]["ComplianceEscalationId"];
            linked_intent_id?: null | components["schemas"]["IntentId"];
            severity: components["schemas"]["IncidentSeverity"];
            title: string;
            trigger_type: components["schemas"]["GovernanceTriggerType"];
        };
        InternalLockdownTriggerResponse: {
            idempotent_replay: boolean;
            incident_id: components["schemas"]["IncidentId"];
            mode: components["schemas"]["GovernanceMode"];
            mode_event_id: components["schemas"]["GovernanceModeEventId"];
            trigger_id: components["schemas"]["GovernanceTriggerId"];
        };
        InterpolateRequest: {
            execution_id: string;
            template: string;
        };
        InterpolateResponse: {
            result: string;
        };
        /**
         * @description Classification of a member/investor.
         * @enum {string}
         */
        InvestorType: "natural_person" | "agent" | "entity";
        /** Format: uuid */
        InvoiceId: string;
        InvoiceResponse: {
            /** Format: int64 */
            amount_cents: number;
            created_at: string;
            customer_name: string;
            description: string;
            /** Format: date */
            due_date: string;
            entity_id: components["schemas"]["EntityId"];
            invoice_id: components["schemas"]["InvoiceId"];
            status: components["schemas"]["InvoiceStatus"];
        };
        /**
         * @description Lifecycle status of an invoice.
         * @enum {string}
         */
        InvoiceStatus: "draft" | "sent" | "paid" | "voided";
        IssueStagedRoundRequest: {
            entity_id: components["schemas"]["EntityId"];
        };
        IssueStagedRoundResponse: {
            agenda_item_id?: null | components["schemas"]["AgendaItemId"];
            meeting_id?: null | components["schemas"]["MeetingId"];
            positions: components["schemas"]["PositionResponse"][];
            round: components["schemas"]["RoundResponse"];
        };
        /** Format: uuid */
        JournalEntryId: string;
        JournalEntryResponse: {
            created_at: string;
            description: string;
            /** Format: date */
            effective_date: string;
            entity_id: components["schemas"]["EntityId"];
            journal_entry_id: components["schemas"]["JournalEntryId"];
            status: components["schemas"]["JournalEntryStatus"];
            /** Format: int64 */
            total_credits_cents: number;
            /** Format: int64 */
            total_debits_cents: number;
        };
        /**
         * @description Status of a journal entry.
         * @enum {string}
         */
        JournalEntryStatus: "draft" | "posted" | "voided";
        /**
         * @description A validated jurisdiction (e.g., "Delaware", "California").
         *
         *     Guarantees: non-empty, at most 200 characters.
         */
        Jurisdiction: string;
        JwksResponse: {
            keys: Record<string, never>[];
        };
        KillResponse: {
            execution_id: components["schemas"]["ExecutionId"];
            status: string;
        };
        LedgerLineRequest: {
            account_id: components["schemas"]["AccountId"];
            /** Format: int64 */
            amount_cents: number;
            memo?: string | null;
            side: components["schemas"]["Side"];
        };
        /** Format: uuid */
        LegalEntityId: string;
        LegalEntityResponse: {
            created_at: string;
            legal_entity_id: components["schemas"]["LegalEntityId"];
            linked_entity_id?: null | components["schemas"]["EntityId"];
            name: string;
            role: components["schemas"]["LegalEntityRole"];
            workspace_id: components["schemas"]["WorkspaceId"];
        };
        /**
         * @description Role this legal entity plays in the ownership/control graph.
         * @enum {string}
         */
        LegalEntityRole: "operating" | "control" | "investment" | "nonprofit" | "spv" | "other";
        /** @description MCP server that runs inside the agent container. */
        MCPServerSpec: {
            args?: string[];
            command: components["schemas"]["NonEmpty"];
            env?: {
                [key: string]: string;
            };
            name: components["schemas"]["NonEmpty"];
            transport?: components["schemas"]["Transport"];
        };
        /** Format: uuid */
        MeetingId: string;
        MeetingQuery: {
            entity_id: components["schemas"]["EntityId"];
        };
        MeetingResponse: {
            agenda_item_ids: components["schemas"]["AgendaItemId"][];
            body_id: components["schemas"]["GovernanceBodyId"];
            created_at: string;
            location: string;
            meeting_id: components["schemas"]["MeetingId"];
            meeting_type: components["schemas"]["MeetingType"];
            quorum_met: components["schemas"]["QuorumStatus"];
            /** Format: date */
            scheduled_date?: string | null;
            status: components["schemas"]["MeetingStatus"];
            title: string;
        };
        /**
         * @description Lifecycle status of a meeting.
         * @enum {string}
         */
        MeetingStatus: "draft" | "noticed" | "convened" | "adjourned" | "cancelled";
        /**
         * @description Type of meeting.
         * @enum {string}
         */
        MeetingType: "board_meeting" | "shareholder_meeting" | "written_consent" | "member_meeting";
        /** @description A member/founder as provided in the formation request. */
        MemberInput: {
            address?: null | components["schemas"]["Address"];
            agent_id?: null | components["schemas"]["AgentId"];
            email?: string | null;
            entity_id?: null | components["schemas"]["EntityId"];
            investor_type: components["schemas"]["InvestorType"];
            /** @description Description of IP being contributed to the company. */
            ip_description?: string | null;
            /** @description Whether this member is the sole incorporator (corporations only). */
            is_incorporator?: boolean | null;
            /** Format: int64 */
            membership_units?: number | null;
            name: string;
            officer_title?: null | components["schemas"]["OfficerTitle"];
            /** Format: double */
            ownership_pct?: number | null;
            role?: null | components["schemas"]["MemberRole"];
            share_class?: string | null;
            /** Format: int64 */
            share_count?: number | null;
            /**
             * Format: int64
             * @description Explicit number of shares being purchased at formation.
             */
            shares_purchased?: number | null;
            vesting?: null | components["schemas"]["VestingSchedule"];
        };
        /**
         * @description Role a member holds in the entity.
         * @enum {string}
         */
        MemberRole: "director" | "officer" | "manager" | "member" | "chair";
        MergeBranchRequest: {
            into?: components["schemas"]["BranchName"];
            squash?: boolean;
        };
        MergeBranchResponse: {
            commit?: string | null;
            merged: boolean;
            strategy: string;
        };
        /** Format: uuid */
        MessageId: string;
        MessageResponse: {
            agent_id: components["schemas"]["AgentId"];
            execution_id?: null | components["schemas"]["ExecutionId"];
            message: string;
            message_id: components["schemas"]["MessageId"];
            status: string;
        };
        MintAgentTokenRequest: {
            agent_id: components["schemas"]["AgentId"];
            /** Format: int64 */
            ttl_seconds?: number | null;
            workspace_id: components["schemas"]["WorkspaceId"];
        };
        MintAgentTokenResponse: {
            access_token: string;
            /** Format: int64 */
            expires_in: number;
            scopes: components["schemas"]["Scope"][];
            token_type: string;
        };
        /** @enum {string} */
        NetworkEgress: "restricted" | "open";
        /**
         * @description A `String` that is guaranteed non-empty and non-whitespace-only.
         *
         *     This is a *parsed* type: the only way to obtain one is through
         *     `NonEmpty::parse()` or serde deserialization, both of which reject
         *     blank strings.  Downstream code never needs to re-check for emptiness.
         */
        NonEmpty: string;
        NotificationPrefsResponse: {
            contact_id: components["schemas"]["ContactId"];
            email_enabled: boolean;
            sms_enabled: boolean;
            updated_at: string;
            webhook_enabled: boolean;
        };
        /** Format: uuid */
        ObligationId: string;
        ObligationResponse: {
            assignee_id?: null | components["schemas"]["ContactId"];
            assignee_type: components["schemas"]["AssigneeType"];
            created_at: string;
            description: string;
            /** Format: date */
            due_date?: string | null;
            entity_id: components["schemas"]["EntityId"];
            expired_at?: string | null;
            fulfilled_at?: string | null;
            intent_id?: null | components["schemas"]["IntentId"];
            obligation_id: components["schemas"]["ObligationId"];
            obligation_type: string;
            status: components["schemas"]["ObligationStatus"];
            waived_at?: string | null;
        };
        /**
         * @description Lifecycle status of a compliance or operational obligation.
         * @enum {string}
         */
        ObligationStatus: "required" | "in_progress" | "fulfilled" | "waived" | "expired";
        /**
         * @description An extensible obligation type represented as a string.
         *
         *     Obligation types are not a fixed enum because they vary by jurisdiction,
         *     entity type, and operational context.
         */
        ObligationType: string;
        ObligationsSummaryResponse: {
            expired: number;
            fulfilled: number;
            pending: number;
            total: number;
            waived: number;
        };
        OfficerInfo: {
            name: string;
            title: string;
        };
        /**
         * @description Officer title for a corporate officer.
         * @enum {string}
         */
        OfficerTitle: "ceo" | "cfo" | "cto" | "coo" | "secretary" | "treasurer" | "president" | "vp" | "other";
        /** Format: uuid */
        PacketId: string;
        PacketItem: {
            document_path: string;
            item_id: string;
            required: boolean;
            title: string;
        };
        /** Format: uuid */
        PacketSignatureId: string;
        PacketSignatureResponse: {
            channel: string;
            signature_id: components["schemas"]["PacketSignatureId"];
            signed_at: string;
            signer_identity: string;
        };
        PayInstructionsResponse: {
            /** Format: int64 */
            amount_cents: number;
            currency: string;
            instructions: string;
            invoice_id: components["schemas"]["InvoiceId"];
            payment_method: string;
        };
        /** Format: uuid */
        PaymentId: string;
        PaymentIntentResponse: {
            /** Format: int64 */
            amount_cents: number;
            client_secret: string;
            created_at: string;
            currency: string;
            entity_id: components["schemas"]["EntityId"];
            payment_intent_id: string;
            status: string;
        };
        /**
         * @description How a payment is made or received.
         * @enum {string}
         */
        PaymentMethod: "bank_transfer" | "card" | "check" | "wire" | "ach";
        PaymentOfferResponse: {
            /** Format: int64 */
            amount_cents: number;
            entity_id: components["schemas"]["EntityId"];
            invoice_id: components["schemas"]["InvoiceId"];
            payment_url: string;
            status: components["schemas"]["InvoiceStatus"];
        };
        PaymentResponse: {
            /** Format: int64 */
            amount_cents: number;
            created_at: string;
            description: string;
            entity_id: components["schemas"]["EntityId"];
            payment_id: components["schemas"]["PaymentId"];
            payment_method: components["schemas"]["PaymentMethod"];
            recipient: string;
            status: components["schemas"]["PaymentStatus"];
        };
        /**
         * @description Lifecycle status of a payment.
         * @enum {string}
         */
        PaymentStatus: "submitted" | "processing" | "completed" | "failed";
        PayoutResponse: {
            /** Format: int64 */
            amount_cents: number;
            created_at: string;
            destination: string;
            entity_id: components["schemas"]["EntityId"];
            payout_id: string;
            status: string;
        };
        /** Format: uuid */
        PayrollRunId: string;
        PayrollRunResponse: {
            created_at: string;
            entity_id: components["schemas"]["EntityId"];
            /** Format: date */
            pay_period_end: string;
            /** Format: date */
            pay_period_start: string;
            payroll_run_id: components["schemas"]["PayrollRunId"];
            status: components["schemas"]["PayrollStatus"];
        };
        /**
         * @description Status of a payroll run.
         * @enum {string}
         */
        PayrollStatus: "pending" | "processing" | "completed";
        PendingFormationResponse: {
            entity_id: components["schemas"]["EntityId"];
            entity_type: components["schemas"]["EntityType"];
            formation_status: components["schemas"]["FormationStatus"];
            jurisdiction: components["schemas"]["Jurisdiction"];
            legal_name: string;
        };
        PendingSecuritiesFile: {
            round_id: components["schemas"]["EquityRoundId"];
            securities: components["schemas"]["PendingSecurity"][];
        };
        PendingSecurity: {
            grant_type?: string | null;
            holder_id: components["schemas"]["HolderId"];
            instrument_id: components["schemas"]["InstrumentId"];
            /** Format: int64 */
            principal_cents?: number;
            /** Format: int64 */
            quantity: number;
            recipient_name: string;
        };
        PolicyConflict: {
            higher_source: components["schemas"]["AuthoritySource"];
            lower_source: components["schemas"]["AuthoritySource"];
            reason: string;
        };
        PolicyDecision: {
            allowed: boolean;
            blockers: string[];
            clause_refs: string[];
            effective_source?: null | components["schemas"]["AuthoritySource"];
            escalation_reasons: string[];
            policy_mapped: boolean;
            precedence_conflicts?: components["schemas"]["PolicyConflict"][];
            precedence_trace?: components["schemas"]["PolicyPrecedenceTrace"][];
            requires_approval: boolean;
            tier: components["schemas"]["AuthorityTier"];
        };
        PolicyPrecedenceTrace: {
            outcome: string;
            reason?: string | null;
            source: components["schemas"]["AuthoritySource"];
        };
        /** Format: uuid */
        PositionId: string;
        PositionResponse: {
            holder_id: components["schemas"]["HolderId"];
            instrument_id: components["schemas"]["InstrumentId"];
            issuer_legal_entity_id: components["schemas"]["LegalEntityId"];
            position_id: components["schemas"]["PositionId"];
            /** Format: int64 */
            principal_cents: number;
            /** Format: int64 */
            quantity_units: number;
            status: components["schemas"]["PositionStatus"];
            updated_at: string;
        };
        /**
         * @description Lifecycle status for a position.
         * @enum {string}
         */
        PositionStatus: "active" | "closed";
        PrepareWorkflowExecutionRequest: {
            approval_artifact_id: components["schemas"]["ApprovalArtifactId"];
            document_request_ids?: components["schemas"]["DocumentRequestId"][];
            entity_id: components["schemas"]["EntityId"];
            intent_id: components["schemas"]["IntentId"];
            phase?: string | null;
        };
        PreviewConversionRequest: {
            entity_id: components["schemas"]["EntityId"];
            round_id: components["schemas"]["EquityRoundId"];
            source_reference?: string | null;
        };
        PreviewDocumentQuery: {
            document_id: string;
            entity_id: components["schemas"]["EntityId"];
        };
        ProvisionWorkspaceRequest: {
            name: string;
            owner_email?: string | null;
        };
        ProvisionWorkspaceResponse: {
            api_key: string;
            api_key_id: components["schemas"]["ApiKeyId"];
            name: string;
            workspace_id: components["schemas"]["WorkspaceId"];
        };
        ProxyResponse: {
            created_at: string;
            description?: string | null;
            name: string;
            secret_count: number;
            url: string;
        };
        /**
         * @description Whether a quorum was met for a meeting.
         *
         *     Replaces `Option<bool>` for clearer semantics.
         *     Backward-compatible deserialization from `Option<bool>` via `From`.
         * @enum {string}
         */
        QuorumStatus: "unknown" | "met" | "not_met";
        /**
         * @description The threshold required for a vote to pass.
         * @enum {string}
         */
        QuorumThreshold: "majority" | "supermajority" | "unanimous";
        ReauthorizeDelegationScheduleRequest: {
            adopted_resolution_id: components["schemas"]["ResolutionId"];
            entity_id: components["schemas"]["EntityId"];
            meeting_id: components["schemas"]["MeetingId"];
            rationale?: string | null;
        };
        /** Format: uuid */
        ReceiptId: string;
        ReceiptResponse: {
            created_at: string;
            executed_at?: string | null;
            idempotency_key: string;
            intent_id: components["schemas"]["IntentId"];
            receipt_id: components["schemas"]["ReceiptId"];
            request_hash: string;
            response_hash?: string | null;
            status: components["schemas"]["ReceiptStatus"];
        };
        /**
         * @description Status of an execution receipt.
         * @enum {string}
         */
        ReceiptStatus: "pending" | "executed" | "failed";
        ReconcileLedgerRequest: {
            /** Format: date */
            as_of_date?: string | null;
            entity_id: components["schemas"]["EntityId"];
        };
        /** Format: uuid */
        ReconciliationId: string;
        ReconciliationResponse: {
            /** Format: date */
            as_of_date: string;
            created_at: string;
            /** Format: int64 */
            difference_cents: number;
            entity_id: components["schemas"]["EntityId"];
            reconciliation_id: components["schemas"]["ReconciliationId"];
            status: components["schemas"]["ReconciliationStatus"];
            /** Format: int64 */
            total_credits_cents: number;
            /** Format: int64 */
            total_debits_cents: number;
        };
        /**
         * @description Status of a reconciliation.
         * @enum {string}
         */
        ReconciliationStatus: "balanced" | "discrepancy";
        RecordFundraisingAcceptanceRequest: {
            accepted_by_contact_id?: null | components["schemas"]["ContactId"];
            entity_id: components["schemas"]["EntityId"];
            intent_id: components["schemas"]["IntentId"];
        };
        RecordFundraisingBoardApprovalRequest: {
            entity_id: components["schemas"]["EntityId"];
            meeting_id: components["schemas"]["MeetingId"];
            resolution_id: components["schemas"]["ResolutionId"];
        };
        RecordFundraisingCloseRequest: {
            entity_id: components["schemas"]["EntityId"];
            intent_id: components["schemas"]["IntentId"];
        };
        RecordTransferBoardApprovalRequest: {
            entity_id: components["schemas"]["EntityId"];
            meeting_id: components["schemas"]["MeetingId"];
            resolution_id: components["schemas"]["ResolutionId"];
        };
        RecordTransferExecutionRequest: {
            entity_id: components["schemas"]["EntityId"];
            intent_id: components["schemas"]["IntentId"];
        };
        RecordTransferReviewRequest: {
            approved: boolean;
            entity_id: components["schemas"]["EntityId"];
            notes: string;
            reviewer: string;
        };
        RecordTransferRofrRequest: {
            entity_id: components["schemas"]["EntityId"];
            offered: boolean;
            waived: boolean;
        };
        RecordWorkflowSignatureRequest: {
            channel?: string | null;
            entity_id: components["schemas"]["EntityId"];
            signer_identity: string;
        };
        /**
         * @description Recurrence pattern for a deadline.
         * @enum {string}
         */
        Recurrence: "one_time" | "monthly" | "quarterly" | "annual";
        RegisteredAgentConsentEvidenceRequest: {
            evidence_type?: string | null;
            evidence_uri: string;
            notes?: string | null;
        };
        /** Format: uuid */
        ResolutionId: string;
        ResolutionResponse: {
            agenda_item_id: components["schemas"]["AgendaItemId"];
            created_at: string;
            document_id?: null | components["schemas"]["DocumentId"];
            /** Format: date */
            effective_date?: string | null;
            meeting_id: components["schemas"]["MeetingId"];
            passed: boolean;
            /** Format: int32 */
            recused_count: number;
            resolution_id: components["schemas"]["ResolutionId"];
            resolution_text: string;
            resolution_type: components["schemas"]["ResolutionType"];
            /** Format: int32 */
            votes_abstain: number;
            /** Format: int32 */
            votes_against: number;
            /** Format: int32 */
            votes_for: number;
        };
        /**
         * @description Type of resolution.
         * @enum {string}
         */
        ResolutionType: "ordinary" | "special" | "unanimous_written_consent";
        ResolveEscalationWithEvidenceRequest: {
            entity_id: components["schemas"]["EntityId"];
            evidence_type?: string | null;
            filing_reference?: string | null;
            notes?: string | null;
            packet_id?: null | components["schemas"]["PacketId"];
            resolve_incident?: boolean;
            resolve_obligation?: boolean;
        };
        ResolveEscalationWithEvidenceResponse: {
            escalation: components["schemas"]["ComplianceEscalationResponse"];
            evidence_link_id: components["schemas"]["ComplianceEvidenceLinkId"];
            incident_resolved: boolean;
            obligation_resolved: boolean;
        };
        ResolveRequest: {
            token: string;
        };
        ResolveResponse: {
            value: string;
        };
        ResolveSecretsRequest: {
            /** @description Which secret keys to resolve. If empty, resolve all. */
            keys?: string[];
            proxy_name: string;
            workspace_id: components["schemas"]["WorkspaceId"];
        };
        ResolveSecretsResponse: {
            proxy_name: string;
            url: string;
            values: {
                [key: string]: string;
            };
        };
        /**
         * @description Risk level for contractor classification.
         * @enum {string}
         */
        RiskLevel: "low" | "medium" | "high";
        RoundResponse: {
            accepted_at?: string | null;
            accepted_by_contact_id?: null | components["schemas"]["ContactId"];
            board_approval_meeting_id?: null | components["schemas"]["MeetingId"];
            board_approval_resolution_id?: null | components["schemas"]["ResolutionId"];
            board_approved_at?: string | null;
            conversion_target_instrument_id?: null | components["schemas"]["InstrumentId"];
            created_at: string;
            issuer_legal_entity_id: components["schemas"]["LegalEntityId"];
            name: string;
            /** Format: int64 */
            pre_money_cents?: number | null;
            round_id: components["schemas"]["EquityRoundId"];
            /** Format: int64 */
            round_price_cents?: number | null;
            rule_set_id?: null | components["schemas"]["EquityRuleSetId"];
            status: components["schemas"]["EquityRoundStatus"];
            /** Format: int64 */
            target_raise_cents?: number | null;
        };
        RuleSetResponse: {
            anti_dilution_method: components["schemas"]["AntiDilutionMethod"];
            conversion_precedence: components["schemas"]["InstrumentKind"][];
            rule_set_id: components["schemas"]["EquityRuleSetId"];
        };
        /**
         * @description Per-agent sandbox (container) configuration.
         *
         *     Numeric resource limits are validated positive — a container with
         *     zero memory or zero CPU cannot start, so we reject at parse time.
         */
        SandboxConfig: {
            /** Format: double */
            cpu_limit?: number;
            /** Format: int64 */
            disk_mb?: number;
            egress_allowlist?: string[];
            enable_code_execution?: boolean;
            /** Format: int64 */
            memory_mb?: number;
            network_egress?: components["schemas"]["NetworkEgress"];
            packages?: string[];
            runtimes?: string[];
            /** Format: int64 */
            timeout_seconds?: number;
        };
        ScanComplianceRequest: {
            entity_id: components["schemas"]["EntityId"];
        };
        ScanExpiredResponse: {
            expired: number;
            scanned: number;
        };
        ScheduleAmendment: {
            added_tier1_intent_types: string[];
            adopted_resolution_id?: null | components["schemas"]["ResolutionId"];
            authority_expansion: boolean;
            /** Format: date-time */
            created_at: string;
            entity_id: components["schemas"]["EntityId"];
            /** Format: int32 */
            from_version: number;
            /** Format: int64 */
            new_tier1_max_amount_cents: number;
            /** Format: int64 */
            previous_tier1_max_amount_cents: number;
            rationale?: string | null;
            removed_tier1_intent_types: string[];
            schedule_amendment_id: components["schemas"]["ScheduleAmendmentId"];
            /** Format: int32 */
            to_version: number;
        };
        /** Format: uuid */
        ScheduleAmendmentId: string;
        ScheduleMeetingRequest: {
            agenda_item_titles?: string[];
            body_id: components["schemas"]["GovernanceBodyId"];
            entity_id: components["schemas"]["EntityId"];
            location?: string | null;
            meeting_type: components["schemas"]["MeetingType"];
            /** Format: int32 */
            notice_days?: number | null;
            /** Format: date */
            scheduled_date?: string | null;
            title: string;
        };
        /**
         * @description A single capability scope that can be granted to an API key or token.
         * @enum {string}
         */
        Scope: "formation_create" | "formation_read" | "formation_sign" | "equity_read" | "equity_write" | "equity_transfer" | "governance_read" | "governance_write" | "governance_vote" | "treasury_read" | "treasury_write" | "treasury_approve" | "contacts_read" | "contacts_write" | "execution_read" | "execution_write" | "branch_create" | "branch_merge" | "branch_delete" | "admin" | "internal_worker_read" | "internal_worker_write" | "secrets_manage" | "all";
        /**
         * @description Role of a seat in a governance body.
         * @enum {string}
         */
        SeatRole: "chair" | "member" | "officer" | "observer";
        /**
         * @description Status of a governance seat.
         * @enum {string}
         */
        SeatStatus: "active" | "resigned" | "expired";
        SecretNamesResponse: {
            names: string[];
            proxy_name: string;
        };
        SeedChartOfAccountsRequest: {
            entity_id: components["schemas"]["EntityId"];
            template?: string;
        };
        SeedChartOfAccountsResponse: {
            accounts_created: number;
            entity_id: components["schemas"]["EntityId"];
            template: string;
        };
        SendMessageRequest: {
            message: string;
            metadata?: Record<string, never>;
        };
        ServiceTokenResponse: {
            /** Format: int64 */
            expires_in: number;
            token: string;
            token_type: string;
        };
        SetGovernanceModeRequest: {
            entity_id: components["schemas"]["EntityId"];
            evidence_refs?: string[];
            incident_ids?: components["schemas"]["IncidentId"][];
            mode: components["schemas"]["GovernanceMode"];
            reason?: string | null;
        };
        SetSecretsRequest: {
            /** @description Key-value pairs. Values are plaintext — the server encrypts before storing. */
            secrets: {
                [key: string]: string;
            };
        };
        /** Format: uuid */
        ShareClassId: string;
        /**
         * @description Debit or credit side of a ledger entry.
         * @enum {string}
         */
        Side: "debit" | "credit";
        SignDocumentRequest: {
            consent_text?: string;
            signature_svg?: string | null;
            signature_text: string;
            signer_email: string;
            signer_name: string;
            signer_role: string;
        };
        SignDocumentResponse: {
            document_id: components["schemas"]["DocumentId"];
            document_status: components["schemas"]["DocumentStatus"];
            signature_id: components["schemas"]["SignatureId"];
            signed_at: string;
        };
        /** Format: uuid */
        SignatureId: string;
        SignatureSummary: {
            signature_id: components["schemas"]["SignatureId"];
            signed_at: string;
            signer_name: string;
            signer_role: string;
        };
        SignerTokenResponse: {
            expires_at: string;
            obligation_id: components["schemas"]["ObligationId"];
            token: string;
        };
        /** @description Contract details included in signing resolve response. */
        SigningContractDetails: {
            counterparty_name: string;
            effective_date: string;
            parameters: unknown;
            rendered_text?: string | null;
            template_label?: string | null;
            template_type: string;
        };
        SigningLinkResponse: {
            document_id: components["schemas"]["DocumentId"];
            signing_url: string;
            token: string;
        };
        /** @description Response for the public resolve endpoint. */
        SigningResolveResponse: {
            contract?: null | components["schemas"]["SigningContractDetails"];
            document_id: components["schemas"]["DocumentId"];
            document_status: string;
            document_title: string;
            entity_id: components["schemas"]["EntityId"];
            /** @description Entity legal name for display. */
            entity_name?: string | null;
            /** @description Public PDF preview URL for the signing page. */
            pdf_url?: string | null;
            /** @description Plain-text preview fallback when a PDF is unavailable. */
            preview_text?: string | null;
            signatures: components["schemas"]["SignatureSummary"][];
        };
        /** Format: uuid */
        SpendingLimitId: string;
        SpendingLimitResponse: {
            /** Format: int64 */
            amount_cents: number;
            category: string;
            created_at: string;
            entity_id: components["schemas"]["EntityId"];
            period: string;
            spending_limit_id: components["schemas"]["SpendingLimitId"];
        };
        StartStagedRoundRequest: {
            entity_id: components["schemas"]["EntityId"];
            issuer_legal_entity_id: components["schemas"]["LegalEntityId"];
            metadata?: Record<string, never>;
            name: string;
            /** Format: int64 */
            pre_money_cents?: number | null;
            /** Format: int64 */
            round_price_cents?: number | null;
            /** Format: int64 */
            target_raise_cents?: number | null;
        };
        StartWorkflowSignaturesRequest: {
            entity_id: components["schemas"]["EntityId"];
        };
        StockDetails: {
            /** Format: int64 */
            authorized_shares: number;
            /** Format: int64 */
            par_value_cents: number;
            share_class?: string;
        };
        StripeAccountResponse: {
            created_at: string;
            entity_id: components["schemas"]["EntityId"];
            status: string;
            stripe_account_id: string;
        };
        SubmitPaymentRequest: {
            /** Format: int64 */
            amount_cents: number;
            description: string;
            entity_id: components["schemas"]["EntityId"];
            payment_method?: components["schemas"]["PaymentMethod"];
            recipient: string;
        };
        SubmitTransferReviewRequest: {
            entity_id: components["schemas"]["EntityId"];
        };
        SubmitValuationForApprovalRequest: {
            entity_id: components["schemas"]["EntityId"];
        };
        SystemHealth: {
            git_storage: string;
            status: string;
            /** Format: int64 */
            uptime_seconds: number;
            version: string;
            workspace_count: number;
        };
        /** Format: uuid */
        TaxFilingId: string;
        TaxFilingResponse: {
            created_at: string;
            document_id: components["schemas"]["DocumentId"];
            document_type: string;
            entity_id: components["schemas"]["EntityId"];
            filing_id: components["schemas"]["TaxFilingId"];
            status: components["schemas"]["TaxFilingStatus"];
            /** Format: int32 */
            tax_year: number;
        };
        /**
         * @description Status of a tax filing.
         * @enum {string}
         */
        TaxFilingStatus: "pending" | "filed" | "accepted" | "rejected";
        TokenExchangeRequest: {
            api_key: string;
            /** Format: int64 */
            ttl_seconds?: number;
        };
        TokenExchangeResponse: {
            access_token: string;
            /** Format: int64 */
            expires_in: number;
            token_type: string;
        };
        /**
         * @description HTTP tool that the agent can call.
         *
         *     `name` and `url` are [`NonEmpty`] — deserialization of blank values
         *     fails with a clear error rather than producing a broken tool.
         */
        ToolSpec: {
            body_schema?: Record<string, never>;
            description?: string | null;
            headers?: {
                [key: string]: string;
            };
            method?: components["schemas"]["HttpMethod"];
            name: components["schemas"]["NonEmpty"];
            parameters?: Record<string, never>;
            url: components["schemas"]["NonEmpty"];
        };
        TransactionPacketResponse: {
            created_at: string;
            entity_id: components["schemas"]["EntityId"];
            finalized_at?: string | null;
            intent_id: components["schemas"]["IntentId"];
            items: components["schemas"]["PacketItem"][];
            manifest_hash: string;
            packet_id: components["schemas"]["PacketId"];
            required_signers: string[];
            signatures: components["schemas"]["PacketSignatureResponse"][];
            status: components["schemas"]["TransactionPacketStatus"];
            workflow_id: string;
            workflow_type: components["schemas"]["WorkflowType"];
        };
        /** @enum {string} */
        TransactionPacketStatus: "drafted" | "ready_for_signature" | "fully_signed" | "executable" | "executed" | "failed";
        /** Format: uuid */
        TransferId: string;
        /**
         * @description Lifecycle status of a share transfer.
         * @enum {string}
         */
        TransferStatus: "draft" | "pending_bylaws_review" | "pending_rofr" | "pending_board_approval" | "approved" | "executed" | "denied" | "cancelled";
        /**
         * @description Type of share transfer.
         * @enum {string}
         */
        TransferType: "gift" | "trust_transfer" | "secondary_sale" | "estate" | "other";
        /** Format: uuid */
        TransferWorkflowId: string;
        TransferWorkflowResponse: {
            active_packet_id?: null | components["schemas"]["PacketId"];
            board_approval_meeting_id?: null | components["schemas"]["MeetingId"];
            board_approval_resolution_id?: null | components["schemas"]["ResolutionId"];
            created_at: string;
            execute_intent_id?: null | components["schemas"]["IntentId"];
            execution_status: string;
            generated_documents: string[];
            last_packet_hash?: string | null;
            prepare_intent_id: components["schemas"]["IntentId"];
            transfer_id: components["schemas"]["TransferId"];
            transfer_status: components["schemas"]["TransferStatus"];
            transfer_workflow_id: components["schemas"]["TransferWorkflowId"];
            updated_at: string;
        };
        /**
         * @description Rights granted to the transferee.
         * @enum {string}
         */
        TransfereeRights: "full_member" | "economic_only" | "limited";
        /** @enum {string} */
        Transport: "stdio" | "http";
        UpdateAgentRequest: {
            budget?: null | components["schemas"]["BudgetConfig"];
            channels?: components["schemas"]["ChannelConfig"][] | null;
            mcp_servers?: components["schemas"]["MCPServerSpec"][] | null;
            model?: string | null;
            name?: string | null;
            parent_agent_id?: null | components["schemas"]["AgentId"];
            sandbox?: null | components["schemas"]["SandboxConfig"];
            scopes?: components["schemas"]["Scope"][] | null;
            status?: null | components["schemas"]["AgentStatus"];
            system_prompt?: string | null;
            tools?: components["schemas"]["ToolSpec"][] | null;
            webhook_url?: string | null;
        };
        UpdateContactRequest: {
            cap_table_access?: null | components["schemas"]["CapTableAccess"];
            category?: null | components["schemas"]["ContactCategory"];
            email?: string | null;
            entity_id: components["schemas"]["EntityId"];
            mailing_address?: string | null;
            name?: string | null;
            notes?: string | null;
            phone?: string | null;
        };
        UpdateGovernanceProfileRequest: {
            adopted_by: string;
            /** Format: int32 */
            board_size?: number | null;
            company_address?: null | components["schemas"]["CompanyAddress"];
            directors?: components["schemas"]["DirectorInfo"][] | null;
            document_options?: null | components["schemas"]["DocumentOptions"];
            /** Format: date */
            effective_date: string;
            fiscal_year_end?: null | components["schemas"]["FiscalYearEnd"];
            founders?: components["schemas"]["FounderInfo"][] | null;
            incomplete_profile?: boolean | null;
            incorporator_address?: string | null;
            incorporator_name?: string | null;
            jurisdiction: string;
            /** Format: date */
            last_reviewed: string;
            legal_name: string;
            /** Format: date */
            next_mandatory_review: string;
            officers?: components["schemas"]["OfficerInfo"][] | null;
            principal_name?: string | null;
            principal_title?: string | null;
            registered_agent_address?: string | null;
            registered_agent_name?: string | null;
            stock_details?: null | components["schemas"]["StockDetails"];
        };
        UpdateNotificationPrefsRequest: {
            email_enabled?: boolean | null;
            entity_id: components["schemas"]["EntityId"];
            sms_enabled?: boolean | null;
            webhook_enabled?: boolean | null;
        };
        /** Format: uuid */
        ValuationId: string;
        /**
         * @description Methodology used for a valuation.
         * @enum {string}
         */
        ValuationMethodology: "income" | "market" | "asset" | "backsolve" | "hybrid" | "other";
        ValuationResponse: {
            agenda_item_id?: null | components["schemas"]["AgendaItemId"];
            board_approval_resolution_id?: null | components["schemas"]["ResolutionId"];
            created_at: string;
            /** Format: date */
            effective_date: string;
            /** Format: int64 */
            enterprise_value_cents?: number | null;
            entity_id: components["schemas"]["EntityId"];
            /** Format: date */
            expiration_date?: string | null;
            /** Format: int64 */
            fmv_per_share_cents?: number | null;
            /** Format: int64 */
            hurdle_amount_cents?: number | null;
            meeting_id?: null | components["schemas"]["MeetingId"];
            methodology: components["schemas"]["ValuationMethodology"];
            provider_contact_id?: null | components["schemas"]["ContactId"];
            report_document_id?: null | components["schemas"]["DocumentId"];
            status: components["schemas"]["ValuationStatus"];
            valuation_id: components["schemas"]["ValuationId"];
            valuation_type: components["schemas"]["ValuationType"];
        };
        /**
         * @description Lifecycle status of a valuation.
         * @enum {string}
         */
        ValuationStatus: "draft" | "pending_approval" | "approved" | "expired" | "superseded";
        /**
         * @description Type of 409A or equivalent valuation.
         * @enum {string}
         */
        ValuationType: "four_oh_nine_a" | "llc_profits_interest" | "fair_market_value" | "gift" | "estate" | "other";
        VerifyGovernanceAuditChainRequest: {
            entity_id: components["schemas"]["EntityId"];
        };
        /** @description Vesting schedule for founder shares. */
        VestingSchedule: {
            /** @description Acceleration type: "single_trigger", "double_trigger", or none. */
            acceleration?: string | null;
            /**
             * Format: int32
             * @description Cliff period in months (e.g. 12).
             */
            cliff_months: number;
            /**
             * Format: int32
             * @description Total vesting period in months (e.g. 48).
             */
            total_months: number;
        };
        /** Format: uuid */
        VoteId: string;
        VoteResponse: {
            agenda_item_id: components["schemas"]["AgendaItemId"];
            cast_at: string;
            signature_hash: string;
            vote_id: components["schemas"]["VoteId"];
            vote_value: components["schemas"]["VoteValue"];
            voter_id: components["schemas"]["ContactId"];
            /** Format: int32 */
            voting_power_applied: number;
        };
        /**
         * @description How a participant voted.
         * @enum {string}
         */
        VoteValue: "for" | "against" | "abstain" | "recusal";
        /**
         * @description How votes are counted.
         * @enum {string}
         */
        VotingMethod: "per_capita" | "per_unit";
        /** Format: uuid */
        WorkItemId: string;
        WorkItemResponse: {
            asap: boolean;
            category: string;
            /** Format: int64 */
            claim_ttl_seconds?: number | null;
            claimed_at?: string | null;
            claimed_by?: string | null;
            completed_at?: string | null;
            completed_by?: string | null;
            created_at: string;
            created_by?: string | null;
            /** Format: date */
            deadline?: string | null;
            description: string;
            effective_status: components["schemas"]["WorkItemStatus"];
            entity_id: components["schemas"]["EntityId"];
            metadata: unknown;
            result?: string | null;
            status: components["schemas"]["WorkItemStatus"];
            title: string;
            work_item_id: components["schemas"]["WorkItemId"];
        };
        /** @enum {string} */
        WorkItemStatus: "open" | "claimed" | "completed" | "cancelled";
        WorkerWorkspaceQuery: {
            workspace_id: components["schemas"]["WorkspaceId"];
        };
        WorkflowStatusResponse: {
            active_packet_id?: null | components["schemas"]["PacketId"];
            execution_status: string;
            fundraising_workflow?: null | components["schemas"]["FundraisingWorkflowResponse"];
            packet?: null | components["schemas"]["TransactionPacketResponse"];
            transfer_workflow?: null | components["schemas"]["TransferWorkflowResponse"];
            workflow_id: string;
            workflow_type: components["schemas"]["WorkflowType"];
        };
        /** @enum {string} */
        WorkflowType: "transfer" | "fundraising";
        WorkspaceClaimRequest: {
            claim_token: string;
        };
        WorkspaceClaimResponse: {
            claimed: boolean;
            workspace_id: components["schemas"]["WorkspaceId"];
        };
        WorkspaceContactSummary: {
            contact_id: string;
            entity_id: string;
        };
        WorkspaceEntitySummary: {
            entity_id: components["schemas"]["EntityId"];
        };
        /** Format: uuid */
        WorkspaceId: string;
        WorkspaceLinkRequest: {
            external_id: string;
            provider: string;
        };
        WorkspaceLinkResponse: {
            linked: boolean;
            provider: string;
            workspace_id: components["schemas"]["WorkspaceId"];
        };
        WorkspaceStatusResponse: {
            entity_count: number;
            name: string;
            status: string;
            workspace_id: components["schemas"]["WorkspaceId"];
        };
        WorkspaceSummary: {
            entity_count: number;
            name: string;
            workspace_id: components["schemas"]["WorkspaceId"];
        };
        WriteGovernanceAuditCheckpointRequest: {
            entity_id: components["schemas"]["EntityId"];
        };
        WrittenConsentRequest: {
            body_id: components["schemas"]["GovernanceBodyId"];
            description: string;
            entity_id: components["schemas"]["EntityId"];
            title: string;
        };
        WrittenConsentResponse: {
            body_id: components["schemas"]["GovernanceBodyId"];
            consent_type: string;
            created_at: string;
            meeting_id: components["schemas"]["MeetingId"];
            status: components["schemas"]["MeetingStatus"];
            title: string;
        };
    };
    responses: never;
    parameters: never;
    requestBodies: never;
    headers: never;
    pathItems: never;
};
export type $defs = Record<string, never>;
export interface operations {
    list_audit_events: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List recent audit events */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["AuditEvent"][];
                };
            };
        };
    };
    system_health: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description System health status */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["SystemHealth"];
                };
            };
        };
    };
    list_workspaces: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List all workspaces */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["WorkspaceSummary"][];
                };
            };
        };
    };
    list_agents: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List all agents */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["AgentResponse"][];
                };
            };
        };
    };
    create_agent: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateAgentRequest"];
            };
        };
        responses: {
            /** @description Agent created */
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["AgentResponse"];
                };
            };
        };
    };
    update_agent: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Agent ID */
                agent_id: components["schemas"]["AgentId"];
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["UpdateAgentRequest"];
            };
        };
        responses: {
            /** @description Agent updated */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["AgentResponse"];
                };
            };
        };
    };
    get_execution: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Agent identifier */
                agent_id: components["schemas"]["AgentId"];
                /** @description Execution identifier */
                execution_id: components["schemas"]["ExecutionId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Execution details */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ExecutionResponse"];
                };
            };
            /** @description Execution not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    kill_execution: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Agent identifier */
                agent_id: components["schemas"]["AgentId"];
                /** @description Execution identifier */
                execution_id: components["schemas"]["ExecutionId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Kill result */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["KillResponse"];
                };
            };
            /** @description Execution not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    get_execution_logs: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Agent identifier */
                agent_id: components["schemas"]["AgentId"];
                /** @description Execution identifier */
                execution_id: components["schemas"]["ExecutionId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Execution log entries */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": unknown[];
                };
            };
            /** @description Execution not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    get_execution_result: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Agent identifier */
                agent_id: components["schemas"]["AgentId"];
                /** @description Execution identifier */
                execution_id: components["schemas"]["ExecutionId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Execution result */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": unknown;
                };
            };
            /** @description Result not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    send_agent_message: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Agent ID */
                agent_id: components["schemas"]["AgentId"];
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["SendMessageRequest"];
            };
        };
        responses: {
            /** @description Message sent to agent */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["MessageResponse"];
                };
            };
        };
    };
    get_agent_message_internal: {
        parameters: {
            query: {
                /** @description Workspace ID */
                workspace_id: components["schemas"]["WorkspaceId"];
            };
            header?: never;
            path: {
                /** @description Agent ID */
                agent_id: components["schemas"]["AgentId"];
                /** @description Message ID */
                message_id: components["schemas"]["MessageId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Get agent message (internal) */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
        };
    };
    get_resolved_agent: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Agent ID */
                agent_id: components["schemas"]["AgentId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Get resolved agent with inherited config */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["AgentResponse"];
                };
            };
        };
    };
    add_agent_skill: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Agent ID */
                agent_id: components["schemas"]["AgentId"];
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["AddSkillRequest"];
            };
        };
        responses: {
            /** @description Skill added to agent */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["AgentResponse"];
                };
            };
        };
    };
    list_api_keys: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List of API keys */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiKeyResponse"][];
                };
            };
        };
    };
    create_api_key: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateApiKeyRequest"];
            };
        };
        responses: {
            /** @description API key created */
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiKeyResponse"];
                };
            };
            /** @description Invalid request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    revoke_api_key: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description API key ID to revoke */
                key_id: components["schemas"]["ApiKeyId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description API key revoked */
            204: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description API key not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    rotate_api_key: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description API key ID to rotate */
                key_id: components["schemas"]["ApiKeyId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Rotated API key */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiKeyResponse"];
                };
            };
            /** @description API key not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    token_exchange: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["TokenExchangeRequest"];
            };
        };
        responses: {
            /** @description Token exchange successful */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["TokenExchangeResponse"];
                };
            };
            /** @description Invalid request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Invalid API key */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    activate_bank_account: {
        parameters: {
            query: {
                entity_id: components["schemas"]["EntityId"];
            };
            header?: never;
            path: {
                /** @description Bank account ID */
                bank_account_id: components["schemas"]["BankAccountId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Bank account activated */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["BankAccountResponse"];
                };
            };
        };
    };
    close_bank_account: {
        parameters: {
            query: {
                entity_id: components["schemas"]["EntityId"];
            };
            header?: never;
            path: {
                /** @description Bank account ID */
                bank_account_id: components["schemas"]["BankAccountId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Bank account closed */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["BankAccountResponse"];
                };
            };
        };
    };
    list_branches: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List of branches */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["BranchListEntry"][];
                };
            };
        };
    };
    create_branch: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateBranchRequest"];
            };
        };
        responses: {
            /** @description Branch created */
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["CreateBranchResponse"];
                };
            };
            /** @description Invalid branch name */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    delete_branch_handler: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Branch name to delete */
                name: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Branch deleted */
            204: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Invalid branch name */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    merge_branch: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Branch name to merge */
                name: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["MergeBranchRequest"];
            };
        };
        responses: {
            /** @description Merge result */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["MergeBranchResponse"];
                };
            };
            /** @description Invalid branch name */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    prune_branch: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Branch name to prune */
                name: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Branch pruned */
            204: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Invalid branch name */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    scan_compliance_escalations: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["ScanComplianceRequest"];
            };
        };
        responses: {
            /** @description Compliance scan completed */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ComplianceScanResponse"];
                };
            };
            /** @description Invalid request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    resolve_escalation_with_evidence: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Escalation ID */
                escalation_id: components["schemas"]["ComplianceEscalationId"];
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["ResolveEscalationWithEvidenceRequest"];
            };
        };
        responses: {
            /** @description Escalation resolved with evidence */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ResolveEscalationWithEvidenceResponse"];
                };
            };
            /** @description Escalation not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    get_config: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description System configuration */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ConfigResponse"];
                };
            };
        };
    };
    create_contact: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateContactRequest"];
            };
        };
        responses: {
            /** @description Contact created */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ContactResponse"];
                };
            };
        };
    };
    get_contact: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Contact ID */
                contact_id: components["schemas"]["ContactId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Contact details */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ContactResponse"];
                };
            };
            /** @description Contact not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    update_contact: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Contact ID */
                contact_id: components["schemas"]["ContactId"];
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["UpdateContactRequest"];
            };
        };
        responses: {
            /** @description Contact updated */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ContactResponse"];
                };
            };
            /** @description Contact not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    get_notification_prefs: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Contact ID */
                contact_id: components["schemas"]["ContactId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Notification preferences */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["NotificationPrefsResponse"];
                };
            };
            /** @description Contact not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    update_notification_prefs: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Contact ID */
                contact_id: components["schemas"]["ContactId"];
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["UpdateNotificationPrefsRequest"];
            };
        };
        responses: {
            /** @description Notification preferences updated */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["NotificationPrefsResponse"];
                };
            };
            /** @description Contact not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    get_contact_profile: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Contact ID */
                contact_id: components["schemas"]["ContactId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Contact profile */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ContactProfileResponse"];
                };
            };
            /** @description Contact not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    classify_contractor: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["ClassifyContractorRequest"];
            };
        };
        responses: {
            /** @description Contractor classified */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ClassificationResponse"];
                };
            };
            /** @description Invalid request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    generate_contract: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["GenerateContractRequest"];
            };
        };
        responses: {
            /** @description Contract generated */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ContractResponse"];
                };
            };
            /** @description Entity not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    create_deadline: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateDeadlineRequest"];
            };
        };
        responses: {
            /** @description Deadline created */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["DeadlineResponse"];
                };
            };
            /** @description Invalid request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    demo_seed: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["DemoSeedRequest"];
            };
        };
        responses: {
            /** @description Seed demo data */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["DemoSeedResponse"];
                };
            };
        };
    };
    list_digests: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List digests */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["DigestSummary"][];
                };
            };
        };
    };
    trigger_digests: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Trigger digest generation */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["DigestTriggerResponse"];
                };
            };
        };
    };
    get_digest: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Digest key */
                digest_key: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Get digest by key */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
        };
    };
    create_distribution: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateDistributionRequest"];
            };
        };
        responses: {
            /** @description Distribution created */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["DistributionResponse"];
                };
            };
        };
    };
    fulfill_document_request: {
        parameters: {
            query: {
                /** @description Entity ID */
                entity_id: components["schemas"]["EntityId"];
            };
            header?: never;
            path: {
                /** @description Document request ID */
                request_id: components["schemas"]["DocumentRequestId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["DocumentRequestResponse"];
                };
            };
            /** @description Document request not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    mark_document_request_na: {
        parameters: {
            query: {
                /** @description Entity ID */
                entity_id: components["schemas"]["EntityId"];
            };
            header?: never;
            path: {
                /** @description Document request ID */
                request_id: components["schemas"]["DocumentRequestId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["DocumentRequestResponse"];
                };
            };
            /** @description Document request not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    preview_document_pdf: {
        parameters: {
            query: {
                entity_id: components["schemas"]["EntityId"];
                document_id: string;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Preview PDF document */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/pdf": unknown;
                };
            };
            /** @description Document definition not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Document does not apply to entity type */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    get_document: {
        parameters: {
            query: {
                /** @description Entity ID */
                entity_id: components["schemas"]["EntityId"];
            };
            header?: never;
            path: {
                /** @description Document ID */
                document_id: components["schemas"]["DocumentId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Document details */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["DocumentResponse"];
                };
            };
            /** @description Document not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    get_amendment_history: {
        parameters: {
            query: {
                /** @description Entity ID */
                entity_id: components["schemas"]["EntityId"];
            };
            header?: never;
            path: {
                /** @description Document ID */
                document_id: components["schemas"]["DocumentId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Amendment history */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["AmendmentHistoryEntry"][];
                };
            };
            /** @description Document not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    get_document_pdf: {
        parameters: {
            query: {
                /** @description Entity ID */
                entity_id: components["schemas"]["EntityId"];
            };
            header?: never;
            path: {
                /** @description Document ID */
                document_id: components["schemas"]["DocumentId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description PDF document */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/pdf": unknown;
                };
            };
            /** @description Document not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Document has no governance tag */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    request_document_copy: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Document ID */
                document_id: components["schemas"]["DocumentId"];
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["DocumentCopyRequest"];
            };
        };
        responses: {
            /** @description Document copy requested */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["DocumentCopyResponse"];
                };
            };
            /** @description Document not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    sign_document: {
        parameters: {
            query: {
                /** @description Entity ID */
                entity_id: components["schemas"]["EntityId"];
            };
            header?: never;
            path: {
                /** @description Document ID */
                document_id: components["schemas"]["DocumentId"];
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["SignDocumentRequest"];
            };
        };
        responses: {
            /** @description Document signed */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["SignDocumentResponse"];
                };
            };
            /** @description Invalid request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Document not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    list_entities: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List of entities */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["FormationStatusResponse"][];
                };
            };
        };
    };
    list_accounts: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Entity ID */
                entity_id: components["schemas"]["EntityId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List of accounts */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["AccountResponse"][];
                };
            };
        };
    };
    list_approval_artifacts: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Entity ID */
                entity_id: components["schemas"]["EntityId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApprovalArtifactResponse"][];
                };
            };
            /** @description Entity not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    list_bank_accounts: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Entity ID */
                entity_id: components["schemas"]["EntityId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List of bank accounts */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["BankAccountResponse"][];
                };
            };
        };
    };
    get_cap_table: {
        parameters: {
            query?: {
                basis?: components["schemas"]["CapTableBasis"];
                issuer_legal_entity_id?: null | components["schemas"]["LegalEntityId"];
            };
            header?: never;
            path: {
                /** @description Entity ID */
                entity_id: components["schemas"]["EntityId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Cap table */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["CapTableResponse"];
                };
            };
            /** @description Entity not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    list_entity_escalations: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Entity ID */
                entity_id: components["schemas"]["EntityId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List of compliance escalations */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ComplianceEscalationResponse"][];
                };
            };
            /** @description Entity not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    list_contacts: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List of contacts for entity */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ContactResponse"][];
                };
            };
        };
    };
    convert_entity: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Entity ID */
                entity_id: components["schemas"]["EntityId"];
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["ConvertEntityRequest"];
            };
        };
        responses: {
            /** @description Entity converted */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["FormationStatusResponse"];
                };
            };
            /** @description Entity not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    get_current_409a: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Entity ID */
                entity_id: components["schemas"]["EntityId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Current 409A valuation */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ValuationResponse"];
                };
            };
            /** @description No current 409A valuation found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    dissolve_entity: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Entity ID */
                entity_id: components["schemas"]["EntityId"];
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["DissolveEntityRequest"];
            };
        };
        responses: {
            /** @description Entity dissolved */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Entity not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    get_financial_statements: {
        parameters: {
            query?: {
                statement_type?: string;
            };
            header?: never;
            path: {
                /** @description Entity ID */
                entity_id: components["schemas"]["EntityId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Financial statements */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["FinancialStatementResponse"];
                };
            };
        };
    };
    list_governance_bodies: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Entity ID */
                entity_id: components["schemas"]["EntityId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List of governance bodies for entity */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["GovernanceBodyResponse"][];
                };
            };
        };
    };
    list_governance_documents: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Entity ID */
                entity_id: components["schemas"]["EntityId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List of governance documents */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["DocumentSummary"][];
                };
            };
            /** @description Entity not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    get_current_governance_document: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Entity ID */
                entity_id: components["schemas"]["EntityId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Current governance document */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["DocumentSummary"];
                };
            };
            /** @description Entity or governance document not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    list_governance_audit_checkpoints: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Entity ID */
                entity_id: components["schemas"]["EntityId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List of governance audit checkpoints */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["GovernanceAuditCheckpoint"][];
                };
            };
        };
    };
    list_governance_audit_entries: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Entity ID */
                entity_id: components["schemas"]["EntityId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List of governance audit entries */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["GovernanceAuditEntry"][];
                };
            };
        };
    };
    list_governance_audit_verifications: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Entity ID */
                entity_id: components["schemas"]["EntityId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List of governance audit verification reports */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["GovernanceAuditVerificationReport"][];
                };
            };
        };
    };
    list_governance_doc_bundles: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Entity ID */
                entity_id: components["schemas"]["EntityId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List of governance doc bundle summaries */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["GovernanceDocBundleSummary"][];
                };
            };
        };
    };
    get_current_governance_doc_bundle: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Entity ID */
                entity_id: components["schemas"]["EntityId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Current governance doc bundle */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["GovernanceDocBundleCurrent"];
                };
            };
            /** @description No bundle generated yet */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    generate_governance_doc_bundle: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Entity ID */
                entity_id: components["schemas"]["EntityId"];
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["GenerateGovernanceDocBundleRequest"];
            };
        };
        responses: {
            /** @description Generated governance doc bundle */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["GenerateGovernanceDocBundleResponse"];
                };
            };
            /** @description Validation error */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    get_governance_doc_bundle: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Entity ID */
                entity_id: components["schemas"]["EntityId"];
                /** @description Bundle ID */
                bundle_id: components["schemas"]["GovernanceDocBundleId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Governance doc bundle manifest */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["GovernanceDocBundleManifest"];
                };
            };
            /** @description Bundle not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    list_incidents: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Entity ID */
                entity_id: components["schemas"]["EntityId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List of governance incidents */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["IncidentResponse"][];
                };
            };
        };
    };
    list_governance_mode_history: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Entity ID */
                entity_id: components["schemas"]["EntityId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List of governance mode change events */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["GovernanceModeChangeEvent"][];
                };
            };
        };
    };
    get_governance_profile: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Entity ID */
                entity_id: components["schemas"]["EntityId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Governance profile */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["GovernanceProfile"];
                };
            };
            /** @description Entity not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    update_governance_profile: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Entity ID */
                entity_id: components["schemas"]["EntityId"];
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["UpdateGovernanceProfileRequest"];
            };
        };
        responses: {
            /** @description Updated governance profile */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["GovernanceProfile"];
                };
            };
            /** @description Validation error */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    list_governance_triggers: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Entity ID */
                entity_id: components["schemas"]["EntityId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List of governance trigger events */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["GovernanceTriggerEvent"][];
                };
            };
        };
    };
    list_intents: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Entity ID */
                entity_id: components["schemas"]["EntityId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["IntentResponse"][];
                };
            };
            /** @description Entity not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    list_invoices: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Entity ID */
                entity_id: components["schemas"]["EntityId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List of invoices */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["InvoiceResponse"][];
                };
            };
        };
    };
    list_journal_entries: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Entity ID */
                entity_id: components["schemas"]["EntityId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List of journal entries */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["JournalEntryResponse"][];
                };
            };
        };
    };
    list_obligations: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Entity ID */
                entity_id: components["schemas"]["EntityId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ObligationResponse"][];
                };
            };
            /** @description Entity not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    list_human_obligations: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Entity ID */
                entity_id: components["schemas"]["EntityId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ObligationResponse"][];
                };
            };
            /** @description Entity not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    obligations_summary: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Entity ID */
                entity_id: components["schemas"]["EntityId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ObligationsSummaryResponse"];
                };
            };
            /** @description Entity not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    list_entity_packets: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Entity ID */
                entity_id: components["schemas"]["EntityId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["TransactionPacketResponse"][];
                };
            };
            /** @description Entity not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    list_legacy_share_transfers: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List of share transfers */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    list_spending_limits: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Entity ID */
                entity_id: components["schemas"]["EntityId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List of spending limits */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["SpendingLimitResponse"][];
                };
            };
        };
    };
    get_stripe_account: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Entity ID */
                entity_id: components["schemas"]["EntityId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Stripe account details */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["StripeAccountResponse"];
                };
            };
        };
    };
    list_valuations: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Entity ID */
                entity_id: components["schemas"]["EntityId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List of valuations */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ValuationResponse"][];
                };
            };
            /** @description Entity not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    list_work_items: {
        parameters: {
            query?: {
                status?: null | components["schemas"]["WorkItemStatus"];
                category?: string | null;
            };
            header?: never;
            path: {
                /** @description Entity ID */
                entity_id: components["schemas"]["EntityId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List of work items */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["WorkItemResponse"][];
                };
            };
        };
    };
    create_work_item: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Entity ID */
                entity_id: components["schemas"]["EntityId"];
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateWorkItemRequest"];
            };
        };
        responses: {
            /** @description Work item created */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["WorkItemResponse"];
                };
            };
        };
    };
    get_work_item: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Entity ID */
                entity_id: components["schemas"]["EntityId"];
                /** @description Work Item ID */
                work_item_id: components["schemas"]["WorkItemId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Work item details */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["WorkItemResponse"];
                };
            };
            /** @description Work item not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    cancel_work_item: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Entity ID */
                entity_id: components["schemas"]["EntityId"];
                /** @description Work Item ID */
                work_item_id: components["schemas"]["WorkItemId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Work item cancelled */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["WorkItemResponse"];
                };
            };
            /** @description Work item not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Invalid state transition */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    claim_work_item: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Entity ID */
                entity_id: components["schemas"]["EntityId"];
                /** @description Work Item ID */
                work_item_id: components["schemas"]["WorkItemId"];
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["ClaimWorkItemRequest"];
            };
        };
        responses: {
            /** @description Work item claimed */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["WorkItemResponse"];
                };
            };
            /** @description Work item not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Invalid state transition */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    complete_work_item: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Entity ID */
                entity_id: components["schemas"]["EntityId"];
                /** @description Work Item ID */
                work_item_id: components["schemas"]["WorkItemId"];
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CompleteWorkItemRequest"];
            };
        };
        responses: {
            /** @description Work item completed */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["WorkItemResponse"];
                };
            };
            /** @description Work item not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Invalid state transition */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    release_work_item: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Entity ID */
                entity_id: components["schemas"]["EntityId"];
                /** @description Work Item ID */
                work_item_id: components["schemas"]["WorkItemId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Claim released */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["WorkItemResponse"];
                };
            };
            /** @description Work item not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Work item is not claimed */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    create_control_link: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateControlLinkRequest"];
            };
        };
        responses: {
            /** @description Control link created */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ControlLinkResponse"];
                };
            };
            /** @description Invalid request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    get_control_map: {
        parameters: {
            query: {
                entity_id: components["schemas"]["EntityId"];
                root_entity_id: components["schemas"]["LegalEntityId"];
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Control map */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ControlMapResponse"];
                };
            };
            /** @description Root entity not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    execute_conversion: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["ExecuteConversionRequest"];
            };
        };
        responses: {
            /** @description Conversion executed */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ConversionExecuteResponse"];
                };
            };
            /** @description Invalid request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Round not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    preview_conversion: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["PreviewConversionRequest"];
            };
        };
        responses: {
            /** @description Conversion preview */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ConversionPreviewResponse"];
                };
            };
            /** @description Invalid request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Round not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    get_dilution_preview: {
        parameters: {
            query: {
                entity_id: components["schemas"]["EntityId"];
                round_id: components["schemas"]["EquityRoundId"];
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Dilution preview */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["DilutionPreviewResponse"];
                };
            };
            /** @description Round not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    create_legal_entity: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateLegalEntityRequest"];
            };
        };
        responses: {
            /** @description Legal entity created */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["LegalEntityResponse"];
                };
            };
            /** @description Invalid request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    create_fundraising_workflow: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateFundraisingWorkflowRequest"];
            };
        };
        responses: {
            /** @description Fundraising workflow created */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["FundraisingWorkflowResponse"];
                };
            };
            /** @description Invalid request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    get_fundraising_workflow: {
        parameters: {
            query: {
                entity_id: components["schemas"]["EntityId"];
            };
            header?: never;
            path: {
                /** @description Fundraising workflow ID */
                workflow_id: components["schemas"]["FundraisingWorkflowId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Fundraising workflow details */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["FundraisingWorkflowResponse"];
                };
            };
            /** @description Workflow not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    apply_fundraising_workflow_terms: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Fundraising workflow ID */
                workflow_id: components["schemas"]["FundraisingWorkflowId"];
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["ApplyFundraisingTermsRequest"];
            };
        };
        responses: {
            /** @description Fundraising workflow terms applied */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["FundraisingWorkflowResponse"];
                };
            };
            /** @description Workflow not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    compile_fundraising_workflow_packet: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Fundraising workflow ID */
                workflow_id: components["schemas"]["FundraisingWorkflowId"];
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CompileWorkflowPacketRequest"];
            };
        };
        responses: {
            /** @description Fundraising workflow packet compiled */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["TransactionPacketResponse"];
                };
            };
            /** @description Workflow not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Workflow not ready for packet compilation */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    finalize_fundraising_workflow: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Fundraising workflow ID */
                workflow_id: components["schemas"]["FundraisingWorkflowId"];
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["FinalizeWorkflowRequest"];
            };
        };
        responses: {
            /** @description Fundraising workflow finalized */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["FundraisingWorkflowResponse"];
                };
            };
            /** @description Workflow not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Workflow not ready for finalization */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    generate_fundraising_board_packet: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Fundraising workflow ID */
                workflow_id: components["schemas"]["FundraisingWorkflowId"];
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["GenerateWorkflowDocsRequest"];
            };
        };
        responses: {
            /** @description Board packet generated */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["FundraisingWorkflowResponse"];
                };
            };
            /** @description Workflow not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    generate_fundraising_closing_packet: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Fundraising workflow ID */
                workflow_id: components["schemas"]["FundraisingWorkflowId"];
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["GenerateWorkflowDocsRequest"];
            };
        };
        responses: {
            /** @description Closing packet generated */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["FundraisingWorkflowResponse"];
                };
            };
            /** @description Workflow not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    prepare_fundraising_workflow_execution: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Fundraising workflow ID */
                workflow_id: components["schemas"]["FundraisingWorkflowId"];
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["PrepareWorkflowExecutionRequest"];
            };
        };
        responses: {
            /** @description Fundraising workflow execution prepared */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["FundraisingWorkflowResponse"];
                };
            };
            /** @description Workflow not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    record_fundraising_workflow_board_approval: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Fundraising workflow ID */
                workflow_id: components["schemas"]["FundraisingWorkflowId"];
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["RecordFundraisingBoardApprovalRequest"];
            };
        };
        responses: {
            /** @description Fundraising board approval recorded */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["FundraisingWorkflowResponse"];
                };
            };
            /** @description Workflow not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    record_fundraising_workflow_close: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Fundraising workflow ID */
                workflow_id: components["schemas"]["FundraisingWorkflowId"];
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["RecordFundraisingCloseRequest"];
            };
        };
        responses: {
            /** @description Fundraising close recorded */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["FundraisingWorkflowResponse"];
                };
            };
            /** @description Workflow not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    record_fundraising_workflow_acceptance: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Fundraising workflow ID */
                workflow_id: components["schemas"]["FundraisingWorkflowId"];
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["RecordFundraisingAcceptanceRequest"];
            };
        };
        responses: {
            /** @description Investor acceptance recorded */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["FundraisingWorkflowResponse"];
                };
            };
            /** @description Workflow not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    record_fundraising_workflow_signature: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Fundraising workflow ID */
                workflow_id: components["schemas"]["FundraisingWorkflowId"];
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["RecordWorkflowSignatureRequest"];
            };
        };
        responses: {
            /** @description Signature recorded */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["TransactionPacketResponse"];
                };
            };
            /** @description Invalid request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Workflow not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    start_fundraising_workflow_signatures: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Fundraising workflow ID */
                workflow_id: components["schemas"]["FundraisingWorkflowId"];
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["StartWorkflowSignaturesRequest"];
            };
        };
        responses: {
            /** @description Signature collection started */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["TransactionPacketResponse"];
                };
            };
            /** @description Workflow not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Workflow has no compiled packet */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    create_legacy_grant: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateLegacyGrantRequest"];
            };
        };
        responses: {
            /** @description Not implemented */
            501: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    create_holder: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateHolderRequest"];
            };
        };
        responses: {
            /** @description Holder created */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["HolderResponse"];
                };
            };
            /** @description Invalid request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    create_instrument: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateInstrumentRequest"];
            };
        };
        responses: {
            /** @description Instrument created */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["InstrumentResponse"];
                };
            };
            /** @description Invalid request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    adjust_position: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["AdjustPositionRequest"];
            };
        };
        responses: {
            /** @description Position adjusted */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["PositionResponse"];
                };
            };
            /** @description Invalid request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    create_round: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateRoundRequest"];
            };
        };
        responses: {
            /** @description Equity round created */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["RoundResponse"];
                };
            };
            /** @description Invalid request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    start_staged_round: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["StartStagedRoundRequest"];
            };
        };
        responses: {
            /** @description Staged round started */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["RoundResponse"];
                };
            };
            /** @description Invalid request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    accept_round: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Equity round ID */
                round_id: components["schemas"]["EquityRoundId"];
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["AcceptRoundRequest"];
            };
        };
        responses: {
            /** @description Round accepted */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["RoundResponse"];
                };
            };
            /** @description Invalid request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Round not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    apply_round_terms: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Equity round ID */
                round_id: components["schemas"]["EquityRoundId"];
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["ApplyRoundTermsRequest"];
            };
        };
        responses: {
            /** @description Round terms applied */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["RuleSetResponse"];
                };
            };
            /** @description Invalid request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Round not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    board_approve_round: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Equity round ID */
                round_id: components["schemas"]["EquityRoundId"];
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["BoardApproveRoundRequest"];
            };
        };
        responses: {
            /** @description Round board-approved */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["RoundResponse"];
                };
            };
            /** @description Invalid request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Round not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    issue_staged_round: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Equity round ID */
                round_id: components["schemas"]["EquityRoundId"];
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["IssueStagedRoundRequest"];
            };
        };
        responses: {
            /** @description Staged round issued */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["IssueStagedRoundResponse"];
                };
            };
            /** @description Invalid request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Round not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    add_round_security: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Equity round ID */
                round_id: components["schemas"]["EquityRoundId"];
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["AddSecurityRequest"];
            };
        };
        responses: {
            /** @description Security added to staged round */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["PendingSecurity"];
                };
            };
            /** @description Invalid request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Round not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    create_transfer_workflow: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateTransferWorkflowRequest"];
            };
        };
        responses: {
            /** @description Transfer workflow created */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["TransferWorkflowResponse"];
                };
            };
            /** @description Invalid request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    get_transfer_workflow: {
        parameters: {
            query: {
                entity_id: components["schemas"]["EntityId"];
            };
            header?: never;
            path: {
                /** @description Transfer workflow ID */
                workflow_id: components["schemas"]["TransferWorkflowId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Transfer workflow details */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["TransferWorkflowResponse"];
                };
            };
            /** @description Workflow not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    compile_transfer_workflow_packet: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Transfer workflow ID */
                workflow_id: components["schemas"]["TransferWorkflowId"];
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CompileWorkflowPacketRequest"];
            };
        };
        responses: {
            /** @description Transfer workflow packet compiled */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["TransactionPacketResponse"];
                };
            };
            /** @description Workflow not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Workflow not ready for packet compilation */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    finalize_transfer_workflow: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Transfer workflow ID */
                workflow_id: components["schemas"]["TransferWorkflowId"];
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["FinalizeWorkflowRequest"];
            };
        };
        responses: {
            /** @description Transfer workflow finalized */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["TransferWorkflowResponse"];
                };
            };
            /** @description Workflow not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Workflow not ready for finalization */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    generate_transfer_workflow_docs: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Transfer workflow ID */
                workflow_id: components["schemas"]["TransferWorkflowId"];
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["GenerateWorkflowDocsRequest"];
            };
        };
        responses: {
            /** @description Transfer workflow documents generated */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["TransferWorkflowResponse"];
                };
            };
            /** @description Workflow not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    prepare_transfer_workflow_execution: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Transfer workflow ID */
                workflow_id: components["schemas"]["TransferWorkflowId"];
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["PrepareWorkflowExecutionRequest"];
            };
        };
        responses: {
            /** @description Transfer workflow execution prepared */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["TransferWorkflowResponse"];
                };
            };
            /** @description Workflow not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    record_transfer_workflow_board_approval: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Transfer workflow ID */
                workflow_id: components["schemas"]["TransferWorkflowId"];
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["RecordTransferBoardApprovalRequest"];
            };
        };
        responses: {
            /** @description Transfer workflow board approval recorded */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["TransferWorkflowResponse"];
                };
            };
            /** @description Workflow not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    record_transfer_workflow_execution: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Transfer workflow ID */
                workflow_id: components["schemas"]["TransferWorkflowId"];
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["RecordTransferExecutionRequest"];
            };
        };
        responses: {
            /** @description Transfer workflow execution recorded */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["TransferWorkflowResponse"];
                };
            };
            /** @description Workflow not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    record_transfer_workflow_review: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Transfer workflow ID */
                workflow_id: components["schemas"]["TransferWorkflowId"];
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["RecordTransferReviewRequest"];
            };
        };
        responses: {
            /** @description Transfer workflow review recorded */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["TransferWorkflowResponse"];
                };
            };
            /** @description Invalid request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Workflow not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    record_transfer_workflow_rofr: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Transfer workflow ID */
                workflow_id: components["schemas"]["TransferWorkflowId"];
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["RecordTransferRofrRequest"];
            };
        };
        responses: {
            /** @description Transfer workflow ROFR recorded */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["TransferWorkflowResponse"];
                };
            };
            /** @description Workflow not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    record_transfer_workflow_signature: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Transfer workflow ID */
                workflow_id: components["schemas"]["TransferWorkflowId"];
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["RecordWorkflowSignatureRequest"];
            };
        };
        responses: {
            /** @description Signature recorded */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["TransactionPacketResponse"];
                };
            };
            /** @description Invalid request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Workflow not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    start_transfer_workflow_signatures: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Transfer workflow ID */
                workflow_id: components["schemas"]["TransferWorkflowId"];
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["StartWorkflowSignaturesRequest"];
            };
        };
        responses: {
            /** @description Signature collection started */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["TransactionPacketResponse"];
                };
            };
            /** @description Workflow not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Workflow has no compiled packet */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    submit_transfer_workflow_for_review: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Transfer workflow ID */
                workflow_id: components["schemas"]["TransferWorkflowId"];
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["SubmitTransferReviewRequest"];
            };
        };
        responses: {
            /** @description Transfer workflow submitted for review */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["TransferWorkflowResponse"];
                };
            };
            /** @description Workflow not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    get_workflow_status: {
        parameters: {
            query: {
                entity_id: components["schemas"]["EntityId"];
            };
            header?: never;
            path: {
                /** @description Workflow type (transfer or fundraising) */
                workflow_type: string;
                /** @description Workflow ID */
                workflow_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Workflow status */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["WorkflowStatusResponse"];
                };
            };
            /** @description Invalid workflow type */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Workflow not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    create_approval_artifact: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateApprovalArtifactRequest"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApprovalArtifactResponse"];
                };
            };
            /** @description Bad request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    create_intent: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateIntentRequest"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["IntentResponse"];
                };
            };
            /** @description Bad request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Unprocessable entity */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    create_obligation: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateObligationRequest"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ObligationResponse"];
                };
            };
            /** @description Bad request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    get_packet: {
        parameters: {
            query: {
                /** @description Entity ID */
                entity_id: components["schemas"]["EntityId"];
            };
            header?: never;
            path: {
                /** @description Packet ID */
                packet_id: components["schemas"]["PacketId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["TransactionPacketResponse"];
                };
            };
            /** @description Packet belongs to a different entity */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Packet not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    create_formation: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateFormationRequest"];
            };
        };
        responses: {
            /** @description Formation created */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["FormationResponse"];
                };
            };
            /** @description Invalid request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    create_pending_formation: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreatePendingFormationRequest"];
            };
        };
        responses: {
            /** @description Pending formation created */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["PendingFormationResponse"];
                };
            };
            /** @description Invalid request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    create_formation_with_cap_table: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateFormationRequest"];
            };
        };
        responses: {
            /** @description Formation created with cap table */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["FormationWithCapTableResponse"];
                };
            };
            /** @description Invalid request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    get_formation: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Entity ID */
                entity_id: components["schemas"]["EntityId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Formation status */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["FormationStatusResponse"];
                };
            };
            /** @description Entity not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    apply_ein: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Entity ID */
                entity_id: components["schemas"]["EntityId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description EIN application submitted */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["FormationStatusResponse"];
                };
            };
            /** @description Entity not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    list_documents: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Entity ID */
                entity_id: components["schemas"]["EntityId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List of formation documents */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["DocumentSummary"][];
                };
            };
            /** @description Entity not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    confirm_ein: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Entity ID */
                entity_id: components["schemas"]["EntityId"];
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["ConfirmEinRequest"];
            };
        };
        responses: {
            /** @description EIN confirmed */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["FormationStatusResponse"];
                };
            };
            /** @description Invalid EIN format */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Entity not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    record_filing_attestation: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Entity ID */
                entity_id: components["schemas"]["EntityId"];
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["FilingAttestationRequest"];
            };
        };
        responses: {
            /** @description Filing attestation recorded */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["FormationGatesResponse"];
                };
            };
            /** @description Entity not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    confirm_filing: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Entity ID */
                entity_id: components["schemas"]["EntityId"];
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["ConfirmFilingRequest"];
            };
        };
        responses: {
            /** @description Filing confirmed */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["FormationStatusResponse"];
                };
            };
            /** @description Entity not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    finalize_pending_formation: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Entity ID */
                entity_id: components["schemas"]["EntityId"];
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["FinalizePendingFormationRequest"];
            };
        };
        responses: {
            /** @description Formation finalized with cap table */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["FormationWithCapTableResponse"];
                };
            };
            /** @description Entity not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    add_founder: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Entity ID */
                entity_id: components["schemas"]["EntityId"];
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["AddFounderRequest"];
            };
        };
        responses: {
            /** @description Founder added */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["AddFounderResponse"];
                };
            };
            /** @description Entity not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    get_formation_gates: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Entity ID */
                entity_id: components["schemas"]["EntityId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Formation gates status */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["FormationGatesResponse"];
                };
            };
            /** @description Entity not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    mark_documents_signed: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Entity ID */
                entity_id: components["schemas"]["EntityId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Documents marked as signed */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["FormationStatusResponse"];
                };
            };
            /** @description Not all documents are fully signed */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Entity not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    add_registered_agent_consent_evidence: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Entity ID */
                entity_id: components["schemas"]["EntityId"];
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["RegisteredAgentConsentEvidenceRequest"];
            };
        };
        responses: {
            /** @description Registered agent consent evidence added */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["FormationGatesResponse"];
                };
            };
            /** @description Entity not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    execute_service_agreement: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Entity ID */
                entity_id: components["schemas"]["EntityId"];
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["ExecuteServiceAgreementRequest"];
            };
        };
        responses: {
            /** @description Service agreement executed */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["FormationGatesResponse"];
                };
            };
            /** @description Entity or contract not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Validation error */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    submit_filing: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Entity ID */
                entity_id: components["schemas"]["EntityId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Filing submitted */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["FormationStatusResponse"];
                };
            };
            /** @description Filing submission blocked */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Entity not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    list_all_governance_bodies: {
        parameters: {
            query: {
                entity_id: components["schemas"]["EntityId"];
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List of all governance bodies */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["GovernanceBodyResponse"][];
                };
            };
        };
    };
    create_governance_body: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateGovernanceBodyRequest"];
            };
        };
        responses: {
            /** @description Created governance body */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["GovernanceBodyResponse"];
                };
            };
        };
    };
    list_meetings: {
        parameters: {
            query: {
                entity_id: components["schemas"]["EntityId"];
            };
            header?: never;
            path: {
                /** @description Governance body ID */
                body_id: components["schemas"]["GovernanceBodyId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List of meetings for body */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["MeetingResponse"][];
                };
            };
        };
    };
    list_seats: {
        parameters: {
            query: {
                entity_id: components["schemas"]["EntityId"];
            };
            header?: never;
            path: {
                /** @description Governance body ID */
                body_id: components["schemas"]["GovernanceBodyId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List of governance seats for body */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["GovernanceSeatResponse"][];
                };
            };
        };
    };
    create_seat: {
        parameters: {
            query: {
                entity_id: components["schemas"]["EntityId"];
            };
            header?: never;
            path: {
                /** @description Governance body ID */
                body_id: components["schemas"]["GovernanceBodyId"];
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateSeatRequest"];
            };
        };
        responses: {
            /** @description Created governance seat */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["GovernanceSeatResponse"];
                };
            };
            /** @description Governance body not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    scan_expired_seats: {
        parameters: {
            query: {
                entity_id: components["schemas"]["EntityId"];
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Scan expired seats result */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ScanExpiredResponse"];
                };
            };
        };
    };
    resign_seat: {
        parameters: {
            query: {
                entity_id: components["schemas"]["EntityId"];
            };
            header?: never;
            path: {
                /** @description Governance seat ID */
                seat_id: components["schemas"]["GovernanceSeatId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Resigned governance seat */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["GovernanceSeatResponse"];
                };
            };
            /** @description Seat not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    write_governance_audit_checkpoint: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["WriteGovernanceAuditCheckpointRequest"];
            };
        };
        responses: {
            /** @description Written governance audit checkpoint */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["GovernanceAuditCheckpoint"];
                };
            };
            /** @description No audit entries to checkpoint */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    create_governance_audit_event: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateGovernanceAuditEventRequest"];
            };
        };
        responses: {
            /** @description Created governance audit entry */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["GovernanceAuditEntry"];
                };
            };
            /** @description Invalid request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    verify_governance_audit_chain: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["VerifyGovernanceAuditChainRequest"];
            };
        };
        responses: {
            /** @description Governance audit chain verification report */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["GovernanceAuditVerificationReport"];
                };
            };
        };
    };
    get_delegation_schedule: {
        parameters: {
            query: {
                entity_id: components["schemas"]["EntityId"];
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Current delegation schedule */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["DelegationSchedule"];
                };
            };
        };
    };
    amend_delegation_schedule: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["AmendDelegationScheduleRequest"];
            };
        };
        responses: {
            /** @description Amended delegation schedule */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["DelegationScheduleChangeResponse"];
                };
            };
            /** @description Invalid request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Authority expansion requires resolution */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    list_delegation_schedule_history: {
        parameters: {
            query: {
                entity_id: components["schemas"]["EntityId"];
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List of delegation schedule amendments */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ScheduleAmendment"][];
                };
            };
        };
    };
    reauthorize_delegation_schedule: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["ReauthorizeDelegationScheduleRequest"];
            };
        };
        responses: {
            /** @description Reauthorized delegation schedule */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["DelegationScheduleChangeResponse"];
                };
            };
            /** @description Meeting or resolution not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Resolution did not pass */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    evaluate_governance: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["EvaluateGovernanceRequest"];
            };
        };
        responses: {
            /** @description Policy evaluation decision */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["PolicyDecision"];
                };
            };
        };
    };
    create_incident: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateIncidentRequest"];
            };
        };
        responses: {
            /** @description Created governance incident */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["IncidentResponse"];
                };
            };
        };
    };
    resolve_incident: {
        parameters: {
            query: {
                entity_id: components["schemas"]["EntityId"];
            };
            header?: never;
            path: {
                /** @description Incident ID */
                incident_id: components["schemas"]["IncidentId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Resolved incident */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["IncidentResponse"];
                };
            };
            /** @description Incident not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    get_governance_mode: {
        parameters: {
            query: {
                entity_id: components["schemas"]["EntityId"];
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Current governance mode */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["GovernanceModeResponse"];
                };
            };
        };
    };
    set_governance_mode: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["SetGovernanceModeRequest"];
            };
        };
        responses: {
            /** @description Updated governance mode */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["GovernanceModeResponse"];
                };
            };
            /** @description Validation error */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    list_global_human_obligations: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ObligationResponse"][];
                };
            };
        };
    };
    fulfill_human_obligation: {
        parameters: {
            query: {
                /** @description Entity ID */
                entity_id: components["schemas"]["EntityId"];
            };
            header?: never;
            path: {
                /** @description Obligation ID */
                obligation_id: components["schemas"]["ObligationId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ObligationResponse"];
                };
            };
            /** @description Obligation not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    generate_signer_token: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Obligation ID */
                obligation_id: components["schemas"]["ObligationId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["SignerTokenResponse"];
                };
            };
        };
    };
    get_signing_pdf: {
        parameters: {
            query: {
                /** @description Signing token */
                token: string;
            };
            header?: never;
            path: {
                /** @description Document ID */
                document_id: components["schemas"]["DocumentId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description PDF preview for signing */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/pdf": unknown;
                };
            };
            /** @description Invalid token or document not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    resolve_signing_link: {
        parameters: {
            query: {
                /** @description Signing token */
                token: string;
            };
            header?: never;
            path: {
                /** @description Document ID */
                document_id: components["schemas"]["DocumentId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Document metadata for signing UI */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["SigningResolveResponse"];
                };
            };
            /** @description Invalid token or document not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    submit_signing: {
        parameters: {
            query: {
                /** @description Signing token */
                token: string;
            };
            header?: never;
            path: {
                /** @description Document ID */
                document_id: components["schemas"]["DocumentId"];
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["SignDocumentRequest"];
            };
        };
        responses: {
            /** @description Document signed */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["SignDocumentResponse"];
                };
            };
            /** @description Invalid request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Invalid token or document not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    authorize_intent: {
        parameters: {
            query: {
                /** @description Entity ID */
                entity_id: components["schemas"]["EntityId"];
            };
            header?: never;
            path: {
                /** @description Intent ID */
                intent_id: components["schemas"]["IntentId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["IntentResponse"];
                };
            };
            /** @description Intent not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Blocked by policy or missing prerequisites */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    bind_approval_artifact_to_intent: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Intent ID */
                intent_id: components["schemas"]["IntentId"];
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["BindApprovalArtifactRequest"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["IntentResponse"];
                };
            };
            /** @description Intent or approval artifact not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    bind_document_request_to_intent: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Intent ID */
                intent_id: components["schemas"]["IntentId"];
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["BindDocumentRequestRequest"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["IntentResponse"];
                };
            };
            /** @description Intent or document request not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Document request belongs to a different entity */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    cancel_intent: {
        parameters: {
            query: {
                /** @description Entity ID */
                entity_id: components["schemas"]["EntityId"];
            };
            header?: never;
            path: {
                /** @description Intent ID */
                intent_id: components["schemas"]["IntentId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["IntentResponse"];
                };
            };
            /** @description Intent not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    evaluate_intent: {
        parameters: {
            query: {
                /** @description Entity ID */
                entity_id: components["schemas"]["EntityId"];
            };
            header?: never;
            path: {
                /** @description Intent ID */
                intent_id: components["schemas"]["IntentId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["IntentResponse"];
                };
            };
            /** @description Intent not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    execute_intent: {
        parameters: {
            query: {
                /** @description Entity ID */
                entity_id: components["schemas"]["EntityId"];
            };
            header?: never;
            path: {
                /** @description Intent ID */
                intent_id: components["schemas"]["IntentId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["IntentResponse"];
                };
            };
            /** @description Intent not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Blocked by policy or missing prerequisites */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    list_receipts_by_intent: {
        parameters: {
            query: {
                /** @description Entity ID */
                entity_id: components["schemas"]["EntityId"];
            };
            header?: never;
            path: {
                /** @description Intent ID */
                intent_id: components["schemas"]["IntentId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ReceiptResponse"][];
                };
            };
            /** @description Entity not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    mint_agent_token: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["MintAgentTokenRequest"];
            };
        };
        responses: {
            /** @description Mint an agent token (internal) */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["MintAgentTokenResponse"];
                };
            };
        };
    };
    list_active_agents_internal: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List active agents with cron channels (internal) */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["InternalCronAgentResponse"][];
                };
            };
        };
    };
    get_resolved_agent_internal: {
        parameters: {
            query: {
                /** @description Workspace ID */
                workspace_id: components["schemas"]["WorkspaceId"];
            };
            header?: never;
            path: {
                /** @description Agent ID */
                agent_id: components["schemas"]["AgentId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Get resolved agent definition (internal) */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
        };
    };
    resolve_secrets: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["ResolveSecretsRequest"];
            };
        };
        responses: {
            /** @description Resolved secrets */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ResolveSecretsResponse"];
                };
            };
            /** @description Proxy or workspace not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    ingest_lockdown_trigger: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Workspace ID */
                workspace_id: components["schemas"]["WorkspaceId"];
                /** @description Entity ID */
                entity_id: components["schemas"]["EntityId"];
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["InternalLockdownTriggerRequest"];
            };
        };
        responses: {
            /** @description Lockdown trigger result */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["InternalLockdownTriggerResponse"];
                };
            };
            /** @description Entity not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    from_agent_request: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["AgentInvoiceRequest"];
            };
        };
        responses: {
            /** @description Invoice created from agent request */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["PaymentOfferResponse"];
                };
            };
        };
    };
    get_invoice: {
        parameters: {
            query: {
                entity_id: components["schemas"]["EntityId"];
            };
            header?: never;
            path: {
                /** @description Invoice ID */
                invoice_id: components["schemas"]["InvoiceId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Invoice details */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["InvoiceResponse"];
                };
            };
        };
    };
    mark_invoice_paid: {
        parameters: {
            query: {
                entity_id: components["schemas"]["EntityId"];
            };
            header?: never;
            path: {
                /** @description Invoice ID */
                invoice_id: components["schemas"]["InvoiceId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Invoice marked as paid */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["InvoiceResponse"];
                };
            };
        };
    };
    get_pay_instructions: {
        parameters: {
            query: {
                entity_id: components["schemas"]["EntityId"];
            };
            header?: never;
            path: {
                /** @description Invoice ID */
                invoice_id: components["schemas"]["InvoiceId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Payment instructions for invoice */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["PayInstructionsResponse"];
                };
            };
        };
    };
    send_invoice: {
        parameters: {
            query: {
                entity_id: components["schemas"]["EntityId"];
            };
            header?: never;
            path: {
                /** @description Invoice ID */
                invoice_id: components["schemas"]["InvoiceId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Invoice sent */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["InvoiceResponse"];
                };
            };
        };
    };
    get_invoice_status: {
        parameters: {
            query: {
                entity_id: components["schemas"]["EntityId"];
            };
            header?: never;
            path: {
                /** @description Invoice ID */
                invoice_id: components["schemas"]["InvoiceId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Invoice status */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["InvoiceResponse"];
                };
            };
        };
    };
    post_journal_entry: {
        parameters: {
            query: {
                entity_id: components["schemas"]["EntityId"];
            };
            header?: never;
            path: {
                /** @description Journal entry ID */
                entry_id: components["schemas"]["JournalEntryId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Journal entry posted */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["JournalEntryResponse"];
                };
            };
        };
    };
    void_journal_entry: {
        parameters: {
            query: {
                entity_id: components["schemas"]["EntityId"];
            };
            header?: never;
            path: {
                /** @description Journal entry ID */
                entry_id: components["schemas"]["JournalEntryId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Journal entry voided */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["JournalEntryResponse"];
                };
            };
        };
    };
    get_jwks: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Get JWKS keys */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["JwksResponse"];
                };
            };
        };
    };
    reconcile_ledger: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["ReconcileLedgerRequest"];
            };
        };
        responses: {
            /** @description Ledger reconciled */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ReconciliationResponse"];
                };
            };
        };
    };
    proxy_handler: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Upstream API path */
                path: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Proxied LLM response */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    list_all_meetings: {
        parameters: {
            query: {
                entity_id: components["schemas"]["EntityId"];
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List of all meetings */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["MeetingResponse"][];
                };
            };
        };
    };
    schedule_meeting: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["ScheduleMeetingRequest"];
            };
        };
        responses: {
            /** @description Scheduled meeting */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["MeetingResponse"];
                };
            };
            /** @description Governance body not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    written_consent: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["WrittenConsentRequest"];
            };
        };
        responses: {
            /** @description Written consent meeting created */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["WrittenConsentResponse"];
                };
            };
            /** @description Governance body not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    adjourn_meeting: {
        parameters: {
            query: {
                entity_id: components["schemas"]["EntityId"];
            };
            header?: never;
            path: {
                /** @description Meeting ID */
                meeting_id: components["schemas"]["MeetingId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Adjourned meeting */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["MeetingResponse"];
                };
            };
            /** @description Meeting not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    list_agenda_items: {
        parameters: {
            query: {
                entity_id: components["schemas"]["EntityId"];
            };
            header?: never;
            path: {
                /** @description Meeting ID */
                meeting_id: components["schemas"]["MeetingId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List of agenda items for meeting */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["AgendaItemResponse"][];
                };
            };
            /** @description Meeting not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    finalize_agenda_item: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Meeting ID */
                meeting_id: components["schemas"]["MeetingId"];
                /** @description Agenda item ID */
                item_id: components["schemas"]["AgendaItemId"];
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["FinalizeAgendaItemRequest"];
            };
        };
        responses: {
            /** @description Finalized agenda item */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["AgendaItemResponse"];
                };
            };
            /** @description Meeting or agenda item not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Agenda item already finalized */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Cannot finalize without resolution */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    compute_resolution: {
        parameters: {
            query: {
                entity_id: components["schemas"]["EntityId"];
            };
            header?: never;
            path: {
                /** @description Meeting ID */
                meeting_id: components["schemas"]["MeetingId"];
                /** @description Agenda item ID */
                item_id: components["schemas"]["AgendaItemId"];
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["ComputeResolutionRequest"];
            };
        };
        responses: {
            /** @description Computed resolution */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ResolutionResponse"];
                };
            };
            /** @description Meeting or agenda item not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Resolution already exists for agenda item */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    cast_vote: {
        parameters: {
            query: {
                entity_id: components["schemas"]["EntityId"];
            };
            header?: never;
            path: {
                /** @description Meeting ID */
                meeting_id: components["schemas"]["MeetingId"];
                /** @description Agenda item ID */
                item_id: components["schemas"]["AgendaItemId"];
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CastVoteRequest"];
            };
        };
        responses: {
            /** @description Cast vote */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["VoteResponse"];
                };
            };
            /** @description Meeting or agenda item not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Duplicate vote */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    list_votes: {
        parameters: {
            query: {
                entity_id: components["schemas"]["EntityId"];
            };
            header?: never;
            path: {
                /** @description Meeting ID */
                meeting_id: components["schemas"]["MeetingId"];
                /** @description Agenda item ID */
                item_id: components["schemas"]["AgendaItemId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List of votes for agenda item */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["VoteResponse"][];
                };
            };
        };
    };
    cancel_meeting: {
        parameters: {
            query: {
                entity_id: components["schemas"]["EntityId"];
            };
            header?: never;
            path: {
                /** @description Meeting ID */
                meeting_id: components["schemas"]["MeetingId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Cancelled meeting */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["MeetingResponse"];
                };
            };
            /** @description Meeting not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    convene_meeting: {
        parameters: {
            query: {
                entity_id: components["schemas"]["EntityId"];
            };
            header?: never;
            path: {
                /** @description Meeting ID */
                meeting_id: components["schemas"]["MeetingId"];
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["ConveneMeetingRequest"];
            };
        };
        responses: {
            /** @description Convened meeting */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["MeetingResponse"];
                };
            };
            /** @description Meeting not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    send_notice: {
        parameters: {
            query: {
                entity_id: components["schemas"]["EntityId"];
            };
            header?: never;
            path: {
                /** @description Meeting ID */
                meeting_id: components["schemas"]["MeetingId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Meeting with notice sent */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["MeetingResponse"];
                };
            };
            /** @description Meeting not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    list_resolutions: {
        parameters: {
            query: {
                entity_id: components["schemas"]["EntityId"];
            };
            header?: never;
            path: {
                /** @description Meeting ID */
                meeting_id: components["schemas"]["MeetingId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List of resolutions for meeting */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ResolutionResponse"][];
                };
            };
        };
    };
    attach_resolution_document: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Meeting ID */
                meeting_id: components["schemas"]["MeetingId"];
                /** @description Resolution ID */
                resolution_id: components["schemas"]["ResolutionId"];
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["AttachResolutionDocumentRequest"];
            };
        };
        responses: {
            /** @description Resolution with attached document */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ResolutionResponse"];
                };
            };
            /** @description Resolution or document not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Resolution already has a document attached */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    global_obligations_summary: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ObligationsSummaryResponse"];
                };
            };
        };
    };
    assign_obligation: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Obligation ID */
                obligation_id: components["schemas"]["ObligationId"];
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["AssignObligationRequest"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ObligationResponse"];
                };
            };
            /** @description Obligation not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    list_document_requests: {
        parameters: {
            query: {
                /** @description Entity ID */
                entity_id: components["schemas"]["EntityId"];
            };
            header?: never;
            path: {
                /** @description Obligation ID */
                obligation_id: components["schemas"]["ObligationId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["DocumentRequestResponse"][];
                };
            };
            /** @description Obligation not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    create_document_request: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Obligation ID */
                obligation_id: components["schemas"]["ObligationId"];
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateDocumentRequestPayload"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["DocumentRequestResponse"];
                };
            };
            /** @description Obligation not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    expire_obligation: {
        parameters: {
            query: {
                /** @description Entity ID */
                entity_id: components["schemas"]["EntityId"];
            };
            header?: never;
            path: {
                /** @description Obligation ID */
                obligation_id: components["schemas"]["ObligationId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ObligationResponse"];
                };
            };
            /** @description Obligation not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    fulfill_obligation: {
        parameters: {
            query: {
                /** @description Entity ID */
                entity_id: components["schemas"]["EntityId"];
            };
            header?: never;
            path: {
                /** @description Obligation ID */
                obligation_id: components["schemas"]["ObligationId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ObligationResponse"];
                };
            };
            /** @description Obligation not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    waive_obligation: {
        parameters: {
            query: {
                /** @description Entity ID */
                entity_id: components["schemas"]["EntityId"];
            };
            header?: never;
            path: {
                /** @description Obligation ID */
                obligation_id: components["schemas"]["ObligationId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ObligationResponse"];
                };
            };
            /** @description Obligation not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    submit_payment: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["SubmitPaymentRequest"];
            };
        };
        responses: {
            /** @description Payment submitted */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["PaymentResponse"];
                };
            };
        };
    };
    execute_payment: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["ExecutePaymentRequest"];
            };
        };
        responses: {
            /** @description Payment executed */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["PaymentResponse"];
                };
            };
        };
    };
    create_payroll_run: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreatePayrollRunRequest"];
            };
        };
        responses: {
            /** @description Payroll run created */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["PayrollRunResponse"];
                };
            };
        };
    };
    get_receipt: {
        parameters: {
            query: {
                /** @description Entity ID */
                entity_id: components["schemas"]["EntityId"];
            };
            header?: never;
            path: {
                /** @description Receipt ID */
                receipt_id: components["schemas"]["ReceiptId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ReceiptResponse"];
                };
            };
            /** @description Receipt not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    interpolate_template: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["InterpolateRequest"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["InterpolateResponse"];
                };
            };
        };
    };
    resolve_token: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["ResolveRequest"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ResolveResponse"];
                };
            };
        };
    };
    get_service_token: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Get a service token */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ServiceTokenResponse"];
                };
            };
        };
    };
    create_legacy_share_transfer: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateLegacyShareTransferRequest"];
            };
        };
        responses: {
            /** @description Transfer created */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    get_signing_link: {
        parameters: {
            query: {
                /** @description Entity ID */
                entity_id: components["schemas"]["EntityId"];
            };
            header?: never;
            path: {
                /** @description Document ID */
                document_id: components["schemas"]["DocumentId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Signing link */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["SigningLinkResponse"];
                };
            };
            /** @description Document not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    create_spending_limit: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateSpendingLimitRequest"];
            };
        };
        responses: {
            /** @description Spending limit created */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["SpendingLimitResponse"];
                };
            };
        };
    };
    file_tax_document: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["FileTaxDocumentRequest"];
            };
        };
        responses: {
            /** @description Tax document filed */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["TaxFilingResponse"];
                };
            };
            /** @description Invalid request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    create_account: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateAccountRequest"];
            };
        };
        responses: {
            /** @description Account created */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["AccountResponse"];
                };
            };
        };
    };
    create_bank_account: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateBankAccountRequest"];
            };
        };
        responses: {
            /** @description Bank account created */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["BankAccountResponse"];
                };
            };
        };
    };
    get_chart_of_accounts: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Entity ID */
                entity_id: components["schemas"]["EntityId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Chart of accounts */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ChartOfAccountsResponse"];
                };
            };
        };
    };
    create_invoice: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateInvoiceRequest"];
            };
        };
        responses: {
            /** @description Invoice created */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["InvoiceResponse"];
                };
            };
        };
    };
    create_journal_entry: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateJournalEntryRequest"];
            };
        };
        responses: {
            /** @description Journal entry created */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["JournalEntryResponse"];
                };
            };
        };
    };
    create_payment_intent: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreatePaymentIntentRequest"];
            };
        };
        responses: {
            /** @description Payment intent created */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["PaymentIntentResponse"];
                };
            };
        };
    };
    create_payout: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreatePayoutRequest"];
            };
        };
        responses: {
            /** @description Payout created */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["PayoutResponse"];
                };
            };
        };
    };
    seed_chart_of_accounts: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["SeedChartOfAccountsRequest"];
            };
        };
        responses: {
            /** @description Chart of accounts seeded */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["SeedChartOfAccountsResponse"];
                };
            };
        };
    };
    create_stripe_account: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateStripeAccountRequest"];
            };
        };
        responses: {
            /** @description Stripe account created */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["StripeAccountResponse"];
                };
            };
        };
    };
    treasury_stripe_webhook: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": string;
            };
        };
        responses: {
            /** @description Webhook received */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": unknown;
                };
            };
        };
    };
    create_valuation: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateValuationRequest"];
            };
        };
        responses: {
            /** @description Valuation created */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ValuationResponse"];
                };
            };
            /** @description Invalid request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    approve_valuation: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Valuation ID */
                valuation_id: components["schemas"]["ValuationId"];
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["ApproveValuationRequest"];
            };
        };
        responses: {
            /** @description Valuation approved */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ValuationResponse"];
                };
            };
            /** @description Invalid request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Valuation not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    submit_valuation_for_approval: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Valuation ID */
                valuation_id: components["schemas"]["ValuationId"];
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["SubmitValuationForApprovalRequest"];
            };
        };
        responses: {
            /** @description Valuation submitted for approval */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ValuationResponse"];
                };
            };
            /** @description Invalid request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Valuation not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    list_workspace_entities: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List entities in current workspace */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["WorkspaceEntitySummary"][];
                };
            };
        };
    };
    workspace_status: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Current workspace status */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["WorkspaceStatusResponse"];
                };
            };
        };
    };
    claim_workspace: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["WorkspaceClaimRequest"];
            };
        };
        responses: {
            /** @description Claim a workspace */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["WorkspaceClaimResponse"];
                };
            };
        };
    };
    link_workspace: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["WorkspaceLinkRequest"];
            };
        };
        responses: {
            /** @description Link workspace to external provider */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["WorkspaceLinkResponse"];
                };
            };
        };
    };
    provision_workspace: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["ProvisionWorkspaceRequest"];
            };
        };
        responses: {
            /** @description Workspace provisioned */
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ProvisionWorkspaceResponse"];
                };
            };
            /** @description Invalid request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    workspace_contacts: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Workspace ID */
                workspace_id: components["schemas"]["WorkspaceId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List contacts in workspace */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["WorkspaceContactSummary"][];
                };
            };
        };
    };
    workspace_entities_by_path: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Workspace ID */
                workspace_id: components["schemas"]["WorkspaceId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List entities in workspace */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["WorkspaceEntitySummary"][];
                };
            };
        };
    };
    list_proxies: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List of secret proxies */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ProxyResponse"][];
                };
            };
        };
    };
    create_proxy: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateProxyRequest"];
            };
        };
        responses: {
            /** @description Secret proxy created */
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ProxyResponse"];
                };
            };
            /** @description Invalid request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Proxy already exists */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    get_proxy: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Secret proxy details */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ProxyResponse"];
                };
            };
            /** @description Proxy not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    list_secret_names: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List of secret names */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["SecretNamesResponse"];
                };
            };
            /** @description Proxy not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    set_secrets: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["SetSecretsRequest"];
            };
        };
        responses: {
            /** @description Secrets updated */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["SecretNamesResponse"];
                };
            };
            /** @description Proxy not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    workspace_status_by_path: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Workspace ID */
                workspace_id: components["schemas"]["WorkspaceId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Workspace status by ID */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["WorkspaceStatusResponse"];
                };
            };
        };
    };
}
