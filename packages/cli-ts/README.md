# @thecorporation/cli

Corporate governance from the terminal. Every command validates input, passes it through the governance kernel, and commits the result to your git-backed corporate repo. The command runs. The commit appears. The corporate record updates.

`corp` handles entity formation, equity management, payroll, tax filings, governance, and agent management — with an AI assistant that can execute any corporate action via `corp chat`.

Part of [TheCorporation](https://thecorporation.ai) — version-controlled governance, autonomous agents, and open-source corporate infrastructure.

## Install

```bash
npm install -g @thecorporation/cli
```

## Quick Start

```bash
corp setup                          # interactive first-run wizard
corp status                         # workspace summary
corp chat                           # AI assistant with full tool access
corp form --type llc --name "Acme"  # form a new entity
```

## Commands

### Core

| Command | Description |
|---|---|
| `corp setup` | Interactive first-run wizard |
| `corp status` | Workspace summary |
| `corp chat` | AI assistant with corporate tools |
| `corp link` | Generate a claim code to pair a device |
| `corp claim <code>` | Redeem a claim code to join a workspace |
| `corp serve` | Start the API server locally |
| `corp demo --name <name>` | Seed a demo corporation |

### Entities

| Command | Description |
|---|---|
| `corp form` | Form a new LLC or corporation |
| `corp entities` | List entities |
| `corp entities show <id>` | Show entity details |
| `corp entities convert <id>` | Convert entity type |
| `corp entities dissolve <id>` | Dissolve an entity |

### Cap Table & Equity

| Command | Description |
|---|---|
| `corp cap-table` | View cap table |
| `corp cap-table issue-equity` | Issue shares or membership units |
| `corp cap-table issue-safe` | Issue a SAFE note |
| `corp cap-table transfer` | Transfer shares |
| `corp cap-table distribute` | Calculate a distribution |
| `corp cap-table safes` | List SAFE notes |
| `corp cap-table transfers` | List share transfers |
| `corp cap-table valuations` | View valuation history |
| `corp cap-table 409a` | Current 409A valuation |

Round close gating (v1, February 28, 2026):
- Conversion execution now requires an authorized execute intent (`equity.round.execute_conversion`) and `intent_id`.
- Required sequence is: apply terms -> board approve -> accept round -> execute conversion.

### Finance

| Command | Description |
|---|---|
| `corp finance invoice` | Create an invoice |
| `corp finance payroll` | Run payroll |
| `corp finance pay` | Submit a payment |
| `corp finance open-account` | Open a business bank account |
| `corp finance classify-contractor` | Analyze contractor classification risk |
| `corp finance reconcile` | Reconcile the ledger |

### Governance

| Command | Description |
|---|---|
| `corp governance` | List governance bodies |
| `corp governance convene` | Convene a meeting |
| `corp governance vote` | Cast a vote |
| `corp governance seats <body-id>` | List seats |
| `corp governance meetings <body-id>` | List meetings |
| `corp governance resolutions <meeting-id>` | List resolutions |

### Documents & Compliance

| Command | Description |
|---|---|
| `corp documents` | List documents |
| `corp documents generate` | Generate a contract |
| `corp documents signing-link <id>` | Get a signing link |
| `corp tax file` | File a tax document |
| `corp tax deadline` | Track a compliance deadline |

### Contacts

| Command | Description |
|---|---|
| `corp contacts` | List contacts |
| `corp contacts show <id>` | Show contact details |
| `corp contacts add` | Add a contact |
| `corp contacts edit <id>` | Edit a contact |

### Agents

| Command | Description |
|---|---|
| `corp agents` | List agents |
| `corp agents create` | Create an agent |
| `corp agents show <id>` | Show agent details |
| `corp agents message <id>` | Send a message to an agent |
| `corp agents pause <id>` | Pause an agent |
| `corp agents resume <id>` | Resume an agent |
| `corp agents delete <id>` | Delete an agent |
| `corp agents skill <id>` | Add a skill to an agent |
| `corp agents executions <id>` | List agent executions |

### Billing & Approvals

| Command | Description |
|---|---|
| `corp billing` | Show billing status |
| `corp billing portal` | Open Stripe Customer Portal |
| `corp billing upgrade` | Upgrade plan |
| `corp approvals` | List pending approvals |
| `corp approvals approve <id>` | Approve a pending action |
| `corp approvals reject <id>` | Reject a pending action |

### Config

| Command | Description |
|---|---|
| `corp config set <key> <value>` | Set a config value |
| `corp config get <key>` | Get a config value |
| `corp config list` | List all config |

## Chat Commands

Inside `corp chat`, these slash commands are available:

| Command | Description |
|---|---|
| `/status` | Workspace summary |
| `/obligations` | Compliance obligations |
| `/digest` | Daily digest |
| `/config` | Show config |
| `/model <name>` | Switch LLM model |
| `/cost` | Show session token usage |
| `/clear` | Clear conversation |
| `/help` | Available commands |
| `/quit` | Exit chat |

## Configuration

Config is stored at `~/.corp/config.json`. The `corp setup` wizard will populate it, or set values manually:

```bash
corp config set api_url https://api.thecorporation.ai
corp config set api_key sk_...
corp config set workspace_id ws_...
```

For the chat command, also configure:

```bash
corp config set llm.provider openai   # or anthropic
corp config set llm.api_key sk-...
corp config set llm.model gpt-4o      # or claude-sonnet-4-6
```

## Links

- [thecorporation.ai](https://thecorporation.ai)
- [GitHub](https://github.com/thecorporationai/thecorporation-mono/tree/main/packages/cli-ts)

## License

MIT
