# CLI Command Registry — Single Source of Truth

**Date:** 2026-03-20
**Status:** Approved

## Problem

The `@thecorporation/cli` TypeScript package and the web terminal (`cli.astro`) define the same command-to-API-endpoint mappings independently. The CLI has 29 command files covering ~105 subcommands with imperative fetch+display logic. The web terminal has a manual `ROUTES` table (~45 entries) that must be kept in sync. Every new command requires edits in both places. Subcommand detection in the web terminal is heuristic-based and fragile.

## Solution

Replace the imperative command files and manual route table with a **declarative command registry**. Each command is defined once as a data structure capturing its identity, API route, display format, and behavior. Both the CLI and web terminal are derived from this single source of truth.

## Architecture

### CommandDef — the registry entry type

```typescript
interface CommandDef {
  name: string;              // "contacts", "contacts add", "governance seats"
  description: string;
  aliases?: string[];        // "whoami" → "context"

  // API route (omit for local-only commands)
  route?: {
    method: "GET" | "POST" | "PUT" | "DELETE";
    path: string;            // "/v1/entities/{eid}/contacts"
  };

  // Entity scoping
  entity?: boolean | "query";  // true=in path, "query"=query param

  // Arguments and options
  args?: ArgDef[];
  options?: OptionDef[];
  optQP?: string[];            // CLI opts forwarded as query params (e.g. obligations --tier)

  // Display config (enables generic execution in both CLI and web)
  display?: {
    title: string;
    cols?: string[];           // column specs (see Column Spec Syntax below)
    listKey?: string;          // unwrap response[key] before display
  };

  // Custom handler (CLI-specific: multi-fetch, prompts, complex logic)
  handler?: (ctx: CommandContext) => Promise<void>;

  // Flags
  local?: boolean;             // no API call (setup, config, serve)
  hidden?: boolean;            // omit from help
  dryRun?: boolean;            // auto-add --dry-run option
  passThroughOptions?: boolean; // Commander's .passThroughOptions() (used by `form`)

  // Help text
  examples?: string[];         // shown after --help via addHelpText
}

interface ArgDef {
  name: string;
  required?: boolean;
  description?: string;
  variadic?: boolean;
  choices?: string[];
}

interface OptionDef {
  flags: string;               // "--json", "--entity-id <ref>", "--tier <tier>"
  description: string;
  required?: boolean;          // maps to Commander's .requiredOption()
  choices?: string[];
  default?: unknown;
  type?: "string" | "int" | "float" | "array";  // coerce/parse behavior
  // "int" → parseInt, "float" → parseFloat, "array" → accumulator function
  // "string" is the default; omit for most options
}
```

### CommandContext — what handlers receive

```typescript
interface CommandContext {
  client: CorpAPIClient;
  positional: string[];
  opts: Record<string, unknown>;
  entityId?: string;           // resolved entity ID (if entity command)
  resolver: ReferenceResolver; // resolves @last, short IDs, names → canonical IDs
  writer: OutputWriter;
  quiet: boolean;              // global --quiet flag
  dryRun: boolean;             // --dry-run flag (if dryRun: true on CommandDef)
}
```

### OutputWriter — shared output interface

```typescript
interface OutputWriter {
  writeln(text?: string): void;
  json(data: unknown): void;
  table(title: string, columns: string[], rows: unknown[][]): void;
  panel(title: string, color: string, lines: string[]): void;
  error(msg: string): void;
  success(msg: string): void;
  writeResult(result: Record<string, unknown>, kind?: string): void; // printWriteResult
  quietId(id: string): void;   // --quiet mode: just print the ID
}
```

### Column spec syntax

Used in `display.cols` arrays. Each spec is a string with optional prefix + field path + display label:

| Prefix | Meaning | Formatter |
|--------|---------|-----------|
| (none) | Raw string | `String(val)` |
| `$` | Money (cents) | `$X,XXX.XX` |
| `@` | Date | `YYYY-MM-DD` |
| `#` | Short ID | First 8 chars + `…` |

Field paths support fallbacks with `|`: `"investor_name|investor>Investor"` tries `investor_name` first, then `investor`.

Full format: `[prefix]field1|field2>Label`

Examples:
- `"name>Name"` — field `name`, header "Name"
- `"$amount_cents>Amount"` — field `amount_cents`, formatted as money, header "Amount"
- `"@due_date>Due"` — field `due_date`, formatted as date, header "Due"
- `"#obligation_id>ID"` — field `obligation_id`, truncated ID, header "ID"
- `"investor_name|investor>Investor"` — try `investor_name`, fallback to `investor`

### Classification rules (derived from fields, not declared)

| Has `display`? | Has `handler`? | `route.method` | `local`? | Classification | CLI behavior | Web terminal behavior |
|---|---|---|---|---|---|---|
| yes | no | GET | no | **generic read** | Generic executor | Generic route executor |
| yes | yes | GET | no | **custom read** | Handler always wins | Generic executor (has `display`) |
| no | yes | POST | no | **write** | Handler | "Run from CLI" with help |
| no | no | — | yes | **local** | Handler | "Not available" |
| no | yes | — | no | **informational** | Handler | "Run from CLI" |

When both `display` and `handler` are present, the CLI **always uses the handler**. The `display` field is used only for web manifest generation (so the web terminal can execute generically).

### Parent-child option inheritance

Commands with space-separated names (e.g., `"governance seats"`) establish a parent-child relationship. `buildCLI()` handles this:

1. `"governance"` is registered as a Commander group command
2. `"governance seats"` is registered as a subcommand under it
3. Parent-level options (`--entity-id`, `--json`, `--dry-run`) are automatically inherited by all child commands via Commander's `.passThroughOptions()` and `inheritOption()` pattern
4. The handler's `ctx.opts` includes both parent and child options (merged)

The `entity` field on a parent command (e.g., `"governance"` with `entity: true`) does NOT automatically propagate to children. Each child command declares its own `entity` scoping because some children use `entity: true` (in path) while others use `entity: "query"` (query param) or have no entity at all.

### File structure

```
packages/cli-ts/src/
  registry/
    types.ts              # CommandDef, ArgDef, OptionDef, CommandContext, OutputWriter
    index.ts              # aggregates all registry files, exports full registry array
    workspace.ts          # status, context, use, next, billing, obligations, digests
    entities.ts           # entities, entities show, entities convert, entities dissolve
    formation.ts          # form, form create, form add-founder, form finalize, form activate
    governance.ts         # governance + all 17 subcommands
    cap-table.ts          # cap-table + all 18 subcommands
    documents.ts          # documents, signing-link, sign, sign-all, generate, preview-pdf
    finance.ts            # finance + all 14 subcommands
    compliance.ts         # tax, filings, deadlines, tax file, tax deadline
    agents.ts             # agents + all 8 subcommands
    services.ts           # services, catalog, list, show, buy, fulfill, cancel
    work-items.ts         # work-items, show, create, claim, complete, release, cancel
    admin.ts              # setup, config (set/get/list), schema, serve, demo, api-keys,
                          # chat, link, claim, feedback, resolve, find, approvals
  cli.ts                  # buildCLI(registry) → Commander program
  generic-executor.ts     # shared fetch+display logic for generic read commands
  output.ts               # existing output helpers (unchanged)
  config.ts               # existing config helpers (unchanged)
  api-client.ts           # existing API client re-export (unchanged)
  references.ts           # existing ReferenceResolver (unchanged)
  spinner.ts              # existing spinner (unchanged)
  index.ts                # entry point: import registry, buildCLI, parse argv
```

### How it works

#### CLI side

`buildCLI(registry)` iterates every `CommandDef` and creates Commander `.command()` entries:

- **Generic read commands** (has `display`, no `handler`): Wires up the generic executor which calls `client.get(route.path)` inside `withSpinner()`, resolves `{eid}`/`{pos}`/`{wid}` placeholders using the `ReferenceResolver`, and displays using `display.cols` or auto-detected columns.
- **Custom handlers** (has `handler`): Wires up the handler function. The handler receives a `CommandContext` with resolved client, positional args, merged opts (parent + child), entity ID, reference resolver, and output writer.
- **Local commands** (has `local: true`): Wires up the handler (e.g., setup, config).

For options with `required: true`, `buildCLI` uses Commander's `.requiredOption()`. For options with `type: "int"`, it attaches `parseInt` as the coerce function. For `type: "array"`, it attaches an accumulator `(val, prev) => [...(prev || []), val]`.

If `dryRun: true` is set on the `CommandDef`, `buildCLI` automatically adds a `--dry-run` option.

Global options (`--quiet`, `--json`) are registered on the program and propagated to all commands via `CommandContext`.

#### Generic executor

`generic-executor.ts` implements the shared fetch+display logic used by both the CLI (for generic read commands) and conceptually mirrored by the web terminal's `execRoute()`:

1. Resolve `{eid}` → entity ID (via `ReferenceResolver` or `--entity-id` option)
2. Resolve `{pos}` → positional argument
3. Resolve `{wid}` → workspace ID
4. Forward `optQP` options as query params
5. Call `client.get(path, params)` inside `withSpinner()`
6. Unwrap `listKey` if specified
7. Display as table (if array) or panel (if object) using `display.cols` or auto-detect

#### Web manifest generation

`generateWebRoutes(registry)` iterates the registry and emits a JSON manifest with only the fields the web terminal needs (no handler functions, no TS-specific config):

```json
{
  "commands": {
    "contacts": {
      "method": "GET",
      "path": "/v1/entities/{eid}/contacts",
      "entity": true,
      "title": "Contacts",
      "cols": ["name>Name", "email>Email", "category>Category", "#contact_id>ID"]
    },
    "contacts add": { "method": "POST", "write": true },
    "setup": { "local": true },
    "status": { "method": "GET", "custom": true, "title": "Corp Status" },
    "obligations": {
      "method": "GET",
      "path": "/v1/obligations/summary",
      "title": "Obligations",
      "listKey": "obligations",
      "cols": ["obligation_type>Type", "urgency>Urgency", "@due_at>Due", "status>Status", "#obligation_id>ID"],
      "optQP": ["tier"]
    }
  }
}
```

A command gets `"custom": true` in the manifest when it has both `display` and `handler` — telling the web terminal to use its own CUSTOM handler rather than the generic executor.

A command gets `"write": true` when `route.method !== "GET"` and no `display` is present.

Generated at Docker build time: `corp schema --web-routes > web-routes.json`

#### Schema generation

`generateSchema(registry)` iterates the registry and emits cli-schema.json for tab completion. Same shape as today — name, description, args, options, subcommands — but derived from the registry instead of Commander introspection. The space-separated `name` field is used to reconstruct the parent-child hierarchy.

#### Web terminal

`cli.astro` fetches `/cli/web-routes.json` at startup (alongside `/cli/schema.json`). The fetched manifest replaces the hardcoded `ROUTES` table. The existing `execRoute()` function and `CUSTOM` handlers work as-is — they're just fed from the manifest instead of a static table.

Subcommand detection becomes deterministic: `if (manifest[cmd + ' ' + sub])` — no heuristics, no guessing whether `add` is a subcommand or entity reference.

### Migration strategy

The ~105 subcommands across 29 command files fall into three buckets:

1. **Pure read → collapse into registry entries** (~30 subcommands): contacts, obligations, entities list, governance bodies/seats/meetings/resolutions/agenda-items/incidents/profile/mode, documents list, finance invoices/payments/bank-accounts/payroll/distributions/reconciliations/classifications/statements, agents list/show, digests, work-items list/show, tax filings/deadlines, services catalog/list/show, cap-table safes/transfers/instruments/share-classes/rounds/valuations/409a/dilution/control-map. These are pure fetch+table — the command file is deleted entirely.

2. **Custom display → registry entry + handler** (~6 commands): status, entities show, cap-table (summary), finance (summary), billing, next, context. The handler contains multi-fetch or panel-formatting logic.

3. **Write/interactive → registry entry + handler** (~40+ subcommands): all POST/PUT operations across form, contacts, governance, cap-table, finance, tax, agents, work-items, services, billing, documents, entities, api-keys, plus link, claim, feedback, demo. The handler contains input gathering, validation, and the API call.

Commands with handlers keep their logic — it's just relocated from `commands/*.ts` into the registry file alongside the declaration. The handler function body is unchanged.

### Files to create/modify

**New files (mono repo — packages/cli-ts/):**
- `src/registry/types.ts` — CommandDef, ArgDef, OptionDef, CommandContext, OutputWriter types
- `src/registry/index.ts` — aggregates all registry files, exports full array
- `src/registry/workspace.ts` — status, context, use, next, billing, obligations, digests
- `src/registry/entities.ts` — entities, entities show/convert/dissolve
- `src/registry/formation.ts` — form, form create/add-founder/finalize/activate
- `src/registry/governance.ts` — governance + 17 subcommands
- `src/registry/cap-table.ts` — cap-table + 18 subcommands
- `src/registry/documents.ts` — documents + 5 subcommands
- `src/registry/finance.ts` — finance + 14 subcommands
- `src/registry/compliance.ts` — tax + 4 subcommands
- `src/registry/agents.ts` — agents + 8 subcommands
- `src/registry/services.ts` — services + 5 subcommands
- `src/registry/work-items.ts` — work-items + 6 subcommands
- `src/registry/admin.ts` — setup, config, schema, serve, demo, api-keys, chat, link, claim, feedback, resolve, find, approvals
- `src/cli.ts` — `buildCLI()` function
- `src/generic-executor.ts` — shared fetch+display for generic read commands

**Modified files (mono repo):**
- `src/index.ts` — simplified to: import registry, buildCLI, parse
- `src/commands/schema.ts` — add `--web-routes` flag, generate from registry

**Deleted files (mono repo):**
- Most of `src/commands/*.ts` — logic relocated into registry handler functions
- Exact list determined during implementation; some files may be kept if they contain shared utilities

**Modified files (internal repo):**
- `services/chat-ws/Dockerfile` — add web-routes.json generation step
- `services/chat-ws/src/index.ts` — serve `/cli/web-routes.json`
- `ops/Caddyfile` — route `/cli/web-routes.json` to chat-ws
- `services/web/packages/humans/src/pages/cli.astro` — replace hardcoded ROUTES with fetched manifest

## Scope exclusions

- The web terminal's `CUSTOM` handlers (status, cap-table, finance, next, billing) stay in cli.astro — flagged as `custom: true` in the manifest
- The web terminal's `execRoute()`, output helpers, and shell logic are NOT changed
- No new features — this is a structural refactor for DRYness
- The `CorpAPIClient` class is NOT changed
- `ReferenceResolver`, `config.ts`, `output.ts`, `spinner.ts` are NOT changed
