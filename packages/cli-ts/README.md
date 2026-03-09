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
corp setup                          # authenticate via magic link
corp status                         # workspace summary
corp chat                           # AI assistant with full tool access
corp form --type llc --name "Acme"  # form a new entity
```

## Authentication

`corp setup` authenticates via magic link:

1. Enter your name and email
2. Check your email for a sign-in link from TheCorporation
3. Copy the code from the link URL and paste it into the terminal
4. Credentials are saved to `~/.corp/config.json`

Your workspace is shared across the CLI, [MCP server](https://www.npmjs.com/package/@thecorporation/mcp-server), and [chat](https://humans.thecorporation.ai/chat) — all keyed on your email.

For self-hosted setups (`CORP_API_URL` pointing to your own server), `corp setup` provisions a workspace directly without requiring a magic link.

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
| `corp cap-table create-valuation` | Create a valuation (409A, FMV, etc.) |
| `corp cap-table submit-valuation <id>` | Submit valuation for board approval |
| `corp cap-table approve-valuation <id>` | Approve a valuation |

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
| `corp governance seats <body-id>` | List seats |
| `corp governance meetings <body-id>` | List meetings |
| `corp governance resolutions <meeting-id>` | List resolutions |
| `corp governance agenda-items <meeting-id>` | List agenda items |
| `corp governance convene` | Schedule and convene a meeting |
| `corp governance notice <meeting-id>` | Send meeting notice |
| `corp governance vote <meeting-id> <item-id>` | Cast a vote |
| `corp governance resolve <meeting-id> <item-id>` | Compute a resolution |
| `corp governance finalize-item <meeting-id> <item-id>` | Finalize an agenda item |
| `corp governance adjourn <meeting-id>` | Adjourn a meeting |
| `corp governance cancel <meeting-id>` | Cancel a meeting |
| `corp governance written-consent` | Create a written consent action |

#### Meeting Lifecycle

```
schedule → notice → convene → vote → resolve → finalize → adjourn
```

**Board meeting example:**
```bash
# 1. Schedule meeting with agenda items
corp governance convene --body <body-id> --type BoardMeeting \
  --title "Q1 Board Meeting" --date 2026-03-15 \
  --agenda "Approve budget" --agenda "Elect officers"

# 2. Send notice to participants
corp governance notice <meeting-id>

# 3. Convene with present members (checks quorum)
# (done via MCP tool: convene_meeting with present_seat_ids)

# 4. Cast votes on agenda items
corp governance vote <meeting-id> <item-id> --voter <contact-id> --vote for

# 5. Compute resolution (tallies votes)
corp governance resolve <meeting-id> <item-id> --text "Budget approved for Q1"

# 6. Finalize agenda items
corp governance finalize-item <meeting-id> <item-id> --status voted

# 7. Adjourn meeting
corp governance adjourn <meeting-id>
```

**Written consent (no physical meeting):**
```bash
corp governance written-consent --body <body-id> \
  --title "Approve stock option plan" \
  --description "Unanimous written consent to approve 2026 stock option plan"
```

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

Config is stored at `~/.corp/config.json`. `corp setup` populates it via magic link auth. You can also set values manually:

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
