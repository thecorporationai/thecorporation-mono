# The Corporation

Git-native corporate operations for agents and humans.

## What it is

The Corporation provides:

- Rust backend (`services/api-rs`) for corporate operations
- TypeScript CLI (`@thecorporation/cli`)
- TypeScript MCP server (`@thecorporation/mcp-server`)
- Git-backed state and audit trail per workspace/entity

## Quick start (npm-first)

### MCP server

```bash
npx -y @thecorporation/mcp-server
```

### Claude Desktop config

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

## Core model

All side effects run through the execution lifecycle:

1. create intent (`POST /v1/execution/intents`)
2. evaluate (`POST /v1/intents/{intent_id}/evaluate`)
3. authorize (`POST /v1/intents/{intent_id}/authorize`)
4. execute (`POST /v1/intents/{intent_id}/execute`)
5. read receipt (`GET /v1/receipts/{receipt_id}`)

## Runtime

Primary services:

- `backend`
- `agents`
- `agent-worker`
- `agents-redis`
- `chat-ws`
- `caddy`
- static surfaces under `services/web`

## Docs

- Architecture: `ARCHITECTURE/`
- Web docs app: `services/web/packages/docs`
- Generated references (MCP/CLI/API): in docs package content tree

## License

Apache 2.0
