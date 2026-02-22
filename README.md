# The Corporation

**Your agent formed an LLC in 4 tool calls.**

The Corporation is an MCP server that gives AI agents full corporate operations capabilities. Entity formation, equity management, payroll, contracts, banking, tax compliance — 12 tools, zero corporate law expertise required.

Built for agent developers who need their agents to operate businesses without learning about K-1 allocations, 409A valuations, or multi-state payroll withholding.

## Get Started (60 seconds)

### Claude Desktop

Add to `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "thecorporation": {
      "command": "uvx",
      "args": ["--from", "git+https://github.com/yourusername/thecorporation#subdirectory=mcp-server", "thecorporation"]
    }
  }
}
```

Then ask Claude: *"Form a Wyoming LLC called Acme AI for my consulting business"*

### Python SDK

```python
from thecorporation.server import form_entity, create_invoice, open_bank_account

# Form an LLC
entity = form_entity("Acme AI LLC", "llc", "WY")

# Open a bank account
account = open_bank_account(entity["entity_id"])

# Send your first invoice
invoice = create_invoice(entity["entity_id"], "Client Corp", 500000, "2025-04-01")
```

## Tools

| Tool | What it does |
|------|-------------|
| `form_entity` | Form an LLC or corporation in any US state |
| `issue_equity` | Issue shares or membership units |
| `issue_safe` | Issue SAFE notes (pre-money, post-money, MFN) |
| `create_invoice` | Create invoices for customers |
| `run_payroll` | Run payroll for employees and contractors |
| `submit_payment` | Send payments via ACH, wire, or check |
| `sign_document` | E-sign formation docs, contracts, resolutions |
| `convene_meeting` | Convene board or member meetings |
| `cast_vote` | Cast votes on agenda items |
| `open_bank_account` | Open business bank accounts (auto-assembles KYB) |
| `generate_contract` | Generate NDAs, contractor agreements, offer letters |
| `file_tax_document` | Generate 1099s, K-1s, estimated tax filings |

## How It Works

```
Agent Tool Call
       |
       v
   Intent ──> Policy Evaluation ──> Execution ──> Receipt
                    |                    |
              Human Obligation     Audit Trail
              (if legally required)
```

Every operation flows through a deterministic governance kernel. No side effect without an auditable intent. Every receipt is hash-bound and immutable. When a legally required human action is needed (signing formation docs, board approval for a fundraising round), the system generates a `HumanObligation` and blocks until fulfilled.

## Architecture

19 phases of corporate infrastructure, fully implemented:

- **Formation & Governance** — Entity creation, board meetings, voting, resolutions, bylaws
- **Equity & Cap Table** — Share classes, grants, vesting, 409A valuations, share transfers
- **Fundraising** — SAFEs (pre/post-money, MFN), priced rounds, cap table waterfall, investor management
- **Treasury & Payments** — Double-entry ledger, Stripe integration, invoicing, spending controls
- **Workforce** — Employee/contractor classification, multi-state payroll, benefits, workers' comp
- **Tax & Compliance** — 1099-NEC, K-1, estimated taxes, compliance calendar with 60+ deadlines
- **Banking** — KYB assembly, bank account opening, bank feed ingestion, reconciliation
- **Contracts** — Template engine, lifecycle management, deadline tracking
- **Agent Commerce** — Machine-readable payment protocol for agent-to-agent transactions
- **Observability** — Immutable audit trail, OpenTelemetry tracing, Prometheus metrics

## 21 Business Scenarios

Tested against real-world business scenarios from SaaS startups to agent-run cooperatives:

**Standard:** SaaS Startup, Consulting LLC, Real Estate Syndication, VC Fund, Food Truck Fleet, Freelance Agency, Vending Machine Network, Apartment Building

**Agent-Native:** Agent Venture Studio, Agent Marketplace, Agent Compute Cooperative, Autonomous Fleet Operator

**Edge Cases:** Founder Breakup, Down Round Crisis, Multi-State Compliance, LLC-to-C-Corp Conversion, Zombie Company Wind-Down, Rapid Scaling Chaos, Holding Company Empire

**Small Business:** Family Restaurant, E-Commerce Dropship, Construction Contractor

## What This Is (and Isn't)

**This is** infrastructure for AI agents that need to operate businesses. It handles corporate law, tax compliance, and financial plumbing so your agent doesn't have to.

**This is not** legal advice, tax advice, or a substitute for professional counsel.

## License

Apache 2.0
