// AUTO-GENERATED from backend OpenAPI spec — do not edit by hand.
// Regenerate: make generate-tools

export const GENERATED_TOOL_DEFINITIONS: Record<string, unknown>[] = [
  {
    "type": "function",
    "function": {
      "name": "get_workspace_status",
      "description": "Get workspace status summary",
      "parameters": {
        "type": "object",
        "properties": {},
        "required": []
      }
    }
  },
  {
    "type": "function",
    "function": {
      "name": "list_entities",
      "description": "List all entities in the workspace",
      "parameters": {
        "type": "object",
        "properties": {},
        "required": []
      }
    }
  },
  {
    "type": "function",
    "function": {
      "name": "get_cap_table",
      "description": "Get cap table for an entity",
      "parameters": {
        "type": "object",
        "properties": {
          "entity_id": {
            "type": "string"
          }
        },
        "required": [
          "entity_id"
        ]
      }
    }
  },
  {
    "type": "function",
    "function": {
      "name": "list_documents",
      "description": "List documents for an entity",
      "parameters": {
        "type": "object",
        "properties": {
          "entity_id": {
            "type": "string"
          }
        },
        "required": [
          "entity_id"
        ]
      }
    }
  },
  {
    "type": "function",
    "function": {
      "name": "list_safe_notes",
      "description": "List SAFE notes for an entity",
      "parameters": {
        "type": "object",
        "properties": {
          "entity_id": {
            "type": "string"
          }
        },
        "required": [
          "entity_id"
        ]
      }
    }
  },
  {
    "type": "function",
    "function": {
      "name": "list_agents",
      "description": "List all agents in the workspace",
      "parameters": {
        "type": "object",
        "properties": {},
        "required": []
      }
    }
  },
  {
    "type": "function",
    "function": {
      "name": "list_obligations",
      "description": "List obligations with urgency tiers",
      "parameters": {
        "type": "object",
        "properties": {
          "tier": {
            "type": "string"
          }
        },
        "required": []
      }
    }
  },
  {
    "type": "function",
    "function": {
      "name": "get_billing_status",
      "description": "Get billing status and plans",
      "parameters": {
        "type": "object",
        "properties": {},
        "required": []
      }
    }
  },
  {
    "type": "function",
    "function": {
      "name": "form_entity",
      "description": "Form a new business entity (LLC or corporation) and initialize its cap table with founding members",
      "parameters": {
        "type": "object",
        "properties": {
          "entity_type": {
            "type": "string",
            "enum": [
              "llc",
              "corporation"
            ]
          },
          "entity_name": {
            "type": "string"
          },
          "jurisdiction": {
            "type": "string"
          },
          "fiscal_year_end": {
            "type": "string",
            "description": "Fiscal year end, e.g. '12-31'. Defaults to '12-31'."
          },
          "s_corp_election": {
            "type": "boolean",
            "description": "Whether the company will elect S-Corp tax treatment."
          },
          "transfer_restrictions": {
            "type": "boolean",
            "description": "Include transfer restrictions in bylaws (corp). Default true."
          },
          "right_of_first_refusal": {
            "type": "boolean",
            "description": "Include right of first refusal in bylaws (corp). Default true."
          },
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
            "items": {
              "type": "object",
              "properties": {
                "name": {
                  "type": "string"
                },
                "investor_type": {
                  "type": "string",
                  "enum": [
                    "natural_person",
                    "agent",
                    "entity"
                  ]
                },
                "email": {
                  "type": "string"
                },
                "agent_id": {
                  "type": "string"
                },
                "entity_id": {
                  "type": "string"
                },
                "ownership_pct": {
                  "type": "number"
                },
                "membership_units": {
                  "type": "integer"
                },
                "share_count": {
                  "type": "integer"
                },
                "share_class": {
                  "type": "string"
                },
                "role": {
                  "type": "string",
                  "enum": ["director", "officer", "manager", "member", "chair"]
                },
                "officer_title": {
                  "type": "string",
                  "enum": ["ceo", "cfo", "secretary", "president", "vp", "other"],
                  "description": "Officer title (corporations only)"
                },
                "shares_purchased": {
                  "type": "integer",
                  "description": "Number of shares being purchased at formation"
                },
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
                    "total_months": { "type": "integer", "description": "Total vesting period in months (e.g. 48)" },
                    "cliff_months": { "type": "integer", "description": "Cliff period in months (e.g. 12)" },
                    "acceleration": { "type": "string", "enum": ["single_trigger", "double_trigger"] }
                  }
                },
                "ip_description": {
                  "type": "string",
                  "description": "Description of IP being contributed to the company"
                },
                "is_incorporator": {
                  "type": "boolean",
                  "description": "Whether this member is the sole incorporator (corporations only)"
                }
              },
              "required": [
                "name",
                "investor_type"
              ]
            }
          }
        },
        "required": [
          "entity_type",
          "entity_name",
          "jurisdiction",
          "members"
        ]
      }
    }
  },
  {
    "type": "function",
    "function": {
      "name": "issue_equity",
      "description": "Issue an equity grant",
      "parameters": {
        "type": "object",
        "properties": {
          "entity_id": {
            "type": "string"
          },
          "grant_type": {
            "type": "string"
          },
          "shares": {
            "type": "integer"
          },
          "recipient_name": {
            "type": "string"
          },
          "vesting_schedule": {
            "type": "string"
          }
        },
        "required": [
          "entity_id",
          "grant_type",
          "shares",
          "recipient_name"
        ]
      }
    }
  },
  {
    "type": "function",
    "function": {
      "name": "issue_safe",
      "description": "Issue a SAFE note",
      "parameters": {
        "type": "object",
        "properties": {
          "entity_id": {
            "type": "string"
          },
          "investor_name": {
            "type": "string"
          },
          "principal_amount_cents": {
            "type": "integer"
          },
          "safe_type": {
            "type": "string"
          },
          "valuation_cap_cents": {
            "type": "integer"
          }
        },
        "required": [
          "entity_id",
          "investor_name",
          "principal_amount_cents",
          "safe_type",
          "valuation_cap_cents"
        ]
      }
    }
  },
  {
    "type": "function",
    "function": {
      "name": "transfer_shares",
      "description": "Transfer shares between holders",
      "parameters": {
        "type": "object",
        "properties": {
          "entity_id": {
            "type": "string"
          },
          "share_class_id": {
            "type": "string"
          },
          "from_holder": {
            "type": "string"
          },
          "to_holder": {
            "type": "string"
          },
          "transfer_type": {
            "type": "string"
          },
          "shares": {
            "type": "integer"
          }
        },
        "required": [
          "entity_id",
          "from_holder",
          "to_holder",
          "shares"
        ]
      }
    }
  },
  {
    "type": "function",
    "function": {
      "name": "calculate_distribution",
      "description": "Calculate a distribution",
      "parameters": {
        "type": "object",
        "properties": {
          "entity_id": {
            "type": "string"
          },
          "total_amount_cents": {
            "type": "integer"
          },
          "distribution_type": {
            "type": "string"
          }
        },
        "required": [
          "entity_id",
          "total_amount_cents"
        ]
      }
    }
  },
  {
    "type": "function",
    "function": {
      "name": "create_invoice",
      "description": "Create an invoice",
      "parameters": {
        "type": "object",
        "properties": {
          "entity_id": {
            "type": "string"
          },
          "customer_name": {
            "type": "string"
          },
          "amount_cents": {
            "type": "integer"
          },
          "description": {
            "type": "string"
          },
          "due_date": {
            "type": "string"
          }
        },
        "required": [
          "entity_id",
          "customer_name",
          "amount_cents",
          "description",
          "due_date"
        ]
      }
    }
  },
  {
    "type": "function",
    "function": {
      "name": "run_payroll",
      "description": "Run payroll",
      "parameters": {
        "type": "object",
        "properties": {
          "entity_id": {
            "type": "string"
          },
          "pay_period_start": {
            "type": "string"
          },
          "pay_period_end": {
            "type": "string"
          }
        },
        "required": [
          "entity_id",
          "pay_period_start",
          "pay_period_end"
        ]
      }
    }
  },
  {
    "type": "function",
    "function": {
      "name": "submit_payment",
      "description": "Submit a payment",
      "parameters": {
        "type": "object",
        "properties": {
          "entity_id": {
            "type": "string"
          },
          "amount_cents": {
            "type": "integer"
          },
          "recipient": {
            "type": "string"
          },
          "description": {
            "type": "string"
          }
        },
        "required": [
          "entity_id",
          "amount_cents",
          "recipient"
        ]
      }
    }
  },
  {
    "type": "function",
    "function": {
      "name": "open_bank_account",
      "description": "Open a business bank account",
      "parameters": {
        "type": "object",
        "properties": {
          "entity_id": {
            "type": "string"
          },
          "institution_name": {
            "type": "string"
          }
        },
        "required": [
          "entity_id"
        ]
      }
    }
  },
  {
    "type": "function",
    "function": {
      "name": "generate_contract",
      "description": "Generate a contract from a template",
      "parameters": {
        "type": "object",
        "properties": {
          "entity_id": {
            "type": "string"
          },
          "template_type": {
            "type": "string"
          },
          "parameters": {
            "type": "object"
          }
        },
        "required": [
          "entity_id",
          "template_type"
        ]
      }
    }
  },
  {
    "type": "function",
    "function": {
      "name": "file_tax_document",
      "description": "File a tax document",
      "parameters": {
        "type": "object",
        "properties": {
          "entity_id": {
            "type": "string"
          },
          "document_type": {
            "type": "string"
          },
          "tax_year": {
            "type": "integer"
          }
        },
        "required": [
          "entity_id",
          "document_type",
          "tax_year"
        ]
      }
    }
  },
  {
    "type": "function",
    "function": {
      "name": "track_deadline",
      "description": "Track a compliance deadline",
      "parameters": {
        "type": "object",
        "properties": {
          "entity_id": {
            "type": "string"
          },
          "deadline_type": {
            "type": "string"
          },
          "due_date": {
            "type": "string"
          },
          "description": {
            "type": "string"
          },
          "recurrence": {
            "type": "string"
          }
        },
        "required": [
          "entity_id",
          "deadline_type",
          "due_date",
          "description"
        ]
      }
    }
  },
  {
    "type": "function",
    "function": {
      "name": "classify_contractor",
      "description": "Classify contractor risk",
      "parameters": {
        "type": "object",
        "properties": {
          "entity_id": {
            "type": "string"
          },
          "contractor_name": {
            "type": "string"
          },
          "state": {
            "type": "string"
          },
          "hours_per_week": {
            "type": "integer"
          },
          "exclusive_client": {
            "type": "boolean"
          },
          "duration_months": {
            "type": "integer"
          },
          "provides_tools": {
            "type": "boolean"
          }
        },
        "required": [
          "entity_id",
          "contractor_name",
          "state",
          "hours_per_week"
        ]
      }
    }
  },
  {
    "type": "function",
    "function": {
      "name": "reconcile_ledger",
      "description": "Reconcile an entity's ledger",
      "parameters": {
        "type": "object",
        "properties": {
          "entity_id": {
            "type": "string"
          },
          "start_date": {
            "type": "string"
          },
          "end_date": {
            "type": "string"
          }
        },
        "required": [
          "entity_id",
          "start_date",
          "end_date"
        ]
      }
    }
  },
  {
    "type": "function",
    "function": {
      "name": "convene_meeting",
      "description": "Convene a governance meeting",
      "parameters": {
        "type": "object",
        "properties": {
          "entity_id": {
            "type": "string"
          },
          "meeting_id": {
            "type": "string"
          },
          "present_seat_ids": {
            "type": "array",
            "items": {
              "type": "string"
            }
          }
        },
        "required": [
          "entity_id",
          "meeting_id",
          "present_seat_ids"
        ]
      }
    }
  },
  {
    "type": "function",
    "function": {
      "name": "cast_vote",
      "description": "Cast a vote on an agenda item",
      "parameters": {
        "type": "object",
        "properties": {
          "entity_id": {
            "type": "string"
          },
          "meeting_id": {
            "type": "string"
          },
          "agenda_item_id": {
            "type": "string"
          },
          "voter_id": {
            "type": "string"
          },
          "vote_value": {
            "type": "string",
            "description": "for, against, abstain, or recusal"
          }
        },
        "required": [
          "entity_id",
          "meeting_id",
          "agenda_item_id",
          "voter_id",
          "vote_value"
        ]
      }
    }
  },
  {
    "type": "function",
    "function": {
      "name": "schedule_meeting",
      "description": "Schedule a board or member meeting",
      "parameters": {
        "type": "object",
        "properties": {
          "entity_id": {
            "type": "string"
          },
          "body_id": {
            "type": "string"
          },
          "meeting_type": {
            "type": "string"
          },
          "title": {
            "type": "string"
          },
          "scheduled_date": {
            "type": "string"
          },
          "agenda_item_titles": {
            "type": "array",
            "items": {
              "type": "string"
            }
          }
        },
        "required": [
          "entity_id",
          "body_id",
          "meeting_type",
          "title"
        ]
      }
    }
  },
  {
    "type": "function",
    "function": {
      "name": "get_signer_link",
      "description": "Generate a signing link for a human obligation",
      "parameters": {
        "type": "object",
        "properties": {
          "obligation_id": {
            "type": "string"
          }
        },
        "required": [
          "obligation_id"
        ]
      }
    }
  },
  {
    "type": "function",
    "function": {
      "name": "get_document_link",
      "description": "Get a download link for a document",
      "parameters": {
        "type": "object",
        "properties": {
          "document_id": {
            "type": "string"
          }
        },
        "required": [
          "document_id"
        ]
      }
    }
  },
  {
    "type": "function",
    "function": {
      "name": "convert_entity",
      "description": "Convert entity type",
      "parameters": {
        "type": "object",
        "properties": {
          "entity_id": {
            "type": "string"
          },
          "new_entity_type": {
            "type": "string"
          },
          "new_jurisdiction": {
            "type": "string"
          }
        },
        "required": [
          "entity_id",
          "new_entity_type"
        ]
      }
    }
  },
  {
    "type": "function",
    "function": {
      "name": "dissolve_entity",
      "description": "Dissolve an entity",
      "parameters": {
        "type": "object",
        "properties": {
          "entity_id": {
            "type": "string"
          },
          "reason": {
            "type": "string"
          },
          "effective_date": {
            "type": "string"
          }
        },
        "required": [
          "entity_id",
          "reason"
        ]
      }
    }
  },
  {
    "type": "function",
    "function": {
      "name": "add_agent_skill",
      "description": "Add a skill to an agent",
      "parameters": {
        "type": "object",
        "properties": {
          "agent_id": {
            "type": "string"
          },
          "skill_name": {
            "type": "string"
          },
          "description": {
            "type": "string"
          },
          "instructions": {
            "type": "string"
          }
        },
        "required": [
          "agent_id",
          "skill_name",
          "description"
        ]
      }
    }
  },
  {
    "type": "function",
    "function": {
      "name": "get_checklist",
      "description": "Get the user's onboarding checklist",
      "parameters": {
        "type": "object",
        "properties": {},
        "required": []
      }
    }
  },
  {
    "type": "function",
    "function": {
      "name": "update_checklist",
      "description": "Update the user's onboarding checklist",
      "parameters": {
        "type": "object",
        "properties": {
          "checklist": {
            "type": "string"
          }
        },
        "required": [
          "checklist"
        ]
      }
    }
  },
  {
    "type": "function",
    "function": {
      "name": "get_signing_link",
      "description": "Get a signing link for a document",
      "parameters": {
        "type": "object",
        "properties": {
          "document_id": {
            "type": "string"
          }
        },
        "required": [
          "document_id"
        ]
      }
    }
  },
  {
    "type": "function",
    "function": {
      "name": "create_agent",
      "description": "Create a new agent",
      "parameters": {
        "type": "object",
        "properties": {
          "name": {
            "type": "string"
          },
          "system_prompt": {
            "type": "string"
          },
          "model": {
            "type": "string"
          }
        },
        "required": [
          "name",
          "system_prompt"
        ]
      }
    }
  },
  {
    "type": "function",
    "function": {
      "name": "send_agent_message",
      "description": "Send a message to an agent",
      "parameters": {
        "type": "object",
        "properties": {
          "agent_id": {
            "type": "string"
          },
          "body": {
            "type": "string"
          }
        },
        "required": [
          "agent_id",
          "body"
        ]
      }
    }
  },
  {
    "type": "function",
    "function": {
      "name": "update_agent",
      "description": "Update an agent",
      "parameters": {
        "type": "object",
        "properties": {
          "agent_id": {
            "type": "string"
          },
          "status": {
            "type": "string"
          }
        },
        "required": [
          "agent_id"
        ]
      }
    }
  }
];
