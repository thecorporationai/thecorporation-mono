# `npx corp next` — Actionable Next-Steps Command

**Date:** 2026-03-19
**Status:** Approved

## Problem

Users of `npx corp` must remember which commands to run and in what order. After forming an entity, signing documents, or setting up governance, there is no single place to see what remains. The `status` command shows metrics but not actions; `obligations` shows compliance deadlines but not formation or governance gaps.

## Solution

A new `next` command that answers "what should I do right now?" with a single top recommendation and a categorized backlog of remaining actions. Each recommendation includes a copy-pasteable command.

## Architecture

### Hybrid approach

- **Server:** New API endpoints analyze entity/workspace state and return structured recommendations.
- **CLI:** Enriches server response with local checks (missing config, no active entity, etc.).

### Server endpoints

#### `GET /v1/entities/{id}/next-steps`

Entity-scoped. Returns recommendations for a single entity based on its lifecycle state.

#### `GET /v1/workspaces/{workspace_id}/next-steps`

Workspace-scoped. Aggregates recommendations across all entities. Follows the existing `/v1/workspaces/{workspace_id}/...` URL convention.

### Response schema

```json
{
  "top": {
    "category": "formation",
    "title": "Finalize incorporation for Acme Inc",
    "description": "Formation is pending — finalize to activate your entity",
    "command": "npx corp form finalize ent_abc123",
    "urgency": "critical"
  },
  "backlog": [
    {
      "category": "documents",
      "title": "Sign Certificate of Incorporation",
      "description": "Document awaiting your signature",
      "command": "npx corp documents signing-link doc_xyz",
      "urgency": "high"
    }
  ],
  "summary": {
    "critical": 0,
    "high": 0,
    "medium": 0,
    "low": 0
  }
}
```

**Empty-state rules:**
- `top` is `null` when there are no recommendations.
- `backlog` is always an array (empty `[]` when no items).
- `summary` always includes all four keys, with `0` for tiers that have no items.
- The CLI renders the "all caught up" message when `top` is `null`.

**Valid categories:** `formation`, `documents`, `governance`, `cap_table`, `compliance`, `finance`, `agents`.

**Display-name mapping:**

| Category | Display header |
|----------|---------------|
| `formation` | Formation |
| `documents` | Documents |
| `governance` | Governance |
| `cap_table` | Cap Table |
| `compliance` | Compliance |
| `finance` | Finance |
| `agents` | Agents |

### Urgency tiers

The next-steps endpoint uses four semantic tiers. These are distinct from the obligation time-horizon tiers (`overdue`, `due_today`, `d1`, `d7`, `d14`, `d30`, `upcoming`) which are deadline-relative. Next-steps tiers reflect action priority, not calendar proximity.

| Tier | Meaning | Examples | Maps from obligation tiers |
|------|---------|----------|---------------------------|
| `critical` | Blocking progress | Unfinalized formation, overdue filings | `overdue`, `due_today` |
| `high` | Needs attention soon | Unsigned documents, expiring deadlines | `d1`, `d7` |
| `medium` | Should do | Missing governance setup, unfilled seats | `d14`, `d30` |
| `low` | Nice to have | Optional configuration, informational | `upcoming`, no deadline |

The server maps obligation urgency tiers to next-steps tiers using the table above. Non-obligation recommendations (formation gaps, missing governance) are assigned tiers directly by the recommendation rules.

### Server-side recommendation rules

The endpoint inspects entity state and generates recommendations. Rules evaluated in priority order:

1. **Formation incomplete** — finalize or activate the entity
2. **Unsigned documents** — generate signing links for each
3. **No governance bodies** — create a board of directors
4. **Empty board seats** — appoint directors
5. **No cap table instruments** — create share classes, issue founder stock
6. **Overdue obligations** — surface with deadlines
7. **Upcoming deadlines (7/30 day window)** — surface with dates
8. **No bank account** — open one
9. **Pending approvals** — vote or approve
10. **Active meetings needing votes** — cast vote

### CLI-side local checks

Before calling the API, the CLI checks local state and prepends recommendations if needed:

1. **No config file** — recommend `npx corp setup`
2. **No API key** — recommend `npx corp setup`
3. **No active entity** — recommend `npx corp use <entity>` (list available entities)
4. **Config present but no workspace** — recommend `npx corp claim <code>`

Local recommendations take priority over server recommendations (a user with no config can't do anything else).

### CLI command definition

```
npx corp next [options]

Options:
  --entity-id <ref>   Entity to check (default: active entity)
  --workspace         Show recommendations across all entities
  --json              Output raw JSON
```

`--entity-id` and `--workspace` are mutually exclusive. If both are provided, the CLI exits with an error.

### Help text prominence

The `next` command must be the first command listed in `npx corp --help`, placed above `setup` and `status`, with added help text that makes it the obvious entry point:

```
Usage: corp [options] [command]

corp — Corporate governance from the terminal

  Get started:
    next          See what to do next (recommended)

  Setup & context:
    setup         Interactive setup wizard
    status        Workspace summary
    ...
```

The `next` command should also include `.addHelpText("after", ...)` with usage examples.

### CLI output format

Default (human-readable):

```
  Next up:
   Finalize incorporation for Acme Inc
   → npx corp form finalize ent_abc123

  More to do:

  Documents (2)
   • Sign Certificate of Incorporation
     → npx corp documents signing-link doc_xyz
   • Sign Bylaws
     → npx corp documents signing-link doc_def

  Governance (1)
   • Create board of directors
     → npx corp governance --entity-id ent_abc123 create-body --name "Board of Directors" --body-type board_of_directors

  Compliance (1)
   • File EIN application — due Apr 15
     → npx corp tax --entity-id ent_abc123 file --type ein

  7 items total (1 critical, 2 high, 3 medium, 1 low)
```

When everything is done:

```
All caught up! No pending actions for Acme Inc.
```

JSON mode (`--json`): outputs the raw server response merged with local checks.

### Type generation

Response types follow the project's generated-types pattern. The implementation sequence is:

1. Define the OpenAPI schema in the Rust API (`NextStepsResponse`, `NextStepItem`)
2. Regenerate `api-types.generated.ts` in `corp-tools`
3. Export type aliases from `api-schemas.ts` (e.g. `export type NextStepsResponse = components["schemas"]["NextStepsResponse"]`)
4. Use those types in `api-client.ts` and `commands/next.ts`

No hand-written TypeScript interfaces for the response — all types flow from the Rust OpenAPI schema.

### Command string accuracy

The server generates copy-pasteable command strings. Commands must match actual CLI syntax (positional args vs flags, exact option names). The server maintains a command template registry keyed by recommendation rule. Commands use positional arguments where the CLI expects them (e.g. `corp form finalize <entity-ref>`, not `--entity-id`). The CLI does NOT rewrite or validate command strings — the server is the source of truth.

### Files to create/modify

**New files:**
- `packages/cli-ts/src/commands/next.ts` — command implementation
- Server-side endpoint handler (location TBD based on API service structure)
- Server-side next-steps analysis module

**Modified files:**
- `packages/cli-ts/src/index.ts` — register `next` command
- `packages/corp-tools/src/api-client.ts` — add `getEntityNextSteps()` and `getWorkspaceNextSteps()` methods
- `packages/cli-ts/src/output.ts` — add `printNextSteps()` formatting function

## Scope exclusions

- No interactive mode (no prompts, no "do it for me")
- No automatic execution of recommended commands
- No LLM/AI analysis — purely rule-based state inspection
- No persistent state tracking (recommendations are computed fresh each call)
