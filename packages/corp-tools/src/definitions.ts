/**
 * Tool registry and classification helpers.
 * Re-exports from existing modules + builds a registry from generated defs.
 */

import { GENERATED_TOOL_DEFINITIONS } from "./tool-defs.generated.js";

export { GENERATED_TOOL_DEFINITIONS } from "./tool-defs.generated.js";
export { isWriteTool } from "./tools.js";
export { describeToolCall } from "./tool-descriptions.js";

interface ToolFunction {
  name: string;
  description: string;
  parameters: Record<string, unknown>;
}

interface ToolDef {
  type: string;
  function: ToolFunction;
}

/** Registry: tool name → function metadata (description + parameters). */
export const TOOL_REGISTRY: Record<string, ToolFunction> = {};

for (const td of GENERATED_TOOL_DEFINITIONS as unknown as ToolDef[]) {
  TOOL_REGISTRY[td.function.name] = td.function;
}

export const READ_ONLY_TOOLS = new Set([
  "get_workspace_status", "list_entities", "get_cap_table", "list_documents",
  "list_safe_notes", "list_agents", "get_checklist",
  "get_signing_link", "list_obligations", "get_billing_status",
]);
