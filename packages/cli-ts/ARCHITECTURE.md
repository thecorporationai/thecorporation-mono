# CLI Architecture Guide

This document is the definitive reference for maintaining and extending the `corp` CLI. It describes how the system is organized, how a command travels from the terminal to the API, and the exact conventions to follow when adding new commands.

---

## 1. Directory Structure

```
packages/cli-ts/
├── src/
│   ├── cli.ts                   # Engine: buildCLI(), wireCommand()
│   ├── generic-executor.ts      # Auto-handler for GET/POST/PATCH/DELETE
│   ├── references.ts            # ReferenceResolver (Node adapter + caching)
│   ├── resource-kinds.ts        # KINDS and ENTITY_SCOPED_KINDS sets
│   ├── config.ts                # Config I/O, file lock, updateConfig/saveConfig
│   ├── output.ts                # Display helpers and domain table printers
│   ├── writer.ts                # OutputWriter factory (createWriter)
│   ├── api-client.ts            # Re-exports CorpAPIClient from corp-tools
│   ├── formation-automation.ts  # Multi-step formation workflow helpers
│   ├── spinner.ts               # withSpinner() for long-running calls
│   ├── types.ts                 # CorpConfig, ApiRecord, shared type aliases
│   └── registry/
│       ├── types.ts             # CommandDef, ArgDef, OptionDef, CommandContext, OutputWriter
│       ├── index.ts             # registry[], generateWebRoutes(), generateSchema()
│       ├── workspace.ts
│       ├── entities.ts
│       ├── formation.ts
│       ├── cap-table.ts
│       ├── finance.ts
│       ├── governance.ts
│       ├── documents.ts
│       ├── compliance.ts
│       ├── agents.ts
│       ├── work-items.ts
│       ├── services.ts
│       ├── admin.ts
│       ├── execution.ts
│       ├── secret-proxies.ts
│       ├── treasury.ts
│       └── branches.ts
```

**Rule of thumb:** Domain commands live exclusively in `registry/`. The files in `src/` root are shared infrastructure that commands consume; they do not define commands themselves.

---

## 2. Command Lifecycle

For a concrete command like `corp cap-table issue-equity`:

```
Terminal input
  │
  ▼
Commander parse
  │  buildCLI() turns registry[] into a Commander program.
  │  "cap-table" is the parent; "issue-equity" is attached as a subcommand.
  │
  ▼
wireCommand() — cmd.action(async (...) => { ... })
  │  1. Extracts positional args and opts from Commander's callback.
  │  2. Merges parent opts (--quiet, --json, --entity-id, --dry-run).
  │  3. Creates an OutputWriter via createWriter().
  │  4. For API commands: calls requireConfig(), builds CorpAPIClient + ReferenceResolver.
  │  5. Resolves entityId for entity-scoped commands.
  │  6. Builds CommandContext and dispatches:
  │       def.handler  → custom handler (full control)
  │       def.display  → executeGenericRead()
  │       non-GET route → executeGenericWrite()
  │
  ▼
handler / executeGenericRead / executeGenericWrite
  │  • Resolves path placeholders: {eid}, {pos}, {pos2}, {wid}
  │  • Calls ctx.client.fetchJSON() or ctx.client.submitJSON()
  │
  ▼
OutputWriter
  │  • ctx.writer.table() / panel() / success() / json()
  │
  ▼
@last hint (cli.ts, post-handler)
     After a generic write that declares produces.kind, cli.ts prints:
     "  Ref: @last:<kind> → <short-id>"
```

`process.exit(1)` is called only inside `wireCommand()`'s catch block and in `requireConfig()`. Everything else throws.

---

## 3. Registry System

### 3.1 CommandDef Contract

Every command is a plain `CommandDef` object in `registry/types.ts`. Key fields:

| Field | Purpose |
|---|---|
| `name` | Space-separated command path: `"governance create-body"`. Single word = top-level. |
| `description` | Shown in `--help`. Keep it short (imperative, no trailing period). |
| `route` | `{ method, path }` with `{eid}`, `{pos}`, `{pos2}`, `{wid}` placeholders. |
| `entity` | `true` → entity ID injected into path as `{eid}`. `"query"` → sent as `?entity_id=`. |
| `args` | Positional arguments in declaration order. Use `posKind` for reference resolution. |
| `options` | Named flags. Use `choices[]` for any enum. `type: "int"/"float"/"array"` for coercion. |
| `optQP` | Option names forwarded as query parameters on GET requests. |
| `display` | `{ title, cols?, listKey? }` — drives `executeGenericRead` output rendering. |
| `handler` | Custom async function. When present, skips all generic executors entirely. |
| `local` | Skip API client setup; handler receives `client: null`. For offline commands. |
| `hidden` | Suppressed from `--help` and `web-routes.json`. |
| `dryRun` | Adds `--dry-run` flag; executor checks `ctx.dryRun` before submitting. |
| `produces` | Declares the resource this command creates (see §3.4). |
| `successTemplate` | Human-readable string like `"Created {name} ({body_id})"`. Consumed by web terminal. |
| `examples` | Shown under `--help`. Must be full invocations including all required args. |
| `aliases` | Commander `.alias()` shorthands. |
| `passThroughOptions` | Forward unknown flags to a child process. |

### 3.2 How Generic Executors Work

**`executeGenericRead`** (triggered when `def.display` is set and no `handler`):

1. Resolves `{eid}` from config or `--entity-id`.
2. Resolves `{pos}` via `resolvePositional()` (uses `posKind` when set, otherwise raw value).
3. Appends `optQP` keys as query parameters.
4. Calls `ctx.client.fetchJSON(path, qp)`.
5. If `listKey` is set, unwraps `data[listKey]` before rendering.
6. Renders array → `ctx.writer.table()`, object → panel, else JSON.
7. Column specs in `cols` use a mini-DSL (see §3.3).

**`executeGenericWrite`** (triggered when `def.route.method !== "GET"` and no `handler`):

1. Resolves `{eid}`, `{pos}`, `{pos2}` with the same logic as the read executor.
2. Builds the request body from `def.options`: each `--foo-bar` flag becomes `foo_bar` in the body. `entity_id` is injected when `def.entity` is set and `ctx.entityId` is available.
3. Calls `ctx.writer.dryRun()` early if `ctx.dryRun`.
4. Calls `ctx.client.submitJSON(method, path, body)`.
5. Prints `ctx.writer.success()` with an ID extracted from the response.

When a custom `handler` is present, neither executor runs. The handler has full control over every step.

### 3.3 Column Spec DSL

`display.cols` is an array of spec strings parsed by `parseCol()` in `generic-executor.ts`:

```
[$|@|#]<field>[|<altField>...][><Label>]
```

- `$field` → format as money (cents ÷ 100)
- `@field` → format as ISO date
- `#field` → format as short ID (first 8 chars)
- `field|alt` → try `field`, fall back to `alt`
- `field>Label` → use `Label` as the column header instead of the field name

Example: `"$principal_amount_cents|investment_amount|amount>Amount"` shows a money-formatted column labeled "Amount", trying three field names in order.

### 3.4 The `entity` Flag and Entity Resolution

`entity: true` means the command is scoped to a specific legal entity, and the executor will replace `{eid}` in the path with the resolved entity ID.

- For custom handlers: call `ctx.resolver.resolveEntity(ctx.opts.entityId as string | undefined)` at the start of every handler that needs the entity. This handles `--entity-id`, the active entity from config, and `@last:entity` references.
- For generic executors (no handler): `wireCommand` performs a soft resolve and passes `ctx.entityId`. The executor handles missing entity gracefully.
- `entity: "query"` means the entity ID is sent as `?entity_id=<eid>` instead of as a path segment. Use this when the resource endpoint is not nested under `/entities/{eid}/`.

### 3.5 The `produces` Metadata and @last Tracking

`produces` tells the system what resource kind a write command creates:

```typescript
produces: {
  kind: "body",           // ResourceKind — what was created
  idField?: "body_id",   // Response field with the ID (default: "${kind}_id")
  trackEntity?: true,    // Also set the active entity from the response
}
```

After a generic write that declares `produces.kind`, `wireCommand` automatically:
1. Reads the last remembered ID for that kind from the resolver.
2. Prints `  Ref: @last:<kind> → <short-id>`.

This means users can immediately use `@last:body` in the next command without copying an ID.

Custom handlers should call `ctx.resolver.rememberFromRecord(kind, result, entityId)` explicitly to register the resource so `@last` works.

`successTemplate` is a companion field used by the web terminal. Interpolate field names from the API response in curly braces: `"Created {name} ({body_id})"`.

---

## 4. Reference Resolution

### 4.1 What Can Be a Reference

The `ReferenceResolver` accepts four input forms for any resource kind:

| Form | Example | Behavior |
|---|---|---|
| Full UUID | `3fa85f64-5717-4562-b3fc-2c963f66afa6` | Returned as-is, remembered |
| Short ID | `3fa85f64` | Prefix-matched against the full UUID list |
| Label / handle | `alice` or `board-of-directors` | Token-matched against `describeReferenceRecord()` output |
| `@last` reference | `@last` or `@last:body` | Recalled from config's `last_references` map |

If a reference is ambiguous (multiple matches), the resolver throws with a list of candidates and a `corp find <kind> <query>` hint. Never silently pick one.

### 4.2 The Resolver Cache

`ReferenceResolver` holds one in-memory `Map<string, ApiRecord[]>` per entity-scoped resource kind (e.g. `contactsCache`, `bodiesCache`). The cache key is the `entityId` string, or `"entityId:bodyId"` for hierarchical resources like meetings.

For workspace-scoped kinds (`entity`, `agent`), a single array is cached for the lifetime of the resolver instance.

**Implication:** the resolver is constructed once per command invocation in `wireCommand`. Caches are not shared between invocations. If a handler calls `resolver.resolveBody(eid, ref)` twice, only one API call is made.

The `attachStableHandles()` method is called after every `listRecords()` call. It batch-syncs any records missing a `handle` field with the API in chunks of 400, enabling stable human-readable references. Custom handlers call `ctx.resolver.stabilizeRecords(kind, records, eid)` to trigger this.

### 4.3 `posKind` and Generic Executor Resolution

When a positional argument declares `posKind`, the generic executor resolves it through `ctx.resolver.resolveByKind(kind, rawValue, entityId)` before URL-encoding:

```typescript
args: [
  { name: "body-ref", required: true, posKind: "body" }
]
route: { method: "GET", path: "/v1/governance-bodies/{pos}/seats" }
```

Without `posKind`, the raw string is URL-encoded directly. This backward-compatible default allows commands that take IDs from other sources (e.g. already-resolved UUIDs from a parent command) without triggering a network lookup.

### 4.4 `@last` Persistence

`@last` references are stored in `~/.corp/config.json` under `last_references`, keyed by a scoped string:

```
workspace:<wid>:entity:<eid>:<kind>   — entity-scoped resources
workspace:<wid>:<kind>                — workspace-scoped resources
```

The map is capped at 4096 entries (FIFO eviction). Write via `resolver.remember(kind, id, entityId)` or `resolver.rememberFromRecord(kind, record, entityId)`. Read via `resolver.getLastId(kind, entityId)`.

---

## 5. Output System

### 5.1 Writer vs Output

There are two output layers:

**`src/output.ts`** — low-level display primitives and domain-specific table renderers. Functions like `printSuccess`, `printError`, `printCapTable`, `printGovernanceTable`, `money()`, `date()`, `s()` live here. These functions call `console.log` directly and return void. Custom handlers import and call them when they need rich, domain-specific output.

**`src/writer.ts`** (`createWriter()`) — creates an `OutputWriter` instance conforming to the `OutputWriter` interface in `registry/types.ts`. This is the object passed as `ctx.writer` to every command. It wraps the low-level functions from `output.ts` behind a consistent interface.

Use `ctx.writer` inside handlers. Import from `output.ts` directly only for domain-specific renderers that do not fit the generic `table/panel/success` abstractions.

### 5.2 OutputWriter Interface

```typescript
ctx.writer.writeln(text?)        // raw console.log
ctx.writer.json(data)            // pretty-printed JSON
ctx.writer.table(title, cols, rows) // cli-table3 with bold title
ctx.writer.panel(title, color, lines) // colored bordered panel
ctx.writer.error(msg)            // red "Error:" prefix to stderr
ctx.writer.success(msg)          // green text
ctx.writer.warning(msg)          // yellow text
ctx.writer.writeResult(result, message, options?) // write result with optional reference summary
ctx.writer.quietId(id)           // prints bare ID for scripting
ctx.writer.dryRun(operation, payload) // JSON dry-run preview
```

Check `ctx.opts.json` before printing human-formatted output. Pattern:

```typescript
if (ctx.opts.json) { ctx.writer.json(data); return; }
// ... human-formatted output below ...
```

### 5.3 Formatting Helpers in output.ts

| Helper | Purpose |
|---|---|
| `s(val, maxLen?)` | Safe string coercion; returns `""` for null/undefined |
| `money(val, cents?)` | Formats a number as USD, divides by 100 when `cents=true` (default) |
| `date(val)` | Parses and formats any date-like value to `YYYY-MM-DD` |
| `shortId(id)` | Returns the first 8 characters of a UUID |
| `printReferenceSummary(kind, record, opts)` | Prints Ref + ID + optional `@last` hint block |
| `printWriteResult(result, msg, options)` | Handles quiet/json/normal write output uniformly |

---

## 6. Config System

### 6.1 Files

Config is split across two files to separate sensitive credentials:

- `~/.corp/config.json` — non-sensitive: `hosting_mode`, `llm.*` (no key), `user.*`, `active_entity_id`, `active_entity_ids`, `last_references`
- `~/.corp/auth.json` — sensitive: `api_url`, `api_key`, `workspace_id`, `llm.api_key`, `server_secrets`

`loadConfig()` reads both files and merges them (auth values win). `saveConfig()` writes both files in a single lock. Both files are `chmod 600`; the directory is `chmod 700`.

### 6.2 The File Lock

All reads and writes go through `withConfigLock()`, which uses `mkdirSync` on `~/.corp/config.lock` as a POSIX-atomic mutex:

- Acquires by creating the directory (fails with `EEXIST` if held).
- Retries every 25 ms with a busy spin.
- Times out after 5 seconds.
- Considers locks older than 60 seconds stale and removes them.

**`updateConfig(mutator)`** is the correct way to mutate config from a running handler. It acquires the lock, reads the current on-disk state, applies the mutation, and writes atomically. This prevents race conditions when two CLI processes run concurrently (e.g. a script issuing multiple commands in parallel).

**`saveConfig(cfg)`** takes an already-modified `CorpConfig` and writes it. Use only when you hold a complete config object and want to replace it wholesale (e.g. `corp setup`).

Do not call `writeFileSync` on the config files directly. Always go through `saveConfig` or `updateConfig`.

### 6.3 Allowed Keys

Only keys in `ALLOWED_CONFIG_KEYS` can be set via `corp config set`. The set includes `api_url`, `api_key`, `workspace_id`, `hosting_mode`, `llm.*`, `user.*`, `active_entity_id`, `data_dir`. Attempts to set undeclared keys throw.

`api_url`, `api_key`, and `workspace_id` are additionally gated behind a `--force` flag when set via `setValue()` to prevent accidental credential replacement.

---

## 7. Patterns and Conventions

### 7.1 Handlers Throw, Never `process.exit`

Handlers must throw `Error` for all failure cases. `process.exit(1)` is reserved for `wireCommand`'s top-level catch and `requireConfig`. This keeps handlers testable and composable.

```typescript
// Correct
if (!bodyRef) throw new Error("Missing required argument <body-ref>.");

// Wrong
if (!bodyRef) { console.error("missing"); process.exit(1); }
```

### 7.2 Use `posKind` for Positional Reference Resolution

When a positional argument accepts a reference that should resolve through `@last`, short IDs, or handles, declare `posKind` on the `ArgDef`:

```typescript
args: [{ name: "instrument-ref", required: true, posKind: "instrument" }]
```

The generic executor then automatically calls `ctx.resolver.resolveByKind("instrument", rawValue, entityId)`. Without `posKind`, only raw UUID passthrough works.

### 7.3 Use `choices[]` on Enum Flags

Commander validates the value at parse time when `choices` is set on an `OptionDef`. This produces a clean error message before the handler ever runs:

```typescript
{ flags: "--body-type <type>", description: "...", required: true,
  choices: ["board_of_directors", "llc_member_vote"] }
```

Never validate enum values inside a handler when `choices` can do it.

### 7.4 Examples Must Be Complete

Every entry in `examples` is shown verbatim under `--help`. Include all required flags and realistic placeholder values. Use `@last:<kind>` for ID arguments to demonstrate the reference system.

```typescript
examples: [
  "corp governance create-body --name 'Board of Directors' --body-type board_of_directors",
  "corp governance add-seat @last:body --holder alice",
]
```

### 7.5 `successTemplate` Must Be Human-Readable

`successTemplate` is consumed by the web terminal to render the result of a write command. Write it as a complete sentence fragment that makes sense without additional context:

```typescript
successTemplate: "Governance body created: {name}"
```

Field names in `{braces}` are interpolated from the API response object. Always include at least one human-readable field (name, title, etc.) rather than relying solely on an ID.

### 7.6 Stable Handle Hydration

After fetching a list of records for display, call `ctx.resolver.stabilizeRecords(kind, records, entityId)`. This batch-syncs missing handles so the Ref column shows `alice [3fa85f64]` instead of a bare UUID. Skip this only for read-only pass-through to `--json` or when the records come from a sub-field that the resolver does not track.

### 7.7 Check `ctx.opts.json` Before Printing

All custom handlers must check `ctx.opts.json` and emit `ctx.writer.json(data)` before any formatted output. This keeps `--json` output machine-parseable regardless of how elaborate the normal display is.

---

## 8. Corp-Tools Integration

`src/api-client.ts` is a single re-export:

```typescript
export { CorpAPIClient, SessionExpiredError, provisionWorkspace } from "@thecorporation/corp-tools";
```

All HTTP calls go through `CorpAPIClient`. The client is instantiated once per command invocation in `wireCommand` using the loaded config (`api_url`, `api_key`, `workspace_id`).

The `@thecorporation/corp-tools` package also exports:

- **`ReferenceTracker`**, `ReferenceStorage`, `shortId`, `normalize`, `validateReferenceInput`, `describeReferenceRecord`, etc. — the pure, framework-agnostic reference logic. `ReferenceResolver` in `references.ts` wraps these with Node-specific caching and API calls.
- **Business logic workflows** such as `issueEquity`, `issueSafe`, `entityHasActiveBoard`, `writtenConsent` — imported directly into registry files. This keeps protocol and business rules in the shared package so the web client can also use them.
- **Type exports** such as `ResourceKind`, `MatchRecord`, `NextStepItem`, `NextStepsSummary`.

`formation-automation.ts` is a CLI-side multi-step workflow that orchestrates several API calls and is not suitable for the shared package because it drives terminal output and needs the Node-specific `ReferenceResolver`. It exports `autoSignFormationDocuments`, `activateFormationEntity`, and related helpers.

---

## 9. Web Terminal Integration

### 9.1 generateWebRoutes

`generateWebRoutes(commands)` in `registry/index.ts` emits a `web-routes.json` manifest from the registry. Each non-hidden command becomes an entry in the `commands` record keyed by `def.name`.

Key fields emitted per command:

| Field | Source | Purpose |
|---|---|---|
| `method` / `path` | `def.route` | Web generic executor uses this to call the API directly |
| `entity` | `def.entity` | Signals entity scoping to the web executor |
| `title` / `cols` / `listKey` | `def.display` | Drive web-side rendering |
| `optQP` | `def.optQP` | Query parameters to forward |
| `write: true` | non-GET method | Marks mutation commands |
| `custom: true` | `def.handler` present | Tells the web client a custom handler exists; falls back to web-side implementation or disables the command |
| `local: true` | `def.local` | Command is offline-only; web client skips it |
| `produces` | `def.produces` | Forwarded for @last tracking in the web terminal |
| `successTemplate` | `def.successTemplate` | Displayed after successful writes in the web terminal |

Hidden commands (`def.hidden`) are excluded entirely. Commands with neither a `route` nor a `handler` are also excluded.

### 9.2 Web Terminal Consumption

The generated `web-routes.json` is consumed by the internal repo at:

```
/root/repos/thecorporation-internal/services/web/packages/humans/src/pages/cli.astro
```

The web terminal reads the manifest to:
1. Build its command list and autocomplete suggestions.
2. Execute generic GET/POST commands directly against the API using the `path` and `method` fields.
3. Display results using the same `cols`/`listKey`/`title` metadata that drives the CLI table renderer.
4. Show `successTemplate` text after mutations.
5. Track `@last` references in browser session storage using `produces.kind`.

Commands with `custom: true` require a corresponding web-side implementation or are presented as unsupported in the terminal UI.

---

## 10. Adding a New Command

This is the complete procedure for adding a command. The example adds `corp governance add-observer <body-ref>` as a simple POST.

### Step 1 — Choose the registry file

The command belongs to governance. Open `/root/repos/thecorporation-mono/packages/cli-ts/src/registry/governance.ts` and add a new entry to the `governanceCommands` array.

### Step 2 — Write the CommandDef

```typescript
{
  name: "governance add-observer",
  description: "Add an observer to a governance body",
  route: { method: "POST", path: "/v1/governance-bodies/{pos}/observers" },
  entity: true,             // entity_id will be injected into body and resolved from config
  dryRun: true,             // adds --dry-run flag automatically
  args: [
    {
      name: "body-ref",
      required: true,
      description: "Governance body reference",
      posKind: "body",      // enables @last:body, short IDs, and handles
    },
  ],
  options: [
    {
      flags: "--contact <ref>",
      description: "Contact reference for the observer",
      required: true,
    },
    {
      flags: "--role <role>",
      description: "Observer role",
      choices: ["observer", "advisor"],   // Commander validates at parse time
      default: "observer",
    },
  ],
  produces: { kind: "seat" },
  successTemplate: "Observer added to body: {seat_id}",
  examples: [
    "corp governance add-observer @last:body --contact alice --role observer",
    "corp governance add-observer board-of-directors --contact alice --dry-run",
  ],
  handler: async (ctx) => {
    const bodyRef = ctx.positional[0];
    if (!bodyRef) throw new Error("Missing required argument <body-ref>.");

    const eid = await ctx.resolver.resolveEntity(ctx.opts.entityId as string | undefined);
    const bodyId = await ctx.resolver.resolveBody(eid, bodyRef);
    const contactId = await ctx.resolver.resolveContact(eid, ctx.opts.contact as string);

    const payload = {
      entity_id: eid,
      contact_id: contactId,
      role: ctx.opts.role as string,
    };

    if (ctx.dryRun) {
      ctx.writer.dryRun("governance.add_observer", payload);
      return;
    }

    const result = await ctx.client.submitJSON("POST",
      `/v1/governance-bodies/${encodeURIComponent(bodyId)}/observers`, payload);

    ctx.resolver.rememberFromRecord("seat", result as ApiRecord, eid);

    if (ctx.opts.json) { ctx.writer.json(result); return; }
    ctx.writer.success(`Observer added: ${(result as ApiRecord).seat_id ?? "OK"}`);
    printReferenceSummary("seat", result as ApiRecord, { showReuseHint: true });
  },
},
```

### Step 3 — No changes needed to index.ts

`registry/index.ts` spreads all domain arrays into `registry`. Adding an entry to `governanceCommands` is sufficient.

### Step 4 — Regenerate derived artifacts

If the project has a `gen:routes` or `gen:schema` script, run it to update `web-routes.json` and `cli-schema.json`. These are consumed by the web terminal and tab-completion respectively.

### Step 5 — Verify

```
# Build
cd packages/cli-ts && npm run build

# Smoke test
node dist/index.js governance add-observer --help
node dist/index.js governance add-observer @last:body --contact alice --dry-run
```

### When to use a generic executor instead of a handler

If your command simply forwards options to a POST body and the default success message is sufficient, you can omit `handler` entirely:

```typescript
{
  name: "governance add-observer",
  route: { method: "POST", path: "/v1/governance-bodies/{pos}/observers" },
  entity: true,
  args: [{ name: "body-ref", required: true, posKind: "body" }],
  options: [
    { flags: "--contact <ref>", description: "Contact reference", required: true },
    { flags: "--role <role>", description: "Observer role",
      choices: ["observer", "advisor"], default: "observer" },
  ],
  produces: { kind: "seat" },
  successTemplate: "Observer added: {seat_id}",
  examples: ["corp governance add-observer @last:body --contact alice"],
}
```

`executeGenericWrite` will resolve `{pos}` via `posKind: "body"`, collect `--contact` and `--role` as body fields (`contact` and `role`), call the API, and print a success message. Use a custom handler only when you need reference stabilization, multi-step logic, rich output, dry-run validation, or disambiguation.

---

## Quick Reference: Path Placeholder Summary

| Placeholder | Resolved from |
|---|---|
| `{eid}` | Active entity or `--entity-id`; resolved via `ctx.resolver.resolveEntity()` |
| `{pos}` | First positional arg; resolved via `resolvePositional()` if `posKind` is set |
| `{pos2}` | Second positional arg; same resolution logic |
| `{wid}` / `{workspace_id}` | `ctx.client.workspaceId` from config (no resolution needed) |
