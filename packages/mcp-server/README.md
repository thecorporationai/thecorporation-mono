# @thecorporation/mcp-server

MCP server that gives AI agents full corporate operations capabilities — entity formation, equity management, payroll, contracts, banking, and tax compliance. 35 tools, zero corporate law expertise required.

Part of [The Corporation](https://thecorporation.ai) — agent-native corporate infrastructure.

## Install

```bash
npm install -g @thecorporation/mcp-server
```

Or run directly:

```bash
npx @thecorporation/mcp-server
```

## Configure with Claude Desktop

Add to your `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "thecorporation": {
      "command": "npx",
      "args": ["-y", "@thecorporation/mcp-server"]
    }
  }
}
```

## Configure with Claude Code

```bash
claude mcp add thecorporation -- npx -y @thecorporation/mcp-server
```

## Authentication

On first run, the server automatically provisions a workspace and saves credentials to `~/.corp/config.json`. To use an existing workspace, set environment variables:

```json
{
  "mcpServers": {
    "thecorporation": {
      "command": "npx",
      "args": ["-y", "@thecorporation/mcp-server"],
      "env": {
        "CORP_API_KEY": "sk_...",
        "CORP_WORKSPACE_ID": "ws_..."
      }
    }
  }
}
```

| Env var | Description | Default |
|---|---|---|
| `CORP_API_URL` | API base URL | `https://api.thecorporation.ai` |
| `CORP_API_KEY` | API key | auto-provisioned |
| `CORP_WORKSPACE_ID` | Workspace ID | auto-provisioned |

## Tools

| Tool | Description |
|---|---|
| `form_entity` | Form an LLC or corporation in any US state |
| `convert_entity` | Convert entity type (e.g. LLC to C-Corp) |
| `dissolve_entity` | Initiate entity dissolution |
| `issue_equity` | Issue shares or membership units |
| `issue_safe` | Issue SAFE notes (pre-money, post-money, MFN) |
| `transfer_shares` | Transfer shares between parties |
| `calculate_distribution` | Calculate and record distributions |
| `create_invoice` | Create and send invoices |
| `run_payroll` | Run payroll for employees and contractors |
| `submit_payment` | Send payments via ACH, wire, or check |
| `open_bank_account` | Open a business bank account |
| `reconcile_ledger` | Reconcile the entity ledger |
| `generate_contract` | Generate NDAs, contractor agreements, offer letters |
| `get_signing_link` | Generate a signing URL for documents |
| `get_document_link` | Get a document download/preview link |
| `file_tax_document` | Generate 1099s, K-1s, estimated tax filings |
| `track_deadline` | Track compliance and filing deadlines |
| `classify_contractor` | Analyze contractor classification risk |
| `convene_meeting` | Convene board, shareholder, or member meetings |
| `schedule_meeting` | Schedule a governance meeting |
| `cast_vote` | Cast votes on meeting agenda items |
| `create_agent` | Create an autonomous AI agent |
| `send_agent_message` | Send a message to an agent |
| `update_agent` | Update agent configuration |
| `add_agent_skill` | Add a skill to an agent |
| `get_workspace_status` | Workspace summary |
| `list_entities` | List all entities |
| `get_cap_table` | View cap table |
| `list_documents` | List documents |
| `list_safe_notes` | List SAFE notes |
| `list_agents` | List agents |
| `list_obligations` | List compliance obligations |
| `get_checklist` | Read the workspace checklist |
| `update_checklist` | Update the workspace checklist |
| `get_billing_status` | Show billing tier and usage |

## Example Prompts

- "Form a Delaware LLC called Acme AI for my consulting business"
- "Issue a $500K post-money SAFE with a $10M cap to Jane Smith"
- "Generate an NDA between my company and Acme Corp"
- "Run payroll for January 2025"
- "File 1099-NECs for all my contractors"

## How It Works

Every operation flows through a deterministic governance kernel:

```
Agent Tool Call → Intent → Policy Evaluation → Execution → Receipt
```

- No side effect without an auditable intent
- Every receipt is hash-bound and immutable
- Human obligations auto-generated for legally required signatures
- Double-entry ledger tracks every dollar

## Links

- [thecorporation.ai](https://thecorporation.ai)
- [GitHub](https://github.com/thecorporationai/thecorporation-mono/tree/main/packages/mcp-server)

## License

Apache-2.0
