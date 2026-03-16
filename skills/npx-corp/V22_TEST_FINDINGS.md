# npx corp v26.3.22 — Full Command Test Results

Date: 2026-03-15
CLI Version: 26.3.22

## Breaking Changes from v21

### Formation
- `--type` → `--entity-type`, `--name` → `--legal-name`, `--state`/`--jurisdiction` → `--jurisdiction`
- `--member` address format changed to pipe-delimited: `address=street|city|state|zip`
- `role=founder` no longer valid. Roles: `director`, `officer`, `manager`, `member`, `chair`
- `--pct` now required for `add-founder` (was optional)
- `cto` now accepted in `--officer-title` choices (was rejected in v20)
- `--incorporator` is now a boolean flag (was `--is-incorporator`)
- New `--address` flag for company address (comma-delimited: `street,city,state,zip`)
- New `--fiscal-year-end` flag (default `12-31`, format `MM-DD`)
- New `--s-corp`, `--transfer-restrictions`, `--rofr` boolean flags
- `--members-file` requires file in CWD (set `CORP_ALLOW_UNSAFE_FILE_INPUT=1` for external paths)
- `form create` (staged) uses `--type`/`--name` (NOT `--entity-type`/`--legal-name` — different from one-shot)
- `form create` adds: `--registered-agent-name`, `--registered-agent-address`, `--formation-date`, `--company-address`

### Governance
- `convene`: `--body-id` → `--body`, `--scheduled-date` → `--date`, new `--type` required, `--agenda` inline
- `add-seat`: `--body-id` → positional `<body-ref>`, `--held-by` → `--holder`, `--seat-name` removed
- `resolve`: now requires positional `<item-ref>` and `--text`
- `finalize-item`: `approved` removed from status choices. Valid: `discussed`, `voted`, `tabled`, `withdrawn`
- `written-consent`: `--signer` removed. Consent starts as `convened`, requires manual vote + resolve
- `meetings`, `resolutions`, `seats`, `agenda-items`: require positional body/meeting ref (not flags)

### Cap Table
- `create-instrument`: `--instrument-type`/`--name` → `--kind`/`--symbol`, `--authorized-shares` → `--authorized-units`, `--par-value` → `--issue-price-cents`
- `issue-equity`: `--holder-id` → `--recipient`, new `--grant-type` required, `--price-per-share` removed, `--meeting-id` now required alongside `--resolution-id`
- `issue-safe`: `--holder-id` → `--investor`, `--invested-amount` → `--amount`, `--valuation-cap` in cents, `--meeting-id` required
- `issue-safe`: new `--safe-type` flag (default `post_money`, also accepts `pre_money`)
- `create-valuation`: `--valuation-type 409a` → `--type four_oh_nine_a`, `--enterprise-value` in cents, new `--fmv` (cents) required, new `--date` and `--methodology` required
- `cap-table overview` removed — use `cap-table --json` (no subcommand) for overview
- `start-round --issuer-legal-entity-id`: short IDs and `@last` don't work; must use full UUID from `cap-table --json` output
- `distribute`: `--type` accepts `dividend`, `return`, `liquidation`

### Finance
- `invoice`: `--to` → `--customer`, new `--due-date` required
- `pay`: `--invoice-id` removed, uses `--recipient` + `--amount`
- `payroll`: `--contact-id`/`--gross-pay` → `--period-start`/`--period-end`
- `open-account`: `--bank` → `--institution`
- `classify-contractor`: `--name` instead of positional arg, new flags `--state`, `--hours`, `--exclusive`, `--duration`, `--provides-tools`
- `reconcile`: new `--start-date` required

### Agents
- `create`: `--entity-id` and `--role` removed, new `--prompt` required
- No entity scoping on agent creation (agents are workspace-level now)

### Work Items
- `claim`: `--agent-name` → `--by` (alias `--claimer`)
- `complete`: new `--by` required, new `--result`/`--notes` for completion notes
- `create --created-by`: expects contact or agent name/ref, not literal "agent"
- New `--asap` flag for priority work items
- `claim` supports `--ttl` for time-limited claims

### Documents
- `contractor_agreement` and `custom` templates work
- `custom` template requires `--param` with at least one key=value pair
- `signing-link` returns full URL with token

### Tax
- `deadline` supports `--recurrence annual` flag

### Config
- `config list --json` not supported (but already outputs JSON by default)
- `resolve --json` not supported (but already outputs JSON by default)

### Contacts
- `add`: new `--type` (individual/organization), `--category`, `--cap-table-access`, `--phone`, `--notes` flags
- `edit`: takes positional `<contact-ref>`, same field flags as `add`
- Categories: `employee`, `contractor`, `board_member`, `investor`, `law_firm`, `valuation_firm`, `accounting_firm`, `officer`, `founder`, `member`, `other`
- Cap table access levels: `none`, `summary`, `detailed`

### Entities
- `dissolve`: `--effective-date` cannot be in the future (must be today or past)
- `convert`: supports `--jurisdiction` to change jurisdiction during conversion

### Services (NEW in v22)
- `services catalog` — list available services with pricing
- `services buy <slug>` — purchase a service
- `services list` — list service requests
- `services show <ref>` — show request detail
- `services fulfill <ref>` — mark fulfilled (operator)
- `services cancel <ref>` — cancel request

### Obligations (NEW in v22)
- `obligations` — list obligations with urgency tiers
- `obligations --tier <tier>` — filter by urgency tier

### Other New Commands
- `demo --minimal --name <name>` — `--name` now required for demo
- `chat` — interactive LLM chat session
- `link --external-id --provider` — link workspace to external provider
- `digest --trigger` — trigger digest generation

## Bugs Found

### 1. `documents generate --param` not reaching server (CONFIRMED)
- `--param department=Engineering` etc. accepted by CLI but server still returns "missing required fields"
- Affects `employment_offer` template which requires `department`, `dispute_resolution_terms`, `equity_grant_type`, `equity_shares`
- The `--param` key=value pairs are not being mapped to the document template fields
- Note: `--param` works fine for `custom` template — bug is specific to named templates with required fields

### 2. `preview-pdf` not available in process transport
- Returns: `getPreviewPdfUrl is not available in process transport mode`
- Expected for local server but error message could be more helpful

### 3. Bank account workflow gap
- `finance pay` requires active bank account but `open-account` creates in `pending_review` state
- No way to activate/approve a bank account via CLI

### 4. `cap-table transfer` broken
- Requires `--share-class-id` but `share-classes` returns `[]` — no share classes exist
- No `create-share-class` CLI command
- Transfer workflow is non-functional without manual share class creation

### 5. `services buy` broken
- Server returns: `unknown variant 'service_request', expected one of 'entity', 'contact', ...`
- The `service_request` entity type is not registered in the server's deserializer
- `--dry-run` works (shows correct payload) but actual execution fails

### 6. `demo` scenarios broken (except `--minimal`)
- `demo --scenario startup` — fails: missing `incorporator_address`
- `demo --scenario restaurant` — fails: missing `scheduled_date`
- `demo --scenario llc` — fails: missing `principal_name`
- `demo --minimal --name <name>` — PASS (creates minimal workspace)

### 7. `--member-json` and `--members-file` silently drop `membership_pct`
- When using JSON member formats, `membership_pct` field is accepted but not mapped to the payload
- Only `--member` key=value syntax with `pct=N` correctly includes ownership percentage
- Affects LLC formation where ownership percentages are required for operating agreement generation

### 8. `form create` uses different flag names than one-shot `form`
- Staged `form create` uses `--type`/`--name` while one-shot `form` uses `--entity-type`/`--legal-name`
- Inconsistent API surface — could confuse agents/users

### 9. `form create` dry-run drops some flags from payload
- `--fiscal-year-end`, `--s-corp`, `--transfer-restrictions`, `--rofr` are accepted by CLI
- But these flags don't appear in the `form create` dry-run payload (they DO appear in one-shot `form` dry-run)

### 10. `claim` (redeem code) not available in process transport
- Returns transport-related error when running against local server

### 11. `feedback` not available on local server
- Requires cloud API endpoint

## Commands Tested (All Pass Unless Noted)

| Category | Commands | Status |
|----------|----------|--------|
| System | context, status, config list/get/set, schema, resolve, find | PASS |
| Entities | list, show, dissolve, convert | PASS |
| Contacts | list, show, add, edit | PASS |
| Formation | form (one-shot LLC/C-Corp), form create, add-founder, finalize, activate | PASS |
| Formation | --member, --member-json, --members-file, --address, --s-corp, --rofr, --transfer-restrictions | PASS (but see bugs 7-9) |
| Documents | list, generate (nda, consulting_agreement, contractor_agreement, custom), signing-link | PASS |
| Documents | generate employment_offer | FAIL (bug 1) |
| Documents | preview-pdf | FAIL (bug 2, process transport only) |
| Governance | create-body, add-seat, seats, meetings, resolutions, agenda-items | PASS |
| Governance | convene, notice, open, vote, resolve, finalize-item, adjourn | PASS |
| Governance | written-consent, reopen, cancel | PASS |
| Cap Table | (default overview), instruments, share-classes, safes, transfers, rounds, valuations, 409a | PASS |
| Cap Table | create-instrument, issue-equity, issue-safe (post_money + pre_money) | PASS |
| Cap Table | create-valuation, submit-valuation, approve-valuation, distribute (dividend + liquidation) | PASS |
| Cap Table | start-round, add-security, issue-round (staged round flow) | PASS |
| Cap Table | transfer | FAIL (bug 4) |
| Finance | invoices, payments, bank-accounts, payroll-runs, classifications, reconciliations, distributions | PASS |
| Finance | invoice, payroll, classify-contractor, reconcile, open-account | PASS |
| Finance | pay | FAIL (bug 3) |
| Tax | filings, deadlines, file, deadline (with --recurrence) | PASS |
| Agents | create, show, skill, message, pause, resume, delete | PASS |
| Work Items | list (with --status/--category filters), show, create, claim (with --ttl), complete, release, cancel | PASS |
| Services | catalog, list | PASS |
| Services | buy | FAIL (bug 5) |
| Services | fulfill, show, cancel | NOT TESTED (no service requests can be created due to bug 5) |
| Obligations | list (with --tier filter) | PASS |
| Billing | (default status), portal, upgrade | PASS |
| Misc | approvals (redirects to governance), api-keys, digest (list + --trigger), link | PASS |
| Misc | demo --minimal | PASS |
| Misc | demo --scenario (startup/restaurant/llc) | FAIL (bug 6) |
| Misc | claim (redeem code) | FAIL (bug 10, process transport only) |
| Misc | feedback | FAIL (bug 11, local server only) |
| Misc | chat | NOT TESTED (interactive, requires TTY) |
| Misc | serve | PASS (used for all testing) |
| Misc | --dry-run (on form, issue-equity, convene, issue-safe, distribute, services buy) | PASS |

## Summary Statistics

- **Total unique commands tested**: ~95
- **Commands passing**: ~85
- **Commands failing (bugs)**: 8
- **Commands not testable**: 2 (chat, services fulfill/show/cancel)
- **New commands discovered**: services (catalog/buy/list/show/fulfill/cancel), obligations, chat, link, digest --trigger
- **Breaking changes documented**: 50+ flag/behavior changes from v21
