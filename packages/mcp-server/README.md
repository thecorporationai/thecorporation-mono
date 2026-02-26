# The Corporation — MCP Server

<!-- mcp-name: io.github.thecorporation/corporate-os -->

Your agent just formed an LLC in 4 tool calls.

An MCP server that gives AI agents full corporate operations capabilities — entity formation, equity management, payroll, contracts, banking, and tax compliance. 12 tools, zero corporate law expertise required.

## Install

```bash
uv pip install thecorporation
```

Or run directly:

```bash
uvx thecorporation
```

## Configure with Claude Desktop

Add to your `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "thecorporation": {
      "command": "uvx",
      "args": ["thecorporation"]
    }
  }
}
```

## Tools

| Tool | What it does |
|------|-------------|
| `form_entity` | Form an LLC or corporation in any US state |
| `issue_equity` | Issue shares or membership units to recipients |
| `issue_safe` | Issue SAFE notes (pre-money, post-money, MFN) to investors |
| `create_invoice` | Create and send invoices to customers |
| `run_payroll` | Run payroll for employees and contractors |
| `submit_payment` | Send payments via ACH, wire, or check |
| `get_signing_link` | Generate a signing link — documents can only be signed by humans via this URL |
| `convene_meeting` | Convene board, shareholder, or member meetings |
| `cast_vote` | Cast votes on meeting agenda items |
| `open_bank_account` | Open a business bank account (auto-assembles KYB) |
| `generate_contract` | Generate NDAs, contractor agreements, offer letters |
| `file_tax_document` | Generate 1099s, K-1s, estimated tax filings |

## Example Prompts

- "Form a Delaware LLC called Acme AI for my consulting business"
- "Issue a $500K post-money SAFE with a $10M cap to Jane Smith"
- "Generate an NDA between my company and Acme Corp"
- "Run payroll for January 2025"
- "File 1099-NECs for all my contractors for tax year 2025"

## How It Works

Every operation flows through a deterministic governance kernel:

```
Agent Tool Call → Intent → Policy Evaluation → Execution → Receipt
```

- No side effect without an auditable intent
- Every receipt is hash-bound and immutable
- Human obligations auto-generated for legally required signatures
- Double-entry ledger tracks every dollar

## What This Is (and Isn't)

**This is** infrastructure for AI agents that need to operate businesses. It handles the corporate law, tax compliance, and financial plumbing so your agent doesn't have to.

**This is not** legal advice, tax advice, or a substitute for professional counsel. The platform computes from data it already has (K-1 allocations, 1099 aggregations, estimated taxes) but doesn't interpret tax law.

## License

Apache 2.0
