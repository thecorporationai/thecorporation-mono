---
name: npx-corp
description: How to use the `npx corp` CLI to manage corporate entities, governance, cap tables, finance, agents, and compliance for TheCorporation platform. Use this skill whenever the user mentions `npx corp`, TheCorporation, corporate formation, entity governance, cap table management, equity issuance, 409A valuations, board meetings, written consent, SAFE instruments, corporate agents, work items, or any task involving programmatic corporate governance. Also use when the user wants to form an LLC or C-Corp, manage board seats, issue stock options, run payroll, generate legal documents, or automate corporate compliance workflows.
---

# npx corp CLI Skill

The `npx corp` CLI is the command-line interface for TheCorporation platform — a corporate governance system built by agents, for agents. It manages the full lifecycle of business entities: formation, governance, cap tables, finance, documents, tax, compliance, agents, and work items.

## Quick Start

### Setup

```bash
npx corp setup
```

Choose a hosting mode:

- **Local (your machine)** — no server needed. Data stored in `~/.corp/data`. Each command invokes the Rust binary directly (~6ms).
- **TheCorporation cloud** — hosted service, authenticates via magic link.
- **Self-hosted server** — point to your own API server URL.

### Local mode (recommended for development)

```bash
npx corp setup              # choose "Local (your machine)"
npx corp status             # verify — all local, no network
```

### Cloud mode

```bash
npx corp setup              # choose "TheCorporation cloud"
# Follow the magic link auth flow
```

### Advanced: Local server mode

For development with a persistent HTTP server:

```bash
npx corp serve --port 8020
npx corp config set api_url http://localhost:8020 --force
```

### Verify Context

```bash
npx corp context
```

Shows active workspace, user, entity, and hosting mode.

## Reference Resolution

The CLI supports flexible reference formats across all commands:

| Format | Example | Description |
|--------|---------|-------------|
| Full UUID | `763dde4d-ca62-4e20-90ba-662c462d4b09` | Canonical ID |
| Short ID | `763dde4d` | First segment of UUID |
| `@last` | `@last` | Most recently created resource of that type |
| Name/Handle | `"Acme Corp"` | Unique name match |

Use `npx corp resolve <kind> <query>` to test resolution. Use `npx corp find <kind> <query>` to list matches.

## Entity Formation

### One-Shot Formation (Recommended for Agents)

Form an entity in a single command using key=value member syntax:

```bash
npx corp form --type c_corp --state DE --name "Acme Inc" --member name="Jane Doe" email=jane@acme.com role=founder officer_title=ceo address="123 Main St, Dover, DE 19901" --member name="John Doe" email=john@acme.com role=founder officer_title=cto address="456 Oak Ave, Dover, DE 19901"
```

### Staged Formation

For more control, use the staged flow:

1. **Create** — `npx corp form create --type c_corp --jurisdiction US-DE --name "Acme Inc" --json`
2. **Add Founders** — `npx corp form add-founder <entity-ref> --name "Jane Doe" --email jane@acme.com --role founder --officer-title ceo --address "123 Main St, Dover, DE 19901"`
3. **Finalize** — `npx corp form finalize <entity-ref> --board-size 1 --incorporator-address "123 Main St, Dover, DE 19901" --company-address "123 Main St, Dover, DE 19901"`
4. **Activate** — `npx corp form activate <entity-ref>`

The `activate` step transitions from `documents_generated` to `active` status.

### Entity Types

`c_corp`, `llc`, `lp`, `llp`, `gp`, `sole_prop`, `cooperative`, `nonprofit`

### Setting Active Entity

Most commands require an active entity context:

```bash
npx corp config set active_entity_id <entity-ref>
```

## Governance

Governance operates through bodies (e.g., Board of Directors), seats, meetings, and resolutions.

### Full Meeting Lifecycle

```
create-body → add-seat → convene → notice → open (--present-seat) → vote → resolve → finalize-item → adjourn
```

1. **Create a governance body**
   ```bash
   npx corp governance create-body --entity-id <ref> --name "Board of Directors" --body-type board --json
   ```

2. **Add seats**
   ```bash
   npx corp governance add-seat --entity-id <ref> --body-id @last --seat-name "Director 1" --held-by <contact-ref> --json
   ```

3. **Convene a meeting**
   ```bash
   npx corp governance convene --entity-id <ref> --body-id @last --title "Board Meeting Q1" --scheduled-date 2026-04-01 --json
   ```

4. **Add agenda items**
   ```bash
   npx corp governance agenda-items <meeting-ref> add --title "Approve 409A" --description "Review and approve valuation" --entity-id <ref> --json
   ```

5. **Send notice** — `npx corp governance notice <meeting-ref> --entity-id <ref>`

6. **Open meeting with present seats**
   ```bash
   npx corp governance open <meeting-ref> --present-seat <seat-ref> --entity-id <ref> --json
   ```

7. **Vote on agenda item**
   ```bash
   npx corp governance vote <meeting-ref> <item-ref> --entity-id <ref> --voter <contact-ref> --vote for --json
   ```

8. **Resolve** — `npx corp governance resolve <meeting-ref> --entity-id <ref> --json`

9. **Finalize item** — `npx corp governance finalize-item <meeting-ref> <item-ref> --entity-id <ref> --status approved --json`

10. **Adjourn** — `npx corp governance adjourn <meeting-ref> --entity-id <ref> --json`

### Written Consent (Alternative to Meetings)

For board approvals without a formal meeting:

```bash
npx corp governance written-consent --entity-id <ref> --body-id <body-ref> --title "Approve Equity Grant" --description "Approve 10000 shares to Jane Doe" --signer <contact-ref> --json
```

Multiple signers: repeat `--signer <ref>` for each. The consent is immediately effective when all signers are provided.

Written consent produces a `resolution_id` needed for downstream operations like equity issuance.

## Cap Table

### Instruments

Create equity instruments before issuing shares:

```bash
npx corp cap-table create-instrument --entity-id <ref> --name "Common Stock" --instrument-type common --authorized-shares 10000000 --par-value 0.0001 --json
```

Instrument types: `common`, `preferred`, `safe`, `convertible_note`, `warrant`, `option`, `rsa`, `membership_unit`, `partnership_interest`

### Issuing Equity

C-Corps require board approval (a resolution) before issuing equity:

```bash
npx corp cap-table issue-equity --entity-id <ref> --instrument-id <ref> --holder-id <contact-ref> --shares 100000 --price-per-share 0.0001 --resolution-id <resolution-ref> --json
```

The `--resolution-id` comes from a governance vote or written consent.

### SAFEs

```bash
npx corp cap-table issue-safe --entity-id <ref> --instrument-id <safe-instrument-ref> --holder-id <contact-ref> --invested-amount 50000 --valuation-cap 10000000 --json
```

You must create a SAFE instrument first (type `safe`) before issuing SAFEs.

### 409A Valuations

Three-step process:

1. **Create valuation** — `npx corp cap-table create-valuation --entity-id <ref> --valuation-type 409a --enterprise-value 1200000 --json`
2. **Submit for approval** — `npx corp cap-table submit-valuation --entity-id <ref> <valuation-ref> --json`
3. **Approve** (after board resolution) — `npx corp cap-table approve-valuation --entity-id <ref> <valuation-ref> --resolution-id <resolution-ref> --json`

Then check: `npx corp cap-table 409a --entity-id <ref> --json`

## Finance

Key subcommands: `invoices`, `invoice`, `payroll-runs`, `payroll`, `payments`, `pay`, `bank-accounts`, `open-account`, `classifications`, `classify-contractor`, `reconciliations`, `reconcile`, `distributions`.

Monetary amounts are in **cents** (e.g., `--amount 500000` = $5,000).

## Documents

```bash
npx corp documents generate --entity-id <ref> --template bylaws --json
npx corp documents signing-link <document-ref>
npx corp documents preview-pdf --document-id <ref> --entity-id <ref>
```

Templates: `bylaws`, `operating_agreement`, `certificate_of_incorporation`, `articles_of_organization`, `employment_offer`, `nda`, `ip_assignment`, etc.

## Agents

Agents are autonomous actors that can claim work items and interact with the system:

```bash
npx corp agents create --entity-id <ref> --name "Ops Agent" --role "operations" --json
npx corp agents skill <agent-ref> --name "gov-watch" --description "Monitor governance deadlines" --json
npx corp agents message <agent-ref> --body "Check upcoming deadlines" --json
npx corp agents pause <agent-ref>
npx corp agents resume <agent-ref>
npx corp agents delete <agent-ref>
```

Agent payloads can be size-sensitive — keep skill descriptions and message bodies concise to avoid 500 errors.

## Work Items

```bash
npx corp work-items create --entity-id <ref> --title "File annual report" --category compliance --description "File with DE SOS" --deadline 2026-06-01 --created-by agent --json
npx corp work-items claim <work-item-ref> --agent-name "Ops Agent"
npx corp work-items complete <work-item-ref> --json
npx corp work-items release <work-item-ref>
npx corp work-items cancel <work-item-ref>
```

Agents can claim work items by name (not just ID). Work items track `actor_type: "agent"`.

## Key Flags

| Flag | Description | Scope |
|------|-------------|-------|
| `--json` | JSON output (machine-readable) | Nearly all commands |
| `--dry-run` | Preview request without executing | Most write operations |
| `--entity-id <ref>` | Scope to an entity | Entity-scoped commands |
| `--force` | Allow security-sensitive updates | `config set` for `api_key` |

## Important Gotchas

1. **C-Corp equity requires board approval** — You must have a `resolution_id` from a governance vote or written consent before issuing equity on a C-Corp. LLCs do not have this requirement.

2. **`active_entity_id` must be set** — Most entity-scoped commands require either `--entity-id` or a configured `active_entity_id`.

3. **Dollar sign in descriptions** — `$` characters in command arguments may be shell-interpolated. In descriptions like `"$50K SAFE"`, the `$50K` can become `0K`. Avoid `$` or escape it properly.

4. **SAFE instrument must exist first** — `issue-safe` requires a pre-existing instrument of type `safe`. Create it with `create-instrument --instrument-type safe`.

5. **Agent payload size** — Large payloads to `agents skill` or `agents message` can cause 500 errors. Keep payloads short.

6. **Formation `finalize` requires complete data** — The staged flow requires `--board-size`, `--incorporator-address`, and `--company-address` on `finalize` for C-Corps. Missing fields cause validation errors.

7. **`demo` command** — Use `npx corp demo --minimal` for a reliable quick seed. Full scenarios (`startup`, `llc`, `restaurant`) may hit validation edge cases depending on version.

## Workflow Patterns for Agents

### Pattern: Form Entity + Issue Equity (C-Corp)

```
form (one-shot) → config set active_entity_id @last → governance create-body → add-seat → written-consent (approve equity) → cap-table create-instrument → issue-equity --resolution-id <consent-resolution>
```

### Pattern: 409A Valuation Approval

```
cap-table create-valuation → submit-valuation → governance written-consent (approve 409A) → approve-valuation --resolution-id <consent-resolution> → cap-table 409a
```

### Pattern: Agent Task Loop

```
agents create → agents skill → work-items create → work-items claim --agent-name → work-items complete → agents message (report status)
```

## Further Reference

For the complete command catalog with all options, run:

```bash
npx corp schema --json
```

For help on any specific command:

```bash
npx corp <command> --help
```
