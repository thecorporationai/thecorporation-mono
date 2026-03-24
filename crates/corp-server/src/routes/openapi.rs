//! OpenAPI 3.0 spec generation.
//!
//! `GET /openapi.json` returns a hand-maintained OpenAPI 3.0.3 document that
//! describes every endpoint exposed by this server.  The spec is built
//! programmatically with `serde_json::json!` so there is no macro magic or
//! proc-macro dependency (utoipa, etc.).
//!
//! To add a new endpoint: add a path entry under `paths` in [`build_spec`].

use axum::{Json, Router, routing::get};
use serde_json::{Value, json};

use crate::state::AppState;

// ── Router ────────────────────────────────────────────────────────────────────

pub fn routes() -> Router<AppState> {
    Router::new().route("/openapi.json", get(openapi_spec))
}

async fn openapi_spec() -> Json<Value> {
    Json(build_spec())
}

// ── Spec builder ──────────────────────────────────────────────────────────────

fn build_spec() -> Value {
    json!({
        "openapi": "3.0.3",
        "info": {
            "title": "TheCorporation API",
            "version": env!("CARGO_PKG_VERSION"),
            "description": "Corporate governance, cap table, treasury, and execution API.",
            "contact": {
                "name": "TheCorporation",
                "url": "https://thecorporation.com"
            }
        },
        "servers": [
            { "url": "https://api.thecorporation.com", "description": "Production" }
        ],
        "components": {
            "securitySchemes": {
                "BearerAuth": {
                    "type": "http",
                    "scheme": "bearer",
                    "description": "API key issued by POST /v1/api-keys"
                }
            },
            "schemas": schemas()
        },
        "security": [{ "BearerAuth": [] }],
        "paths": paths()
    })
}

// ── Shared schema helpers ─────────────────────────────────────────────────────

fn ref_schema(name: &str) -> Value {
    json!({ "$ref": format!("#/components/schemas/{name}") })
}

fn id_schema() -> Value {
    json!({ "type": "string", "format": "uuid" })
}

fn string_schema() -> Value {
    json!({ "type": "string" })
}

fn date_schema() -> Value {
    json!({ "type": "string", "format": "date" })
}

fn datetime_schema() -> Value {
    json!({ "type": "string", "format": "date-time" })
}

fn int64_schema() -> Value {
    json!({ "type": "integer", "format": "int64" })
}

fn uint32_schema() -> Value {
    json!({ "type": "integer", "format": "int32", "minimum": 0 })
}

fn bool_schema() -> Value {
    json!({ "type": "boolean" })
}

fn array_of(schema: Value) -> Value {
    json!({ "type": "array", "items": schema })
}

fn ok_response(description: &str, schema: Value) -> Value {
    json!({
        "200": {
            "description": description,
            "content": { "application/json": { "schema": schema } }
        },
        "400": { "description": "Bad request", "content": { "application/json": { "schema": ref_schema("ErrorResponse") } } },
        "401": { "description": "Unauthorized" },
        "403": { "description": "Forbidden — missing required scope" },
        "404": { "description": "Not found", "content": { "application/json": { "schema": ref_schema("ErrorResponse") } } },
        "500": { "description": "Internal server error", "content": { "application/json": { "schema": ref_schema("ErrorResponse") } } }
    })
}

fn post_op(summary: &str, tag: &str, request_ref: &str, response_schema: Value, scopes: &[&str]) -> Value {
    json!({
        "summary": summary,
        "tags": [tag],
        "security": [{ "BearerAuth": scopes }],
        "requestBody": {
            "required": true,
            "content": {
                "application/json": {
                    "schema": ref_schema(request_ref)
                }
            }
        },
        "responses": ok_response("Success", response_schema)
    })
}

fn get_op(summary: &str, tag: &str, response_schema: Value, scopes: &[&str]) -> Value {
    json!({
        "summary": summary,
        "tags": [tag],
        "security": [{ "BearerAuth": scopes }],
        "responses": ok_response("Success", response_schema)
    })
}

fn post_empty_op(summary: &str, tag: &str, response_schema: Value, scopes: &[&str]) -> Value {
    json!({
        "summary": summary,
        "tags": [tag],
        "security": [{ "BearerAuth": scopes }],
        "responses": ok_response("Success", response_schema)
    })
}

fn entity_id_param() -> Value {
    json!({
        "name": "entity_id",
        "in": "path",
        "required": true,
        "schema": id_schema(),
        "description": "The entity UUID"
    })
}

fn path_param(name: &str, description: &str) -> Value {
    json!({
        "name": name,
        "in": "path",
        "required": true,
        "schema": id_schema(),
        "description": description
    })
}

// ── Schemas ───────────────────────────────────────────────────────────────────

fn schemas() -> Value {
    json!({
        "ErrorResponse": {
            "type": "object",
            "properties": {
                "error": { "type": "string" }
            }
        },

        // ── Formation ─────────────────────────────────────────────────────────
        "Entity": {
            "type": "object",
            "properties": {
                "entity_id": id_schema(),
                "workspace_id": id_schema(),
                "legal_name": string_schema(),
                "entity_type": { "type": "string", "enum": ["c_corp", "llc"] },
                "jurisdiction": string_schema(),
                "formation_status": {
                    "type": "string",
                    "enum": ["pending", "documents_generated", "signed", "filed", "active", "dissolved"]
                },
                "created_at": datetime_schema(),
                "dissolved_at": { "type": "string", "format": "date", "nullable": true }
            }
        },
        "CreateEntityRequest": {
            "type": "object",
            "required": ["legal_name", "entity_type", "jurisdiction"],
            "properties": {
                "legal_name": string_schema(),
                "entity_type": { "type": "string", "enum": ["c_corp", "llc"] },
                "jurisdiction": { "type": "string", "example": "DE" }
            }
        },
        "DissolveEntityRequest": {
            "type": "object",
            "properties": {
                "reason": { "type": "string", "nullable": true }
            }
        },
        "Document": {
            "type": "object",
            "properties": {
                "document_id": id_schema(),
                "entity_id": id_schema(),
                "document_type": string_schema(),
                "status": { "type": "string", "enum": ["draft", "signed", "filed"] },
                "content_hash": string_schema(),
                "created_at": datetime_schema()
            }
        },
        "SignDocumentRequest": {
            "type": "object",
            "required": ["signer_name", "signer_role", "signer_email", "signature_text", "consent_text"],
            "properties": {
                "signer_name": string_schema(),
                "signer_role": string_schema(),
                "signer_email": { "type": "string", "format": "email" },
                "signature_text": string_schema(),
                "consent_text": string_schema(),
                "signature_svg": { "type": "string", "nullable": true }
            }
        },
        "Filing": {
            "type": "object",
            "properties": {
                "filing_id": id_schema(),
                "entity_id": id_schema(),
                "filing_type": string_schema(),
                "status": { "type": "string", "enum": ["pending", "submitted", "filed"] },
                "confirmation_number": { "type": "string", "nullable": true },
                "confirmed_at": { "type": "string", "format": "date-time", "nullable": true }
            }
        },
        "ConfirmFilingRequest": {
            "type": "object",
            "properties": {
                "confirmation_number": { "type": "string", "nullable": true }
            }
        },
        "TaxProfile": {
            "type": "object",
            "properties": {
                "tax_profile_id": id_schema(),
                "entity_id": id_schema(),
                "ein": { "type": "string", "nullable": true },
                "ein_status": { "type": "string", "enum": ["pending", "active"] },
                "irs_tax_classification": string_schema()
            }
        },
        "ConfirmEinRequest": {
            "type": "object",
            "required": ["ein"],
            "properties": {
                "ein": { "type": "string", "example": "12-3456789" }
            }
        },

        // ── Equity ────────────────────────────────────────────────────────────
        "CapTable": {
            "type": "object",
            "properties": {
                "cap_table_id": id_schema(),
                "entity_id": id_schema(),
                "created_at": datetime_schema()
            }
        },
        "ShareClass": {
            "type": "object",
            "properties": {
                "share_class_id": id_schema(),
                "entity_id": id_schema(),
                "cap_table_id": id_schema(),
                "class_code": string_schema(),
                "stock_type": { "type": "string", "enum": ["common", "preferred"] },
                "par_value": string_schema(),
                "authorized_shares": int64_schema(),
                "liquidation_preference": { "type": "string", "nullable": true }
            }
        },
        "CreateShareClassRequest": {
            "type": "object",
            "required": ["cap_table_id", "class_code", "stock_type", "par_value", "authorized_shares"],
            "properties": {
                "cap_table_id": id_schema(),
                "class_code": string_schema(),
                "stock_type": { "type": "string", "enum": ["common", "preferred"] },
                "par_value": string_schema(),
                "authorized_shares": int64_schema(),
                "liquidation_preference": { "type": "string", "nullable": true }
            }
        },
        "EquityGrant": {
            "type": "object",
            "properties": {
                "grant_id": id_schema(),
                "entity_id": id_schema(),
                "cap_table_id": id_schema(),
                "share_class_id": id_schema(),
                "recipient_contact_id": id_schema(),
                "recipient_name": string_schema(),
                "grant_type": { "type": "string", "enum": ["restricted_stock", "option", "rsu", "warrant", "other"] },
                "shares": int64_schema(),
                "price_per_share": { "type": "integer", "nullable": true },
                "vesting_start": { "type": "string", "format": "date", "nullable": true },
                "vesting_months": { "type": "integer", "nullable": true },
                "cliff_months": { "type": "integer", "nullable": true },
                "created_at": datetime_schema()
            }
        },
        "CreateGrantRequest": {
            "type": "object",
            "required": ["cap_table_id", "share_class_id", "recipient_contact_id", "recipient_name", "grant_type", "shares"],
            "properties": {
                "cap_table_id": id_schema(),
                "share_class_id": id_schema(),
                "recipient_contact_id": id_schema(),
                "recipient_name": string_schema(),
                "grant_type": { "type": "string", "enum": ["restricted_stock", "option", "rsu", "warrant", "other"] },
                "shares": int64_schema(),
                "price_per_share": { "type": "integer", "nullable": true },
                "vesting_start": { "type": "string", "format": "date", "nullable": true },
                "vesting_months": { "type": "integer", "nullable": true },
                "cliff_months": { "type": "integer", "nullable": true }
            }
        },
        "SafeNote": {
            "type": "object",
            "properties": {
                "safe_note_id": id_schema(),
                "entity_id": id_schema(),
                "cap_table_id": id_schema(),
                "investor_contact_id": id_schema(),
                "investor_name": string_schema(),
                "safe_type": string_schema(),
                "investment_amount_cents": int64_schema(),
                "valuation_cap_cents": { "type": "integer", "nullable": true },
                "discount_percent": { "type": "integer", "nullable": true },
                "status": { "type": "string", "enum": ["outstanding", "converted", "cancelled"] },
                "created_at": datetime_schema()
            }
        },
        "IssueSafeRequest": {
            "type": "object",
            "required": ["cap_table_id", "investor_contact_id", "investor_name", "safe_type", "investment_amount_cents"],
            "properties": {
                "cap_table_id": id_schema(),
                "investor_contact_id": id_schema(),
                "investor_name": string_schema(),
                "safe_type": { "type": "string", "enum": ["valuation_cap", "discount", "mfn", "valuation_cap_and_discount"] },
                "investment_amount_cents": int64_schema(),
                "valuation_cap_cents": { "type": "integer", "nullable": true },
                "discount_percent": { "type": "integer", "nullable": true }
            }
        },
        "Valuation": {
            "type": "object",
            "properties": {
                "valuation_id": id_schema(),
                "entity_id": id_schema(),
                "cap_table_id": id_schema(),
                "valuation_type": { "type": "string", "enum": ["board", "external_409a"] },
                "methodology": string_schema(),
                "valuation_amount_cents": int64_schema(),
                "effective_date": date_schema(),
                "prepared_by": { "type": "string", "nullable": true },
                "status": { "type": "string", "enum": ["draft", "submitted", "approved"] },
                "approved_by": { "type": "string", "nullable": true },
                "created_at": datetime_schema()
            }
        },
        "CreateValuationRequest": {
            "type": "object",
            "required": ["cap_table_id", "valuation_type", "methodology", "valuation_amount_cents", "effective_date"],
            "properties": {
                "cap_table_id": id_schema(),
                "valuation_type": { "type": "string", "enum": ["board", "external_409a"] },
                "methodology": { "type": "string", "enum": ["board_determination", "third_party_appraisal", "formula"] },
                "valuation_amount_cents": int64_schema(),
                "effective_date": date_schema(),
                "prepared_by": { "type": "string", "nullable": true }
            }
        },
        "ApproveValuationRequest": {
            "type": "object",
            "required": ["approved_by"],
            "properties": {
                "approved_by": string_schema()
            }
        },
        "ShareTransfer": {
            "type": "object",
            "properties": {
                "transfer_id": id_schema(),
                "entity_id": id_schema(),
                "cap_table_id": id_schema(),
                "from_holder_id": id_schema(),
                "to_holder_id": id_schema(),
                "share_class_id": id_schema(),
                "shares": int64_schema(),
                "transfer_type": string_schema(),
                "price_per_share_cents": { "type": "integer", "nullable": true },
                "status": { "type": "string", "enum": ["pending", "approved", "executed", "rejected"] },
                "created_at": datetime_schema()
            }
        },
        "CreateTransferRequest": {
            "type": "object",
            "required": ["cap_table_id", "from_holder_id", "to_holder_id", "share_class_id", "shares", "transfer_type"],
            "properties": {
                "cap_table_id": id_schema(),
                "from_holder_id": id_schema(),
                "to_holder_id": id_schema(),
                "share_class_id": id_schema(),
                "shares": int64_schema(),
                "transfer_type": { "type": "string", "enum": ["secondary_sale", "gift", "estate", "other"] },
                "price_per_share_cents": { "type": "integer", "nullable": true }
            }
        },
        "FundingRound": {
            "type": "object",
            "properties": {
                "round_id": id_schema(),
                "entity_id": id_schema(),
                "cap_table_id": id_schema(),
                "name": string_schema(),
                "target_amount_cents": int64_schema(),
                "price_per_share_cents": { "type": "integer", "nullable": true },
                "status": { "type": "string", "enum": ["term_sheet", "diligence", "closing", "closed"] },
                "created_at": datetime_schema()
            }
        },
        "CreateRoundRequest": {
            "type": "object",
            "required": ["cap_table_id", "name", "target_amount_cents"],
            "properties": {
                "cap_table_id": id_schema(),
                "name": string_schema(),
                "target_amount_cents": int64_schema(),
                "price_per_share_cents": { "type": "integer", "nullable": true }
            }
        },
        "Holder": {
            "type": "object",
            "properties": {
                "holder_id": id_schema(),
                "entity_id": id_schema(),
                "contact_id": { "type": "string", "format": "uuid", "nullable": true },
                "name": string_schema(),
                "holder_type": { "type": "string", "enum": ["individual", "institution", "trust", "other"] },
                "created_at": datetime_schema()
            }
        },
        "CreateHolderRequest": {
            "type": "object",
            "required": ["name", "holder_type"],
            "properties": {
                "contact_id": { "type": "string", "format": "uuid", "nullable": true },
                "name": string_schema(),
                "holder_type": { "type": "string", "enum": ["individual", "institution", "trust", "other"] }
            }
        },

        // ── Governance ────────────────────────────────────────────────────────
        "GovernanceBody": {
            "type": "object",
            "properties": {
                "body_id": id_schema(),
                "entity_id": id_schema(),
                "name": string_schema(),
                "body_type": { "type": "string", "enum": ["board_of_directors", "llc_member_vote"] },
                "quorum_rule": { "type": "string", "enum": ["majority", "supermajority", "unanimous"] },
                "voting_method": { "type": "string", "enum": ["per_capita", "per_unit"] },
                "active": bool_schema(),
                "created_at": datetime_schema()
            }
        },
        "CreateBodyRequest": {
            "type": "object",
            "required": ["name", "body_type", "quorum_rule", "voting_method"],
            "properties": {
                "name": string_schema(),
                "body_type": { "type": "string", "enum": ["board_of_directors", "llc_member_vote"] },
                "quorum_rule": { "type": "string", "enum": ["majority", "supermajority", "unanimous"] },
                "voting_method": { "type": "string", "enum": ["per_capita", "per_unit"] }
            }
        },
        "GovernanceSeat": {
            "type": "object",
            "properties": {
                "seat_id": id_schema(),
                "entity_id": id_schema(),
                "body_id": id_schema(),
                "holder_id": id_schema(),
                "role": { "type": "string", "enum": ["chair", "member", "officer", "observer"] },
                "appointed_date": date_schema(),
                "term_expiration": { "type": "string", "format": "date", "nullable": true },
                "voting_power": uint32_schema(),
                "resigned": bool_schema()
            }
        },
        "CreateSeatRequest": {
            "type": "object",
            "required": ["body_id", "holder_id", "role", "appointed_date", "voting_power"],
            "properties": {
                "body_id": id_schema(),
                "holder_id": id_schema(),
                "role": { "type": "string", "enum": ["chair", "member", "officer", "observer"] },
                "appointed_date": date_schema(),
                "term_expiration": { "type": "string", "format": "date", "nullable": true },
                "voting_power": uint32_schema()
            }
        },
        "Meeting": {
            "type": "object",
            "properties": {
                "meeting_id": id_schema(),
                "entity_id": id_schema(),
                "body_id": id_schema(),
                "meeting_type": { "type": "string", "enum": ["board_meeting", "shareholder_meeting", "written_consent", "member_meeting"] },
                "title": string_schema(),
                "status": { "type": "string", "enum": ["scheduled", "notice_sent", "in_progress", "adjourned", "cancelled"] },
                "scheduled_date": { "type": "string", "format": "date-time", "nullable": true },
                "location": { "type": "string", "nullable": true },
                "created_at": datetime_schema()
            }
        },
        "CreateMeetingRequest": {
            "type": "object",
            "required": ["body_id", "meeting_type", "title"],
            "properties": {
                "body_id": id_schema(),
                "meeting_type": { "type": "string", "enum": ["board_meeting", "shareholder_meeting", "written_consent", "member_meeting"] },
                "title": string_schema(),
                "scheduled_date": { "type": "string", "format": "date-time", "nullable": true },
                "location": { "type": "string", "nullable": true },
                "notice_days": { "type": "integer", "nullable": true }
            }
        },
        "RecordAttendanceRequest": {
            "type": "object",
            "required": ["seat_ids"],
            "properties": {
                "seat_ids": { "type": "array", "items": id_schema() }
            }
        },
        "AgendaItem": {
            "type": "object",
            "properties": {
                "item_id": id_schema(),
                "meeting_id": id_schema(),
                "title": string_schema(),
                "item_type": { "type": "string", "enum": ["resolution", "report", "discussion", "election"] },
                "description": { "type": "string", "nullable": true },
                "resolution_text": { "type": "string", "nullable": true },
                "status": { "type": "string", "enum": ["pending", "approved", "rejected", "tabled"] }
            }
        },
        "CreateAgendaItemRequest": {
            "type": "object",
            "required": ["title", "item_type"],
            "properties": {
                "title": string_schema(),
                "item_type": { "type": "string", "enum": ["resolution", "report", "discussion", "election"] },
                "description": { "type": "string", "nullable": true },
                "resolution_text": { "type": "string", "nullable": true }
            }
        },
        "Vote": {
            "type": "object",
            "properties": {
                "vote_id": id_schema(),
                "meeting_id": id_schema(),
                "agenda_item_id": id_schema(),
                "seat_id": id_schema(),
                "value": { "type": "string", "enum": ["for", "against", "abstain", "recusal"] },
                "cast_at": datetime_schema()
            }
        },
        "CastVoteRequest": {
            "type": "object",
            "required": ["agenda_item_id", "seat_id", "value"],
            "properties": {
                "agenda_item_id": id_schema(),
                "seat_id": id_schema(),
                "value": { "type": "string", "enum": ["for", "against", "abstain", "recusal"] }
            }
        },
        "ResolveItemRequest": {
            "type": "object",
            "required": ["resolution_type", "resolution_text"],
            "properties": {
                "resolution_type": { "type": "string", "enum": ["approved", "rejected", "tabled"] },
                "resolution_text": string_schema()
            }
        },
        "GovernanceProfile": {
            "type": "object",
            "properties": {
                "entity_id": id_schema(),
                "entity_type": string_schema(),
                "legal_name": string_schema(),
                "jurisdiction": string_schema(),
                "effective_date": date_schema()
            }
        },
        "UpdateProfileRequest": {
            "type": "object",
            "required": ["entity_type", "legal_name", "jurisdiction", "effective_date", "founders", "directors", "officers"],
            "properties": {
                "entity_type": string_schema(),
                "legal_name": string_schema(),
                "jurisdiction": string_schema(),
                "effective_date": date_schema(),
                "registered_agent_name": { "type": "string", "nullable": true },
                "registered_agent_address": { "type": "string", "nullable": true },
                "board_size": { "type": "integer", "nullable": true },
                "principal_name": { "type": "string", "nullable": true },
                "founders": { "type": "array", "items": { "type": "object" } },
                "directors": { "type": "array", "items": { "type": "object" } },
                "officers": { "type": "array", "items": { "type": "object" } }
            }
        },
        "CreateWrittenConsentRequest": {
            "type": "object",
            "required": ["body_id", "title", "resolution_text"],
            "properties": {
                "body_id": id_schema(),
                "title": string_schema(),
                "resolution_text": string_schema()
            }
        },
        "QuickApproveRequest": {
            "type": "object",
            "required": ["body_id", "title", "resolution_text"],
            "properties": {
                "body_id": id_schema(),
                "title": string_schema(),
                "resolution_text": string_schema()
            }
        },

        // ── Treasury ──────────────────────────────────────────────────────────
        "Account": {
            "type": "object",
            "properties": {
                "account_id": id_schema(),
                "entity_id": id_schema(),
                "account_code": string_schema(),
                "account_name": string_schema(),
                "currency": string_schema(),
                "created_at": datetime_schema()
            }
        },
        "CreateAccountRequest": {
            "type": "object",
            "required": ["account_code", "account_name", "currency"],
            "properties": {
                "account_code": string_schema(),
                "account_name": string_schema(),
                "currency": { "type": "string", "example": "usd" }
            }
        },
        "JournalEntry": {
            "type": "object",
            "properties": {
                "entry_id": id_schema(),
                "entity_id": id_schema(),
                "date": date_schema(),
                "description": string_schema(),
                "lines": { "type": "array", "items": ref_schema("JournalLine") },
                "status": { "type": "string", "enum": ["draft", "posted", "voided"] }
            }
        },
        "JournalLine": {
            "type": "object",
            "required": ["account_id", "debit_cents", "credit_cents"],
            "properties": {
                "account_id": id_schema(),
                "debit_cents": int64_schema(),
                "credit_cents": int64_schema(),
                "description": { "type": "string", "nullable": true }
            }
        },
        "CreateJournalEntryRequest": {
            "type": "object",
            "required": ["date", "description", "lines"],
            "properties": {
                "date": date_schema(),
                "description": string_schema(),
                "lines": { "type": "array", "items": ref_schema("JournalLine") }
            }
        },
        "Invoice": {
            "type": "object",
            "properties": {
                "invoice_id": id_schema(),
                "entity_id": id_schema(),
                "customer_name": string_schema(),
                "customer_email": { "type": "string", "format": "email", "nullable": true },
                "amount_cents": int64_schema(),
                "currency": string_schema(),
                "description": string_schema(),
                "due_date": date_schema(),
                "status": { "type": "string", "enum": ["draft", "sent", "paid", "overdue", "cancelled"] },
                "created_at": datetime_schema()
            }
        },
        "CreateInvoiceRequest": {
            "type": "object",
            "required": ["customer_name", "amount_cents", "currency", "description", "due_date"],
            "properties": {
                "customer_name": string_schema(),
                "customer_email": { "type": "string", "format": "email", "nullable": true },
                "amount_cents": int64_schema(),
                "currency": { "type": "string", "example": "usd" },
                "description": string_schema(),
                "due_date": date_schema()
            }
        },
        "Payment": {
            "type": "object",
            "properties": {
                "payment_id": id_schema(),
                "entity_id": id_schema(),
                "recipient_name": string_schema(),
                "amount_cents": int64_schema(),
                "method": { "type": "string", "enum": ["ach", "wire", "check", "credit_card", "other"] },
                "reference": { "type": "string", "nullable": true },
                "paid_at": datetime_schema()
            }
        },
        "CreatePaymentRequest": {
            "type": "object",
            "required": ["recipient_name", "amount_cents", "method", "paid_at"],
            "properties": {
                "recipient_name": string_schema(),
                "amount_cents": int64_schema(),
                "method": { "type": "string", "enum": ["ach", "wire", "check", "credit_card", "other"] },
                "reference": { "type": "string", "nullable": true },
                "paid_at": datetime_schema()
            }
        },
        "BankAccount": {
            "type": "object",
            "properties": {
                "bank_account_id": id_schema(),
                "entity_id": id_schema(),
                "institution": string_schema(),
                "account_type": { "type": "string", "enum": ["checking", "savings", "money_market"] },
                "account_number_last4": { "type": "string", "nullable": true },
                "routing_number_last4": { "type": "string", "nullable": true },
                "status": { "type": "string", "enum": ["pending_review", "active", "closed"] },
                "created_at": datetime_schema()
            }
        },
        "CreateBankAccountRequest": {
            "type": "object",
            "required": ["institution", "account_type"],
            "properties": {
                "institution": string_schema(),
                "account_type": { "type": "string", "enum": ["checking", "savings", "money_market"] },
                "account_number_last4": { "type": "string", "nullable": true },
                "routing_number_last4": { "type": "string", "nullable": true }
            }
        },
        "PayrollRun": {
            "type": "object",
            "properties": {
                "payroll_run_id": id_schema(),
                "entity_id": id_schema(),
                "period_start": date_schema(),
                "period_end": date_schema(),
                "total_gross_cents": int64_schema(),
                "total_net_cents": int64_schema(),
                "employee_count": uint32_schema(),
                "status": { "type": "string", "enum": ["draft", "approved", "processed"] },
                "created_at": datetime_schema()
            }
        },
        "CreatePayrollRunRequest": {
            "type": "object",
            "required": ["period_start", "period_end", "total_gross_cents", "total_net_cents", "employee_count"],
            "properties": {
                "period_start": date_schema(),
                "period_end": date_schema(),
                "total_gross_cents": int64_schema(),
                "total_net_cents": int64_schema(),
                "employee_count": uint32_schema()
            }
        },
        "Reconciliation": {
            "type": "object",
            "properties": {
                "reconciliation_id": id_schema(),
                "entity_id": id_schema(),
                "account_id": id_schema(),
                "period_end": date_schema(),
                "statement_balance_cents": int64_schema(),
                "book_balance_cents": int64_schema(),
                "created_at": datetime_schema()
            }
        },
        "CreateReconciliationRequest": {
            "type": "object",
            "required": ["account_id", "period_end", "statement_balance_cents", "book_balance_cents"],
            "properties": {
                "account_id": id_schema(),
                "period_end": date_schema(),
                "statement_balance_cents": int64_schema(),
                "book_balance_cents": int64_schema()
            }
        },

        // ── Execution ─────────────────────────────────────────────────────────
        "Intent": {
            "type": "object",
            "properties": {
                "intent_id": id_schema(),
                "entity_id": id_schema(),
                "workspace_id": id_schema(),
                "intent_type": string_schema(),
                "authority_tier": { "type": "string", "enum": ["officer", "board", "shareholder"] },
                "description": string_schema(),
                "status": { "type": "string", "enum": ["pending", "evaluated", "authorized", "executed", "cancelled"] },
                "metadata": { "type": "object" },
                "created_at": datetime_schema()
            }
        },
        "CreateIntentRequest": {
            "type": "object",
            "required": ["intent_type", "authority_tier", "description"],
            "properties": {
                "intent_type": string_schema(),
                "authority_tier": { "type": "string", "enum": ["officer", "board", "shareholder"] },
                "description": string_schema(),
                "metadata": { "type": "object" }
            }
        },
        "Obligation": {
            "type": "object",
            "properties": {
                "obligation_id": id_schema(),
                "entity_id": id_schema(),
                "intent_id": { "type": "string", "format": "uuid", "nullable": true },
                "obligation_type": string_schema(),
                "assignee_type": { "type": "string", "enum": ["entity", "contact", "agent"] },
                "assignee_id": { "type": "string", "format": "uuid", "nullable": true },
                "description": string_schema(),
                "due_date": { "type": "string", "format": "date", "nullable": true },
                "status": { "type": "string", "enum": ["pending", "in_progress", "fulfilled", "waived"] },
                "created_at": datetime_schema()
            }
        },
        "CreateObligationRequest": {
            "type": "object",
            "required": ["obligation_type", "assignee_type", "description"],
            "properties": {
                "obligation_type": string_schema(),
                "assignee_type": { "type": "string", "enum": ["entity", "contact", "agent"] },
                "assignee_id": { "type": "string", "format": "uuid", "nullable": true },
                "description": string_schema(),
                "due_date": { "type": "string", "format": "date", "nullable": true },
                "intent_id": { "type": "string", "format": "uuid", "nullable": true }
            }
        },
        "Receipt": {
            "type": "object",
            "properties": {
                "receipt_id": id_schema(),
                "entity_id": id_schema(),
                "obligation_id": { "type": "string", "format": "uuid", "nullable": true },
                "description": string_schema(),
                "created_at": datetime_schema()
            }
        },

        // ── Contacts ──────────────────────────────────────────────────────────
        "Contact": {
            "type": "object",
            "properties": {
                "contact_id": id_schema(),
                "entity_id": id_schema(),
                "workspace_id": id_schema(),
                "contact_type": { "type": "string", "enum": ["individual", "organization"] },
                "name": string_schema(),
                "category": { "type": "string", "enum": ["employee", "contractor", "board_member", "investor", "founder", "officer", "member", "other"] },
                "email": { "type": "string", "format": "email", "nullable": true },
                "phone": { "type": "string", "nullable": true },
                "mailing_address": { "type": "string", "nullable": true },
                "active": bool_schema(),
                "created_at": datetime_schema()
            }
        },
        "CreateContactRequest": {
            "type": "object",
            "required": ["contact_type", "name", "category"],
            "properties": {
                "contact_type": { "type": "string", "enum": ["individual", "organization"] },
                "name": string_schema(),
                "category": { "type": "string", "enum": ["employee", "contractor", "board_member", "investor", "founder", "officer", "member", "other"] },
                "email": { "type": "string", "format": "email", "nullable": true },
                "phone": { "type": "string", "nullable": true },
                "mailing_address": { "type": "string", "nullable": true },
                "notes": { "type": "string", "nullable": true }
            }
        },
        "UpdateContactRequest": {
            "type": "object",
            "properties": {
                "name": { "type": "string", "nullable": true },
                "email": { "type": "string", "format": "email", "nullable": true },
                "phone": { "type": "string", "nullable": true },
                "mailing_address": { "type": "string", "nullable": true },
                "category": { "type": "string", "nullable": true },
                "notes": { "type": "string", "nullable": true }
            }
        },

        // ── Agents ────────────────────────────────────────────────────────────
        "Agent": {
            "type": "object",
            "properties": {
                "agent_id": id_schema(),
                "workspace_id": id_schema(),
                "name": string_schema(),
                "system_prompt": { "type": "string", "nullable": true },
                "model": { "type": "string", "nullable": true },
                "entity_id": { "type": "string", "format": "uuid", "nullable": true },
                "status": { "type": "string", "enum": ["active", "paused"] },
                "skills": { "type": "array", "items": ref_schema("AgentSkill") },
                "created_at": datetime_schema()
            }
        },
        "AgentSkill": {
            "type": "object",
            "required": ["name", "description"],
            "properties": {
                "name": string_schema(),
                "description": string_schema(),
                "instructions": { "type": "string", "nullable": true }
            }
        },
        "CreateAgentRequest": {
            "type": "object",
            "required": ["name"],
            "properties": {
                "name": string_schema(),
                "system_prompt": { "type": "string", "nullable": true },
                "model": { "type": "string", "nullable": true },
                "entity_id": { "type": "string", "format": "uuid", "nullable": true }
            }
        },
        "UpdateAgentRequest": {
            "type": "object",
            "properties": {
                "name": { "type": "string", "nullable": true },
                "system_prompt": { "type": "string", "nullable": true },
                "model": { "type": "string", "nullable": true }
            }
        },
        "AddSkillRequest": {
            "type": "object",
            "required": ["name", "description"],
            "properties": {
                "name": string_schema(),
                "description": string_schema(),
                "instructions": { "type": "string", "nullable": true }
            }
        },

        // ── Work Items ────────────────────────────────────────────────────────
        "WorkItem": {
            "type": "object",
            "properties": {
                "work_item_id": id_schema(),
                "entity_id": id_schema(),
                "title": string_schema(),
                "description": string_schema(),
                "category": string_schema(),
                "deadline": { "type": "string", "format": "date", "nullable": true },
                "asap": bool_schema(),
                "status": { "type": "string", "enum": ["open", "claimed", "completed", "cancelled"] },
                "claimed_by": { "type": "string", "nullable": true },
                "completed_by": { "type": "string", "nullable": true },
                "result": { "type": "string", "nullable": true },
                "created_at": datetime_schema()
            }
        },
        "CreateWorkItemRequest": {
            "type": "object",
            "required": ["title", "description", "category"],
            "properties": {
                "title": string_schema(),
                "description": string_schema(),
                "category": string_schema(),
                "deadline": { "type": "string", "format": "date", "nullable": true },
                "asap": bool_schema()
            }
        },
        "ClaimWorkItemRequest": {
            "type": "object",
            "required": ["claimed_by"],
            "properties": {
                "claimed_by": string_schema(),
                "claim_ttl_seconds": { "type": "integer", "format": "int64", "nullable": true }
            }
        },
        "CompleteWorkItemRequest": {
            "type": "object",
            "required": ["completed_by"],
            "properties": {
                "completed_by": string_schema(),
                "result": { "type": "string", "nullable": true }
            }
        },

        // ── Services ──────────────────────────────────────────────────────────
        "ServiceRequest": {
            "type": "object",
            "properties": {
                "request_id": id_schema(),
                "entity_id": id_schema(),
                "service_slug": string_schema(),
                "amount_cents": int64_schema(),
                "status": { "type": "string", "enum": ["pending", "in_checkout", "paid", "fulfilling", "fulfilled", "cancelled"] },
                "fulfillment_note": { "type": "string", "nullable": true },
                "created_at": datetime_schema()
            }
        },
        "CreateServiceRequestRequest": {
            "type": "object",
            "required": ["service_slug", "amount_cents"],
            "properties": {
                "service_slug": string_schema(),
                "amount_cents": int64_schema()
            }
        },
        "FulfillServiceRequestRequest": {
            "type": "object",
            "properties": {
                "fulfillment_note": { "type": "string", "nullable": true }
            }
        },

        // ── Admin ─────────────────────────────────────────────────────────────
        "ApiKeyRecord": {
            "type": "object",
            "properties": {
                "key_id": id_schema(),
                "name": string_schema(),
                "scopes": { "type": "array", "items": string_schema() },
                "entity_id": { "type": "string", "format": "uuid", "nullable": true },
                "deleted": bool_schema(),
                "created_at": datetime_schema()
            }
        },
        "CreateApiKeyRequest": {
            "type": "object",
            "required": ["name", "scopes"],
            "properties": {
                "name": string_schema(),
                "scopes": { "type": "array", "items": string_schema() },
                "entity_id": { "type": "string", "format": "uuid", "nullable": true }
            }
        },
        "CreateApiKeyResponse": {
            "type": "object",
            "properties": {
                "key_id": id_schema(),
                "raw_key": { "type": "string", "description": "Plaintext key — shown once only" },
                "name": string_schema(),
                "scopes": { "type": "array", "items": string_schema() }
            }
        },
        "WorkspaceSummary": {
            "type": "object",
            "properties": {
                "workspace_id": id_schema()
            }
        },
        "StatusResponse": {
            "type": "object",
            "properties": {
                "status": string_schema(),
                "version": string_schema(),
                "service": string_schema()
            }
        }
    })
}

// ── Paths ─────────────────────────────────────────────────────────────────────

fn paths() -> Value {
    json!({
        // ── Health / status ───────────────────────────────────────────────────
        "/health": {
            "get": {
                "summary": "Server liveness probe",
                "tags": ["Health"],
                "security": [],
                "responses": {
                    "200": {
                        "description": "Server is alive",
                        "content": { "application/json": { "schema": { "type": "object", "properties": { "status": string_schema() } } } }
                    }
                }
            }
        },
        "/v1/status": {
            "get": {
                "summary": "API status summary",
                "tags": ["Health"],
                "security": [{ "BearerAuth": [] }],
                "responses": ok_response("API status", ref_schema("StatusResponse"))
            }
        },

        // ── Formation ─────────────────────────────────────────────────────────
        "/v1/entities": {
            "get": get_op("List entities in the caller's workspace", "Formation", array_of(ref_schema("Entity")), &["FormationRead"]),
            "post": post_op("Create a new legal entity", "Formation", "CreateEntityRequest", ref_schema("Entity"), &["FormationCreate"])
        },
        "/v1/entities/{entity_id}": {
            "parameters": [entity_id_param()],
            "get": get_op("Get a single entity", "Formation", ref_schema("Entity"), &["FormationRead"])
        },
        "/v1/entities/{entity_id}/dissolve": {
            "parameters": [entity_id_param()],
            "post": post_empty_op("Dissolve a legal entity", "Formation", ref_schema("Entity"), &["FormationCreate"])
        },
        "/v1/formations/{entity_id}/advance": {
            "parameters": [entity_id_param()],
            "post": post_empty_op("Advance the entity's formation status one step", "Formation", ref_schema("Entity"), &["FormationCreate"])
        },
        "/v1/formations/{entity_id}/documents": {
            "parameters": [entity_id_param()],
            "get": get_op("List formation documents for an entity", "Formation", array_of(ref_schema("Document")), &["FormationRead"])
        },
        "/v1/formations/{entity_id}/documents/{document_id}": {
            "parameters": [entity_id_param(), path_param("document_id", "Document UUID")],
            "get": get_op("Get a formation document", "Formation", ref_schema("Document"), &["FormationRead"])
        },
        "/v1/documents/{document_id}/sign": {
            "parameters": [path_param("document_id", "Document UUID")],
            "post": post_op("Sign a formation document", "Formation", "SignDocumentRequest", ref_schema("Document"), &["FormationSign"])
        },
        "/v1/formations/{entity_id}/filing": {
            "parameters": [entity_id_param()],
            "get": get_op("Get the state filing record for an entity", "Formation", ref_schema("Filing"), &["FormationRead"])
        },
        "/v1/formations/{entity_id}/filing/confirm": {
            "parameters": [entity_id_param()],
            "post": post_op("Confirm state acceptance of a filing", "Formation", "ConfirmFilingRequest", ref_schema("Filing"), &["FormationCreate"])
        },
        "/v1/formations/{entity_id}/tax": {
            "parameters": [entity_id_param()],
            "get": get_op("Get the tax profile for an entity", "Formation", ref_schema("TaxProfile"), &["FormationRead"])
        },
        "/v1/formations/{entity_id}/tax/confirm-ein": {
            "parameters": [entity_id_param()],
            "post": post_op("Record the IRS-assigned EIN", "Formation", "ConfirmEinRequest", ref_schema("TaxProfile"), &["FormationCreate"])
        },

        // ── Equity / Cap Table ────────────────────────────────────────────────
        "/v1/entities/{entity_id}/cap-table": {
            "parameters": [entity_id_param()],
            "get": get_op("Get the cap table aggregate(s) for an entity", "Equity", array_of(ref_schema("CapTable")), &["EquityRead"]),
            "post": post_empty_op("Initialise a cap table for an entity", "Equity", ref_schema("CapTable"), &["EquityWrite"])
        },
        "/v1/entities/{entity_id}/share-classes": {
            "parameters": [entity_id_param()],
            "get": get_op("List share classes", "Equity", array_of(ref_schema("ShareClass")), &["EquityRead"]),
            "post": post_op("Create a share class", "Equity", "CreateShareClassRequest", ref_schema("ShareClass"), &["EquityWrite"])
        },
        "/v1/entities/{entity_id}/grants": {
            "parameters": [entity_id_param()],
            "get": get_op("List equity grants", "Equity", array_of(ref_schema("EquityGrant")), &["EquityRead"]),
            "post": post_op("Create an equity grant", "Equity", "CreateGrantRequest", ref_schema("EquityGrant"), &["EquityWrite"])
        },
        "/v1/entities/{entity_id}/grants/{grant_id}": {
            "parameters": [entity_id_param(), path_param("grant_id", "Grant UUID")],
            "get": get_op("Get a single equity grant", "Equity", ref_schema("EquityGrant"), &["EquityRead"])
        },
        "/v1/entities/{entity_id}/safes": {
            "parameters": [entity_id_param()],
            "get": get_op("List SAFE notes", "Equity", array_of(ref_schema("SafeNote")), &["EquityRead"]),
            "post": post_op("Issue a SAFE note", "Equity", "IssueSafeRequest", ref_schema("SafeNote"), &["EquityWrite"])
        },
        "/v1/entities/{entity_id}/safes/{safe_id}/convert": {
            "parameters": [entity_id_param(), path_param("safe_id", "SafeNote UUID")],
            "post": post_empty_op("Convert a SAFE note to equity", "Equity", ref_schema("SafeNote"), &["EquityWrite"])
        },
        "/v1/entities/{entity_id}/valuations": {
            "parameters": [entity_id_param()],
            "get": get_op("List valuations", "Equity", array_of(ref_schema("Valuation")), &["EquityRead"]),
            "post": post_op("Create a valuation", "Equity", "CreateValuationRequest", ref_schema("Valuation"), &["EquityWrite"])
        },
        "/v1/entities/{entity_id}/valuations/{valuation_id}/submit": {
            "parameters": [entity_id_param(), path_param("valuation_id", "Valuation UUID")],
            "post": post_empty_op("Submit valuation for approval", "Equity", ref_schema("Valuation"), &["EquityWrite"])
        },
        "/v1/entities/{entity_id}/valuations/{valuation_id}/approve": {
            "parameters": [entity_id_param(), path_param("valuation_id", "Valuation UUID")],
            "post": post_op("Approve a valuation", "Equity", "ApproveValuationRequest", ref_schema("Valuation"), &["EquityWrite"])
        },
        "/v1/entities/{entity_id}/transfers": {
            "parameters": [entity_id_param()],
            "get": get_op("List share transfers", "Equity", array_of(ref_schema("ShareTransfer")), &["EquityRead"]),
            "post": post_op("Create a share transfer", "Equity", "CreateTransferRequest", ref_schema("ShareTransfer"), &["EquityWrite"])
        },
        "/v1/entities/{entity_id}/transfers/{transfer_id}/approve": {
            "parameters": [entity_id_param(), path_param("transfer_id", "Transfer UUID")],
            "post": post_empty_op("Approve a share transfer", "Equity", ref_schema("ShareTransfer"), &["EquityWrite"])
        },
        "/v1/entities/{entity_id}/transfers/{transfer_id}/execute": {
            "parameters": [entity_id_param(), path_param("transfer_id", "Transfer UUID")],
            "post": post_empty_op("Execute an approved share transfer", "Equity", ref_schema("ShareTransfer"), &["EquityWrite"])
        },
        "/v1/entities/{entity_id}/rounds": {
            "parameters": [entity_id_param()],
            "get": get_op("List funding rounds", "Equity", array_of(ref_schema("FundingRound")), &["EquityRead"]),
            "post": post_op("Create a funding round", "Equity", "CreateRoundRequest", ref_schema("FundingRound"), &["EquityWrite"])
        },
        "/v1/entities/{entity_id}/rounds/{round_id}/advance": {
            "parameters": [entity_id_param(), path_param("round_id", "FundingRound UUID")],
            "post": post_empty_op("Advance a funding round to the next stage", "Equity", ref_schema("FundingRound"), &["EquityWrite"])
        },
        "/v1/entities/{entity_id}/rounds/{round_id}/close": {
            "parameters": [entity_id_param(), path_param("round_id", "FundingRound UUID")],
            "post": post_empty_op("Close a funding round", "Equity", ref_schema("FundingRound"), &["EquityWrite"])
        },
        "/v1/entities/{entity_id}/holders": {
            "parameters": [entity_id_param()],
            "get": get_op("List holders", "Equity", array_of(ref_schema("Holder")), &["EquityRead"]),
            "post": post_op("Create a holder", "Equity", "CreateHolderRequest", ref_schema("Holder"), &["EquityWrite"])
        },

        // ── Governance ────────────────────────────────────────────────────────
        "/v1/entities/{entity_id}/governance/bodies": {
            "parameters": [entity_id_param()],
            "get": get_op("List governance bodies", "Governance", array_of(ref_schema("GovernanceBody")), &["GovernanceRead"]),
            "post": post_op("Create a governance body", "Governance", "CreateBodyRequest", ref_schema("GovernanceBody"), &["GovernanceWrite"])
        },
        "/v1/entities/{entity_id}/governance/bodies/{body_id}": {
            "parameters": [entity_id_param(), path_param("body_id", "GovernanceBody UUID")],
            "get": get_op("Get a governance body", "Governance", ref_schema("GovernanceBody"), &["GovernanceRead"])
        },
        "/v1/entities/{entity_id}/governance/bodies/{body_id}/deactivate": {
            "parameters": [entity_id_param(), path_param("body_id", "GovernanceBody UUID")],
            "post": post_empty_op("Deactivate a governance body", "Governance", ref_schema("GovernanceBody"), &["GovernanceWrite"])
        },
        "/v1/entities/{entity_id}/governance/seats": {
            "parameters": [entity_id_param()],
            "get": get_op("List all seats", "Governance", array_of(ref_schema("GovernanceSeat")), &["GovernanceRead"]),
            "post": post_op("Create a seat", "Governance", "CreateSeatRequest", ref_schema("GovernanceSeat"), &["GovernanceWrite"])
        },
        "/v1/entities/{entity_id}/governance/seats/{seat_id}": {
            "parameters": [entity_id_param(), path_param("seat_id", "GovernanceSeat UUID")],
            "get": get_op("Get a seat", "Governance", ref_schema("GovernanceSeat"), &["GovernanceRead"])
        },
        "/v1/entities/{entity_id}/governance/seats/{seat_id}/resign": {
            "parameters": [entity_id_param(), path_param("seat_id", "GovernanceSeat UUID")],
            "post": post_empty_op("Resign a seat", "Governance", ref_schema("GovernanceSeat"), &["GovernanceWrite"])
        },
        "/v1/entities/{entity_id}/governance/meetings": {
            "parameters": [entity_id_param()],
            "get": get_op("List meetings", "Governance", array_of(ref_schema("Meeting")), &["GovernanceRead"]),
            "post": post_op("Create a meeting", "Governance", "CreateMeetingRequest", ref_schema("Meeting"), &["GovernanceWrite"])
        },
        "/v1/entities/{entity_id}/governance/meetings/{meeting_id}": {
            "parameters": [entity_id_param(), path_param("meeting_id", "Meeting UUID")],
            "get": get_op("Get a meeting", "Governance", ref_schema("Meeting"), &["GovernanceRead"])
        },
        "/v1/entities/{entity_id}/governance/meetings/{meeting_id}/notice": {
            "parameters": [entity_id_param(), path_param("meeting_id", "Meeting UUID")],
            "post": post_empty_op("Send meeting notice (Scheduled → Notice Sent)", "Governance", ref_schema("Meeting"), &["GovernanceWrite"])
        },
        "/v1/entities/{entity_id}/governance/meetings/{meeting_id}/convene": {
            "parameters": [entity_id_param(), path_param("meeting_id", "Meeting UUID")],
            "post": post_empty_op("Convene a meeting (→ In Progress)", "Governance", ref_schema("Meeting"), &["GovernanceWrite"])
        },
        "/v1/entities/{entity_id}/governance/meetings/{meeting_id}/adjourn": {
            "parameters": [entity_id_param(), path_param("meeting_id", "Meeting UUID")],
            "post": post_empty_op("Adjourn a meeting", "Governance", ref_schema("Meeting"), &["GovernanceWrite"])
        },
        "/v1/entities/{entity_id}/governance/meetings/{meeting_id}/cancel": {
            "parameters": [entity_id_param(), path_param("meeting_id", "Meeting UUID")],
            "post": post_empty_op("Cancel a meeting", "Governance", ref_schema("Meeting"), &["GovernanceWrite"])
        },
        "/v1/entities/{entity_id}/governance/meetings/{meeting_id}/reopen": {
            "parameters": [entity_id_param(), path_param("meeting_id", "Meeting UUID")],
            "post": post_empty_op("Reopen an adjourned meeting", "Governance", ref_schema("Meeting"), &["GovernanceWrite"])
        },
        "/v1/entities/{entity_id}/governance/meetings/{meeting_id}/attendance": {
            "parameters": [entity_id_param(), path_param("meeting_id", "Meeting UUID")],
            "post": post_op("Record attendance for a meeting", "Governance", "RecordAttendanceRequest", ref_schema("Meeting"), &["GovernanceWrite"])
        },
        "/v1/entities/{entity_id}/governance/meetings/{meeting_id}/items": {
            "parameters": [entity_id_param(), path_param("meeting_id", "Meeting UUID")],
            "get": get_op("List agenda items for a meeting", "Governance", array_of(ref_schema("AgendaItem")), &["GovernanceRead"]),
            "post": post_op("Add an agenda item to a meeting", "Governance", "CreateAgendaItemRequest", ref_schema("AgendaItem"), &["GovernanceWrite"])
        },
        "/v1/entities/{entity_id}/governance/meetings/{meeting_id}/votes": {
            "parameters": [entity_id_param(), path_param("meeting_id", "Meeting UUID")],
            "get": get_op("List votes for a meeting", "Governance", array_of(ref_schema("Vote")), &["GovernanceRead"]),
            "post": post_op("Cast a vote", "Governance", "CastVoteRequest", ref_schema("Vote"), &["GovernanceVote"])
        },
        "/v1/entities/{entity_id}/governance/meetings/{meeting_id}/items/{item_id}/resolve": {
            "parameters": [entity_id_param(), path_param("meeting_id", "Meeting UUID"), path_param("item_id", "AgendaItem UUID")],
            "post": post_op("Resolve an agenda item", "Governance", "ResolveItemRequest", ref_schema("AgendaItem"), &["GovernanceWrite"])
        },
        "/v1/entities/{entity_id}/governance/profile": {
            "parameters": [entity_id_param()],
            "get": get_op("Get the governance profile", "Governance", ref_schema("GovernanceProfile"), &["GovernanceRead"]),
            "put": {
                "summary": "Update (replace) the governance profile",
                "tags": ["Governance"],
                "security": [{ "BearerAuth": ["GovernanceWrite"] }],
                "requestBody": {
                    "required": true,
                    "content": { "application/json": { "schema": ref_schema("UpdateProfileRequest") } }
                },
                "responses": ok_response("Updated profile", ref_schema("GovernanceProfile"))
            }
        },
        "/v1/entities/{entity_id}/governance/written-consent": {
            "parameters": [entity_id_param()],
            "post": post_op("Create a written consent resolution", "Governance", "CreateWrittenConsentRequest",
                json!({ "type": "object", "properties": { "meeting": ref_schema("Meeting"), "agenda_item": ref_schema("AgendaItem") } }),
                &["GovernanceWrite"])
        },
        "/v1/entities/{entity_id}/governance/quick-approve": {
            "parameters": [entity_id_param()],
            "post": post_op("Quick-approve a resolution (unanimous written consent)", "Governance", "QuickApproveRequest",
                json!({ "type": "object" }),
                &["GovernanceWrite"])
        },

        // ── Treasury ──────────────────────────────────────────────────────────
        "/v1/entities/{entity_id}/accounts": {
            "parameters": [entity_id_param()],
            "get": get_op("List GL accounts", "Treasury", array_of(ref_schema("Account")), &["TreasuryRead"]),
            "post": post_op("Create a GL account", "Treasury", "CreateAccountRequest", ref_schema("Account"), &["TreasuryWrite"])
        },
        "/v1/entities/{entity_id}/journal-entries": {
            "parameters": [entity_id_param()],
            "get": get_op("List journal entries", "Treasury", array_of(ref_schema("JournalEntry")), &["TreasuryRead"]),
            "post": post_op("Create a journal entry (draft)", "Treasury", "CreateJournalEntryRequest", ref_schema("JournalEntry"), &["TreasuryWrite"])
        },
        "/v1/entities/{entity_id}/journal-entries/{entry_id}/post": {
            "parameters": [entity_id_param(), path_param("entry_id", "JournalEntry UUID")],
            "post": post_empty_op("Post a journal entry (Draft → Posted)", "Treasury", ref_schema("JournalEntry"), &["TreasuryWrite"])
        },
        "/v1/entities/{entity_id}/journal-entries/{entry_id}/void": {
            "parameters": [entity_id_param(), path_param("entry_id", "JournalEntry UUID")],
            "post": post_empty_op("Void a journal entry", "Treasury", ref_schema("JournalEntry"), &["TreasuryWrite"])
        },
        "/v1/entities/{entity_id}/invoices": {
            "parameters": [entity_id_param()],
            "get": get_op("List invoices", "Treasury", array_of(ref_schema("Invoice")), &["TreasuryRead"]),
            "post": post_op("Create an invoice", "Treasury", "CreateInvoiceRequest", ref_schema("Invoice"), &["TreasuryWrite"])
        },
        "/v1/entities/{entity_id}/invoices/{invoice_id}/send": {
            "parameters": [entity_id_param(), path_param("invoice_id", "Invoice UUID")],
            "post": post_empty_op("Send an invoice (Draft → Sent)", "Treasury", ref_schema("Invoice"), &["TreasuryWrite"])
        },
        "/v1/entities/{entity_id}/invoices/{invoice_id}/pay": {
            "parameters": [entity_id_param(), path_param("invoice_id", "Invoice UUID")],
            "post": post_empty_op("Mark an invoice as paid (Sent → Paid)", "Treasury", ref_schema("Invoice"), &["TreasuryWrite"])
        },
        "/v1/entities/{entity_id}/payments": {
            "parameters": [entity_id_param()],
            "get": get_op("List payments", "Treasury", array_of(ref_schema("Payment")), &["TreasuryRead"]),
            "post": post_op("Record a payment", "Treasury", "CreatePaymentRequest", ref_schema("Payment"), &["TreasuryWrite"])
        },
        "/v1/entities/{entity_id}/bank-accounts": {
            "parameters": [entity_id_param()],
            "get": get_op("List bank accounts", "Treasury", array_of(ref_schema("BankAccount")), &["TreasuryRead"]),
            "post": post_op("Open a bank account", "Treasury", "CreateBankAccountRequest", ref_schema("BankAccount"), &["TreasuryWrite"])
        },
        "/v1/entities/{entity_id}/bank-accounts/{bank_id}/activate": {
            "parameters": [entity_id_param(), path_param("bank_id", "BankAccount UUID")],
            "post": post_empty_op("Activate a bank account (PendingReview → Active)", "Treasury", ref_schema("BankAccount"), &["TreasuryWrite"])
        },
        "/v1/entities/{entity_id}/bank-accounts/{bank_id}/close": {
            "parameters": [entity_id_param(), path_param("bank_id", "BankAccount UUID")],
            "post": post_empty_op("Close a bank account (Active → Closed)", "Treasury", ref_schema("BankAccount"), &["TreasuryWrite"])
        },
        "/v1/entities/{entity_id}/payroll-runs": {
            "parameters": [entity_id_param()],
            "get": get_op("List payroll runs", "Treasury", array_of(ref_schema("PayrollRun")), &["TreasuryRead"]),
            "post": post_op("Create a payroll run", "Treasury", "CreatePayrollRunRequest", ref_schema("PayrollRun"), &["TreasuryWrite"])
        },
        "/v1/entities/{entity_id}/payroll-runs/{run_id}/approve": {
            "parameters": [entity_id_param(), path_param("run_id", "PayrollRun UUID")],
            "post": post_empty_op("Approve a payroll run (Draft → Approved)", "Treasury", ref_schema("PayrollRun"), &["TreasuryWrite"])
        },
        "/v1/entities/{entity_id}/payroll-runs/{run_id}/process": {
            "parameters": [entity_id_param(), path_param("run_id", "PayrollRun UUID")],
            "post": post_empty_op("Process an approved payroll run (Approved → Processed)", "Treasury", ref_schema("PayrollRun"), &["TreasuryWrite"])
        },
        "/v1/entities/{entity_id}/reconciliations": {
            "parameters": [entity_id_param()],
            "get": get_op("List reconciliations", "Treasury", array_of(ref_schema("Reconciliation")), &["TreasuryRead"]),
            "post": post_op("Create a reconciliation", "Treasury", "CreateReconciliationRequest", ref_schema("Reconciliation"), &["TreasuryWrite"])
        },

        // ── Execution ─────────────────────────────────────────────────────────
        "/v1/entities/{entity_id}/intents": {
            "parameters": [entity_id_param()],
            "get": get_op("List execution intents", "Execution", array_of(ref_schema("Intent")), &["ExecutionRead"]),
            "post": post_op("Create an execution intent", "Execution", "CreateIntentRequest", ref_schema("Intent"), &["ExecutionWrite"])
        },
        "/v1/entities/{entity_id}/intents/{intent_id}": {
            "parameters": [entity_id_param(), path_param("intent_id", "Intent UUID")],
            "get": get_op("Get an intent", "Execution", ref_schema("Intent"), &["ExecutionRead"])
        },
        "/v1/entities/{entity_id}/intents/{intent_id}/evaluate": {
            "parameters": [entity_id_param(), path_param("intent_id", "Intent UUID")],
            "post": post_empty_op("Evaluate an intent (Pending → Evaluated)", "Execution", ref_schema("Intent"), &["ExecutionWrite"])
        },
        "/v1/entities/{entity_id}/intents/{intent_id}/authorize": {
            "parameters": [entity_id_param(), path_param("intent_id", "Intent UUID")],
            "post": post_empty_op("Authorize an intent (Evaluated → Authorized)", "Execution", ref_schema("Intent"), &["ExecutionWrite"])
        },
        "/v1/entities/{entity_id}/intents/{intent_id}/execute": {
            "parameters": [entity_id_param(), path_param("intent_id", "Intent UUID")],
            "post": post_empty_op("Execute an authorized intent (Authorized → Executed)", "Execution", ref_schema("Intent"), &["ExecutionWrite"])
        },
        "/v1/entities/{entity_id}/intents/{intent_id}/cancel": {
            "parameters": [entity_id_param(), path_param("intent_id", "Intent UUID")],
            "post": post_empty_op("Cancel an intent", "Execution", ref_schema("Intent"), &["ExecutionWrite"])
        },
        "/v1/entities/{entity_id}/obligations": {
            "parameters": [entity_id_param()],
            "get": get_op("List obligations", "Execution", array_of(ref_schema("Obligation")), &["ExecutionRead"]),
            "post": post_op("Create an obligation", "Execution", "CreateObligationRequest", ref_schema("Obligation"), &["ExecutionWrite"])
        },
        "/v1/entities/{entity_id}/obligations/{obligation_id}": {
            "parameters": [entity_id_param(), path_param("obligation_id", "Obligation UUID")],
            "get": get_op("Get an obligation", "Execution", ref_schema("Obligation"), &["ExecutionRead"])
        },
        "/v1/entities/{entity_id}/obligations/{obligation_id}/start": {
            "parameters": [entity_id_param(), path_param("obligation_id", "Obligation UUID")],
            "post": post_empty_op("Start an obligation (Pending → InProgress)", "Execution", ref_schema("Obligation"), &["ExecutionWrite"])
        },
        "/v1/entities/{entity_id}/obligations/{obligation_id}/fulfill": {
            "parameters": [entity_id_param(), path_param("obligation_id", "Obligation UUID")],
            "post": post_empty_op("Fulfill an obligation (InProgress → Fulfilled)", "Execution", ref_schema("Obligation"), &["ExecutionWrite"])
        },
        "/v1/entities/{entity_id}/obligations/{obligation_id}/waive": {
            "parameters": [entity_id_param(), path_param("obligation_id", "Obligation UUID")],
            "post": post_empty_op("Waive an obligation", "Execution", ref_schema("Obligation"), &["ExecutionWrite"])
        },
        "/v1/entities/{entity_id}/receipts": {
            "parameters": [entity_id_param()],
            "get": get_op("List receipts", "Execution", array_of(ref_schema("Receipt")), &["ExecutionRead"])
        },
        "/v1/entities/{entity_id}/receipts/{receipt_id}": {
            "parameters": [entity_id_param(), path_param("receipt_id", "Receipt UUID")],
            "get": get_op("Get a receipt", "Execution", ref_schema("Receipt"), &["ExecutionRead"])
        },

        // ── Contacts ──────────────────────────────────────────────────────────
        "/v1/entities/{entity_id}/contacts": {
            "parameters": [entity_id_param()],
            "get": get_op("List contacts", "Contacts", array_of(ref_schema("Contact")), &["ContactsRead"]),
            "post": post_op("Create a contact", "Contacts", "CreateContactRequest", ref_schema("Contact"), &["ContactsWrite"])
        },
        "/v1/entities/{entity_id}/contacts/{contact_id}": {
            "parameters": [entity_id_param(), path_param("contact_id", "Contact UUID")],
            "get": get_op("Get a contact", "Contacts", ref_schema("Contact"), &["ContactsRead"]),
            "patch": {
                "summary": "Update a contact",
                "tags": ["Contacts"],
                "security": [{ "BearerAuth": ["ContactsWrite"] }],
                "requestBody": {
                    "required": true,
                    "content": { "application/json": { "schema": ref_schema("UpdateContactRequest") } }
                },
                "responses": ok_response("Updated contact", ref_schema("Contact"))
            }
        },
        "/v1/entities/{entity_id}/contacts/{contact_id}/deactivate": {
            "parameters": [entity_id_param(), path_param("contact_id", "Contact UUID")],
            "post": post_empty_op("Deactivate a contact", "Contacts", ref_schema("Contact"), &["ContactsWrite"])
        },

        // ── Agents ────────────────────────────────────────────────────────────
        "/v1/agents": {
            "get": get_op("List agents in the workspace", "Agents", array_of(ref_schema("Agent")), &["AgentsRead"]),
            "post": post_op("Create an agent", "Agents", "CreateAgentRequest", ref_schema("Agent"), &["AgentsWrite"])
        },
        "/v1/agents/{agent_id}": {
            "parameters": [path_param("agent_id", "Agent UUID")],
            "get": get_op("Get an agent", "Agents", ref_schema("Agent"), &["AgentsRead"]),
            "patch": {
                "summary": "Update an agent",
                "tags": ["Agents"],
                "security": [{ "BearerAuth": ["AgentsWrite"] }],
                "requestBody": {
                    "required": true,
                    "content": { "application/json": { "schema": ref_schema("UpdateAgentRequest") } }
                },
                "responses": ok_response("Updated agent", ref_schema("Agent"))
            },
            "delete": {
                "summary": "Delete an agent",
                "tags": ["Agents"],
                "security": [{ "BearerAuth": ["AgentsWrite"] }],
                "responses": {
                    "204": { "description": "Deleted" },
                    "404": { "description": "Not found" }
                }
            }
        },
        "/v1/agents/{agent_id}/skills": {
            "parameters": [path_param("agent_id", "Agent UUID")],
            "post": post_op("Add a skill to an agent", "Agents", "AddSkillRequest", ref_schema("Agent"), &["AgentsWrite"])
        },
        "/v1/agents/{agent_id}/pause": {
            "parameters": [path_param("agent_id", "Agent UUID")],
            "post": post_empty_op("Pause an agent", "Agents", ref_schema("Agent"), &["AgentsWrite"])
        },
        "/v1/agents/{agent_id}/resume": {
            "parameters": [path_param("agent_id", "Agent UUID")],
            "post": post_empty_op("Resume a paused agent", "Agents", ref_schema("Agent"), &["AgentsWrite"])
        },

        // ── Work Items ────────────────────────────────────────────────────────
        "/v1/entities/{entity_id}/work-items": {
            "parameters": [entity_id_param()],
            "get": get_op("List work items", "WorkItems", array_of(ref_schema("WorkItem")), &["WorkItemsRead"]),
            "post": post_op("Create a work item", "WorkItems", "CreateWorkItemRequest", ref_schema("WorkItem"), &["WorkItemsWrite"])
        },
        "/v1/entities/{entity_id}/work-items/{item_id}": {
            "parameters": [entity_id_param(), path_param("item_id", "WorkItem UUID")],
            "get": get_op("Get a work item", "WorkItems", ref_schema("WorkItem"), &["WorkItemsRead"])
        },
        "/v1/entities/{entity_id}/work-items/{item_id}/claim": {
            "parameters": [entity_id_param(), path_param("item_id", "WorkItem UUID")],
            "post": post_op("Claim a work item", "WorkItems", "ClaimWorkItemRequest", ref_schema("WorkItem"), &["WorkItemsWrite"])
        },
        "/v1/entities/{entity_id}/work-items/{item_id}/release": {
            "parameters": [entity_id_param(), path_param("item_id", "WorkItem UUID")],
            "post": post_empty_op("Release a claimed work item", "WorkItems", ref_schema("WorkItem"), &["WorkItemsWrite"])
        },
        "/v1/entities/{entity_id}/work-items/{item_id}/complete": {
            "parameters": [entity_id_param(), path_param("item_id", "WorkItem UUID")],
            "post": post_op("Complete a work item", "WorkItems", "CompleteWorkItemRequest", ref_schema("WorkItem"), &["WorkItemsWrite"])
        },
        "/v1/entities/{entity_id}/work-items/{item_id}/cancel": {
            "parameters": [entity_id_param(), path_param("item_id", "WorkItem UUID")],
            "post": post_empty_op("Cancel a work item", "WorkItems", ref_schema("WorkItem"), &["WorkItemsWrite"])
        },

        // ── Service Requests ──────────────────────────────────────────────────
        "/v1/entities/{entity_id}/service-requests": {
            "parameters": [entity_id_param()],
            "get": get_op("List service requests", "Services", array_of(ref_schema("ServiceRequest")), &["ServicesRead"]),
            "post": post_op("Create a service request", "Services", "CreateServiceRequestRequest", ref_schema("ServiceRequest"), &["ServicesWrite"])
        },
        "/v1/entities/{entity_id}/service-requests/{request_id}": {
            "parameters": [entity_id_param(), path_param("request_id", "ServiceRequest UUID")],
            "get": get_op("Get a service request", "Services", ref_schema("ServiceRequest"), &["ServicesRead"])
        },
        "/v1/entities/{entity_id}/service-requests/{request_id}/checkout": {
            "parameters": [entity_id_param(), path_param("request_id", "ServiceRequest UUID")],
            "post": post_empty_op("Begin checkout (Pending → InCheckout)", "Services", ref_schema("ServiceRequest"), &["ServicesWrite"])
        },
        "/v1/entities/{entity_id}/service-requests/{request_id}/pay": {
            "parameters": [entity_id_param(), path_param("request_id", "ServiceRequest UUID")],
            "post": post_empty_op("Mark a service request as paid", "Services", ref_schema("ServiceRequest"), &["ServicesWrite"])
        },
        "/v1/entities/{entity_id}/service-requests/{request_id}/fulfill": {
            "parameters": [entity_id_param(), path_param("request_id", "ServiceRequest UUID")],
            "post": post_op("Fulfill a service request", "Services", "FulfillServiceRequestRequest", ref_schema("ServiceRequest"), &["ServicesWrite"])
        },

        // ── Admin ─────────────────────────────────────────────────────────────
        "/v1/workspaces": {
            "get": get_op("List all workspaces (super-admin)", "Admin", array_of(ref_schema("WorkspaceSummary")), &["Admin"])
        },
        "/v1/workspaces/{workspace_id}/entities": {
            "parameters": [path_param("workspace_id", "Workspace UUID")],
            "get": get_op("List entities in a workspace (super-admin)", "Admin", array_of(id_schema()), &["Admin"])
        },
        "/v1/api-keys": {
            "get": get_op("List API keys in the caller's workspace", "Admin", array_of(ref_schema("ApiKeyRecord")), &["Admin"]),
            "post": post_op("Create an API key", "Admin", "CreateApiKeyRequest", ref_schema("CreateApiKeyResponse"), &["Admin"])
        },
        "/v1/api-keys/{key_id}/revoke": {
            "parameters": [path_param("key_id", "ApiKey UUID")],
            "post": post_empty_op("Revoke an API key", "Admin", ref_schema("ApiKeyRecord"), &["Admin"])
        }
    })
}
