# CLI Command Registry Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the CLI's imperative command files and the web terminal's manual ROUTES table with a declarative command registry — one definition per command that drives both the TypeScript CLI and the browser terminal.

**Architecture:** A `registry/` directory contains domain-grouped command definitions. `buildCLI()` converts these into a Commander.js program. `generateWebRoutes()` emits a JSON manifest for the web terminal. Pure read commands use a generic executor; write/custom commands keep their handler functions relocated alongside their declarations.

**Tech Stack:** TypeScript, Commander.js, chalk, cli-table3, xterm.js (web terminal)

**Spec:** `docs/superpowers/specs/2026-03-20-cli-command-registry-design.md`

**Monorepo:** `/root/repos/thecorporation-mono` (CLI package)
**Internal repo:** `/root/repos/thecorporation-internal` (web terminal, chat-ws, ops)

---

## File Structure

### New files (packages/cli-ts/src/)
| File | Responsibility |
|------|---------------|
| `registry/types.ts` | CommandDef, ArgDef, OptionDef, CommandContext, OutputWriter interfaces |
| `registry/index.ts` | Aggregates all registry files, exports the full command array |
| `registry/workspace.ts` | status, context, use, next, billing, obligations, digests |
| `registry/entities.ts` | entities, entities show/convert/dissolve |
| `registry/formation.ts` | form, form create/add-founder/finalize/activate |
| `registry/governance.ts` | governance + ~20 subcommands |
| `registry/cap-table.ts` | cap-table + ~22 subcommands |
| `registry/documents.ts` | documents + 5 subcommands |
| `registry/finance.ts` | finance + ~14 subcommands |
| `registry/compliance.ts` | tax + 4 subcommands |
| `registry/agents.ts` | agents + ~10 subcommands |
| `registry/services.ts` | services + 5 subcommands |
| `registry/work-items.ts` | work-items + 6 subcommands |
| `registry/admin.ts` | setup, config, schema, serve, demo, api-keys, chat, link, claim, feedback, resolve, find, approvals |
| `cli.ts` | `buildCLI(registry)` → Commander program |
| `generic-executor.ts` | Shared fetch+display for generic read commands |
| `writer.ts` | OutputWriter implementation wrapping existing output.ts helpers |

### Modified files
| File | Change |
|------|--------|
| `src/index.ts` | Replace ~2000 lines of manual registration with: import registry, buildCLI, parse |
| `tsup.config.ts` | No change needed (single entry point stays) |

### Deleted files (after migration complete)
| File | Reason |
|------|--------|
| `src/commands/status.ts` | Handler moved to registry/workspace.ts |
| `src/commands/obligations.ts` | Collapsed into pure registry entry |
| `src/commands/context.ts` | Handler moved to registry/workspace.ts |
| `src/commands/use.ts` | Handler moved to registry/workspace.ts |
| `src/commands/next.ts` | Handler moved to registry/workspace.ts |
| `src/commands/billing.ts` | Handler moved to registry/workspace.ts |
| `src/commands/digest.ts` | Collapsed into pure registry entry |
| `src/commands/entities.ts` | Handler moved to registry/entities.ts |
| `src/commands/form.ts` | Handler moved to registry/formation.ts |
| `src/commands/governance.ts` | Handler moved to registry/governance.ts |
| `src/commands/cap-table.ts` | Handler moved to registry/cap-table.ts |
| `src/commands/documents.ts` | Handler moved to registry/documents.ts |
| `src/commands/finance.ts` | Handler moved to registry/finance.ts |
| `src/commands/tax.ts` | Handler moved to registry/compliance.ts |
| `src/commands/agents.ts` | Handler moved to registry/agents.ts |
| `src/commands/work-items.ts` | Handler moved to registry/work-items.ts |
| `src/commands/services.ts` | Handler moved to registry/services.ts |
| `src/commands/contacts.ts` | Handler moved to registry/entities.ts |
| `src/commands/api-keys.ts` | Handler moved to registry/admin.ts |
| `src/commands/setup.ts` | Handler moved to registry/admin.ts |
| `src/commands/config.ts` | Handler moved to registry/admin.ts |
| `src/commands/demo.ts` | Handler moved to registry/admin.ts |
| `src/commands/serve.ts` | Handler moved to registry/admin.ts |
| `src/commands/feedback.ts` | Handler moved to registry/admin.ts |
| `src/commands/claim.ts` | Handler moved to registry/admin.ts |
| `src/commands/link.ts` | Handler moved to registry/admin.ts |
| `src/commands/resolve.ts` | Handler moved to registry/admin.ts |
| `src/commands/find.ts` | Handler moved to registry/admin.ts |
| `src/commands/approvals.ts` | Handler moved to registry/admin.ts |
| `src/commands/schema.ts` | Logic moved to registry/admin.ts + generateSchema in registry/index.ts |
| `src/command-options.ts` | inheritOption logic absorbed into cli.ts |

### Internal repo changes
| File | Change |
|------|--------|
| `services/chat-ws/Dockerfile` | Add web-routes.json generation step |
| `services/chat-ws/src/index.ts` | Serve `/cli/web-routes.json` |
| `ops/Caddyfile` | Route `/cli/web-routes.json` to chat-ws |
| `services/web/packages/humans/src/pages/cli.astro` | Replace hardcoded ROUTES with fetched manifest |

---

## Task 1: Registry types and OutputWriter

**Files:**
- Create: `packages/cli-ts/src/registry/types.ts`
- Create: `packages/cli-ts/src/writer.ts`

This task establishes the type system that all other tasks depend on. No command logic yet.

- [ ] **Step 1: Create `registry/types.ts`**

Read the spec at `docs/superpowers/specs/2026-03-20-cli-command-registry-design.md` for the exact type definitions. Create the file with:

- `CommandDef` interface (all fields from spec: name, description, aliases, route, entity, args, options, optQP, display, handler, local, hidden, dryRun, passThroughOptions, examples)
- `ArgDef` interface (name, required, description, variadic, choices)
- `OptionDef` interface (flags, description, required, choices, default, type)
- `CommandContext` interface (client, positional, opts, entityId, resolver, writer, quiet, dryRun)
- `OutputWriter` interface (writeln, json, table, panel, error, success, writeResult, quietId)
- `WebRouteEntry` interface (the shape emitted in web-routes.json: method, path, entity, title, cols, listKey, optQP, write, local, custom)

Export everything. No runtime code — pure types.

- [ ] **Step 2: Create `writer.ts` — OutputWriter implementation**

This wraps the existing `output.ts` functions into the `OutputWriter` interface:

```typescript
import chalk from "chalk";
import { printError, printSuccess, printJson, printWriteResult, printQuietId, printDryRun } from "./output.js";
import type { OutputWriter } from "./registry/types.js";

export function createWriter(): OutputWriter {
  return {
    writeln(text = "") { console.log(text); },
    json(data) { printJson(data); },
    table(title, columns, rows) {
      // Use the existing makeTable pattern from output.ts
      // Import makeTable or reimplement the cli-table3 wrapper
    },
    panel(title, color, lines) {
      // Use the existing panel pattern from output.ts
    },
    error(msg) { printError(msg); },
    success(msg) { printSuccess(msg); },
    writeResult(result, kind) { printWriteResult(result, "", {}); },
    quietId(id) { console.log(id); },
  };
}
```

Read `output.ts` carefully — the `makeTable`, `printStatusPanel`, and panel pattern need to be exposed through the writer. The writer should import and delegate to existing output.ts functions, NOT reimplement them.

- [ ] **Step 3: Verify TypeScript compiles**

```bash
cd /root/repos/thecorporation-mono/packages/cli-ts && npx tsc --noEmit
```

- [ ] **Step 4: Commit**

```bash
git add src/registry/types.ts src/writer.ts
git commit -m "feat(cli): add command registry types and OutputWriter"
```

---

## Task 2: Generic executor

**Files:**
- Create: `packages/cli-ts/src/generic-executor.ts`

The generic executor handles all pure-read commands: resolve path params, fetch, display. This is the CLI-side equivalent of the web terminal's `execRoute()`.

- [ ] **Step 1: Create `generic-executor.ts`**

Read the existing `execRoute()` function in the web terminal (`/root/repos/thecorporation-internal/services/web/packages/humans/src/pages/cli.astro`, around line 756) to understand the path resolution pattern.

The generic executor must:
1. Resolve `{eid}` → entity ID (using `resolver.resolveEntity()` from `--entity-id` opt or positional arg)
2. Resolve `{pos}` → next positional argument
3. Resolve `{wid}` → workspace ID from config
4. Forward `optQP` options as query params
5. Call `client.get(path, params)` wrapped in `withSpinner()`
6. Unwrap `listKey` if specified
7. Display: if `display.cols` is defined, parse column specs and render table; if data is an array without cols, auto-detect columns; if data is a single object, render as panel

For column parsing, read the existing `parseCol()` function in cli.astro (around line 711) and the column spec syntax from the spec. Implement column parsing with the prefix conventions: `$` = money, `@` = date, `#` = shortId, `|` = fallback fields, `>` = label.

Reference existing helpers in `output.ts`: `s()` for safe strings, `money()` for cents formatting, `date()` for ISO dates.

```typescript
import type { CommandDef, CommandContext } from "./registry/types.js";
import { withSpinner } from "./spinner.js";

export async function executeGenericRead(def: CommandDef, ctx: CommandContext): Promise<void> {
  // 1. Build the URL from def.route.path
  // 2. Resolve {eid}, {pos}, {wid} placeholders
  // 3. Build query params from optQP
  // 4. Fetch with withSpinner
  // 5. Unwrap listKey
  // 6. Display (--json → ctx.writer.json; array → table; object → panel)
}
```

The implementation should handle all the cases the web terminal's `execRoute()` handles. Read that function carefully and mirror its logic.

- [ ] **Step 2: Verify it compiles**

```bash
cd /root/repos/thecorporation-mono/packages/cli-ts && npx tsc --noEmit
```

- [ ] **Step 3: Commit**

```bash
git add src/generic-executor.ts
git commit -m "feat(cli): add generic executor for declarative read commands"
```

---

## Task 3: buildCLI and registry index

**Files:**
- Create: `packages/cli-ts/src/cli.ts`
- Create: `packages/cli-ts/src/registry/index.ts`

`buildCLI()` converts the registry array into a Commander.js program. `registry/index.ts` aggregates all registry files (initially empty — will be populated as domain files are created).

- [ ] **Step 1: Create `registry/index.ts`**

```typescript
import type { CommandDef } from "./types.js";

// Domain registries will be imported here as they're created
// import { workspaceCommands } from "./workspace.js";
// import { governanceCommands } from "./governance.js";
// etc.

export const registry: CommandDef[] = [
  // ...workspaceCommands,
  // ...governanceCommands,
  // etc.
];

// Web routes manifest generation
export function generateWebRoutes(commands: CommandDef[]): Record<string, unknown> {
  const entries: Record<string, unknown> = {};
  for (const cmd of commands) {
    const key = cmd.name;
    if (cmd.local) {
      entries[key] = { local: true };
    } else if (cmd.route && cmd.display && !cmd.handler) {
      // Generic read — full route config for web
      entries[key] = {
        method: cmd.route.method,
        path: cmd.route.path,
        ...(cmd.entity !== undefined && { entity: cmd.entity }),
        title: cmd.display.title,
        ...(cmd.display.cols && { cols: cmd.display.cols }),
        ...(cmd.display.listKey && { listKey: cmd.display.listKey }),
        ...(cmd.optQP && { optQP: cmd.optQP }),
      };
    } else if (cmd.route && cmd.display && cmd.handler) {
      // Custom read — web uses generic executor, flag as custom
      entries[key] = {
        method: cmd.route.method,
        path: cmd.route.path,
        ...(cmd.entity !== undefined && { entity: cmd.entity }),
        title: cmd.display.title,
        ...(cmd.display.cols && { cols: cmd.display.cols }),
        ...(cmd.display.listKey && { listKey: cmd.display.listKey }),
        custom: true,
      };
    } else if (cmd.route && cmd.route.method !== "GET") {
      // Write command
      entries[key] = { method: cmd.route.method, write: true };
    } else if (cmd.handler && !cmd.local) {
      // Informational / custom with no display
      entries[key] = { custom: true };
    }
  }
  return { commands: entries };
}

// Schema generation (for tab completion)
export function generateSchema(commands: CommandDef[], programName: string, version: string): unknown {
  // Build hierarchical schema from flat command list
  // Group by parent (split name on space)
  // Emit: { name, version, commands: [...] } matching existing cli-schema.json shape
  // Read existing schema.ts for the exact output shape
}
```

- [ ] **Step 2: Create `cli.ts` — buildCLI function**

This is the core wiring function. It must:

1. Create a Commander `Program`
2. For each `CommandDef`, determine if it's a parent or child (by space in `name`)
3. Create parent group commands first, then attach children
4. For each command:
   - Add `.description()`, `.aliases()`
   - Add args via `.argument()`
   - Add options via `.option()` or `.requiredOption()` (based on `required` flag)
   - Handle `type` field: "int" → `parseInt` coerce, "array" → accumulator, "float" → `parseFloat`
   - If `dryRun: true`, auto-add `--dry-run` option
   - Add `--json` to all commands (standard option)
   - Add `.addHelpText("after", ...)` for `examples`
   - If `passThroughOptions: true`, call `.enablePositionalOptions().passThroughOptions()`
5. Wire `.action()`:
   - If command has `handler`: build `CommandContext` (resolve entity, create client+resolver+writer), call handler
   - If command has `display` + no `handler`: call `executeGenericRead()`
   - If command has `local: true` + `handler`: call handler (no API client needed, handler creates its own)
6. Handle parent-child option inheritance: child commands merge parent opts via `cmd.parent!.opts()`

Read the existing `index.ts` carefully — especially the pattern where parent opts are extracted via `cmd.parent!.opts()` and merged with `inheritOption()`. The `buildCLI` function must reproduce this behavior.

Key reference patterns from index.ts:
- Lines 37-55: setup, status, context — simple top-level
- Lines 101-108: schema with options
- Lines 300-800: governance parent + subcommands — the core parent-child pattern
- Lines 800-1200: cap-table — largest group with requiredOptions, integer parsing

```typescript
import { Command } from "commander";
import type { CommandDef, CommandContext } from "./registry/types.js";
import { executeGenericRead } from "./generic-executor.js";
import { createWriter } from "./writer.js";
import { requireConfig, resolveEntityId, loadConfig } from "./config.js";
import { CorpAPIClient } from "./api-client.js";
import { ReferenceResolver } from "./references.js";

export function buildCLI(registry: CommandDef[], version: string): Command {
  const program = new Command();
  program.name("corp").description("corp — Corporate governance from the terminal").version(version);
  program.option("-q, --quiet", "Only output the resource ID (for scripting)");
  program.enablePositionalOptions();

  // Group commands by parent
  const parents = new Map<string, Command>();

  // First pass: create parent commands
  for (const def of registry) {
    if (!def.name.includes(" ")) {
      // Top-level command
      const cmd = createCommand(program, def, registry);
      parents.set(def.name, cmd);
    }
  }

  // Second pass: create subcommands
  for (const def of registry) {
    if (def.name.includes(" ")) {
      const [parentName, ...rest] = def.name.split(" ");
      const subName = rest.join(" ");
      const parent = parents.get(parentName);
      if (parent) {
        createCommand(parent, { ...def, name: subName }, registry);
      }
    }
  }

  // Add help tip
  program.addHelpText("after", '\nTip: Run "corp next" to see your recommended next actions.\n');

  return program;
}

function createCommand(parent: Command, def: CommandDef, registry: CommandDef[]): Command {
  // Build the command string with args
  let cmdStr = def.name;
  for (const arg of (def.args || [])) {
    cmdStr += arg.required ? ` <${arg.name}>` : ` [${arg.name}]`;
  }

  const cmd = parent.command(cmdStr).description(def.description);

  // Aliases
  for (const alias of (def.aliases || [])) cmd.alias(alias);

  // Options
  if (def.dryRun) {
    cmd.option("--dry-run", "Preview the request without executing");
  }
  cmd.option("--json", "Output as JSON");

  // Entity option for entity-scoped commands
  if (def.entity) {
    cmd.option("--entity-id <ref>", "Entity reference (overrides active entity)");
  }

  for (const opt of (def.options || [])) {
    const coerce = opt.type === "int" ? parseInt
                 : opt.type === "float" ? parseFloat
                 : opt.type === "array" ? ((v, prev) => [...(prev || []), v])
                 : undefined;
    if (opt.required) {
      cmd.requiredOption(opt.flags, opt.description, coerce, opt.default);
    } else {
      cmd.option(opt.flags, opt.description, coerce, opt.default);
    }
  }

  // Help text
  if (def.examples?.length) {
    cmd.addHelpText("after", "\nExamples:\n" + def.examples.map(e => `  $ ${e}`).join("\n") + "\n");
  }

  // passThroughOptions
  if (def.passThroughOptions) {
    cmd.enablePositionalOptions().passThroughOptions();
  }

  // Action
  cmd.action(async (...actionArgs) => {
    // Extract positional args and opts from Commander's action args
    // Last arg is always the Command instance, second-to-last is opts
    // ... (implementation details)

    // Build CommandContext, call handler or generic executor
  });

  return cmd;
}
```

This is a skeleton. The implementer MUST read `index.ts` to understand:
- How positional args are extracted in `.action()` callbacks (Commander passes them as individual args before opts)
- How parent opts are merged (`cmd.parent!.opts()`)
- The exact `inheritOption()` pattern
- Error handling pattern (try/catch with `printError` + `process.exit(1)`)

- [ ] **Step 3: Verify it compiles (with empty registry)**

```bash
cd /root/repos/thecorporation-mono/packages/cli-ts && npx tsc --noEmit
```

- [ ] **Step 4: Commit**

```bash
git add src/cli.ts src/registry/index.ts
git commit -m "feat(cli): add buildCLI and registry index with web-routes generation"
```

---

## Task 4: Migrate workspace commands

**Files:**
- Create: `packages/cli-ts/src/registry/workspace.ts`

This is the first real migration batch. These commands exercise every pattern: pure reads, custom display, and the `next` command with its local checks.

Commands to migrate:
- `status` — custom display (panel), handler from `commands/status.ts`
- `context` (alias `whoami`) — custom display, handler from `commands/context.ts`
- `use` — custom handler, from `commands/use.ts`
- `next` — custom handler with local checks, from `commands/next.ts`
- `obligations` — pure read, from `commands/obligations.ts`
- `digest` — pure read (list) + write (trigger), from `commands/digest.ts`
- `billing` — custom display, handler from `commands/billing.ts`

- [ ] **Step 1: Read source files**

Read all of these command files to extract their handler logic:
- `src/commands/status.ts` (20 lines)
- `src/commands/context.ts` (94 lines)
- `src/commands/use.ts` (20 lines)
- `src/commands/next.ts` (122 lines)
- `src/commands/obligations.ts` (15 lines)
- `src/commands/digest.ts` (38 lines)
- `src/commands/billing.ts` (59 lines)

Also read the corresponding sections of `index.ts` to get all options, args, and registration details.

- [ ] **Step 2: Create `registry/workspace.ts`**

Export `workspaceCommands: CommandDef[]` containing all workspace-domain commands.

For pure-read commands like `obligations`, the entry is just data — no handler:

```typescript
{
  name: "obligations",
  description: "List obligations with urgency tiers",
  route: { method: "GET", path: "/v1/obligations/summary" },
  display: {
    title: "Obligations",
    listKey: "obligations",
    cols: ["obligation_type>Type", "urgency>Urgency", "@due_at>Due", "status>Status", "#obligation_id>ID"],
  },
  optQP: ["tier"],
  options: [{ flags: "--tier <tier>", description: "Filter by urgency tier" }],
}
```

For custom handlers like `status`, relocate the handler body from `commands/status.ts`:

```typescript
{
  name: "status",
  description: "Workspace summary",
  route: { method: "GET", path: "/v1/workspaces/{wid}/status" },
  display: { title: "Corp Status" },
  handler: async (ctx) => {
    const data = await withSpinner("Loading", () => ctx.client.getStatus(), ctx.opts.json as boolean);
    if (ctx.opts.json) { ctx.writer.json(data); return; }
    // Relocate printStatusPanel logic or call it directly
    printStatusPanel(data);
  },
}
```

For `next`, relocate the full handler from `commands/next.ts` including localChecks().

- [ ] **Step 3: Update `registry/index.ts` to import workspace commands**

```typescript
import { workspaceCommands } from "./workspace.js";
export const registry: CommandDef[] = [...workspaceCommands];
```

- [ ] **Step 4: Verify it compiles**

```bash
cd /root/repos/thecorporation-mono/packages/cli-ts && npx tsc --noEmit
```

- [ ] **Step 5: Commit**

```bash
git add src/registry/workspace.ts src/registry/index.ts
git commit -m "feat(cli): migrate workspace commands to registry"
```

---

## Task 5: Migrate entities + formation + contacts commands

**Files:**
- Create: `packages/cli-ts/src/registry/entities.ts`
- Create: `packages/cli-ts/src/registry/formation.ts`

Commands to migrate:
- `entities` (list) — custom display (table with entity tracking)
- `entities show` — custom display (panel)
- `entities convert` — write command
- `entities dissolve` — write command
- `contacts` (list) — pure read
- `contacts show` — pure read
- `contacts add` — write command
- `contacts edit` — write command
- `form` — parent with passThroughOptions
- `form create` — write command (complex: one-shot formation)
- `form add-founder` — write command
- `form finalize` — write command
- `form activate` — write command

- [ ] **Step 1: Read source files**

Read `commands/entities.ts` (119 lines), `commands/contacts.ts` (166 lines), `commands/form.ts` (966 lines — the second-largest file). Read the corresponding index.ts sections.

- [ ] **Step 2: Create the registry files**

For `entities.ts`: export `entityCommands: CommandDef[]` with entities + contacts entries. Contacts is a flat subcommand group (contacts, contacts show, contacts add, contacts edit).

For `formation.ts`: export `formationCommands: CommandDef[]`. The `form` parent needs `passThroughOptions: true`. The `form create` handler is complex (one-shot formation with member JSON parsing). Relocate the entire handler body from `commands/form.ts`.

- [ ] **Step 3: Update `registry/index.ts`**

Add imports for entityCommands and formationCommands.

- [ ] **Step 4: Verify and commit**

```bash
npx tsc --noEmit && git add src/registry/entities.ts src/registry/formation.ts src/registry/index.ts && git commit -m "feat(cli): migrate entities, contacts, formation commands to registry"
```

---

## Task 6: Migrate governance commands

**Files:**
- Create: `packages/cli-ts/src/registry/governance.ts`

This is one of the largest groups (~20 subcommands, 543 lines of handlers). Commands:
- `governance` (list bodies) — pure read with custom entity handling
- `governance create-body` — write, requiredOptions
- `governance add-seat` — write, requiredOptions
- `governance seats` — pure read (positional body ref)
- `governance meetings` — pure read (positional body ref)
- `governance resolutions` — pure read (positional meeting ref)
- `governance agenda-items` — pure read (positional meeting ref)
- `governance convene` — write (schedule meeting)
- `governance open` — write (open meeting)
- `governance vote` — write (cast vote)
- `governance notice` — write
- `governance adjourn` — write
- `governance reopen` — write
- `governance cancel` — write
- `governance finalize-item` — write (with choices for status)
- `governance resolve` — write
- `governance written-consent` — write (complex: auto-creates meeting with all seats)
- `governance mode` — read/write hybrid (GET with optional set)
- `governance resign` — write
- `governance incidents` — pure read
- `governance profile` — pure read

- [ ] **Step 1: Read source files**

Read `commands/governance.ts` (543 lines) fully — every exported function. Read the governance section of `index.ts` (approximately lines 300-800) for all option definitions and the parent-child wiring.

- [ ] **Step 2: Create `registry/governance.ts`**

Export `governanceCommands: CommandDef[]`. For pure reads (seats, meetings, resolutions, agenda-items, incidents, profile), use just the declarative entry with display config. For writes, relocate the handler function body.

Important: the governance parent command has `--entity-id`, `--body-id`, `--json`, `--dry-run` options that are inherited by subcommands. In the registry, each subcommand declares its own options, and `buildCLI()` handles the parent-child merge.

- [ ] **Step 3: Update `registry/index.ts`, verify, commit**

```bash
git commit -m "feat(cli): migrate governance commands to registry"
```

---

## Task 7: Migrate cap-table commands

**Files:**
- Create: `packages/cli-ts/src/registry/cap-table.ts`

The largest group (~22 subcommands, 1095 lines of handlers). This file will include helper functions alongside the registry entries.

Read commands (pure declarative):
- `cap-table` (summary) — custom display
- `cap-table safes`, `transfers`, `instruments`, `share-classes`, `rounds`, `valuations`, `409a` — pure reads
- `cap-table control-map`, `dilution` — pure reads with entity query

Write commands (with handlers):
- `cap-table create-instrument`, `issue-equity`, `issue-safe`, `transfer`, `distribute`
- `cap-table start-round`, `add-security`, `issue-round`
- `cap-table create-valuation`, `submit-valuation`, `approve-valuation`
- `cap-table preview-conversion`, `convert`

- [ ] **Step 1: Read `commands/cap-table.ts` fully**

Pay special attention to helper functions: `normalizedGrantType()`, `expectedInstrumentKinds()`, `grantRequiresCurrent409a()`, `buildInstrumentCreationHint()`, `resolveInstrumentForGrant()`, `entityHasActiveBoard()`, `ensureIssuancePreflight()`. These must be relocated with the handlers.

- [ ] **Step 2: Create `registry/cap-table.ts`**

Include the helper functions at the top of the file, then export `capTableCommands: CommandDef[]`. Read commands use declarative entries; write commands include handlers that call the helper functions.

- [ ] **Step 3: Update index, verify, commit**

```bash
git commit -m "feat(cli): migrate cap-table commands to registry"
```

---

## Task 8: Migrate documents, finance, compliance commands

**Files:**
- Create: `packages/cli-ts/src/registry/documents.ts`
- Create: `packages/cli-ts/src/registry/finance.ts`
- Create: `packages/cli-ts/src/registry/compliance.ts`

Documents (262 lines): documents list (read), signing-link (read), sign (write), sign-all (write), generate (write), preview-pdf (write)

Finance (404 lines): finance summary (custom), invoices/payments/bank-accounts/payroll/distributions/reconciliations/classifications/statements (reads), invoice/payroll/pay/open-account/activate-account/classify-contractor/reconcile (writes)

Compliance/Tax (122 lines): tax/filings/deadlines (reads), tax file/deadline (writes)

- [ ] **Step 1: Read source files**

Read `commands/documents.ts`, `commands/finance.ts`, `commands/tax.ts`, and their corresponding index.ts sections.

- [ ] **Step 2: Create registry files**

For each domain, export the `*Commands: CommandDef[]` array. Most finance reads are pure declarative entries. The finance summary handler is custom (multi-fetch). Documents signing-link is a special case (returns a URL, not table data).

- [ ] **Step 3: Update index, verify, commit**

```bash
git commit -m "feat(cli): migrate documents, finance, compliance commands to registry"
```

---

## Task 9: Migrate agents, work-items, services commands

**Files:**
- Create: `packages/cli-ts/src/registry/agents.ts`
- Create: `packages/cli-ts/src/registry/work-items.ts`
- Create: `packages/cli-ts/src/registry/services.ts`

Agents (284 lines): agents list (read), agents show (read), create/pause/resume/delete/message/skill (writes), execution/execution-result/kill (reads/writes)

Work Items (168 lines): work-items list (read), show (read), create/claim/complete/release/cancel (writes)

Services (182 lines): services catalog (read), list (read), show (read), buy/fulfill/cancel (writes)

- [ ] **Step 1: Read source files and create registry files**

Same pattern as previous tasks. Read the command files and index.ts sections, create registry entries.

- [ ] **Step 2: Update index, verify, commit**

```bash
git commit -m "feat(cli): migrate agents, work-items, services commands to registry"
```

---

## Task 10: Migrate admin/utility commands

**Files:**
- Create: `packages/cli-ts/src/registry/admin.ts`

This captures all the remaining commands: setup (271 lines), config (63 lines), schema (91 lines), demo (300 lines), serve (72 lines), api-keys (68 lines), chat, link (16 lines), claim (38 lines), feedback (44 lines), resolve (196 lines), find (131 lines), approvals (33 lines).

Most of these are `local: true` with handlers that don't call the standard API client pattern.

- [ ] **Step 1: Read source files**

Read all admin command files. Special attention to:
- `setup.ts` — interactive wizard using @inquirer/prompts
- `demo.ts` — complex demo workspace creation
- `resolve.ts` — reference resolution utility
- `find.ts` — fuzzy search across resource kinds
- `schema.ts` — schema generation (this will be replaced by `generateSchema()` from the registry)

- [ ] **Step 2: Create `registry/admin.ts`**

For `schema`, the handler should call `generateSchema()` and `generateWebRoutes()` from `registry/index.ts` based on flags:

```typescript
{
  name: "schema",
  description: "Dump the CLI command catalog as JSON",
  local: true,
  options: [
    { flags: "--compact", description: "Emit compact JSON" },
    { flags: "--web-routes", description: "Emit web-routes manifest" },
  ],
  handler: async (ctx) => {
    if (ctx.opts.webRoutes) {
      console.log(JSON.stringify(generateWebRoutes(registry)));
    } else {
      // Generate schema from registry
      const schema = generateSchema(registry, "corp", version);
      if (ctx.opts.compact) console.log(JSON.stringify(schema));
      else ctx.writer.json(schema);
    }
  },
}
```

- [ ] **Step 3: Update index, verify, commit**

```bash
git commit -m "feat(cli): migrate admin and utility commands to registry"
```

---

## Task 11: Replace index.ts with registry-based entry point

**Files:**
- Modify: `packages/cli-ts/src/index.ts` (complete rewrite — ~2000 lines → ~30 lines)

This is the payoff. The massive index.ts is replaced with a minimal entry point.

- [ ] **Step 1: Rewrite index.ts**

```typescript
import { createRequire } from "node:module";
import { buildCLI } from "./cli.js";
import { registry } from "./registry/index.js";

const require = createRequire(import.meta.url);
const pkg = require("../package.json");

const program = buildCLI(registry, pkg.version);
program.parseAsync(process.argv).catch((err) => {
  console.error(err);
  process.exit(1);
});
```

- [ ] **Step 2: Build and test the CLI**

```bash
cd /root/repos/thecorporation-mono/packages/cli-ts && npm run build
```

Test key commands:
```bash
node dist/index.js --help                    # Should show all commands
node dist/index.js next --json               # Test custom handler
node dist/index.js schema --compact          # Test schema generation
node dist/index.js schema --web-routes       # Test web-routes manifest
node dist/index.js obligations --help        # Test read command help
node dist/index.js governance --help         # Test parent with subcommands
```

- [ ] **Step 3: Verify the web-routes manifest**

```bash
node dist/index.js schema --web-routes | python3 -c "import sys,json; d=json.load(sys.stdin); print(f'{len(d[\"commands\"])} routes')"
```

Should output 100+ routes.

- [ ] **Step 4: Delete old command files**

Delete all files listed in the "Deleted files" section above. Verify they're no longer imported anywhere:

```bash
grep -r "from.*commands/" src/ --include="*.ts" | grep -v "node_modules"
```

Should return no results (all imports should be from `registry/` now).

- [ ] **Step 5: Verify full build**

```bash
npx tsc --noEmit && npm run build
```

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "feat(cli): replace index.ts with registry-based entry point, delete old command files"
```

---

## Task 12: Web terminal integration (internal repo)

**Files:**
- Modify: `/root/repos/thecorporation-internal/services/chat-ws/Dockerfile`
- Modify: `/root/repos/thecorporation-internal/services/chat-ws/src/index.ts`
- Modify: `/root/repos/thecorporation-internal/ops/Caddyfile`
- Modify: `/root/repos/thecorporation-internal/services/web/packages/humans/src/pages/cli.astro`

- [ ] **Step 1: Update Dockerfile**

After the existing `cli-schema.json` generation line, add:

```dockerfile
RUN node packages/cli-ts/dist/index.js schema --web-routes > services/chat-ws/dist/web-routes.json 2>/dev/null || echo '{"commands":{}}' > services/chat-ws/dist/web-routes.json
```

- [ ] **Step 2: Update chat-ws `src/index.ts`**

Add a `/cli/web-routes.json` endpoint (mirror the existing `/cli/schema.json` pattern):

```typescript
if ((req.url === "/cli/web-routes.json" || req.url?.startsWith("/cli/web-routes.json?")) && req.method === "GET") {
  try {
    const routes = readFileSync(join(__dirname, "web-routes.json"), "utf-8");
    res.writeHead(200, {
      "Content-Type": "application/json",
      "Cache-Control": "public, max-age=3600",
      "Access-Control-Allow-Origin": "*",
    });
    res.end(routes);
  } catch {
    res.writeHead(200, { "Content-Type": "application/json" });
    res.end('{"commands":{}}');
  }
  return;
}
```

- [ ] **Step 3: Update Caddyfile**

After the `/cli/schema.json` handle block, add:

```caddy
handle /cli/web-routes.json {
    reverse_proxy chat-ws:8000
}
```

- [ ] **Step 4: Update cli.astro**

Three changes:

**A.** Replace the hardcoded `ROUTES` table (lines ~665-708) with:
```javascript
let ROUTES = {};
```

**B.** Update `loadSchema()` to also fetch web-routes.json:
```javascript
async function loadSchema() {
  const base = location.hostname === 'localhost' ? `http://${location.hostname}:8000` : '';
  try {
    const resp = await fetch(`${base}/cli/schema.json`);
    const raw = await resp.json();
    const cmds = raw.commands || [];
    for (const cmd of cmds) buildTree(cmd, commandTree);
    schemaMap = buildSchemaMap(cmds);
  } catch {}
  try {
    const resp = await fetch(`${base}/cli/web-routes.json`);
    const raw = await resp.json();
    ROUTES = raw.commands || {};
  } catch {}
}
```

**C.** Update the dispatch logic in `runCommand()` to use manifest flags (`write`, `local`, `custom`). Replace the subcommand detection block (around lines 1157-1195) with the manifest-driven version:

```javascript
// Determine route key deterministically from manifest
let routeKey = cmd;
let routePositional = positional;
if (positional.length > 0) {
  const subKey = `${cmd} ${positional[0]}`;
  if (ROUTES[subKey] || CUSTOM[subKey]) {
    routeKey = subKey;
    routePositional = positional.slice(1);
  }
}

const route = ROUTES[routeKey];

if (route?.local) {
  writeln(`'${routeKey}' is not available in the web terminal.`);
  return;
}
if (route?.write) {
  writeln(`${c(A.yellow, routeKey)} is not yet available in the web terminal.`);
  writeln(dim(`Run this from the CLI: corp ${routeKey}`));
  if (schema) { const parts = routeKey.split(' '); showSchemaHelp(parts[0], parts[1]); }
  return;
}
if (CUSTOM[routeKey]) { /* existing custom handler dispatch */ }
if (route?.path) { /* existing execRoute dispatch */ }
```

- [ ] **Step 5: Commit (internal repo)**

```bash
cd /root/repos/thecorporation-internal
git add services/chat-ws/Dockerfile services/chat-ws/src/index.ts ops/Caddyfile services/web/packages/humans/src/pages/cli.astro
git commit -m "feat(web): replace hardcoded ROUTES with fetched web-routes manifest"
```

---

## Task 13: Final verification

- [ ] **Step 1: Build and test CLI**

```bash
cd /root/repos/thecorporation-mono/packages/cli-ts
npm run build
node dist/index.js --help
node dist/index.js schema --web-routes | head -30
node dist/index.js next --json 2>&1 || true  # May fail without server, that's ok
```

- [ ] **Step 2: Run TypeScript checks**

```bash
npx tsc --noEmit
```

- [ ] **Step 3: Verify no old command imports remain**

```bash
grep -rn "from.*./commands/" src/ --include="*.ts" | grep -v __tests__ | grep -v node_modules
```

Should return zero results.

- [ ] **Step 4: Verify web-routes manifest completeness**

```bash
node dist/index.js schema --web-routes | python3 -c "
import sys, json
d = json.load(sys.stdin)
cmds = d['commands']
reads = sum(1 for v in cmds.values() if v.get('path') and not v.get('write') and not v.get('local'))
writes = sum(1 for v in cmds.values() if v.get('write'))
local = sum(1 for v in cmds.values() if v.get('local'))
custom = sum(1 for v in cmds.values() if v.get('custom'))
print(f'{len(cmds)} total: {reads} read, {writes} write, {custom} custom, {local} local')
"
```

Expected: 100+ total routes.

- [ ] **Step 5: Push both repos**

```bash
cd /root/repos/thecorporation-mono && git push origin main
cd /root/repos/thecorporation-internal && git push origin main
```
