---
name: corp-claw
version: 1.0.0
description: Corporate governance and formation tools for autonomous agents. Form entities, manage cap tables, track compliance, and run treasury operations — all version-controlled in git.
tags:
  - corporate
  - governance
  - formation
  - compliance
  - cap-table
  - mcp
  - finance
  - legal
metadata:
  openclaw:
    requires:
      bins:
        - npx
        - git
      env:
        - CORP_DATA_DIR
    primaryEnv: CORP_DATA_DIR
    emoji: 🏛️
    homepage: https://thecorporation.ai
---

# Corp Claw

Corporate governance tools for autonomous agents. Every action is a git commit.

## What this skill does

This skill gives your agent the ability to form and manage corporate entities — Delaware C-Corps, LLCs, cap tables, compliance deadlines, banking, contracts, and fundraising — all backed by a version-controlled git repository.

Your agent gets access to 36 MCP tools spanning:

- **Entity formation** — form, convert, or dissolve legal entities
- **Cap table management** — issue equity, SAFEs, track vesting schedules
- **Compliance monitoring** — deadlines, annual reports, franchise tax, BOI filings
- **Treasury operations** — bank accounts, KYB verification, ledger reconciliation
- **Document generation** — contracts, IP assignments, NDAs, board resolutions
- **Governance** — board meetings, voting, resolutions

## Setup

### Option 1: MCP Server (recommended)

Connect any MCP-compatible agent to the corporate tools:

```bash
npx -y @thecorporation/mcp-server
```

For Claude Desktop, add to `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "thecorporation": {
      "command": "npx",
      "args": ["-y", "@thecorporation/mcp-server"]
    }
  }
}
```

For Claude Code:

```bash
claude mcp add thecorporation -- npx -y @thecorporation/mcp-server
```

### Option 2: CLI

```bash
npm install -g @thecorporation/cli
corp setup   # choose local, cloud, or self-hosted
```

### Option 3: Local mode

Run everything on your machine — no server, no cloud:

```bash
npx @thecorporation/cli setup   # choose "Local (your machine)"
```

Data is stored in `~/.corp/data`. Each command invokes the Rust binary directly (~6ms). The MCP server automatically picks up local-mode credentials from `~/.corp/`.

## Usage

When the user asks to form a company, manage equity, handle compliance, or perform any corporate governance task, use the TheCorporation MCP tools.

### Forming an entity

Use `form_entity` to create a new legal entity. The formation data is committed to the git repo as structured TOML.

```
tool_call("form_entity", {
  "name": "Acme Inc",
  "type": "corporation",
  "jurisdiction": "US-DE"
})
```

### Issuing equity

Use `issue_equity` or `issue_safe` to manage the cap table.

```
tool_call("issue_equity", {
  "holder": "Alice Chen",
  "shares": 4000000,
  "class": "Common",
  "vesting": "4y/1y-cliff"
})
```

### Tracking compliance

Use `track_deadline` and `list_obligations` to monitor filing requirements.

```
tool_call("list_obligations", {})
```

### Generating documents

Use `generate_contract` for templated legal documents that are committed to the repo.

```
tool_call("generate_contract", {
  "template": "ip-assignment",
  "party": "Alice Chen"
})
```

## How it works

```
Agent / MCP Client          TheCorporation            Git Repo
─────────────────────       ─────────────────────     ─────────────────
tool_call(                  validate + policy          git commit
  "form_entity",            execute action             signed log
  {name: "Acme"}            return result              push remote
)
```

Every action passes through a single policy gate, producing one audit trail and one source of truth. Your corporate history is `git log`. Undo any change with `git revert`.

## Data format

All corporate data is stored as plain TOML and JSON — readable by any tool, any language, any agent:

```
acme-inc/
├── entity/formation.toml
├── cap-table/founders.toml
├── compliance/deadlines.toml
├── agents/compliance-bot.toml
├── governance/resolutions/
└── .corp/config.toml
```

## Available MCP tools

| Domain | Tools |
|---|---|
| **Entity** | `form_entity` · `convert_entity` |
| **Cap Table** | `issue_equity` · `issue_safe` |
| **Compliance** | `track_deadline` · `list_obligations` |
| **Banking** | `open_bank_account` · `reconcile_ledger` |
| **Documents** | `generate_contract` · `sign_document` |
| **Governance** | `convene_meeting` · `cast_vote` |

## Links

- [Website](https://thecorporation.ai)
- [Documentation](https://docs.thecorporation.ai)
- [GitHub](https://github.com/thecorporationai/thecorporation-mono)
- [How it Works](https://thecorporation.ai/how-it-works)
