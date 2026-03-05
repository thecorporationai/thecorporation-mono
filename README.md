# The Corporation

Corporate infrastructure for the agentic era.

Version-controlled governance, autonomous agents, and open-source tooling — from formation to exit. Every action is a git commit. You own the data.

**Every action is a git commit** · **You own your data** · **Open source** · **Self-hostable**

---

## Two entry points

### Your agent needs a corporation

AI agents need real-world infrastructure — bank accounts, legal entities, equity cap tables, contracts. TheCorporation exposes 36 MCP tools that give your agent the corporate powers it needs. Point your agent at the MCP server. It handles the rest.

```json
// claude_desktop_config.json
{
  "mcpServers": {
    "corp": {
      "command": "npx",
      "args": ["-y", "@thecorporation/mcp-server"]
    }
  }
}
```

### Your corporation needs an agent

Compliance deadlines, annual filings, cap table updates, franchise taxes — the mechanical work of corporate existence. Agents handle it autonomously. Every action is a git commit. Every commit is auditable and reversible.

```bash
$ corp form --name "Acme Inc" --type corporation --jurisdiction US-DE
  ✔ Formation submitted: Acme Inc (US-DE)

$ corp agents create --name "Ops Bot" --prompt "Handle recurring operations"
  ✔ Agent created: Ops Bot
  ✔ 36 tools available
```

## Quick start

### MCP server

```bash
npx -y @thecorporation/mcp-server
```

### Claude Desktop

Add to your `claude_desktop_config.json`:

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

### Claude Code

```bash
claude mcp add thecorporation -- npx -y @thecorporation/mcp-server
```

### CLI

```bash
npm install -g @thecorporation/cli
corp setup
corp status
```

## Your corporation is a git repo

Not a database behind someone else's API. Not a PDF export you have to request. A git repo on your machine that you fully control.

### Version controlled

Every corporate action — formation, equity issuance, board resolution, compliance filing — is an atomic git commit. Your complete corporate history is `git log`. Undo any change with `git revert`.

```
$ git log --oneline
  e6f7a8b corp: adopt bylaws v1.0
  b3c4d5e corp: issue SAFE "Sequoia" ($500K)
  7d8e9f0 corp: add founder "Alice" (4M shares)
  4a1b2c3 corp: form entity "Acme Inc" (DE C-Corp)
  a0b1c2d corp: init repository
```

### Portable

To leave, `git clone`. To back up, `git push`. To audit, `git diff`. Your data is plain TOML and JSON — readable by any tool, any language, any agent.

```
acme-inc/
├── entity/formation.toml
├── cap-table/founders.toml
├── compliance/deadlines.toml
├── agents/compliance-bot.toml
├── governance/resolutions/
└── .corp/config.toml
```

### Diffable

See exactly what changed, when, and who authorized it. Every mutation is a structured diff you can review before it happens and inspect after.

```diff
# git diff HEAD~1
  cap-table/founders.toml
  + [[holders]]
  + name = "Alice Chen"
  + shares = 4_000_000
  + class = "Common"
  + vesting = "4y/1y-cliff"
```

## 36 MCP tools

| Domain | Tools |
|---|---|
| **Entity** | `form_entity` · `convert_entity` — form, convert, dissolve. |
| **Cap Table** | `issue_equity` · `issue_safe` — machine-readable equity from day one. |
| **Compliance** | `track_deadline` · `list_obligations` — continuous monitoring, zero missed filings. |
| **Banking** | `open_bank_account` · `reconcile_ledger` — treasury operations the agent can drive. |
| **Documents** | `generate_contract` · `sign_document` — templated legal docs, committed to the repo. |
| **Governance** | `convene_meeting` · `cast_vote` — board actions, mechanically enforced. |

```bash
$ npx -y @thecorporation/mcp-server
  ✔ 36 tools loaded
  ✔ MCP server ready
```

## The full stack

| Capability | What it does |
|---|---|
| **Formation** | Delaware C-Corp in one tool call. Bylaws, EIN, board consent — generated and committed. |
| **Cap Table** | Machine-readable equity your agent can query. Full history via `git log`. |
| **Compliance** | Annual reports, BOI filings, franchise tax. Your agent never misses a deadline. |
| **Banking** | Account opening, KYB verification, transaction feed ingestion. Treasury as code. |
| **Tax** | 83(b) elections, W-9 collection, estimated tax. Every filing versioned in the repo. |
| **Contracts** | IP assignment, NDAs, advisor agreements — generated, signed, and committed. |
| **Fundraising** | SAFE issuance, priced rounds, conversion mechanics. `git log` is your paper trail. |
| **Registered Agent** | Licensed registered agent in your state. Mail scanning, forwarding, service of process. |

## Agents

Agents are TOML config files, version-controlled and reviewable like everything else. Change an agent's behavior by editing a file and committing. Roll back a bad deploy with `git revert`.

```toml
# agents/compliance-bot.toml
name = "compliance-bot"
model = "anthropic/claude-sonnet-4-6"
system_prompt = """
  Monitor compliance deadlines.
  File reports when due.
"""

[channels]
type = "cron"
schedule = "0 9 1 * *"  # 1st of month

[budget]
max_turns = 20
max_monthly_cost_cents = 5000

[sandbox]
memory_mb = 512
timeout_seconds = 300
network_egress = "restricted"
```

Agents support cron schedules, email triggers, webhook events, sandboxed containers, budget controls, and attached MCP servers.

## How it works

```
Agent / MCP Client          TheCorporation            Git Repo
─────────────────────       ─────────────────────     ─────────────────
tool_call(                  validate + policy          git commit
  "form_entity",            execute action             signed log
  {name: "Acme"}            return result              push remote
)
```

Every action — whether initiated by a human at the terminal, an agent on a cron schedule, or an MCP tool call — passes through the same pipeline. One policy gate, one audit trail, one source of truth.

## Project structure

```
thecorporation-mono/
├── packages/
│   ├── cli-ts/          @thecorporation/cli — TypeScript CLI
│   ├── mcp-server/      @thecorporation/mcp-server — MCP server
│   ├── corp-tools/      @thecorporation/corp-tools — shared API client & tool definitions
│   └── server/          @thecorporation/server — pre-built server binaries
├── skills/
│   └── corp-claw/       ClawHub skill — corporate tools for autonomous agents
├── services/
│   ├── api-rs/          Rust backend — the governance kernel
│   └── agent-worker/    Rust agent execution worker
├── governance/          Governance definitions
└── tests/               Integration tests
```

## Comparison

| | TheCorporation | Stripe Atlas | Doola |
|---|---|---|---|
| **Data ownership** | git repo (yours) | their database | their database |
| **Version control** | every commit | — | — |
| **Agent / MCP access** | 36 MCP tools | — | — |
| **Open source** | yes | — | — |
| **Self-hostable** | yes | — | — |
| **Data portability** | `git clone` | PDF only | export request |
| **Agent workers** | git-configured | — | — |
| **Price** | Free / $299/yr | $500 once | $297/yr |
| **Cap table** | yes | — | — |
| **Ongoing compliance** | yes | — | yes |
| **CLI / API** | yes | — | — |

## Links

- [thecorporation.ai](https://thecorporation.ai)
- [Docs](https://docs.thecorporation.ai)
- [How it Works](https://thecorporation.ai/how-it-works)
- [GitHub](https://github.com/thecorporation)

## License

Apache 2.0
