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

/**
 * Read-only tool:action pairs that don't require user confirmation.
 * @deprecated Use isWriteTool(name, args) instead for action-aware checking.
 */
export const READ_ONLY_TOOLS = new Set([
  "workspace", "checklist",
]);
