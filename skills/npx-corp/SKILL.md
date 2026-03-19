---
name: thecorporation-form-and-operate
description: How to use the `npx corp` CLI to manage corporate entities, governance, cap tables, finance, agents, and compliance for TheCorporation platform. Use this skill whenever the user mentions `npx corp`, TheCorporation, corporate formation, entity governance, cap table management, equity issuance, 409A valuations, board meetings, written consent, SAFE instruments, corporate agents, work items, or any task involving programmatic corporate governance. Also use when the user wants to form an LLC or C-Corp, manage board seats, issue stock options, run payroll, generate legal documents, or automate corporate compliance workflows.
homepage: https://github.com/thecorporationai/thecorporation-mono
install:
  - kind: node
    package: "@thecorporation/cli"
    bins:
      - corp
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

### Advanced: Local HTTP server mode

For development with a persistent HTTP server:

```bash
npx corp serve --port 8020
npx corp config set api_url http://localhost:8020 --force
```

Or provision a workspace manually:

```bash
curl -s -X POST http://localhost:8020/v1/workspaces/provision -H "Content-Type: application/json" | cat
```

Returns `{"workspace_id": "...", "api_key": "..."}`. Configure:

```bash
npx corp config set api_url http://localhost:8020
npx corp config set api_key <key> --force
npx corp config set workspace_id <workspace_id>
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
| Name/Handle | `"Acme Corp"` or `acme-corp` | Unique name or handle match |

Use `npx corp resolve <kind> <ref>` to test resolution. Use `npx corp find <kind> <query>` to list matches.

## Entity Formation

### One-Shot Formation (Recommended for Agents)

Form an entity in a single command using key=value member syntax:

```bash
npx corp form \
  --type c_corp \
  --name "Acme Inc" \
  --jurisdiction US-DE \
  --member "name=Jane Doe,email=jane@acme.com,role=director,officer_title=ceo,is_incorporator=true,address=123 Main St|Dover|DE|19901" \
  --member "name=John Doe,email=john@acme.com,role=director,officer_title=cto,address=456 Oak Ave|Dover|DE|19901" \
  --json
```

**One-shot flags:**
- `--type <type>` — Entity type (see below)
- `--name <name>` — Legal name of the entity
- `--jurisdiction <jurisdiction>` — e.g. `US-DE`, `US-WY`
- `--member <spec>` — Founder (repeatable); key=value format with keys: `name`, `email`, `role` (director|officer|manager|member|chair), `officer_title` (ceo|cfo|cto|coo|secretary|treasurer|president|vp|other), `is_incorporator` (true|false), `address` (street|city|state|zip), `pct` (ownership %), `shares` (shares purchased)
- `--address <address>` — Company address as `street,city,state,zip`
- `--fiscal-year-end <date>` — Fiscal year end (MM-DD, default "12-31")
- `--s-corp` — Elect S-Corp status
- `--transfer-restrictions` — Enable transfer restrictions
- `--rofr` — Enable right of first refusal

**Important:** Member addresses use pipe `|` as separator (street|city|state|zip), NOT commas.

For LLCs, include `pct` (ownership percentage) and use `role=member`:

```bash
npx corp form --type llc --name "Acme LLC" --jurisdiction US-WY \
  --member "name=Jane Doe,email=jane@acme.com,role=member,pct=60,address=100 Main St|Cheyenne|WY|82001" \
  --json
```

Member roles: `director`, `officer`, `manager`, `member`, `chair` (NOT `founder` — that is not a valid role).

### Staged Formation

For more control, use the staged flow:

1. **Create** — `npx corp form create --type c_corp --name "Acme Inc" --jurisdiction US-DE --json`
2. **Add Founders** — `npx corp form add-founder @last --name "Jane Doe" --email jane@acme.com --role director --pct 100 --officer-title ceo --incorporator --address "123 Main St,Dover,DE,19901" --json`
3. **Finalize** — `npx corp form finalize @last --board-size 1 --company-address "123 Main St,Dover,DE,19901" --json`
4. **Activate** — `npx corp form activate @last --json`

The `activate` step transitions from `documents_generated` to `active` status by auto-signing formation documents.

**Staged `add-founder` flags:**
- `--name` (required), `--email` (required), `--role` (required: director|officer|manager|member|chair), `--pct` (required: ownership %)
- `--officer-title <title>` (choices: ceo, cfo, cto, coo, secretary, treasurer, president, vp, other)
- `--incorporator` (boolean flag — mark as sole incorporator)
- `--address <address>` (as `street,city,state,zip`)

**Staged `finalize` flags:**
- `--board-size <n>`, `--authorized-shares <n>`, `--par-value <value>`
- `--company-address`, `--incorporator-name`, `--incorporator-address`
- `--principal-name` (LLC manager name)

Both one-shot `form` and staged `form create` use the same flags: `--type` and `--name`.

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
create-body → add-seat → convene (with --agenda) → notice → open (--present-seat) → vote → resolve → finalize-item → adjourn
```

1. **Create a governance body**
   ```bash
   npx corp governance create-body --name "Board of Directors" --body-type board_of_directors --json
   ```
   Body types: `board_of_directors`, `llc_member_vote`

2. **Add seats** — `<body-ref>` is a positional argument
   ```bash
   npx corp governance add-seat @last --holder <contact-ref> --role member --json
   ```
   Roles: `chair`, `member`, `officer`, `observer`

3. **Convene a meeting** — `--body`, `--type`, and `--title` are all required
   ```bash
   npx corp governance convene --body @last --type board_meeting --title "Board Meeting Q1" --date 2026-04-01 --agenda "Approve equity grant" --agenda "Review financials" --json
   ```
   Meeting types: `board_meeting`, `shareholder_meeting`, `member_meeting`, `written_consent`

   Agenda items are added via repeatable `--agenda` flags on `convene`. There is no separate `agenda-items add` command.

4. **List agenda items** — `npx corp governance agenda-items <meeting-ref> --json`

5. **Send notice** — `npx corp governance notice <meeting-ref> --json`

6. **Open meeting with present seats**
   ```bash
   npx corp governance open <meeting-ref> --present-seat <seat-ref> --json
   ```
   Repeat `--present-seat` for each seat present.

7. **Vote on agenda item**
   ```bash
   npx corp governance vote <meeting-ref> <item-ref> --voter <contact-ref> --vote for --json
   ```
   Vote values: `for`, `against`, `abstain`, `recusal`

8. **Compute resolution** — requires both `<meeting-ref>` and `<item-ref>` as positional args, plus `--text`
   ```bash
   npx corp governance resolve <meeting-ref> <item-ref> --text "RESOLVED: The board approves the equity grant" --json
   ```

9. **Finalize item**
   ```bash
   npx corp governance finalize-item <meeting-ref> <item-ref> --status voted --json
   ```
   Status choices: `discussed`, `voted`, `tabled`, `withdrawn` (NOT `approved`)

10. **Adjourn** — `npx corp governance adjourn <meeting-ref> --json`

### Written Consent (Alternative to Meetings)

Written consent creates a meeting of type `written_consent` in `convened` status with an auto-generated agenda item. You must then vote and resolve it manually to obtain a `resolution_id`:

```bash
npx corp governance written-consent --body <body-ref> --title "Approve Equity Grant" --description "Approve 10000 shares to Jane Doe" --json
```

Then vote with each director and compute the resolution:

```bash
npx corp governance agenda-items @last --json
npx corp governance vote @last <item-ref> --voter <contact-ref> --vote for --json
npx corp governance resolve @last <item-ref> --text "RESOLVED that equity is approved" --json
```

The resolution must pass quorum (majority of seated directors must vote "for") to get `passed: true`.

## Cap Table

### Instruments

Create equity instruments before issuing shares:

```bash
npx corp cap-table create-instrument --kind common_equity --symbol COMMON --authorized-units 10000000 --issue-price-cents 1 --json
```

**Instrument kinds:** `common_equity`, `preferred_equity`, `membership_unit`, `option_grant`, `safe`

**Flags:**
- `--kind <kind>` (required) — instrument kind
- `--symbol <symbol>` (required) — e.g. COMMON, SERIES-A, OPTION-PLAN
- `--authorized-units <n>` — total authorized shares/units
- `--issue-price-cents <n>` — issue price in cents
- `--issuer-legal-entity-id <ref>` — auto-resolved if omitted
- `--terms-json <json>` — JSON object of additional terms

### Issuing Equity

C-Corps require board approval (both `--meeting-id` and `--resolution-id`) before issuing equity:

```bash
npx corp cap-table issue-equity --grant-type common --shares 100000 --recipient "Jane Doe" --meeting-id <meeting-ref> --resolution-id <resolution-ref> --json
```

**Flags:**
- `--grant-type <type>` (required) — `common`, `preferred`, `membership_unit`, `stock_option`, `iso`, `nso`, `rsa`
- `--shares <n>` (required) — number of shares
- `--recipient <name>` (required) — recipient name (not an ID)
- `--email <email>` — recipient email (auto-creates contact if needed)
- `--instrument-id <ref>` — instrument reference (auto-resolved by grant type if omitted)
- `--meeting-id <ref>` — board meeting reference (required for board-governed entities)
- `--resolution-id <ref>` — board resolution reference (required for board-governed entities)

### SAFEs

```bash
npx corp cap-table issue-safe --investor "Jane Investor" --amount 5000000 --valuation-cap 1000000000 --json
```

**Important:** `--amount` and `--valuation-cap` are in **cents** (e.g., `5000000` = $50,000).

**Flags:**
- `--investor <name>` (required) — investor name
- `--amount <n>` (required) — principal amount in cents
- `--valuation-cap <n>` (required) — valuation cap in cents
- `--safe-type <type>` — SAFE type (default: `post_money`)
- `--meeting-id <ref>` — board meeting reference
- `--resolution-id <ref>` — board resolution reference

### 409A Valuations

Five-step process (create → submit → board approval → approve → check):

1. **Create valuation**
   ```bash
   npx corp cap-table create-valuation --type four_oh_nine_a --date 2026-03-15 --methodology market --enterprise-value 120000000 --json
   ```
   Types: `four_oh_nine_a`, `fair_market_value`
   Methodologies: `income`, `market`, `asset`, `backsolve`, `hybrid`
   `--enterprise-value` and `--fmv` are in cents.

2. **Submit for approval** — `npx corp cap-table submit-valuation <valuation-ref> --json`
   This auto-creates a board meeting with an agenda item.

3. **Approve via governance** — Open the auto-created meeting, vote, and resolve (see meeting lifecycle above).

4. **Approve valuation** — `npx corp cap-table approve-valuation <valuation-ref> --resolution-id <resolution-ref> --json`

5. **Check FMV** — `npx corp cap-table 409a --json`

### Transfers

```bash
npx corp cap-table transfer --from <contact-ref> --to <contact-ref> --shares 1000 --share-class-id <ref> --governing-doc-type bylaws --transferee-rights full_member --json
```

### Distributions

```bash
npx corp cap-table distribute --amount 10000000 --type dividend --description "Q1 distribution" --json
```

Distribution types: `dividend` (default), `return`, `liquidation`

## Finance

All monetary amounts in **cents**.

```bash
npx corp finance invoice --customer "Client Name" --amount 500000 --due-date 2026-04-15 --json
npx corp finance pay --recipient "Vendor" --amount 500000 --method ach --json
npx corp finance payroll --period-start 2026-03-01 --period-end 2026-03-15 --json
npx corp finance open-account --institution "Mercury" --json
npx corp finance classify-contractor --name "Eve" --state DE --hours 40 --exclusive --duration 12 --json
npx corp finance reconcile --start-date 2026-01-01 --end-date 2026-03-15 --json
```

Note: Payments require an active bank account. `open-account` creates accounts in `pending_review` state.

List commands: `invoices`, `payments`, `bank-accounts`, `payroll-runs`, `classifications`, `reconciliations`, `distributions`

## Documents

### Generate a contract

```bash
npx corp documents generate --template employment_offer --counterparty "Jane Doe" --param department=Engineering --param equity_grant_type=iso --param base_salary=150000 --json
```

**Flags:**
- `--template <type>` (required) — `consulting_agreement`, `employment_offer`, `contractor_agreement`, `nda`, `custom`
- `--counterparty <name>` (required) — counterparty name
- `--effective-date <date>` — ISO 8601 date (defaults to today)
- `--base-salary <amount>` — for employment_offer template
- `--param <key=value>` — additional template parameter (repeatable)

### Signing

```bash
npx corp documents signing-link <document-ref>
npx corp documents sign <document-ref> --json
npx corp documents sign-all --json
```

### PDF Preview

```bash
npx corp documents preview-pdf --definition-id bylaws
```

**Known issue:** `preview-pdf` fails in `process://` local transport mode. Use cloud or HTTP server mode for PDF previews.

Templates: `bylaws`, `operating_agreement`, `certificate_of_incorporation`, `articles_of_organization`, `employment_offer`, `nda`, `ip_assignment`, etc.

## Tax

```bash
npx corp tax file --type 1120 --year 2025 --json
npx corp tax deadline --type estimated_tax --due-date 2026-04-15 --description "Q1 estimated" --recurrence annual --json
```

Filing types: `1120`, `1120s`, `1065`, `franchise_tax`, `annual_report`, `83b`, `1099_nec`, `k1`, `941`, `w2`

List commands: `tax filings`, `tax deadlines`

## Agents

Agents are workspace-scoped autonomous actors (no `--entity-id` needed):

```bash
npx corp agents create --name "Ops Agent" --prompt "You monitor governance deadlines" --json
npx corp agents skill <agent-ref> --name "gov-watch" --description "Monitor deadlines" --json
npx corp agents message <agent-ref> --body "Check upcoming deadlines" --json
npx corp agents pause <agent-ref>
npx corp agents resume <agent-ref>
npx corp agents delete <agent-ref>
npx corp agents show <agent-ref> --json
```

**`agents create` flags:**
- `--name <name>` (required) — agent display name
- `--prompt <prompt>` (required) — system prompt for the agent
- `--model <model>` — LLM model to use

Keep skill descriptions and message bodies concise to avoid 500 errors. Use `--body-file` or `--instructions-file` for large content.

## Work Items

```bash
npx corp work-items create --title "File annual report" --category compliance --description "File with DE SOS" --deadline 2026-06-01 --created-by "Ops Agent" --json
npx corp work-items claim <item-ref> --by "Ops Agent" --json
npx corp work-items complete <item-ref> --by "Ops Agent" --result "Filed successfully" --json
npx corp work-items release <item-ref> --json
npx corp work-items cancel <item-ref> --json
npx corp work-items show <item-ref> --json
```

Note: `--by` is required for both `claim` and `complete`. Aliases: `--claimer` for claim, `--completed-by` for complete.

## Contacts

```bash
npx corp contacts --json
npx corp contacts add --name "Jane Doe" --email jane@acme.com --type individual --category investor --cap-table-access detailed --json
npx corp contacts edit <contact-ref> --category board_member --json
npx corp contacts show <contact-ref> --json
```

Contact types: `individual`, `organization`
Categories: `employee`, `contractor`, `board_member`, `investor`, `law_firm`, `valuation_firm`, `accounting_firm`, `officer`, `founder`, `member`, `other`

## Entity Management

```bash
npx corp entities --json
npx corp entities show <entity-ref> --json
npx corp entities convert <entity-ref> --to c_corp --jurisdiction US-DE
npx corp entities dissolve <entity-ref> --reason "Voluntary dissolution" --effective-date 2026-03-15
```

## Services

```bash
npx corp services catalog --json
npx corp services buy <slug> --json
npx corp services list --json
npx corp services show <ref> --json
npx corp services cancel <ref> --json
```

## Obligations

```bash
npx corp obligations --json
npx corp obligations --tier overdue --json
```

## Key Flags

| Flag | Description | Scope |
|------|-------------|-------|
| `--json` | JSON output (machine-readable) | Nearly all commands |
| `--dry-run` | Preview request without executing | Most write operations |
| `--entity-id <ref>` | Scope to an entity | Entity-scoped commands |
| `--force` | Allow security-sensitive updates | `config set` for `api_key` |

## Important Gotchas

1. **C-Corp equity requires board approval** — You must have both `--meeting-id` and `--resolution-id` from a governance vote or written consent before issuing equity or SAFEs on a C-Corp.

2. **`active_entity_id` must be set** — Most entity-scoped commands require either `--entity-id` or a configured `active_entity_id`.

3. **Dollar sign in descriptions** — `$` characters in command arguments may be shell-interpolated. In descriptions like `"$50K SAFE"`, the `$50K` can become `0K`. Avoid `$` or escape it.

4. **Written consent requires manual vote+resolve** — Written consent creates a meeting with an agenda item, but does NOT auto-resolve. You must vote with each director and resolve manually to get a `resolution_id`.

5. **Quorum matters** — If you have multiple board seats, you need majority votes before resolving. A resolution with fewer than quorum votes will have `passed: false`.

6. **All monetary values in cents** — `--amount`, `--enterprise-value`, `--fmv`, `--valuation-cap`, `--issue-price-cents`, `--price-per-share-cents` are all in cents.

7. **Formation `finalize` requires complete data** — The staged flow requires `--board-size` and `--company-address` on `finalize` for C-Corps.

8. **Member role is not `founder`** — Valid roles: `director`, `officer`, `manager`, `member`, `chair`.

9. **Body type is `board_of_directors`** — Not `board`. Valid types: `board_of_directors`, `llc_member_vote`.

10. **Address format varies** — One-shot `--member` addresses use pipe `|` separator. Staged `--address` and `--company-address` use comma `,` separator.

12. **Formation auto-creates board** — Forming a C-Corp automatically creates a Board of Directors governance body with a seat for each director-role member.

13. **LLC requires pct** — LLC members require `pct` (ownership percentage). C-Corp members can use either `pct` or `shares`.

14. **`demo` command** — Use `npx corp demo --name "Acme" --minimal` for a reliable quick seed.

## Workflow Patterns for Agents

### Pattern: Form Entity + Issue Equity (C-Corp)

```
form (one-shot with --type c_corp) → config set active_entity_id @last → written-consent (approve equity) → vote → resolve → cap-table create-instrument → issue-equity --grant-type common --shares <n> --recipient <name> --meeting-id --resolution-id
```

Note: Formation auto-creates a board, so skip `create-body` + `add-seat` if using one-shot formation.

### Pattern: 409A Valuation Approval

```
cap-table create-valuation --type four_oh_nine_a --date <date> --methodology <method> → submit-valuation <ref> → open auto-created meeting → vote → resolve → approve-valuation <ref> --resolution-id <resolution-ref> → cap-table 409a
```

### Pattern: Agent Task Loop

```
agents create --name <name> --prompt <prompt> → agents skill <ref> --name <skill> --description <desc> → work-items create --created-by <agent-name> → work-items claim <ref> --by <agent-name> → work-items complete <ref> --by <agent-name>
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
