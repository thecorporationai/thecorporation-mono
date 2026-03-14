# @thecorporation/mcp-server

36 MCP tools that give AI agents full corporate operations capabilities. Entity formation, equity management, payroll, contracts, banking, tax compliance — every tool call passes through the governance kernel, produces an atomic git commit, and returns a signed receipt. Your agent gets corporate powers. Your corporation gets an audit trail.

Part of [TheCorporation](https://thecorporation.ai) — version-controlled governance, autonomous agents, and open-source corporate infrastructure.

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

The MCP server shares credentials with the CLI. Authenticate once, use everywhere.

### Option 1: Authenticate via CLI (recommended)

```bash
npx @thecorporation/cli setup
```

This sends a magic link to your email. Click the link, paste the code into the terminal, and your credentials are saved to `~/.corp/config.json`. The MCP server reads this file automatically — no additional configuration needed.

Your workspace is the same whether you access it from the CLI, MCP server, or chat at [humans.thecorporation.ai](https://humans.thecorporation.ai/chat).

### Option 2: Environment variables

Set credentials explicitly in your MCP client config:

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

### Option 3: Local mode

Run `corp setup` and choose "Local". The MCP server automatically picks up the local-mode config from `~/.corp/` — no additional env vars needed. All requests go through the Rust binary directly (no network).

```bash
npx @thecorporation/cli setup   # choose "Local"
# MCP server now works automatically
```

### Option 4: Self-hosted

Point to your own API server:

```json
{
  "mcpServers": {
    "thecorporation": {
      "command": "npx",
      "args": ["-y", "@thecorporation/mcp-server"],
      "env": {
        "CORP_API_URL": "http://localhost:8000",
        "CORP_API_KEY": "sk_...",
        "CORP_WORKSPACE_ID": "ws_..."
      }
    }
  }
}
```

| Env var | Description | Default |
|---|---|---|
| `CORP_API_URL` | API base URL (`process://` for local) | `https://api.thecorporation.ai` |
| `CORP_API_KEY` | API key | from `~/.corp/auth.json` |
| `CORP_WORKSPACE_ID` | Workspace ID | from `~/.corp/auth.json` |

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
| `get_signer_link` | Generate a signing link for human obligations |
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
