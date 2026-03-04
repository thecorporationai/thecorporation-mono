# @thecorporation/agent

Corporate governance agent extension for the [Pi coding agent](https://github.com/nicholasgasior/pi-coding-agent). Adds 36 tools for entity formation, equity management, payroll, compliance, and more.

Part of [The Corporation](https://thecorporation.ai) — agent-native corporate infrastructure.

## Install

```bash
npm install -g @mariozechner/pi-coding-agent @thecorporation/agent
```

## Usage

```bash
corp-agent                          # interactive mode — launches Pi with corporate tools
corp-agent -p "list entities"       # one-shot mode
corp-agent -p "form an LLC"         # guided entity formation
```

The `corp-agent` command:

1. Copies the corporate tools extension into your project's `.pi/extensions/`
2. Seeds `.pi/AGENT.md` with corporate context (only if not already present)
3. Launches `pi` with all arguments forwarded

## Configuration

Create `~/.corp/config.json`:

```json
{
  "api_url": "https://api.thecorporation.ai",
  "api_key": "your-api-key",
  "workspace_id": "your-workspace-id"
}
```

Or use environment variables: `CORP_API_URL`, `CORP_CONFIG_DIR`.

## Tools

### Read Tools (auto-approved)

| Tool | Description |
|---|---|
| `get_workspace_status` | Workspace summary with entity/document/grant counts |
| `list_entities` | List all companies in the workspace |
| `get_cap_table` | Full cap table for an entity |
| `list_documents` | List documents for an entity |
| `list_safe_notes` | List SAFE notes for an entity |
| `list_agents` | List autonomous agents |
| `get_checklist` | Read the workspace checklist |
| `get_document_link` | Get a document download/preview link |
| `get_signing_link` | Generate a signing URL |
| `list_obligations` | List compliance obligations with urgency tiers |
| `get_billing_status` | Show billing tier and usage |

### Write Tools (require confirmation)

| Tool | Description |
|---|---|
| `form_entity` | Form a new LLC or corporation |
| `convert_entity` | Convert entity type (e.g. LLC to C-Corp) |
| `dissolve_entity` | Initiate entity dissolution |
| `issue_equity` | Issue shares or membership units |
| `issue_safe` | Issue a SAFE note |
| `transfer_shares` | Transfer shares between parties |
| `calculate_distribution` | Calculate and record a distribution |
| `create_invoice` | Create an invoice |
| `run_payroll` | Run payroll for a pay period |
| `submit_payment` | Submit a payment (ACH, wire, check) |
| `open_bank_account` | Open a business bank account |
| `reconcile_ledger` | Reconcile the entity ledger |
| `generate_contract` | Generate a contract from template |
| `file_tax_document` | File a tax document (1099, K-1, etc.) |
| `track_deadline` | Track a compliance deadline |
| `classify_contractor` | Analyze contractor classification risk |
| `convene_meeting` | Convene a governance meeting |
| `schedule_meeting` | Schedule a meeting |
| `cast_vote` | Cast a vote on a meeting agenda item |
| `update_checklist` | Update the workspace checklist |
| `create_agent` | Create an autonomous AI agent |
| `send_agent_message` | Send a message to an agent |
| `update_agent` | Update agent configuration |
| `add_agent_skill` | Add a skill to an agent |

## Slash Commands

| Command | Description |
|---|---|
| `/status` | Workspace overview |
| `/entities` | Browse entities interactively |
| `/cap-table [entity_id]` | View cap table |
| `/obligations` | Compliance obligations by urgency |
| `/documents [entity_id]` | List documents with signing status |
| `/billing` | Billing tier and usage |

## Links

- [thecorporation.ai](https://thecorporation.ai)
- [GitHub](https://github.com/thecorporationai/thecorporation-mono/tree/main/packages/agent)

## License

MIT
