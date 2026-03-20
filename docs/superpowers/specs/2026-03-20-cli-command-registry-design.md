# CLI Command Registry — Single Source of Truth

**Date:** 2026-03-20
**Status:** Approved

## Problem

The `@thecorporation/cli` TypeScript package and the web terminal (`cli.astro`) define the same command-to-API-endpoint mappings independently. The CLI has 29 command files with imperative fetch+display logic. The web terminal has a manual `ROUTES` table (~45 entries) that must be kept in sync. Every new command requires edits in both places. Subcommand detection in the web terminal is heuristic-based and fragile.

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
  optQP?: string[];            // CLI opts forwarded as query params

  // Display config (enables generic execution in both CLI and web)
  display?: {
    title: string;
    cols?: string[];           // column specs: "name>Name", "$amount>Money"
    listKey?: string;          // unwrap response[key] before display
  };

  // Custom handler (CLI-specific: multi-fetch, prompts, complex logic)
  handler?: (ctx: CommandContext) => Promise<void>;

  // Flags
  local?: boolean;             // no API call (setup, config, serve)
  hidden?: boolean;            // omit from help
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
  choices?: string[];
  default?: unknown;
}

interface CommandContext {
  client: CorpAPIClient;
  positional: string[];
  opts: Record<string, unknown>;
  entityId?: string;           // resolved entity ID (if entity command)
  writer: OutputWriter;        // writeln, writeTable, writePanel, etc.
}
```

### Classification rules (derived from fields, not declared)

| Has `display`? | Has `handler`? | `route.method` | `local`? | Classification | Web terminal behavior |
|---|---|---|---|---|---|
| yes | no | GET | no | **generic read** | Executes via generic route executor |
| yes | yes | GET | no | **custom read** | CLI uses handler; web uses generic executor |
| no | yes | POST | no | **write** | Shows "run from CLI" with help text |
| no | no | — | yes | **local** | Shows "not available" |
| no | yes | GET | no | **custom** | CLI uses handler; web shows "run from CLI" unless display present |

### File structure

```
packages/cli-ts/src/
  registry/
    index.ts              # CommandDef types, registry array, defineCommand()
    workspace.ts          # status, context, use, next, billing, obligations
    entities.ts           # entities, entities show, form *
    governance.ts         # governance, seats, meetings, resolutions, votes, ...
    cap-table.ts          # cap-table, instruments, safes, rounds, transfers, ...
    documents.ts          # documents, signing-link, sign, generate, ...
    finance.ts            # finance, invoices, payments, bank-accounts, ...
    compliance.ts         # tax, filings, deadlines
    agents.ts             # agents, show, create, message, ...
    services.ts           # services, catalog, buy, ...
    work-items.ts         # work-items, create, claim, ...
    admin.ts              # setup, config, schema, serve, demo, api-keys, chat
  cli.ts                  # buildCLI(registry) → Commander program
  generic-executor.ts     # shared fetch+display logic for generic read commands
  output.ts               # existing output helpers (unchanged)
  config.ts               # existing config helpers (unchanged)
  api-client.ts           # existing API client re-export (unchanged)
  index.ts                # entry point: import registry, buildCLI, parse argv
```

### How it works

#### CLI side

`buildCLI(registry)` iterates every `CommandDef` and creates Commander `.command()` entries:

- **Generic read commands** (has `display`, no `handler`): Wires up the generic executor which calls `client.get(route.path)`, resolves `{eid}`/`{pos}` placeholders, and displays using `display.cols` or auto-detect.
- **Custom handlers** (has `handler`): Wires up the handler function. The handler receives a `CommandContext` with the resolved client, positional args, opts, and output writer.
- **Local commands** (has `local: true`): Wires up the handler (e.g., setup, config).

The generic executor is identical in logic to the web terminal's `execRoute()` — resolve entity, substitute path params, fetch, display table/panel.

#### Web manifest generation

`generateWebRoutes(registry)` iterates the registry and emits a JSON manifest:

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
    "status": { "method": "GET", "custom": true, "title": "Corp Status" }
  }
}
```

Generated at Docker build time: `corp schema --web-routes > web-routes.json`

#### Schema generation

`generateSchema(registry)` iterates the registry and emits cli-schema.json for tab completion. Same shape as today's schema output — name, description, args, options, subcommands — but derived from the registry instead of Commander introspection.

#### Web terminal

`cli.astro` fetches `/cli/web-routes.json` at startup (alongside `/cli/schema.json`). The fetched manifest replaces the hardcoded `ROUTES` table. The existing `execRoute()` function and `CUSTOM` handlers work as-is — they're just fed from the manifest.

Subcommand detection becomes deterministic: `if (manifest[cmd + ' ' + sub])` — no heuristics, no guessing whether `add` is a subcommand or entity reference.

### Example: adding a new read command

```typescript
// registry/compliance.ts
{
  name: "tax filings",
  description: "List tax filings",
  route: { method: "GET", path: "/v1/entities/{eid}/tax-filings" },
  entity: true,
  display: {
    title: "Tax Filings",
    cols: ["document_type>Type", "tax_year>Year", "status>Status", "#filing_id>ID"],
  },
}
```

Works in CLI and web. No other files to touch.

### Example: adding a write command

```typescript
// registry/governance.ts
{
  name: "governance vote",
  description: "Cast a vote in a meeting",
  route: { method: "POST", path: "/v1/meetings/{pos}/votes" },
  entity: "query",
  args: [{ name: "meeting-id", required: true }],
  options: [
    { flags: "--value <value>", description: "Vote value", choices: ["yes", "no", "abstain"] },
    { flags: "--seat-id <id>", description: "Seat to vote from" },
  ],
  handler: async (ctx) => {
    // CLI-specific: interactive prompts, validation, POST call
  },
}
```

CLI runs the handler. Web terminal shows "governance vote is not yet available in the web terminal" with help text from the schema.

### Example: custom display command

```typescript
// registry/workspace.ts
{
  name: "status",
  description: "Workspace summary",
  route: { method: "GET", path: "/v1/workspaces/{wid}/status" },
  display: { title: "Corp Status" },
  handler: async (ctx) => {
    const data = await ctx.client.getStatus();
    if (ctx.opts.json) { ctx.writer.json(data); return; }
    ctx.writer.panel("Corp Status", "blue", [
      `Workspace: ${data.workspace_id}`,
      `Entities: ${data.entity_count}`,
    ]);
  },
}
```

CLI uses the handler for the custom panel. Web terminal sees `display` is present + `custom: true` in the manifest, so its existing `CUSTOM.status()` handler runs. Over time, the web terminal's CUSTOM handlers can also be collapsed if the `handler` + `writer` abstraction covers both environments.

### Migration strategy

The 29 existing command files fall into three buckets:

1. **Pure read → collapse into registry entries** (~20 commands): contacts, obligations, governance bodies/seats/meetings/resolutions/agenda-items/incidents/profile/mode, documents, finance invoices/payments/bank-accounts/payroll/distributions/reconciliations/classifications/statements, agents, digests, work-items, tax filings/deadlines, services catalog/list, valuations, safe-notes. These are just fetch+table — the command file is deleted entirely.

2. **Custom display → registry entry + handler function** (~5 commands): status, cap-table (summary), finance (summary), billing, next. The handler contains the multi-fetch or panel-formatting logic.

3. **Write/interactive → registry entry + handler function** (~4+ commands): form create/finalize/activate, contacts add/edit, governance vote/convene/written-consent, documents sign/generate, finance invoice/pay/open-account, etc. The handler contains input gathering and POST logic.

### Files to create/modify

**New files (mono repo — packages/cli-ts/):**
- `src/registry/index.ts` — types, registry array, `defineCommand()`
- `src/registry/workspace.ts` — status, context, use, next, billing, obligations
- `src/registry/entities.ts` — entities, form
- `src/registry/governance.ts` — governance commands
- `src/registry/cap-table.ts` — cap table commands
- `src/registry/documents.ts` — document commands
- `src/registry/finance.ts` — finance commands
- `src/registry/compliance.ts` — tax, deadlines
- `src/registry/agents.ts` — agent commands
- `src/registry/services.ts` — service commands
- `src/registry/work-items.ts` — work item commands
- `src/registry/admin.ts` — local/admin commands
- `src/cli.ts` — `buildCLI()` function
- `src/generic-executor.ts` — shared fetch+display for generic read commands

**Modified files (mono repo):**
- `src/index.ts` — simplified to: import registry, buildCLI, parse
- `src/commands/schema.ts` — add `--web-routes` flag, generate from registry

**Deleted files (mono repo):**
- `src/commands/status.ts` → inlined as handler in registry/workspace.ts
- `src/commands/obligations.ts` → collapsed into registry entry
- `src/commands/context.ts` → inlined as handler
- (and ~15 more pure-read command files)

**Modified files (internal repo):**
- `services/chat-ws/Dockerfile` — add web-routes.json generation step
- `services/chat-ws/src/index.ts` — serve `/cli/web-routes.json`
- `ops/Caddyfile` — route `/cli/web-routes.json` to chat-ws
- `services/web/packages/humans/src/pages/cli.astro` — replace hardcoded ROUTES with fetched manifest

## Scope exclusions

- The web terminal's `CUSTOM` handlers (status, cap-table, finance, next, billing) are NOT rewritten in this change — they stay in cli.astro and are flagged as `custom: true` in the manifest
- The web terminal's `execRoute()`, output helpers, and shell logic are NOT changed
- No new features — this is a structural refactor for DRYness
- The `CorpAPIClient` class is NOT changed
