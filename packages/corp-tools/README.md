# @thecorporation/corp-tools

The shared foundation for TheCorporation's client ecosystem. Typed API client, 10 consolidated tool definitions, and the execution engine that powers the CLI, MCP server, and chat service. Every tool call — whether initiated by a human or an agent — flows through the same pipeline: validate, execute, commit, receipt.

Used by [`@thecorporation/cli`](https://www.npmjs.com/package/@thecorporation/cli), [`@thecorporation/mcp-server`](https://www.npmjs.com/package/@thecorporation/mcp-server), and the chat service.

Part of [TheCorporation](https://thecorporation.ai) — version-controlled governance, autonomous agents, and open-source corporate infrastructure.

## Install

```bash
npm install @thecorporation/corp-tools
```

## Usage

### API Client

```js
import { CorpAPIClient } from "@thecorporation/corp-tools";

const client = new CorpAPIClient(
  "https://api.thecorporation.ai",
  "sk_...",
  "ws_..."
);

const status = await client.getStatus();
const entities = await client.listEntities();
const capTable = await client.getCapTable(entityId);
```

### Equity round close workflow (v1)

```js
// 1) Create round + terms
const round = await client.createEquityRound({
  entity_id,
  issuer_legal_entity_id,
  name: "Series A",
  round_price_cents: 100,
  target_raise_cents: 100000000,
  conversion_target_instrument_id,
  metadata: {}
});

await client.applyEquityRoundTerms(round.round_id, {
  entity_id,
  anti_dilution_method: "none",
  conversion_precedence: ["safe"],
  protective_provisions: {}
});

// 2) Governance board approval
await client.boardApproveEquityRound(round.round_id, {
  entity_id,
  meeting_id,
  resolution_id
});

// 3) Accept with authorized intent
const acceptIntent = await client.createExecutionIntent({
  entity_id,
  intent_type: "equity.round.accept",
  authority_tier: "tier_2",
  description: "Accept approved round",
  metadata: { round_id: round.round_id }
});
await client.evaluateIntent(acceptIntent.intent_id, entity_id);
await client.authorizeIntent(acceptIntent.intent_id, entity_id);
await client.acceptEquityRound(round.round_id, {
  entity_id,
  intent_id: acceptIntent.intent_id
});

// 4) Execute conversion with authorized execute intent
const executeIntent = await client.createExecutionIntent({
  entity_id,
  intent_type: "equity.round.execute_conversion",
  authority_tier: "tier_2",
  description: "Execute round conversion",
  metadata: { round_id: round.round_id }
});
await client.evaluateIntent(executeIntent.intent_id, entity_id);
await client.authorizeIntent(executeIntent.intent_id, entity_id);
await client.executeRoundConversion({
  entity_id,
  round_id: round.round_id,
  intent_id: executeIntent.intent_id
});
```

Breaking change (v1, February 28, 2026):
- `POST /v1/equity/conversions/execute` requires `intent_id`.
- Round close order is `apply-terms -> board-approve -> accept -> execute`.

### Tool Execution

```js
import {
  TOOL_DEFINITIONS,
  TOOL_REGISTRY,
  executeTool,
  isWriteTool,
} from "@thecorporation/corp-tools";

// OpenAI-compatible function definitions for LLM tool calling
console.log(TOOL_DEFINITIONS);

// Execute a tool call (consolidated: tool name + action)
const result = await executeTool("entity", { action: "form", ...args }, client, { dataDir: "." });

// Check if a tool+action mutates state (useful for confirmation gating)
isWriteTool("entity", { action: "form" });          // true
isWriteTool("workspace", { action: "list_entities" }); // false
```

### System Prompt

```js
import { SYSTEM_PROMPT_BASE, formatConfigSection } from "@thecorporation/corp-tools";
```

## Tools

10 consolidated tools with action-based dispatch:

| Tool | Actions |
|---|---|
| **workspace** | status, list_entities, obligations, billing |
| **entity** | get_cap_table, list_documents, list_safe_notes, form, create, add_founder, finalize, convert, dissolve |
| **equity** | start_round, add_security, issue_round, issue, issue_safe, transfer, distribution |
| **valuation** | create, submit, approve |
| **meeting** | schedule, notice, convene, vote, resolve, finalize_item, adjourn, cancel, consent, attach_document, list_items, list_votes |
| **finance** | create_invoice, run_payroll, submit_payment, open_bank_account, reconcile |
| **compliance** | file_tax, track_deadline, classify_contractor, generate_contract |
| **document** | signing_link, signer_link, download_link |
| **checklist** | get, update |
| **agent** | list, create, message, update, add_skill |

## Exports

- `CorpAPIClient` — typed API client with methods for every endpoint
- `TOOL_DEFINITIONS` / `GENERATED_TOOL_DEFINITIONS` — OpenAI-compatible function schemas
- `executeTool()` — dispatches a tool call by name + action and returns the result
- `isWriteTool()` — checks whether a tool+action mutates state
- `describeToolCall()` — human-readable description of a tool call
- `SYSTEM_PROMPT_BASE` / `formatConfigSection()` — system prompt utilities
- `provisionWorkspace()` — provisions a new workspace

## Links

- [thecorporation.ai](https://thecorporation.ai)
- [GitHub](https://github.com/thecorporationai/thecorporation-mono/tree/main/packages/corp-tools)

## License

MIT
