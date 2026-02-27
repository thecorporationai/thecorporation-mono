/**
 * MCP server — registers all tools from corp-tools definitions,
 * dispatches to the backend API via CorpAPIClient.
 */

import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { z } from "zod";
import {
  GENERATED_TOOL_DEFINITIONS,
  CorpAPIClient,
  executeTool,
  type ToolContext,
} from "@thecorporation/corp-tools";
import { join } from "node:path";
import { homedir } from "node:os";

// ---------------------------------------------------------------------------
// Build Zod schemas from JSON Schema properties
// ---------------------------------------------------------------------------

interface ToolFunction {
  name: string;
  description: string;
  parameters: {
    type: string;
    properties: Record<string, { type: string; description?: string; enum?: string[]; items?: { type: string } }>;
    required: string[];
  };
}

interface ToolDef {
  type: string;
  function: ToolFunction;
}

function buildZodShape(fn: ToolFunction): Record<string, z.ZodTypeAny> {
  const shape: Record<string, z.ZodTypeAny> = {};
  const props = fn.parameters.properties || {};
  const required = new Set(fn.parameters.required || []);

  for (const [pname, pinfo] of Object.entries(props)) {
    let schema: z.ZodTypeAny;
    switch (pinfo.type) {
      case "integer":
        schema = z.number().int();
        break;
      case "number":
        schema = z.number();
        break;
      case "boolean":
        schema = z.boolean();
        break;
      case "array":
        schema = z.array(pinfo.items?.type === "object" ? z.record(z.unknown()) : z.string());
        break;
      case "object":
        schema = z.record(z.unknown());
        break;
      default:
        schema = pinfo.enum ? z.enum(pinfo.enum as [string, ...string[]]) : z.string();
    }
    if (pinfo.description) schema = schema.describe(pinfo.description);
    if (!required.has(pname)) schema = schema.optional();
    shape[pname] = schema;
  }
  return shape;
}

// ---------------------------------------------------------------------------
// Create and configure MCP server
// ---------------------------------------------------------------------------

export function createMcpServer(client: CorpAPIClient): McpServer {
  const server = new McpServer({
    name: "thecorporation",
    version: "0.1.0",
  });

  const ctx: ToolContext = {
    dataDir: join(homedir(), ".corp", "data"),
  };

  for (const td of GENERATED_TOOL_DEFINITIONS as unknown as ToolDef[]) {
    const fn = td.function;
    const shape = buildZodShape(fn);

    server.tool(fn.name, fn.description, shape, async (args) => {
      const result = await executeTool(fn.name, args as Record<string, unknown>, client, ctx);
      return { content: [{ type: "text" as const, text: result }] };
    });
  }

  return server;
}
