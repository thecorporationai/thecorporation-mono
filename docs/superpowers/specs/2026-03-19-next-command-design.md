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

#### `GET /v1/workspace/next-steps`

Workspace-scoped. Aggregates recommendations across all entities.

### Response schema

```json
{
  "top": {
    "category": "formation",
    "title": "Finalize incorporation for Acme Inc",
    "description": "Formation is pending — finalize to activate your entity",
    "command": "npx corp form finalize --entity-id ent_abc123",
    "urgency": "critical"
  },
  "backlog": [
    {
      "category": "documents",
      "title": "Sign Certificate of Incorporation",
      "description": "Document awaiting your signature",
      "command": "npx corp documents signing-link --document-id doc_xyz",
      "urgency": "high"
    }
  ],
  "summary": {
    "critical": 1,
    "high": 2,
    "medium": 3,
    "low": 1
  }
}
```

### Urgency tiers

Reuses the existing obligation urgency pattern:

| Tier | Meaning | Examples |
|------|---------|----------|
| `critical` | Blocking progress | Unfinalized formation, overdue filings |
| `high` | Needs attention soon | Unsigned documents, expiring deadlines |
| `medium` | Should do | Missing governance setup, unfilled seats |
| `low` | Nice to have | Optional configuration, informational |

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

### CLI output format

Default (human-readable):

```
⭐ Next up:
   Finalize incorporation for Acme Inc
   → npx corp form finalize --entity-id ent_abc123

📋 More to do:

  Documents (2)
   • Sign Certificate of Incorporation
     → npx corp documents signing-link --document-id doc_xyz
   • Sign Bylaws
     → npx corp documents signing-link --document-id doc_def

  Governance (1)
   • Create board of directors
     → npx corp governance create-body --entity-id ent_abc123 --name "Board of Directors" --kind board

  Compliance (1)
   • File EIN application — due Apr 15
     → npx corp tax file --entity-id ent_abc123 --kind ein

  7 items total (1 critical, 2 high, 3 medium, 1 low)
```

When everything is done:

```
✅ All caught up! No pending actions for Acme Inc.
```

JSON mode (`--json`): outputs the raw server response merged with local checks.

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
