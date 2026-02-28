# @thecorporation/corp-tools

Shared API client, tool definitions, and execution engine for The Corporation. Used by [`@thecorporation/cli`](https://www.npmjs.com/package/@thecorporation/cli), [`@thecorporation/mcp-server`](https://www.npmjs.com/package/@thecorporation/mcp-server), and the chat service.

Part of [The Corporation](https://thecorporation.ai) — agent-native corporate infrastructure.

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

// Execute a tool call
const result = await executeTool("form_entity", args, client, { dataDir: "." });

// Check if a tool mutates state (useful for confirmation gating)
isWriteTool("form_entity");    // true
isWriteTool("list_entities");  // false
```

### System Prompt

```js
import { SYSTEM_PROMPT_BASE, formatConfigSection } from "@thecorporation/corp-tools";
```

## Tools

36 tools across corporate governance domains:

| Category | Tools |
|---|---|
| **Entities** | `form_entity`, `convert_entity`, `dissolve_entity`, `list_entities` |
| **Equity** | `issue_equity`, `issue_safe`, `transfer_shares`, `calculate_distribution`, `get_cap_table`, `list_safe_notes` |
| **Finance** | `create_invoice`, `run_payroll`, `submit_payment`, `open_bank_account`, `reconcile_ledger`, `classify_contractor` |
| **Documents** | `generate_contract`, `list_documents`, `get_document_link`, `get_signing_link` |
| **Governance** | `convene_meeting`, `schedule_meeting`, `cast_vote` |
| **Tax** | `file_tax_document`, `track_deadline` |
| **Agents** | `create_agent`, `send_agent_message`, `update_agent`, `add_agent_skill`, `list_agents` |
| **Workspace** | `get_workspace_status`, `list_obligations`, `get_billing_status`, `get_checklist`, `update_checklist`, `get_signer_link` |

## Exports

- `CorpAPIClient` — typed API client with methods for every endpoint
- `TOOL_DEFINITIONS` / `GENERATED_TOOL_DEFINITIONS` — OpenAI-compatible function schemas
- `TOOL_REGISTRY` — name-to-definition lookup
- `READ_ONLY_TOOLS` — set of tool names that don't mutate state
- `executeTool()` — dispatches a tool call and returns the result
- `isWriteTool()` — checks whether a tool mutates state
- `describeToolCall()` — human-readable description of a tool call
- `SYSTEM_PROMPT_BASE` / `formatConfigSection()` — system prompt utilities
- `provisionWorkspace()` — provisions a new workspace

## Links

- [thecorporation.ai](https://thecorporation.ai)
- [GitHub](https://github.com/thecorporationai/thecorporation-mono/tree/main/packages/corp-tools)

## License

MIT
