/**
 * Entry point — resolve auth, start MCP server on stdio.
 */

import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { CorpAPIClient } from "@thecorporation/corp-tools";
import { createMcpServer } from "./server.js";
import { resolveOrProvisionAuth } from "./auth.js";

async function main(): Promise<void> {
  const apiUrl = process.env.CORP_API_URL || "https://api.thecorporation.ai";

  const ctx = await resolveOrProvisionAuth(apiUrl);
  const client = new CorpAPIClient(apiUrl, ctx.apiKey, ctx.workspaceId);
  const server = createMcpServer(client);
  const transport = new StdioServerTransport();
  await server.connect(transport);
}

main().catch((err) => {
  console.error("Fatal:", err);
  process.exit(1);
});
