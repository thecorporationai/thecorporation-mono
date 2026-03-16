# CLI UX Overhaul Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix all identified UX issues in the `@thecorporation/cli` package — output noise, money input footguns, missing commands, safety confirmations, help text, and API coverage gaps for agentic consumers.

**Architecture:** Surgical edits to existing CLI files (index.ts, output.ts, command modules). New command files for missing API surface. New API client methods in corp-tools for endpoints not yet wrapped. All changes in `packages/cli-ts/` and `packages/corp-tools/`.

**Tech Stack:** TypeScript, Commander.js, Vitest (corp-tools tests), chalk, @inquirer/prompts

---

## Chunk 1: Fix Output Noise (P0)

The CLI dumps raw JSON in human-readable mode after formatted output. This confuses both humans and agents parsing stdout.

### Task 1: Fix `printWriteResult` to stop dumping JSON in human mode

**Files:**
- Modify: `packages/cli-ts/src/output.ts:89-112`

The bug: line 111 calls `printJson(result)` unconditionally after the success message when `jsonOnly` is false.

- [ ] **Step 1: Edit `printWriteResult` in output.ts**

In `packages/cli-ts/src/output.ts`, remove the unconditional `printJson(result)` on line 111. The function should only print JSON when `jsonOnly` is true (already handled on line 96).

```typescript
// BEFORE (lines 89-112):
export function printWriteResult(
  result: unknown,
  successMessage: string,
  options?: WriteResultOptions,
): void {
  const normalized = normalizeWriteResultOptions(options);
  if (normalized.jsonOnly) {
    printJson(result);
    return;
  }
  printSuccess(successMessage);
  if (
    normalized.referenceKind
    && typeof result === "object"
    && result !== null
    && !Array.isArray(result)
  ) {
    printReferenceSummary(normalized.referenceKind, result as ApiRecord, {
      label: normalized.referenceLabel,
      showReuseHint: normalized.showReuseHint,
    });
  }
  printJson(result);  // <-- REMOVE THIS LINE
}
```

- [ ] **Step 2: Build to verify no type errors**

Run: `cd packages/cli-ts && npm run build`
Expected: Clean build

- [ ] **Step 3: Commit**

```bash
git add packages/cli-ts/src/output.ts
git commit -m "fix(cli): stop dumping raw JSON in human-readable output mode

printWriteResult was calling printJson() after the formatted success
message, producing mixed human+JSON output that confused both users
and agents parsing stdout."
```

### Task 2: Remove explicit `printJson` calls after formatted output in governance commands

**Files:**
- Modify: `packages/cli-ts/src/commands/governance.ts`

Multiple governance commands print `printSuccess(...)` + `printReferenceSummary(...)` + `printJson(result)` when NOT in `--json` mode. Remove the trailing `printJson(result)` calls.

- [ ] **Step 1: Edit governance.ts — remove printJson in non-json code paths**

In `packages/cli-ts/src/commands/governance.ts`, remove `printJson(result)` from the non-json branches of these functions:

1. `governanceCreateBodyCommand` — remove `printJson(result)` after the "Next steps" block (around line 41)
2. `governanceAddSeatCommand` — remove `printJson(result)` after `printReferenceSummary` (around line 72)
3. `governanceConveneCommand` — remove `printJson(result)` after "Next steps" block (around line 165)
4. `governanceOpenMeetingCommand` — remove `printJson(result)` after `printSuccess` (around line 197)
5. `governanceVoteCommand` — remove `printJson(result)` after `printSuccess` (around line 228)
6. `sendNoticeCommand` — remove `printJson(result)` after `printSuccess` (around line 263)
7. `adjournMeetingCommand` — remove `printJson(result)` after `printSuccess` (around line 287)
8. `cancelMeetingCommand` — remove `printJson(result)` after `printSuccess` (around line 311)
9. `reopenMeetingCommand` — remove `printJson(result)` after `printSuccess` (around line 335)
10. `finalizeAgendaItemCommand` — remove `printJson(result)` after `printSuccess` (around line 364)
11. `computeResolutionCommand` — remove `printJson(result)` after `printReferenceSummary` (around line 401)
12. `writtenConsentCommand` — remove `printJson(result)` after "Next steps" block (around line 432)

Each of these follows the same pattern:
```typescript
// BEFORE:
if (opts.json) {
  printJson(result);
  return;
}
printSuccess(`...`);
printReferenceSummary("kind", result, { showReuseHint: true });
printJson(result);  // <-- REMOVE

// AFTER:
if (opts.json) {
  printJson(result);
  return;
}
printSuccess(`...`);
printReferenceSummary("kind", result, { showReuseHint: true });
```

- [ ] **Step 2: Build to verify**

Run: `cd packages/cli-ts && npm run build`
Expected: Clean build

- [ ] **Step 3: Commit**

```bash
git add packages/cli-ts/src/commands/governance.ts
git commit -m "fix(cli): remove raw JSON dumps from governance human output

All 12 governance write commands were printing formatted messages
followed by a full JSON dump. Now only --json mode produces JSON."
```

### Task 3: Remove explicit `printJson` calls from cap-table and form commands

**Files:**
- Modify: `packages/cli-ts/src/commands/cap-table.ts`
- Modify: `packages/cli-ts/src/commands/form.ts`

- [ ] **Step 1: Edit cap-table.ts**

Remove `printJson(result)` from the non-json branches of:
1. `issueEquityCommand` — line ~430 after `printReferenceSummary`
2. `issueSafeCommand` — line ~500 after `printReferenceSummary`
3. `issueRoundCommand` — line ~806 after `printReferenceSummary`
4. `submitValuationCommand` — line ~902 after the agenda item ref summary

- [ ] **Step 2: Edit form.ts**

Remove `printJson(formation)` from the non-json branch of `formActivateCommand` (line ~896, after the formatted steps output).

- [ ] **Step 3: Build to verify**

Run: `cd packages/cli-ts && npm run build`
Expected: Clean build

- [ ] **Step 4: Commit**

```bash
git add packages/cli-ts/src/commands/cap-table.ts packages/cli-ts/src/commands/form.ts
git commit -m "fix(cli): remove raw JSON dumps from cap-table and form human output"
```

---

## Chunk 2: Money Inputs & Required Options (P0)

### Task 4: Rename money flags to be explicit about cents

**Files:**
- Modify: `packages/cli-ts/src/index.ts` (flag definitions)

Change flag descriptions and names to make cents explicit. Update both the flag definitions in index.ts AND all references in command handlers (including TypeScript interfaces, dry-run blocks, and success messages).

- [ ] **Step 1: Edit index.ts — finance invoice command**

Find the `finance invoice` command and change:
```typescript
// BEFORE:
.requiredOption("--amount <n>", "Amount in cents", parseInt)

// AFTER:
.requiredOption("--amount-cents <n>", "Amount in cents (e.g. 500000 = $5,000.00)", parseInt)
```

- [ ] **Step 2: Edit index.ts — finance pay command**

```typescript
// BEFORE:
.requiredOption("--amount <n>", "Amount in cents", parseInt)

// AFTER:
.requiredOption("--amount-cents <n>", "Amount in cents (e.g. 500000 = $5,000.00)", parseInt)
```

- [ ] **Step 3: Edit index.ts — cap-table issue-safe command**

```typescript
// BEFORE:
.requiredOption("--amount <n>", "Principal amount in cents", parseInt)
.requiredOption("--valuation-cap <n>", "Valuation cap in cents", parseInt)

// AFTER:
.requiredOption("--amount-cents <n>", "Principal amount in cents (e.g. 5000000000 = $50M)", parseInt)
.requiredOption("--valuation-cap-cents <n>", "Valuation cap in cents (e.g. 1000000000 = $10M)", parseInt)
```

- [ ] **Step 4: Edit index.ts — cap-table distribute command**

```typescript
// BEFORE:
.requiredOption("--amount <n>", "Total distribution amount in cents", parseInt)

// AFTER:
.requiredOption("--amount-cents <n>", "Total distribution amount in cents (e.g. 100000 = $1,000.00)", parseInt)
```

- [ ] **Step 5: Update command handlers to match new flag names**

The Commander camelCase conversion will change `--amount-cents` to `opts.amountCents` and `--valuation-cap-cents` to `opts.valuationCapCents`. Update the command handler files to use the new property names:

In `packages/cli-ts/src/commands/finance.ts`:
- `financeInvoiceCommand`: change `opts.amount` → `opts.amountCents` (in the interface, API payload, and any success messages)
- `financePayCommand`: change `opts.amount` → `opts.amountCents` (in the interface, API payload, and any success messages)

In `packages/cli-ts/src/commands/cap-table.ts`:
- `issueSafeCommand`: change ALL occurrences:
  - TypeScript interface field: `amount: number` → `amountCents: number`
  - TypeScript interface field: `valuationCap: number` → `valuationCapCents: number`
  - Dry-run payload (~line 458): `opts.amount` → `opts.amountCents`
  - Dry-run payload (~line 460): `opts.valuationCap` → `opts.valuationCapCents`
  - API body (~line 484): `opts.amount` → `opts.amountCents`
  - API body (~line 485): `opts.valuationCap` → `opts.valuationCapCents`
  - Success message (~line 498): `opts.amount` → `opts.amountCents`
- `distributeCommand`: change `opts.amount` → `opts.amountCents` (in the interface, API payload, and any success messages)

**Important:** Keep `--amount` as a hidden alias for backwards compatibility. In the index.ts action blocks, merge the old name into the new name before passing to the handler:

```typescript
// In each action block that renamed --amount to --amount-cents, add:
const mergedOpts = {
  ...opts,
  amountCents: opts.amountCents ?? opts.amount,
};
```

For issue-safe, also add:
```typescript
valuationCapCents: opts.valuationCapCents ?? opts.valuationCap,
```

Then add a hidden alias after each renamed option in the command definition:
```typescript
.option("--amount <n>", "", parseInt) // hidden backwards compat
```
Commander hides options with empty descriptions from help.

- [ ] **Step 6: Build to verify**

Run: `cd packages/cli-ts && npm run build`

- [ ] **Step 7: Commit**

```bash
git add packages/cli-ts/src/index.ts packages/cli-ts/src/commands/finance.ts packages/cli-ts/src/commands/cap-table.ts
git commit -m "fix(cli): rename money flags to --amount-cents to prevent dollar/cent confusion

Users were passing dollar amounts where cents were expected, creating
orders of magnitude errors. Flags now explicitly say -cents with
examples. Old --amount flag kept as hidden alias for compatibility."
```

### Task 5: Fix required options not declared as required

**Files:**
- Modify: `packages/cli-ts/src/index.ts`

- [ ] **Step 1: Fix work-items create --category**

In `index.ts`, change the `work-items create` command definition:
```typescript
// BEFORE (line ~1244):
.option("--category <category>", "Work item category")

// AFTER:
.requiredOption("--category <category>", "Work item category")
```

Then in `packages/cli-ts/src/commands/work-items.ts`, remove the manual check at lines 83-86:
```typescript
// REMOVE:
if (!opts.category) {
  printError("Missing required option: --category <category>");
  process.exit(1);
}
```

- [ ] **Step 2: Fix work-items claim — keep --by as option but improve help text**

The `--claimer` alias means we can't use `requiredOption("--by")` — Commander would reject `--claimer` without `--by`. Instead, keep the manual check but improve the error message and mark the option description as required:

In `index.ts`, update the descriptions:
```typescript
// BEFORE:
.option("--by <name>", "Agent or user claiming the item")
.option("--claimer <name>", "Alias for --by")

// AFTER:
.option("--by <name>", "Agent or user claiming the item (required)")
.option("--claimer <name>", "Alias for --by")
```

Keep the existing manual check in the action handler as-is (it's correct).

- [ ] **Step 3: Fix work-items complete — same approach**

```typescript
// BEFORE:
.option("--by <name>", "Agent or user completing the item")
.option("--completed-by <name>", "Alias for --by")

// AFTER:
.option("--by <name>", "Agent or user completing the item (required)")
.option("--completed-by <name>", "Alias for --by")
```

- [ ] **Step 4: Build to verify**

Run: `cd packages/cli-ts && npm run build`

- [ ] **Step 5: Commit**

```bash
git add packages/cli-ts/src/index.ts packages/cli-ts/src/commands/work-items.ts
git commit -m "fix(cli): declare required options as requiredOption so --help is accurate

work-items create --category, claim --by, and complete --by were
optional in Commander but had runtime checks. Now Commander shows
them as required in help text."
```

---

## Chunk 3: UX Improvements (P1)

### Task 6: Add default action to `tax` and `services` parent commands

**Files:**
- Modify: `packages/cli-ts/src/index.ts`
- Modify: `packages/cli-ts/src/commands/tax.ts` (add `taxSummaryCommand` function)

- [ ] **Step 1: Add default action to tax command in index.ts**

Replace the `tax` command definition:
```typescript
// BEFORE:
const taxCmd = program
  .command("tax")
  .description("Tax filings and deadline tracking")
  .option("--entity-id <ref>", "Entity reference (ID, short ID, @last, or unique name)")
  .option("--json", "Output as JSON");

// AFTER:
const taxCmd = program
  .command("tax")
  .description("Tax filings and deadline tracking")
  .option("--entity-id <ref>", "Entity reference (ID, short ID, @last, or unique name)")
  .option("--json", "Output as JSON")
  .action(async (opts) => {
    const { taxSummaryCommand } = await import("./commands/tax.js");
    await taxSummaryCommand(opts);
  });
```

- [ ] **Step 2: Add `taxSummaryCommand` to tax.ts**

In `packages/cli-ts/src/commands/tax.ts`, add this function (import the necessary modules if not already imported):

```typescript
export async function taxSummaryCommand(opts: { entityId?: string; json?: boolean }): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const eid = await resolver.resolveEntity(opts.entityId);
    const [filings, deadlines] = await Promise.all([
      client.listTaxFilings(eid),
      client.listDeadlines(eid),
    ]);
    if (opts.json) {
      printJson({ filings, deadlines });
      return;
    }
    if (filings.length === 0 && deadlines.length === 0) {
      console.log("No tax filings or deadlines found.");
      return;
    }
    if (filings.length > 0) printTaxFilingsTable(filings);
    if (deadlines.length > 0) printDeadlinesTable(deadlines);
  } catch (err) {
    printError(`Failed to fetch tax summary: ${err}`);
    process.exit(1);
  }
}
```

Make sure `printTaxFilingsTable`, `printDeadlinesTable`, `ReferenceResolver` are imported at the top of the file.

- [ ] **Step 3: Add default action to services command in index.ts**

Replace the `services` command definition:
```typescript
// BEFORE:
const servicesCmd = program
  .command("services")
  .description("Service catalog and fulfillment")
  .option("--entity-id <ref>", "Entity reference (ID, short ID, @last, or unique name)")
  .option("--json", "Output as JSON");

// AFTER:
const servicesCmd = program
  .command("services")
  .description("Service catalog and fulfillment")
  .option("--entity-id <ref>", "Entity reference (ID, short ID, @last, or unique name)")
  .option("--json", "Output as JSON")
  .action(async (opts) => {
    const { servicesCatalogCommand } = await import("./commands/services.js");
    await servicesCatalogCommand({ json: opts.json });
  });
```

- [ ] **Step 4: Build and verify**

Run: `cd packages/cli-ts && npm run build`

- [ ] **Step 5: Commit**

```bash
git add packages/cli-ts/src/index.ts packages/cli-ts/src/commands/tax.ts
git commit -m "feat(cli): add default actions to tax and services parent commands

Running 'corp tax' now shows filings + deadlines summary.
Running 'corp services' now shows the service catalog.
Previously both showed Commander help text with no data."
```

### Task 7: Add `corp use <entity-ref>` command

**Files:**
- Modify: `packages/cli-ts/src/index.ts`
- Create: `packages/cli-ts/src/commands/use.ts`

- [ ] **Step 1: Add the command definition in index.ts**

After the `context` command block (~line 66), add:
```typescript
program
  .command("use <entity-ref>")
  .description("Set the active entity by name, short ID, or reference")
  .action(async (entityRef: string) => {
    const { useCommand } = await import("./commands/use.js");
    await useCommand(entityRef);
  });
```

- [ ] **Step 2: Create use.ts command handler**

Create `packages/cli-ts/src/commands/use.ts`:
```typescript
import { loadConfig, saveConfig, setActiveEntityId } from "../config.js";
import { CorpAPIClient } from "../api-client.js";
import { printError, printSuccess } from "../output.js";
import { ReferenceResolver, getReferenceAlias } from "../references.js";

export async function useCommand(entityRef: string): Promise<void> {
  const cfg = loadConfig();
  if (!cfg.api_url || !cfg.api_key || !cfg.workspace_id) {
    printError("Not configured. Run 'corp setup' first.");
    process.exit(1);
  }
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const entityId = await resolver.resolveEntity(entityRef);
    setActiveEntityId(cfg, entityId);
    saveConfig(cfg);
    const alias = getReferenceAlias("entity", { entity_id: entityId }) ?? entityId;
    printSuccess(`Active entity set to ${alias} (${entityId})`);
  } catch (err) {
    printError(`Failed to resolve entity: ${err}`);
    process.exit(1);
  }
}
```

- [ ] **Step 3: Build to verify**

Run: `cd packages/cli-ts && npm run build`

- [ ] **Step 4: Commit**

```bash
git add packages/cli-ts/src/index.ts packages/cli-ts/src/commands/use.ts
git commit -m "feat(cli): add 'corp use <entity-ref>' to set active entity

Replaces the verbose 'corp config set active_entity_id <uuid>' with
a simpler command that supports name/slug/shortID reference resolution."
```

### Task 8: Propagate `--entity-id` to subcommands

**Files:**
- Modify: `packages/cli-ts/src/index.ts`

The `cap-table`, `finance`, `governance`, `tax`, and `documents` subcommands don't accept `--entity-id` directly — it must come before the subcommand. Add `--entity-id` to each subcommand that needs it.

- [ ] **Step 1: Add --entity-id to cap-table subcommands**

For each of these cap-table subcommands, add `.option("--entity-id <ref>", "Entity reference")` and update the action to merge:

```
safes, transfers, instruments, share-classes, rounds, valuations, 409a
```

Example for `safes`:
```typescript
// BEFORE:
capTableCmd.command("safes").description("SAFE notes").action(async (_opts, cmd) => {
  const parent = cmd.parent!.opts();
  const { safesCommand } = await import("./commands/cap-table.js");
  await safesCommand(parent);
});

// AFTER:
capTableCmd.command("safes")
  .option("--entity-id <ref>", "Entity reference")
  .option("--json", "Output as JSON")
  .description("SAFE notes")
  .action(async (opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { safesCommand } = await import("./commands/cap-table.js");
    await safesCommand({
      entityId: opts.entityId ?? parent.entityId,
      json: inheritOption(opts.json, parent.json),
    });
  });
```

Apply the same pattern to: `transfers`, `instruments`, `share-classes`, `rounds`, `valuations`, `409a`.

- [ ] **Step 2: Build to verify**

Run: `cd packages/cli-ts && npm run build`

- [ ] **Step 3: Commit**

```bash
git add packages/cli-ts/src/index.ts
git commit -m "fix(cli): allow --entity-id after subcommand name

Users can now write 'corp cap-table safes --entity-id myco' instead
of being forced to use 'corp cap-table --entity-id myco safes'."
```

### Task 9: Add confirmation prompts for destructive operations

**Files:**
- Modify: `packages/cli-ts/src/commands/entities.ts`
- Modify: `packages/cli-ts/src/commands/agents.ts`
- Modify: `packages/cli-ts/src/commands/work-items.ts`
- Modify: `packages/cli-ts/src/commands/governance.ts`
- Modify: `packages/cli-ts/src/index.ts`

- [ ] **Step 1: Add --yes flag to destructive commands in index.ts**

Add `.option("--yes, -y", "Skip confirmation prompt")` to:
- `entities dissolve`
- `agents delete`
- `work-items cancel`
- `governance cancel`

- [ ] **Step 2: Add confirmation to entitiesDissolveCommand**

In `packages/cli-ts/src/commands/entities.ts`, add:
```typescript
import { confirm } from "@inquirer/prompts";
```

In `entitiesDissolveCommand`, before the API call, add:
```typescript
if (!opts.yes) {
  const ok = await confirm({
    message: `Dissolve entity ${entityId}? This cannot be undone.`,
    default: false,
  });
  if (!ok) {
    console.log("Cancelled.");
    return;
  }
}
```

- [ ] **Step 3: Add confirmation to agentsDeleteCommand**

In `packages/cli-ts/src/commands/agents.ts`, first update the function signature to accept `yes`:
```typescript
// BEFORE:
export async function agentsDeleteCommand(agentId: string, opts: { json?: boolean }): Promise<void> {
// AFTER:
export async function agentsDeleteCommand(agentId: string, opts: { json?: boolean; yes?: boolean }): Promise<void> {
```

Then add `confirm` import and the check before the delete call:
```typescript
if (!opts.yes) {
  const ok = await confirm({
    message: `Delete agent ${resolvedAgentId}? This cannot be undone.`,
    default: false,
  });
  if (!ok) {
    console.log("Cancelled.");
    return;
  }
}
```

- [ ] **Step 4: Add confirmation to workItemsCancelCommand**

In `packages/cli-ts/src/commands/work-items.ts`:
```typescript
if (!opts.yes) {
  const ok = await confirm({
    message: `Cancel work item ${resolvedWorkItemId}?`,
    default: false,
  });
  if (!ok) {
    console.log("Cancelled.");
    return;
  }
}
```

- [ ] **Step 5: Add confirmation to cancelMeetingCommand**

In `packages/cli-ts/src/commands/governance.ts`:
```typescript
if (!opts.yes) {
  const ok = await confirm({
    message: `Cancel meeting ${resolvedMeetingId}?`,
    default: false,
  });
  if (!ok) {
    console.log("Cancelled.");
    return;
  }
}
```

- [ ] **Step 6: Build to verify**

Run: `cd packages/cli-ts && npm run build`

- [ ] **Step 7: Commit**

```bash
git add packages/cli-ts/src/index.ts packages/cli-ts/src/commands/entities.ts packages/cli-ts/src/commands/agents.ts packages/cli-ts/src/commands/work-items.ts packages/cli-ts/src/commands/governance.ts
git commit -m "feat(cli): add confirmation prompts for destructive operations

Entity dissolution, agent deletion, work item cancellation, and
meeting cancellation now prompt for confirmation. Use --yes/-y to
skip in scripts."
```

### Task 10: Fix approvals command to show useful information

**Files:**
- Modify: `packages/cli-ts/src/commands/approvals.ts`

- [ ] **Step 1: Replace the approvals command implementation**

The current implementation is a dead end. Replace it with a redirect that explains how approvals work:

```typescript
import chalk from "chalk";

export async function approvalsListCommand(_opts: Record<string, unknown>): Promise<void> {
  console.log(chalk.bold("Approvals in TheCorporation"));
  console.log();
  console.log("Approvals are handled through governance meetings and execution intents.");
  console.log("Use these commands to manage approvals:");
  console.log();
  console.log(chalk.dim("  Board approval via meeting vote:"));
  console.log(`    corp governance convene --body <body> --type board_meeting --title "Approve X"`);
  console.log(`    corp governance vote <meeting> <item> --voter <contact> --vote for`);
  console.log();
  console.log(chalk.dim("  Written consent (no meeting needed):"));
  console.log(`    corp governance written-consent --body <body> --title "Approve X" --description "..."`);
  console.log();
  console.log(chalk.dim("  View pending items:"));
  console.log(`    corp governance meetings <body>        # see scheduled meetings`);
  console.log(`    corp governance agenda-items <meeting>  # see items awaiting votes`);
  console.log(`    corp cap-table valuations               # see pending valuations`);
}
```

- [ ] **Step 2: Build to verify**

Run: `cd packages/cli-ts && npm run build`

- [ ] **Step 3: Commit**

```bash
git add packages/cli-ts/src/commands/approvals.ts
git commit -m "fix(cli): make approvals command show actionable guidance

Instead of a dead end, now explains how approvals work through
governance meetings and written consent, with copy-pasteable commands."
```

---

## Chunk 4: Polish (P2)

### Task 11: Add `--quiet` output mode for scripting

**Files:**
- Modify: `packages/cli-ts/src/output.ts`
- Modify: `packages/cli-ts/src/index.ts`

- [ ] **Step 1: Add global --quiet flag to program**

In `index.ts`, after `program.version(pkg.version)`, add:
```typescript
program.option("-q, --quiet", "Only output the resource ID (for scripting)");
```

- [ ] **Step 2: Add `printQuietId` function to output.ts**

```typescript
export function printQuietId(record: unknown, ...idFields: string[]): void {
  if (typeof record !== "object" || record === null) return;
  const rec = record as Record<string, unknown>;
  for (const field of idFields) {
    if (typeof rec[field] === "string" && rec[field]) {
      console.log(rec[field]);
      return;
    }
  }
}
```

- [ ] **Step 3: Update `printWriteResult` to support quiet mode**

Add `quiet` to `WriteResultOptions`:
```typescript
type WriteResultOptions =
  | boolean
  | {
      jsonOnly?: boolean;
      quiet?: boolean;
      referenceKind?: ResourceKind;
      referenceLabel?: string;
      showReuseHint?: boolean;
      idFields?: string[];
    };
```

In `printWriteResult`, add quiet handling before the success message:
```typescript
if (normalized.quiet) {
  // Try common domain ID field patterns, then fall back to caller-specified fields
  const defaultIdFields = [
    "entity_id", "agent_id", "meeting_id", "body_id", "seat_id",
    "work_item_id", "document_id", "invoice_id", "payment_id",
    "safe_note_id", "valuation_id", "round_id", "instrument_id",
    "transfer_workflow_id", "distribution_id", "deadline_id",
    "filing_id", "bank_account_id", "classification_id",
    "resolution_id", "agenda_item_id", "contact_id",
    "request_id", "service_request_id", "key_id",
    "formation_id", "execution_id", "incident_id",
    "id",
  ];
  printQuietId(result, ...(normalized.idFields ?? defaultIdFields));
  return;
}
```

- [ ] **Step 4: Build to verify**

Run: `cd packages/cli-ts && npm run build`

- [ ] **Step 5: Commit**

```bash
git add packages/cli-ts/src/output.ts packages/cli-ts/src/index.ts
git commit -m "feat(cli): add --quiet flag for script-friendly output

Scripts and agents can now capture just the resource ID:
  ENTITY_ID=\$(corp form create --type llc --name 'My LLC' --quiet)"
```

### Task 12: Clean up duplicate tax document types in help

**Files:**
- Modify: `packages/cli-ts/src/index.ts`

- [ ] **Step 1: Split into canonical and alias arrays**

Replace the `TAX_DOCUMENT_TYPE_CHOICES` at the top of index.ts:
```typescript
// BEFORE:
const TAX_DOCUMENT_TYPE_CHOICES = [
  "1120", "1120s", "1065", "franchise_tax", "annual_report", "83b",
  "form_1120", "form_1120s", "form_1065",
  "1099_nec", "form_1099_nec",
  "k1", "form_k1",
  "941", "form_941",
  "w2", "form_w2",
] as const;

// AFTER:
const TAX_DOCUMENT_TYPE_DISPLAY = [
  "1120", "1120s", "1065", "franchise_tax", "annual_report", "83b",
  "1099_nec", "k1", "941", "w2",
] as const;
const TAX_DOCUMENT_TYPE_ALIASES: Record<string, string> = {
  form_1120: "1120", form_1120s: "1120s", form_1065: "1065",
  form_1099_nec: "1099_nec", form_k1: "k1", form_941: "941", form_w2: "w2",
};
const TAX_DOCUMENT_TYPE_CHOICES = [
  ...TAX_DOCUMENT_TYPE_DISPLAY,
  ...Object.keys(TAX_DOCUMENT_TYPE_ALIASES),
] as const;
```

Then in the `tax file` command, update the choices to show only the display set:
```typescript
.addOption(
  new Option("--type <type>", `Document type (${TAX_DOCUMENT_TYPE_DISPLAY.join(", ")})`)
    .choices([...TAX_DOCUMENT_TYPE_CHOICES])
    .makeOptionMandatory(),
)
```

In the `taxFileCommand` handler in `packages/cli-ts/src/commands/tax.ts`, normalize the alias:
```typescript
const docType = TAX_DOCUMENT_TYPE_ALIASES[opts.type] ?? opts.type;
```

You'll need to export `TAX_DOCUMENT_TYPE_ALIASES` from index.ts or move it to a shared location. Simplest: just do the normalization inline in the tax.ts handler.

- [ ] **Step 2: Build to verify**

Run: `cd packages/cli-ts && npm run build`

- [ ] **Step 3: Commit**

```bash
git add packages/cli-ts/src/index.ts packages/cli-ts/src/commands/tax.ts
git commit -m "fix(cli): show canonical tax doc types in help, accept form_ aliases

Help text now shows '1120, 1120s, 1065, ...' instead of all 16
variants. Both '1120' and 'form_1120' are still accepted."
```

### Task 13: Add usage examples to major command groups

**Files:**
- Modify: `packages/cli-ts/src/index.ts`

- [ ] **Step 1: Add examples using Commander's addHelpText**

After each major command group definition, add examples. Use `addHelpText('after', ...)` on the parent commands:

```typescript
// After form command definition (~line 1468):
formCmd.addHelpText("after", `
Examples:
  $ corp form --type llc --name "My LLC" --member "Alice,alice@co.com,member,100"
  $ corp form --type c_corp --name "Acme Inc" --jurisdiction US-DE --member-json '{"name":"Bob","email":"bob@acme.com","role":"director","pct":100}'
  $ corp form create --type llc --name "My LLC"
  $ corp form add-founder @last:entity --name "Alice" --email "alice@co.com" --role member --pct 100
  $ corp form finalize @last:entity
  $ corp form activate @last:entity
`);

// After governance command definition (~line 985):
governanceCmd.addHelpText("after", `
Examples:
  $ corp governance create-body --name "Board of Directors" --body-type board_of_directors
  $ corp governance add-seat @last:body --holder "alice"
  $ corp governance convene --body board --type board_meeting --title "Q1 Review" --agenda "Approve budget"
  $ corp governance open @last:meeting --present-seat alice-seat
  $ corp governance vote @last:meeting <item-ref> --voter alice --vote for
  $ corp governance written-consent --body board --title "Approve Option Plan" --description "Board approves 2026 option plan"
`);

// After cap-table command definition (~line 536):
capTableCmd.addHelpText("after", `
Examples:
  $ corp cap-table                                    # view full cap table
  $ corp cap-table issue-equity --grant-type common --shares 1000000 --recipient "Alice Smith"
  $ corp cap-table issue-safe --investor "Seed Fund" --amount-cents 50000000 --valuation-cap-cents 1000000000
  $ corp cap-table create-valuation --type four_oh_nine_a --date 2026-01-01 --methodology market
  $ corp cap-table transfer --from alice --to bob --shares 1000 --share-class-id COMMON --governing-doc-type bylaws --transferee-rights full_member
`);

// After finance command definition (~line 747):
financeCmd.addHelpText("after", `
Examples:
  $ corp finance                                      # financial summary
  $ corp finance invoice --customer "Client Co" --amount-cents 500000 --due-date 2026-04-01
  $ corp finance pay --amount-cents 250000 --recipient "Vendor" --method ach
  $ corp finance payroll --period-start 2026-03-01 --period-end 2026-03-15
  $ corp finance open-account --institution Mercury
`);

// After agents command definition (~line 1214):
agentsCmd.addHelpText("after", `
Examples:
  $ corp agents                                       # list all agents
  $ corp agents create --name "bookkeeper" --prompt "You manage accounts payable"
  $ corp agents message @last:agent --body "Process this month's invoices"
  $ corp agents skill @last:agent --name invoice-processing --description "Process AP invoices"
`);

// After work-items command definition (~line 1329):
workItemsCmd.addHelpText("after", `
Examples:
  $ corp work-items                                   # list open work items
  $ corp work-items create --title "File Q1 taxes" --category compliance --deadline 2026-04-15
  $ corp work-items claim @last:work_item --by bookkeeper-agent
  $ corp work-items complete @last:work_item --by bookkeeper-agent --result "Filed 1120 for Q1"
`);
```

- [ ] **Step 2: Build to verify**

Run: `cd packages/cli-ts && npm run build`

- [ ] **Step 3: Commit**

```bash
git add packages/cli-ts/src/index.ts
git commit -m "feat(cli): add usage examples to major command groups

Running 'corp form --help', 'corp governance --help', etc. now shows
practical examples with real flag values."
```

### Task 14: Improve error message parsing for API errors

**Files:**
- Modify: `packages/corp-tools/src/api-client.ts`

- [ ] **Step 1: Improve the error handling in the API client's fetch wrapper**

Find the error handling logic in `packages/corp-tools/src/api-client.ts` where non-OK responses are thrown. The current pattern likely throws the raw status. Improve it to parse the response body:

Look for the `handleResponse` or similar method. Update it to:
```typescript
// In the response handler, parse error bodies:
if (!response.ok) {
  let detail: string;
  try {
    const body = await response.json() as Record<string, unknown>;
    detail = typeof body.error === "string"
      ? body.error
      : typeof body.message === "string"
        ? body.message
        : JSON.stringify(body);
  } catch {
    detail = await response.text().catch(() => response.statusText);
  }
  if (response.status === 401) {
    throw new SessionExpiredError(detail);
  }
  const prefix = response.status >= 500
    ? "Server error"
    : response.status === 404
      ? "Not found"
      : response.status === 422
        ? "Validation error"
        : `HTTP ${response.status}`;
  throw new Error(`${prefix}: ${detail}`);
}
```

**Note:** Read the actual file first to find the exact error handling code and adapt this pattern to match the existing structure. Do not break the existing `SessionExpiredError` handling.

- [ ] **Step 2: Run corp-tools tests**

Run: `cd packages/corp-tools && npm test`
Expected: All unit tests pass. Update mocks if needed.

- [ ] **Step 3: Commit**

```bash
git add packages/corp-tools/src/api-client.ts
git commit -m "fix(cli): show structured API error messages instead of raw status codes

API errors now show 'Validation error: field X is required' instead of
'Error: 422'. Network errors still show the fetch failure message."
```

---

## Chunk 5: API Coverage — New Commands for Agentic Systems (P1)

### Task 15: Add API key create, revoke, and rotate commands

**Files:**
- Modify: `packages/corp-tools/src/api-client.ts` (add missing methods)
- Modify: `packages/cli-ts/src/index.ts` (add subcommands)
- Create: `packages/cli-ts/src/commands/api-keys.ts` (replace existing)

- [ ] **Step 1: Add API client methods to corp-tools**

In `packages/corp-tools/src/api-client.ts`, add these methods to `CorpAPIClient`:

```typescript
async createApiKey(data: ApiRecord): Promise<ApiRecord> {
  return this.post("/v1/api-keys", data);
}

async revokeApiKey(keyId: string): Promise<void> {
  return this.del(`/v1/api-keys/${keyId}`);
}

async rotateApiKey(keyId: string): Promise<ApiRecord> {
  return this.post(`/v1/api-keys/${keyId}/rotate`, {});
}
```

**Note:** Read the file to find the exact method patterns (e.g. how `post`, `delete`, `get` are implemented) and match them.

- [ ] **Step 2: Convert api-keys to a command group in index.ts**

```typescript
// BEFORE:
program
  .command("api-keys")
  .description("List API keys")
  .option("--json", "Output as JSON")
  .action(async (opts) => {
    const { apiKeysCommand } = await import("./commands/api-keys.js");
    await apiKeysCommand(opts);
  });

// AFTER:
const apiKeysCmd = program
  .command("api-keys")
  .description("API key management")
  .option("--json", "Output as JSON")
  .action(async (opts) => {
    const { apiKeysListCommand } = await import("./commands/api-keys.js");
    await apiKeysListCommand(opts);
  });
apiKeysCmd
  .command("create")
  .requiredOption("--name <name>", "Key name/label")
  .option("--scopes <scopes>", "Comma-separated scopes (e.g. formation:read,equity:write)")
  .option("--json", "Output as JSON")
  .description("Create a new API key")
  .action(async (opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { apiKeysCreateCommand } = await import("./commands/api-keys.js");
    await apiKeysCreateCommand({
      ...opts,
      json: inheritOption(opts.json, parent.json),
    });
  });
apiKeysCmd
  .command("revoke <key-id>")
  .option("--yes", "Skip confirmation")
  .option("--json", "Output as JSON")
  .description("Revoke an API key")
  .action(async (keyId: string, opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { apiKeysRevokeCommand } = await import("./commands/api-keys.js");
    await apiKeysRevokeCommand(keyId, {
      ...opts,
      json: inheritOption(opts.json, parent.json),
    });
  });
apiKeysCmd
  .command("rotate <key-id>")
  .option("--json", "Output as JSON")
  .description("Rotate an API key (returns new key)")
  .action(async (keyId: string, opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { apiKeysRotateCommand } = await import("./commands/api-keys.js");
    await apiKeysRotateCommand(keyId, {
      ...opts,
      json: inheritOption(opts.json, parent.json),
    });
  });
```

- [ ] **Step 3: Implement api-keys.ts commands**

Replace `packages/cli-ts/src/commands/api-keys.ts`:

```typescript
import { confirm } from "@inquirer/prompts";
import { requireConfig } from "../config.js";
import { CorpAPIClient } from "../api-client.js";
import { printError, printJson, printSuccess, printWriteResult } from "../output.js";

export async function apiKeysListCommand(opts: { json?: boolean }): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    const keys = await client.listApiKeys();
    if (opts.json) { printJson(keys); return; }
    if (keys.length === 0) { console.log("No API keys found."); return; }
    for (const k of keys) {
      const name = k.name ?? k.label ?? "unnamed";
      const id = k.key_id ?? k.id;
      const scopes = Array.isArray(k.scopes) ? (k.scopes as string[]).join(", ") : "all";
      console.log(`  ${name} [${id}] scopes: ${scopes}`);
    }
  } catch (err) { printError(`Failed to list API keys: ${err}`); process.exit(1); }
}

export async function apiKeysCreateCommand(opts: {
  name: string; scopes?: string; json?: boolean;
}): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    const data: Record<string, unknown> = { name: opts.name };
    if (opts.scopes) data.scopes = opts.scopes.split(",").map((s) => s.trim());
    const result = await client.createApiKey(data);
    printWriteResult(result, `API key created: ${result.key_id ?? "OK"}`, opts.json);
    if (!opts.json && result.api_key) {
      printSuccess(`Key: ${result.api_key}`);
      console.log("  Save this key — it will not be shown again.");
    }
  } catch (err) { printError(`Failed to create API key: ${err}`); process.exit(1); }
}

export async function apiKeysRevokeCommand(keyId: string, opts: {
  yes?: boolean; json?: boolean;
}): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    if (!opts.yes) {
      const ok = await confirm({ message: `Revoke API key ${keyId}? This cannot be undone.`, default: false });
      if (!ok) { console.log("Cancelled."); return; }
    }
    await client.revokeApiKey(keyId);
    if (opts.json) { printJson({ revoked: true, key_id: keyId }); return; }
    printSuccess(`API key ${keyId} revoked.`);
  } catch (err) { printError(`Failed to revoke API key: ${err}`); process.exit(1); }
}

export async function apiKeysRotateCommand(keyId: string, opts: {
  json?: boolean;
}): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    const result = await client.rotateApiKey(keyId);
    printWriteResult(result, `API key ${keyId} rotated.`, opts.json);
    if (!opts.json && result.api_key) {
      printSuccess(`New key: ${result.api_key}`);
      console.log("  Save this key — it will not be shown again.");
    }
  } catch (err) { printError(`Failed to rotate API key: ${err}`); process.exit(1); }
}
```

- [ ] **Step 4: Build both packages**

Run: `cd packages/corp-tools && npm run build && cd ../cli-ts && npm run build`

- [ ] **Step 5: Commit**

```bash
git add packages/corp-tools/src/api-client.ts packages/cli-ts/src/index.ts packages/cli-ts/src/commands/api-keys.ts
git commit -m "feat(cli): add api-keys create, revoke, and rotate commands

Agents can now provision scoped API keys for sub-agents, revoke
compromised keys, and rotate keys — all from the CLI."
```

### Task 16: Add agent execution status commands

**Files:**
- Modify: `packages/corp-tools/src/api-client.ts`
- Modify: `packages/cli-ts/src/index.ts`
- Modify: `packages/cli-ts/src/commands/agents.ts`

- [ ] **Step 1: Add API client methods**

In `packages/corp-tools/src/api-client.ts`:
```typescript
async getAgentExecution(agentId: string, executionId: string): Promise<ApiRecord> {
  return this.get(`/v1/agents/${agentId}/executions/${executionId}`);
}

async getAgentExecutionResult(agentId: string, executionId: string): Promise<ApiRecord> {
  return this.get(`/v1/agents/${agentId}/executions/${executionId}/result`);
}

async getAgentExecutionLogs(agentId: string, executionId: string): Promise<ApiRecord> {
  return this.get(`/v1/agents/${agentId}/executions/${executionId}/logs`);
}

async killAgentExecution(agentId: string, executionId: string): Promise<ApiRecord> {
  return this.post(`/v1/agents/${agentId}/executions/${executionId}/kill`, {});
}
```

- [ ] **Step 2: Add CLI commands in index.ts**

After the existing `agents skill` command, add:
```typescript
agentsCmd
  .command("execution <agent-ref> <execution-id>")
  .option("--json", "Output as JSON")
  .description("Check execution status")
  .action(async (agentId: string, executionId: string, opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { agentsExecutionCommand } = await import("./commands/agents.js");
    await agentsExecutionCommand(agentId, executionId, {
      json: inheritOption(opts.json, parent.json),
    });
  });
agentsCmd
  .command("execution-result <agent-ref> <execution-id>")
  .option("--json", "Output as JSON")
  .description("Get execution result")
  .action(async (agentId: string, executionId: string, opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { agentsExecutionResultCommand } = await import("./commands/agents.js");
    await agentsExecutionResultCommand(agentId, executionId, {
      json: inheritOption(opts.json, parent.json),
    });
  });
agentsCmd
  .command("kill <agent-ref> <execution-id>")
  .option("--yes", "Skip confirmation")
  .option("--json", "Output as JSON")
  .description("Kill a running execution")
  .action(async (agentId: string, executionId: string, opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { agentsKillCommand } = await import("./commands/agents.js");
    await agentsKillCommand(agentId, executionId, {
      ...opts,
      json: inheritOption(opts.json, parent.json),
    });
  });
```

- [ ] **Step 3: Implement the handlers in agents.ts**

Add to `packages/cli-ts/src/commands/agents.ts`:

```typescript
export async function agentsExecutionCommand(
  agentId: string,
  executionId: string,
  opts: { json?: boolean },
): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const resolvedAgentId = await resolver.resolveAgent(agentId);
    const result = await client.getAgentExecution(resolvedAgentId, executionId);
    if (opts.json) { printJson(result); return; }
    console.log(chalk.magenta("─".repeat(40)));
    console.log(chalk.magenta.bold("  Execution Status"));
    console.log(chalk.magenta("─".repeat(40)));
    console.log(`  ${chalk.bold("Execution:")} ${executionId}`);
    console.log(`  ${chalk.bold("Agent:")} ${resolvedAgentId}`);
    console.log(`  ${chalk.bold("Status:")} ${result.status ?? "N/A"}`);
    if (result.started_at) console.log(`  ${chalk.bold("Started:")} ${result.started_at}`);
    if (result.completed_at) console.log(`  ${chalk.bold("Completed:")} ${result.completed_at}`);
    console.log(chalk.magenta("─".repeat(40)));
  } catch (err) { printError(`Failed to get execution: ${err}`); process.exit(1); }
}

export async function agentsExecutionResultCommand(
  agentId: string,
  executionId: string,
  opts: { json?: boolean },
): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const resolvedAgentId = await resolver.resolveAgent(agentId);
    const result = await client.getAgentExecutionResult(resolvedAgentId, executionId);
    if (opts.json) { printJson(result); return; }
    // Execution results are typically complex data — default to JSON even in human mode
    printSuccess(`Result for execution ${executionId}:`);
    printJson(result);
  } catch (err) { printError(`Failed to get execution result: ${err}`); process.exit(1); }
}

export async function agentsKillCommand(
  agentId: string,
  executionId: string,
  opts: { yes?: boolean; json?: boolean },
): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const resolvedAgentId = await resolver.resolveAgent(agentId);
    if (!opts.yes) {
      const { confirm } = await import("@inquirer/prompts");
      const ok = await confirm({ message: `Kill execution ${executionId}?`, default: false });
      if (!ok) { console.log("Cancelled."); return; }
    }
    const result = await client.killAgentExecution(resolvedAgentId, executionId);
    printWriteResult(result, `Execution ${executionId} killed.`, opts.json);
  } catch (err) { printError(`Failed to kill execution: ${err}`); process.exit(1); }
}
```

Also remove the old `agentsExecutionsCommand` function that just printed an error.

- [ ] **Step 4: Build both packages**

Run: `cd packages/corp-tools && npm run build && cd ../cli-ts && npm run build`

- [ ] **Step 5: Commit**

```bash
git add packages/corp-tools/src/api-client.ts packages/cli-ts/src/index.ts packages/cli-ts/src/commands/agents.ts
git commit -m "feat(cli): add agent execution status, result, and kill commands

Agents can now check execution status, retrieve results, and kill
running executions from the CLI."
```

### Task 17: Add governance mode, seat resignation, and incidents commands

**Files:**
- Modify: `packages/corp-tools/src/api-client.ts`
- Modify: `packages/cli-ts/src/index.ts`
- Modify: `packages/cli-ts/src/commands/governance.ts`

- [ ] **Step 1: Add API client methods**

In `packages/corp-tools/src/api-client.ts`:
```typescript
async getGovernanceMode(entityId: string): Promise<ApiRecord> {
  return this.get(`/v1/governance/mode?entity_id=${entityId}`);
}

async setGovernanceMode(data: ApiRecord): Promise<ApiRecord> {
  return this.post("/v1/governance/mode", data);
}

async resignSeat(seatId: string, entityId: string): Promise<ApiRecord> {
  return this.post(`/v1/governance-seats/${seatId}/resign`, { entity_id: entityId });
}

async createGovernanceIncident(data: ApiRecord): Promise<ApiRecord> {
  return this.post("/v1/governance/incidents", data);
}

async listGovernanceIncidents(entityId: string): Promise<ApiRecord[]> {
  return this.get(`/v1/entities/${entityId}/governance/incidents`);
}

async resolveGovernanceIncident(incidentId: string, data: ApiRecord): Promise<ApiRecord> {
  return this.post(`/v1/governance/incidents/${incidentId}/resolve`, data);
}

async getGovernanceProfile(entityId: string): Promise<ApiRecord> {
  return this.get(`/v1/entities/${entityId}/governance/profile`);
}
```

- [ ] **Step 2: Add CLI commands in index.ts**

After the existing governance commands, add:
```typescript
governanceCmd
  .command("mode")
  .addOption(new Option("--set <mode>", "Set governance mode").choices(["founder", "board", "executive", "normal", "incident_lockdown"]))
  .option("--json", "Output as JSON")
  .description("View or set governance mode")
  .action(async (opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { governanceModeCommand } = await import("./commands/governance.js");
    await governanceModeCommand({
      ...opts,
      entityId: parent.entityId,
      json: inheritOption(opts.json, parent.json),
    });
  });
governanceCmd
  .command("resign <seat-ref>")
  .option("--body-id <ref>", "Governance body reference")
  .option("--json", "Output as JSON")
  .description("Resign from a governance seat")
  .action(async (seatRef: string, opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { governanceResignCommand } = await import("./commands/governance.js");
    await governanceResignCommand(seatRef, {
      ...opts,
      entityId: parent.entityId,
      json: inheritOption(opts.json, parent.json),
    });
  });
governanceCmd
  .command("incidents")
  .option("--json", "Output as JSON")
  .description("List governance incidents")
  .action(async (opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { governanceIncidentsCommand } = await import("./commands/governance.js");
    await governanceIncidentsCommand({
      ...opts,
      entityId: parent.entityId,
      json: inheritOption(opts.json, parent.json),
    });
  });
governanceCmd
  .command("profile")
  .option("--json", "Output as JSON")
  .description("View governance profile and configuration")
  .action(async (opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { governanceProfileCommand } = await import("./commands/governance.js");
    await governanceProfileCommand({
      ...opts,
      entityId: parent.entityId,
      json: inheritOption(opts.json, parent.json),
    });
  });
```

- [ ] **Step 3: Implement handlers in governance.ts**

Add to `packages/cli-ts/src/commands/governance.ts`:

```typescript
export async function governanceModeCommand(opts: {
  entityId?: string; set?: string; json?: boolean;
}): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const eid = await resolver.resolveEntity(opts.entityId);
    if (opts.set) {
      const result = await client.setGovernanceMode({ entity_id: eid, mode: opts.set });
      if (opts.json) { printJson(result); return; }
      printSuccess(`Governance mode set to: ${opts.set}`);
    } else {
      const result = await client.getGovernanceMode(eid);
      if (opts.json) { printJson(result); return; }
      console.log(`  ${chalk.bold("Governance Mode:")} ${result.mode ?? "N/A"}`);
      if (result.reason) console.log(`  ${chalk.bold("Reason:")} ${result.reason}`);
    }
  } catch (err) { printError(`Failed: ${err}`); process.exit(1); }
}

export async function governanceResignCommand(seatRef: string, opts: {
  entityId?: string; bodyId?: string; json?: boolean;
}): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const eid = await resolver.resolveEntity(opts.entityId);
    const seatId = await resolver.resolveSeat(eid, seatRef, opts.bodyId);
    const result = await client.resignSeat(seatId, eid);
    if (opts.json) { printJson(result); return; }
    printSuccess(`Seat ${seatId} resigned.`);
  } catch (err) { printError(`Failed to resign seat: ${err}`); process.exit(1); }
}

export async function governanceIncidentsCommand(opts: {
  entityId?: string; json?: boolean;
}): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const eid = await resolver.resolveEntity(opts.entityId);
    const incidents = await client.listGovernanceIncidents(eid);
    if (opts.json) { printJson(incidents); return; }
    if (incidents.length === 0) { console.log("No governance incidents found."); return; }
    for (const inc of incidents) {
      const status = String(inc.status ?? "open");
      const colored = status === "resolved" ? chalk.green(status) : chalk.red(status);
      console.log(`  [${colored}] ${inc.incident_type ?? "unknown"}: ${inc.description ?? inc.id}`);
    }
  } catch (err) { printError(`Failed to list incidents: ${err}`); process.exit(1); }
}

export async function governanceProfileCommand(opts: {
  entityId?: string; json?: boolean;
}): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const eid = await resolver.resolveEntity(opts.entityId);
    const profile = await client.getGovernanceProfile(eid);
    if (opts.json) { printJson(profile); return; }
    console.log(chalk.blue("─".repeat(40)));
    console.log(chalk.blue.bold("  Governance Profile"));
    console.log(chalk.blue("─".repeat(40)));
    for (const [key, value] of Object.entries(profile)) {
      if (typeof value === "string" || typeof value === "number" || typeof value === "boolean") {
        console.log(`  ${chalk.bold(key.replaceAll("_", " ") + ":")} ${value}`);
      }
    }
    console.log(chalk.blue("─".repeat(40)));
  } catch (err) { printError(`Failed to get governance profile: ${err}`); process.exit(1); }
}
```

- [ ] **Step 4: Build both packages**

Run: `cd packages/corp-tools && npm run build && cd ../cli-ts && npm run build`

- [ ] **Step 5: Commit**

```bash
git add packages/corp-tools/src/api-client.ts packages/cli-ts/src/index.ts packages/cli-ts/src/commands/governance.ts
git commit -m "feat(cli): add governance mode, seat resign, incidents, and profile commands

Agents can now view/set governance mode, resign seats, list incidents,
and view governance profiles from the CLI."
```

### Task 18: Add financial statements and equity conversion commands

**Files:**
- Modify: `packages/corp-tools/src/api-client.ts`
- Modify: `packages/cli-ts/src/index.ts`
- Modify: `packages/cli-ts/src/commands/finance.ts`
- Modify: `packages/cli-ts/src/commands/cap-table.ts`

- [ ] **Step 1: Add API client methods**

In `packages/corp-tools/src/api-client.ts`:
```typescript
async getFinancialStatements(entityId: string, params?: Record<string, string>): Promise<ApiRecord> {
  return this.get(`/v1/treasury/financial-statements`, { entity_id: entityId, ...params });
}

// NOTE: previewRoundConversion and executeRoundConversion already exist in the client
// (POST /v1/equity/conversions/preview and /execute). Use those for SAFE conversions
// instead of creating duplicates. No new methods needed for conversions.

async getDilutionPreview(entityId: string, roundId: string): Promise<ApiRecord> {
  return this.get(`/v1/equity/dilution/preview`, { entity_id: entityId, round_id: roundId });
}

async getControlMap(entityId: string, rootEntityId: string): Promise<ApiRecord> {
  return this.get(`/v1/equity/control-map`, { entity_id: entityId, root_entity_id: rootEntityId });
}
```

- [ ] **Step 2: Add finance statements command in index.ts**

After the existing finance commands:
```typescript
financeCmd
  .command("statements")
  .option("--period <period>", "Period (e.g. 2026-Q1, 2025)")
  .option("--json", "Output as JSON")
  .description("View financial statements (P&L, balance sheet)")
  .action(async (opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { financeStatementsCommand } = await import("./commands/finance.js");
    await financeStatementsCommand({
      ...opts,
      entityId: parent.entityId,
      json: inheritOption(opts.json, parent.json),
    });
  });
```

- [ ] **Step 3: Add equity conversion commands in index.ts**

After existing cap-table commands:
```typescript
capTableCmd
  .command("preview-conversion")
  .requiredOption("--safe-id <ref>", "SAFE note reference to convert")
  .requiredOption("--price-per-share-cents <n>", "Conversion price per share in cents", parseInt)
  .option("--json", "Output as JSON")
  .description("Preview SAFE-to-equity conversion")
  .action(async (opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { previewConversionCommand } = await import("./commands/cap-table.js");
    await previewConversionCommand({
      ...opts,
      entityId: parent.entityId,
      json: inheritOption(opts.json, parent.json),
    });
  });
capTableCmd
  .command("convert")
  .requiredOption("--safe-id <ref>", "SAFE note reference to convert")
  .requiredOption("--price-per-share-cents <n>", "Conversion price per share in cents", parseInt)
  .option("--json", "Output as JSON")
  .option("--dry-run", "Show the request without executing")
  .description("Execute SAFE-to-equity conversion")
  .action(async (opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { executeConversionCommand } = await import("./commands/cap-table.js");
    await executeConversionCommand({
      ...opts,
      entityId: parent.entityId,
      json: inheritOption(opts.json, parent.json),
    });
  });
capTableCmd
  .command("dilution")
  .requiredOption("--round-id <ref>", "Round reference to model dilution for")
  .option("--json", "Output as JSON")
  .description("Preview dilution impact of a round")
  .action(async (opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { dilutionPreviewCommand } = await import("./commands/cap-table.js");
    await dilutionPreviewCommand({
      ...opts,
      entityId: parent.entityId,
      json: inheritOption(opts.json, parent.json),
    });
  });
capTableCmd
  .command("control-map")
  .option("--root-entity-id <ref>", "Root entity for ownership tree (defaults to active entity)")
  .option("--json", "Output as JSON")
  .description("View entity control/ownership map")
  .action(async (opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { controlMapCommand } = await import("./commands/cap-table.js");
    await controlMapCommand({
      ...opts,
      entityId: parent.entityId,
      json: inheritOption(opts.json, parent.json),
    });
  });
```

- [ ] **Step 4: Implement finance statements handler**

Add to `packages/cli-ts/src/commands/finance.ts`:
```typescript
export async function financeStatementsCommand(opts: {
  entityId?: string; period?: string; json?: boolean;
}): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const eid = await resolver.resolveEntity(opts.entityId);
    const params: Record<string, string> = {};
    if (opts.period) params.period = opts.period;
    const result = await client.getFinancialStatements(eid, params);
    if (opts.json) { printJson(result); return; }
    printJson(result); // financial statements are complex, JSON is the most useful format
  } catch (err) {
    printError(`Failed to fetch financial statements: ${err}`);
    process.exit(1);
  }
}
```

- [ ] **Step 5: Implement cap-table conversion and dilution handlers**

Add to `packages/cli-ts/src/commands/cap-table.ts`:
```typescript
export async function previewConversionCommand(opts: {
  entityId?: string; safeId: string; pricePerShareCents: number; json?: boolean;
}): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const eid = await resolver.resolveEntity(opts.entityId);
    const safeId = await resolver.resolveSafeNote(eid, opts.safeId);
    // Use existing previewRoundConversion method (POST /v1/equity/conversions/preview)
    const result = await client.previewRoundConversion({
      entity_id: eid,
      safe_note_id: safeId,
      price_per_share_cents: opts.pricePerShareCents,
    });
    if (opts.json) { printJson(result); return; }
    printSuccess("Conversion Preview:");
    if (result.shares_issued) console.log(`  Shares to issue: ${result.shares_issued}`);
    if (result.ownership_pct) console.log(`  Post-conversion ownership: ${result.ownership_pct}%`);
  } catch (err) { printError(`Failed to preview conversion: ${err}`); process.exit(1); }
}

export async function executeConversionCommand(opts: {
  entityId?: string; safeId: string; pricePerShareCents: number;
  json?: boolean; dryRun?: boolean;
}): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const eid = await resolver.resolveEntity(opts.entityId);
    const safeId = await resolver.resolveSafeNote(eid, opts.safeId);
    const payload = {
      entity_id: eid,
      safe_note_id: safeId,
      price_per_share_cents: opts.pricePerShareCents,
    };
    if (opts.dryRun) { printDryRun("equity.conversion.execute", payload); return; }
    // Use existing executeRoundConversion method (POST /v1/equity/conversions/execute)
    const result = await client.executeRoundConversion(payload);
    printWriteResult(result, `Conversion executed for SAFE ${safeId}`, {
      jsonOnly: opts.json,
    });
  } catch (err) { printError(`Failed to execute conversion: ${err}`); process.exit(1); }
}

export async function dilutionPreviewCommand(opts: {
  entityId?: string; roundId: string; json?: boolean;
}): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const eid = await resolver.resolveEntity(opts.entityId);
    const roundId = await resolver.resolveRound(eid, opts.roundId);
    const result = await client.getDilutionPreview(eid, roundId);
    if (opts.json) { printJson(result); return; }
    printJson(result);
  } catch (err) { printError(`Failed to preview dilution: ${err}`); process.exit(1); }
}

export async function controlMapCommand(opts: {
  entityId?: string; rootEntityId?: string; json?: boolean;
}): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const eid = await resolver.resolveEntity(opts.entityId);
    const rootEntityId = opts.rootEntityId
      ? await resolver.resolveEntity(opts.rootEntityId)
      : eid;
    const result = await client.getControlMap(eid, rootEntityId);
    if (opts.json) { printJson(result); return; }
    printJson(result);
  } catch (err) { printError(`Failed to fetch control map: ${err}`); process.exit(1); }
}
```

- [ ] **Step 6: Build both packages**

Run: `cd packages/corp-tools && npm run build && cd ../cli-ts && npm run build`

- [ ] **Step 7: Commit**

```bash
git add packages/corp-tools/src/api-client.ts packages/cli-ts/src/index.ts packages/cli-ts/src/commands/finance.ts packages/cli-ts/src/commands/cap-table.ts
git commit -m "feat(cli): add financial statements, equity conversions, dilution preview, and control map

Agents can now pull financial statements, preview/execute SAFE
conversions, model dilution scenarios, and view control maps."
```

### Task 19: Auto-set active entity after formation

**Files:**
- Modify: `packages/cli-ts/src/commands/form.ts`

- [ ] **Step 1: Auto-set active entity after formCommand and formCreateCommand**

In `packages/cli-ts/src/commands/form.ts`, import `setActiveEntityId` and `saveConfig`:
```typescript
import { requireConfig, setActiveEntityId, saveConfig } from "../config.js";
```

In `formCommand`, after `resolver.rememberFromRecord("entity", result)` (~line 544), add:
```typescript
if (result.entity_id) {
  setActiveEntityId(cfg, String(result.entity_id));
  saveConfig(cfg);
  console.log(chalk.dim(`  Active entity set to ${result.entity_id}`));
}
```

In `formCreateCommand`, after `resolver.rememberFromRecord("entity", result)` (~line 667), add:
```typescript
if (result.entity_id) {
  setActiveEntityId(cfg, String(result.entity_id));
  saveConfig(cfg);
}
```

- [ ] **Step 2: Build to verify**

Run: `cd packages/cli-ts && npm run build`

- [ ] **Step 3: Commit**

```bash
git add packages/cli-ts/src/commands/form.ts
git commit -m "feat(cli): auto-set active entity after formation

Users no longer need to manually run 'corp config set active_entity_id'
after forming an entity. The new entity becomes active automatically."
```

### Task 20: Final build verification and cleanup

- [ ] **Step 1: Full build of both packages**

Run: `cd /root/repos/thecorporation-mono/packages/corp-tools && npm run build && cd ../cli-ts && npm run build`

- [ ] **Step 2: Run corp-tools tests**

Run: `cd /root/repos/thecorporation-mono/packages/corp-tools && npm test`

If tests fail due to API client changes (new methods), update mocks as needed.

- [ ] **Step 3: Verify help text for key commands**

Run:
```bash
cd /root/repos/thecorporation-mono/packages/cli-ts
node dist/index.js --help
node dist/index.js form --help
node dist/index.js governance --help
node dist/index.js cap-table --help
node dist/index.js finance --help
node dist/index.js tax --help
node dist/index.js agents --help
node dist/index.js work-items --help
node dist/index.js api-keys --help
```

Verify: examples shown, required options marked, money flags say -cents.

- [ ] **Step 4: Final commit if any fixups needed**

```bash
git add -A
git commit -m "chore(cli): final build verification and fixups"
```
