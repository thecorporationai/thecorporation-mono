// AUTO-GENERATED from backend OpenAPI spec — do not edit by hand.
// Regenerate: make generate-tools

export const GENERATED_TOOL_DEFINITIONS: Record<string, unknown>[] = [
  {
    "type": "function",
    "function": {
      "name": "workspace",
      "description": "Workspace-level queries. Actions: status (get workspace summary), list_entities (list all entities), obligations (list obligations, optional tier filter), billing (get billing status and plans), checkout (create Stripe checkout URL — requires plan_id), portal (get Stripe billing portal URL for managing subscription).",
      "parameters": {
        "type": "object",
        "properties": {
          "action": {
            "type": "string",
            "enum": ["status", "list_entities", "obligations", "billing", "checkout", "portal"]
          },
          "tier": {
            "type": "string",
            "description": "obligations: filter by urgency tier"
          },
          "plan_id": {
            "type": "string",
            "description": "checkout: plan to subscribe to (pro, enterprise)"
          }
        },
        "required": ["action"]
      }
    }
  },
  {
    "type": "function",
    "function": {
      "name": "entity",
      "description": "Entity reads and lifecycle. Actions: get_cap_table (entity_id), list_documents (entity_id), list_safe_notes (entity_id), form (entity_type + entity_name + jurisdiction + members — legacy one-shot formation), create (entity_type + entity_name + optional registered agent and company metadata — step 1 of staged formation), add_founder (entity_id + name + email + role + ownership_pct + optional founder address/officer data — step 2), finalize (entity_id + optional registered agent/company/share metadata — step 3, generates docs + cap table), convert (entity_id + target_type), dissolve (entity_id + reason).",
      "parameters": {
        "type": "object",
        "properties": {
          "action": {
            "type": "string",
            "enum": ["get_cap_table", "list_documents", "list_safe_notes", "form", "create", "add_founder", "finalize", "convert", "dissolve"]
          },
          "entity_id": { "type": "string" },
          "entity_type": { "type": "string", "enum": ["llc", "c_corp"] },
          "entity_name": { "type": "string" },
          "jurisdiction": { "type": "string", "description": "e.g. US-DE, US-WY. Defaults to US-WY for LLC, US-DE for corporation." },
          "registered_agent_name": { "type": "string", "description": "create/finalize: registered agent legal name" },
          "registered_agent_address": { "type": "string", "description": "create/finalize: registered agent street/city/state/zip line" },
          "formation_date": { "type": "string", "description": "create/finalize/form: RFC3339 or YYYY-MM-DD formation date" },
          "fiscal_year_end": { "type": "string", "description": "create/finalize/form: fiscal year end e.g. '12-31'" },
          "s_corp_election": { "type": "boolean", "description": "create/finalize/form: elect S-Corp tax treatment" },
          "transfer_restrictions": { "type": "boolean", "description": "create/finalize/form: include transfer restrictions in bylaws (corp)" },
          "right_of_first_refusal": { "type": "boolean", "description": "create/finalize/form: include ROFR in bylaws (corp)" },
          "authorized_shares": { "type": "integer", "description": "finalize: authorized shares for corporations" },
          "par_value": { "type": "string", "description": "finalize: par value per share, e.g. 0.0001" },
          "incorporator_name": { "type": "string", "description": "finalize: incorporator legal name (overrides founder)" },
          "incorporator_address": { "type": "string", "description": "finalize: incorporator mailing address (overrides founder)" },
          "company_address": {
            "type": "object",
            "properties": {
              "street": { "type": "string" },
              "street2": { "type": "string" },
              "city": { "type": "string" },
              "state": { "type": "string" },
              "zip": { "type": "string" }
            },
            "required": ["street", "city", "state", "zip"]
          },
          "members": {
            "type": "array",
            "description": "form: founding members array",
            "items": {
              "type": "object",
              "properties": {
                "name": { "type": "string" },
                "investor_type": { "type": "string", "enum": ["natural_person", "agent", "entity"] },
                "email": { "type": "string" },
                "agent_id": { "type": "string" },
                "entity_id": { "type": "string" },
                "ownership_pct": { "type": "number" },
                "membership_units": { "type": "integer" },
                "share_count": { "type": "integer" },
                "share_class": { "type": "string" },
                "role": { "type": "string", "enum": ["director", "officer", "manager", "member", "chair"] },
                "officer_title": { "type": "string", "enum": ["ceo", "cfo", "cto", "coo", "secretary", "treasurer", "president", "vp", "other"] },
                "shares_purchased": { "type": "integer" },
                "address": {
                  "type": "object",
                  "properties": {
                    "street": { "type": "string" },
                    "street2": { "type": "string" },
                    "city": { "type": "string" },
                    "state": { "type": "string" },
                    "zip": { "type": "string" }
                  }
                },
                "vesting": {
                  "type": "object",
                  "properties": {
                    "total_months": { "type": "integer" },
                    "cliff_months": { "type": "integer" },
                    "acceleration": { "type": "string", "enum": ["single_trigger", "double_trigger"] }
                  }
                },
                "ip_description": { "type": "string" },
                "is_incorporator": { "type": "boolean" }
              },
              "required": ["name", "investor_type"]
            }
          },
          "name": { "type": "string", "description": "add_founder: full legal name" },
          "email": { "type": "string", "description": "add_founder: email address" },
          "role": { "type": "string", "enum": ["director", "officer", "manager", "member", "chair"], "description": "add_founder: role" },
          "ownership_pct": { "type": "number", "description": "add_founder: ownership percentage (e.g. 50 for 50%)" },
          "officer_title": { "type": "string", "enum": ["ceo", "cfo", "cto", "coo", "secretary", "treasurer", "president", "vp", "other"], "description": "add_founder: officer title (corp only)" },
          "is_incorporator": { "type": "boolean", "description": "add_founder: is sole incorporator (corp only)" },
          "address": {
            "type": "object",
            "description": "add_founder: founder mailing address",
            "properties": {
              "street": { "type": "string" },
              "street2": { "type": "string" },
              "city": { "type": "string" },
              "state": { "type": "string" },
              "zip": { "type": "string" }
            }
          },
          "target_type": { "type": "string", "enum": ["llc", "c_corp"], "description": "convert: target entity type" },
          "new_jurisdiction": { "type": "string", "description": "convert: target jurisdiction" },
          "reason": { "type": "string", "description": "dissolve: dissolution reason" },
          "effective_date": { "type": "string", "description": "dissolve: effective date" }
        },
        "required": ["action"]
      }
    }
  },
  {
    "type": "function",
    "function": {
      "name": "equity",
      "description": "Equity and cap table operations. Actions: start_round (entity_id + name + issuer_legal_entity_id — step 1), add_security (round_id + instrument_id + quantity + recipient_name — step 2), issue_round (round_id — step 3, closes round + creates board agenda item), issue (entity_id + grant_type + shares + recipient_name — legacy single grant), issue_safe (entity_id + investor_name + principal_amount_cents + safe_type + valuation_cap_cents), transfer (entity_id + from_holder + to_holder + shares + skip_governance_review=true — bypasses governance), distribution (entity_id + total_amount_cents).",
      "parameters": {
        "type": "object",
        "properties": {
          "action": {
            "type": "string",
            "enum": ["start_round", "add_security", "issue_round", "issue", "issue_safe", "transfer", "distribution"]
          },
          "entity_id": { "type": "string" },
          "name": { "type": "string", "description": "start_round: round name (e.g. 'Seed Round')" },
          "issuer_legal_entity_id": { "type": "string", "description": "start_round: from cap table" },
          "pre_money_cents": { "type": "integer", "description": "start_round: pre-money valuation in cents" },
          "round_price_cents": { "type": "integer", "description": "start_round: price per share in cents" },
          "target_raise_cents": { "type": "integer", "description": "start_round: target raise in cents" },
          "round_id": { "type": "string", "description": "add_security/issue_round: round ID" },
          "instrument_id": { "type": "string", "description": "add_security: instrument ID from cap table" },
          "quantity": { "type": "integer", "description": "add_security: number of shares/units" },
          "recipient_name": { "type": "string", "description": "add_security/issue: recipient name" },
          "holder_id": { "type": "string", "description": "add_security: existing holder ID" },
          "email": { "type": "string", "description": "add_security: recipient email" },
          "principal_cents": { "type": "integer", "description": "add_security: investment amount in cents" },
          "grant_type": { "type": "string", "description": "issue/add_security: e.g. common, preferred, option" },
          "shares": { "type": "integer", "description": "issue/transfer: number of shares" },
          "vesting_schedule": { "type": "string", "description": "issue: vesting schedule" },
          "investor_name": { "type": "string", "description": "issue_safe: investor name" },
          "principal_amount_cents": { "type": "integer", "description": "issue_safe: principal in cents" },
          "safe_type": { "type": "string", "description": "issue_safe: SAFE type" },
          "valuation_cap_cents": { "type": "integer", "description": "issue_safe: valuation cap in cents" },
          "from_holder": { "type": "string", "description": "transfer: source holder" },
          "to_holder": { "type": "string", "description": "transfer: destination holder" },
          "share_class_id": { "type": "string", "description": "transfer: share class" },
          "transfer_type": { "type": "string", "description": "transfer: transfer type" },
          "skip_governance_review": { "type": "boolean", "description": "transfer: must be true to confirm bypassing governance" },
          "total_amount_cents": { "type": "integer", "description": "distribution: total amount in cents" },
          "distribution_type": { "type": "string", "enum": ["dividend", "return", "liquidation"], "description": "distribution: type" },
          "description": { "type": "string", "description": "distribution: description (required)" }
        },
        "required": ["action"]
      }
    }
  },
  {
    "type": "function",
    "function": {
      "name": "valuation",
      "description": "Valuation lifecycle. Actions: create (entity_id + valuation_type + effective_date + methodology → Draft), submit (entity_id + valuation_id → PendingApproval, auto-creates board agenda item), approve (entity_id + valuation_id + optional resolution_id → Approved, auto-supersedes previous 409A).",
      "parameters": {
        "type": "object",
        "properties": {
          "action": {
            "type": "string",
            "enum": ["create", "submit", "approve"]
          },
          "entity_id": { "type": "string" },
          "valuation_id": { "type": "string", "description": "submit/approve: valuation ID" },
          "valuation_type": {
            "type": "string",
            "enum": ["four_oh_nine_a", "llc_profits_interest", "fair_market_value", "gift", "estate", "other"],
            "description": "create: valuation type"
          },
          "effective_date": { "type": "string", "description": "create: effective date (ISO 8601)" },
          "methodology": {
            "type": "string",
            "enum": ["income", "market", "asset", "backsolve", "hybrid", "other"],
            "description": "create: valuation methodology"
          },
          "fmv_per_share_cents": { "type": "integer", "description": "create: FMV per share in cents" },
          "enterprise_value_cents": { "type": "integer", "description": "create: enterprise value in cents" },
          "hurdle_amount_cents": { "type": "integer", "description": "create: hurdle amount in cents" },
          "provider_contact_id": { "type": "string", "description": "create: valuation provider contact ID" },
          "report_document_id": { "type": "string", "description": "create: valuation report document ID" },
          "resolution_id": { "type": "string", "description": "approve: resolution ID from board vote" }
        },
        "required": ["action"]
      }
    }
  },
  {
    "type": "function",
    "function": {
      "name": "meeting",
      "description": "Governance meeting lifecycle. Actions: schedule (entity_id + body_id + meeting_type + title), notice (entity_id + meeting_id — Draft→Noticed), convene (entity_id + meeting_id + present_seat_ids — quorum check), vote (entity_id + meeting_id + agenda_item_id + voter_id + vote_value), resolve (entity_id + meeting_id + agenda_item_id + resolution_text — tally votes), finalize_item (entity_id + meeting_id + agenda_item_id + status), adjourn (entity_id + meeting_id), cancel (entity_id + meeting_id), consent (entity_id + body_id + title + description — written consent, no meeting), attach_document (entity_id + meeting_id + resolution_id + document_id), list_items (entity_id + meeting_id), list_votes (entity_id + meeting_id + agenda_item_id).",
      "parameters": {
        "type": "object",
        "properties": {
          "action": {
            "type": "string",
            "enum": ["schedule", "notice", "convene", "vote", "resolve", "finalize_item", "adjourn", "cancel", "consent", "attach_document", "list_items", "list_votes"]
          },
          "entity_id": { "type": "string" },
          "meeting_id": { "type": "string" },
          "body_id": { "type": "string", "description": "schedule/consent: governance body ID" },
          "meeting_type": { "type": "string", "description": "schedule: meeting type" },
          "title": { "type": "string", "description": "schedule/consent: meeting title" },
          "description": { "type": "string", "description": "consent: description" },
          "scheduled_date": { "type": "string", "description": "schedule: date (ISO 8601)" },
          "agenda_item_titles": { "type": "array", "items": { "type": "string" }, "description": "schedule: agenda items" },
          "present_seat_ids": { "type": "array", "items": { "type": "string" }, "description": "convene: seat IDs present" },
          "agenda_item_id": { "type": "string", "description": "vote/resolve/finalize_item/list_votes: agenda item ID" },
          "voter_id": { "type": "string", "description": "vote: voter seat ID" },
          "vote_value": { "type": "string", "enum": ["for", "against", "abstain", "recusal"], "description": "vote: for, against, abstain, or recusal" },
          "resolution_text": { "type": "string", "description": "resolve: resolution text" },
          "effective_date": { "type": "string", "description": "resolve: optional effective date" },
          "status": { "type": "string", "enum": ["voted", "discussed", "tabled", "withdrawn"], "description": "finalize_item: voted, discussed, tabled, or withdrawn" },
          "resolution_id": { "type": "string", "description": "attach_document: resolution ID" },
          "document_id": { "type": "string", "description": "attach_document: document ID" }
        },
        "required": ["action"]
      }
    }
  },
  {
    "type": "function",
    "function": {
      "name": "finance",
      "description": "Financial operations. Actions: create_invoice (entity_id + customer_name + amount_cents + description + due_date), run_payroll (entity_id + pay_period_start + pay_period_end), submit_payment (entity_id + amount_cents + recipient), open_bank_account (entity_id + bank_name), reconcile (entity_id + start_date + end_date).",
      "parameters": {
        "type": "object",
        "properties": {
          "action": {
            "type": "string",
            "enum": ["create_invoice", "run_payroll", "submit_payment", "open_bank_account", "reconcile"]
          },
          "entity_id": { "type": "string" },
          "customer_name": { "type": "string", "description": "create_invoice: customer name" },
          "amount_cents": { "type": "integer", "description": "create_invoice/submit_payment: amount in cents" },
          "description": { "type": "string", "description": "create_invoice/submit_payment: description" },
          "due_date": { "type": "string", "description": "create_invoice: due date" },
          "pay_period_start": { "type": "string", "description": "run_payroll: start date" },
          "pay_period_end": { "type": "string", "description": "run_payroll: end date" },
          "recipient": { "type": "string", "description": "submit_payment: recipient" },
          "bank_name": { "type": "string", "description": "open_bank_account: bank name (e.g. Mercury, SVB)" },
          "start_date": { "type": "string", "description": "reconcile: start date" },
          "end_date": { "type": "string", "description": "reconcile: end date" }
        },
        "required": ["action"]
      }
    }
  },
  {
    "type": "function",
    "function": {
      "name": "compliance",
      "description": "Compliance and legal operations. Actions: file_tax (entity_id + document_type + tax_year), track_deadline (entity_id + deadline_type + due_date + description), classify_contractor (entity_id + contractor_name + state + hours_per_week), generate_contract (entity_id + template_type + counterparty_name, optional effective_date).",
      "parameters": {
        "type": "object",
        "properties": {
          "action": {
            "type": "string",
            "enum": ["file_tax", "track_deadline", "classify_contractor", "generate_contract"]
          },
          "entity_id": { "type": "string" },
          "document_type": { "type": "string", "description": "file_tax: tax document type" },
          "tax_year": { "type": "integer", "description": "file_tax: tax year" },
          "deadline_type": { "type": "string", "description": "track_deadline: deadline type" },
          "due_date": { "type": "string", "description": "track_deadline: due date" },
          "description": { "type": "string", "description": "track_deadline: description" },
          "recurrence": { "type": "string", "description": "track_deadline: recurrence" },
          "contractor_name": { "type": "string", "description": "classify_contractor: contractor name" },
          "state": { "type": "string", "description": "classify_contractor: state" },
          "hours_per_week": { "type": "integer", "description": "classify_contractor: hours/week" },
          "exclusive_client": { "type": "boolean", "description": "classify_contractor: exclusive?" },
          "duration_months": { "type": "integer", "description": "classify_contractor: duration" },
          "provides_tools": { "type": "boolean", "description": "classify_contractor: provides own tools?" },
          "template_type": { "type": "string", "enum": ["consulting_agreement", "employment_offer", "contractor_agreement", "nda", "custom"], "description": "generate_contract: template type" },
          "counterparty_name": { "type": "string", "description": "generate_contract: counterparty name" },
          "effective_date": { "type": "string", "description": "generate_contract: effective date (ISO 8601, defaults to today)" },
          "parameters": { "type": "object", "description": "generate_contract: additional template parameters" }
        },
        "required": ["action"]
      }
    }
  },
  {
    "type": "function",
    "function": {
      "name": "document",
      "description": "Document access, signing, and preview. Actions: signing_link (entity_id + document_id — get signing link with token for a document), signer_link (obligation_id — generate signing link for a human obligation), download_link (document_id — get download link), preview_pdf (entity_id + document_id — preview a governance document as PDF without requiring a saved document).",
      "parameters": {
        "type": "object",
        "properties": {
          "action": {
            "type": "string",
            "enum": ["signing_link", "signer_link", "download_link", "preview_pdf"]
          },
          "document_id": { "type": "string", "description": "signing_link/download_link/preview_pdf: document ID (or AST definition ID for preview_pdf)" },
          "obligation_id": { "type": "string", "description": "signer_link: obligation ID" },
          "entity_id": { "type": "string", "description": "signing_link/preview_pdf: entity ID (required)" }
        },
        "required": ["action"]
      }
    }
  },
  {
    "type": "function",
    "function": {
      "name": "checklist",
      "description": "Workspace progress checklist. Actions: get (retrieve current checklist), update (checklist — set checklist content in markdown checkbox format).",
      "parameters": {
        "type": "object",
        "properties": {
          "action": {
            "type": "string",
            "enum": ["get", "update"]
          },
          "checklist": { "type": "string", "description": "update: markdown checklist content" }
        },
        "required": ["action"]
      }
    }
  },
  {
    "type": "function",
    "function": {
      "name": "work_item",
      "description": "Long-term work item coordination stored in entity repos. Agents claim items with TTL, complete them, or release/cancel. Actions: list (entity_id, optional status filter), get (entity_id + work_item_id), create (entity_id + title + category + optional deadline/asap/description/metadata/created_by), claim (entity_id + work_item_id + claimed_by + optional ttl_seconds), complete (entity_id + work_item_id + completed_by + optional result), release (entity_id + work_item_id — release a claim), cancel (entity_id + work_item_id).",
      "parameters": {
        "type": "object",
        "properties": {
          "action": {
            "type": "string",
            "enum": ["list", "get", "create", "claim", "complete", "release", "cancel"]
          },
          "entity_id": { "type": "string", "description": "All actions: entity ID" },
          "work_item_id": { "type": "string", "description": "get/claim/complete/release/cancel: work item ID" },
          "title": { "type": "string", "description": "create: work item title" },
          "category": { "type": "string", "description": "create/list: work item category" },
          "description": { "type": "string", "description": "create: work item description" },
          "deadline": { "type": "string", "description": "create: deadline date (YYYY-MM-DD)" },
          "asap": { "type": "boolean", "description": "create: mark as ASAP priority" },
          "metadata": { "type": "object", "description": "create: arbitrary metadata" },
          "created_by": { "type": "string", "description": "create: creator identifier" },
          "claimed_by": { "type": "string", "description": "claim: agent or user identifier" },
          "ttl_seconds": { "type": "integer", "description": "claim: auto-release TTL in seconds" },
          "completed_by": { "type": "string", "description": "complete: agent or user identifier" },
          "result": { "type": "string", "description": "complete: completion result/notes" },
          "status": { "type": "string", "enum": ["open", "claimed", "completed", "cancelled"], "description": "list: filter by effective status" }
        },
        "required": ["action"]
      }
    }
  },
  {
    "type": "function",
    "function": {
      "name": "agent",
      "description": "Agent management. Agents are for delegating recurring tasks — NOT for research or one-off questions. Requires a paid plan. Actions: list (list all agents), create (name + system_prompt), message (agent_id + message), update (agent_id + optional status), add_skill (agent_id + name + description).",
      "parameters": {
        "type": "object",
        "properties": {
          "action": {
            "type": "string",
            "enum": ["list", "create", "message", "update", "add_skill"]
          },
          "agent_id": { "type": "string", "description": "message/update/add_skill: agent ID" },
          "name": { "type": "string", "description": "create: agent name; add_skill: skill name" },
          "system_prompt": { "type": "string", "description": "create: agent system prompt" },
          "model": { "type": "string", "description": "create: model name" },
          "message": { "type": "string", "description": "message: message text" },
          "status": { "type": "string", "enum": ["active", "paused", "disabled"], "description": "update: new status" },
          "description": { "type": "string", "description": "add_skill: skill description" },
          "parameters": { "type": "object", "description": "add_skill: skill parameters schema" }
        },
        "required": ["action"]
      }
    }
  }
];
