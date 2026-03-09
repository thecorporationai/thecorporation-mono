/**
 * MCP server authentication — shared config with CLI.
 *
 * Resolution order:
 * 1. CORP_API_KEY + CORP_WORKSPACE_ID env vars (explicit)
 * 2. ~/.corp/config.json (shared with CLI — run `corp setup` first)
 *
 * MCP servers run over stdio, so interactive auth (magic link) is not
 * possible here. Users authenticate once via `corp setup` (which does
 * the magic link flow for cloud) and the MCP server reuses those
 * credentials from the shared config file.
 */

import { readFileSync } from "node:fs";
import { join } from "node:path";
import { homedir } from "node:os";

export interface McpAuthContext {
  workspaceId: string;
  apiKey: string;
  scopes: string[];
}

const CONFIG_FILE = join(homedir(), ".corp", "config.json");

function loadConfig(): Record<string, string> {
  try {
    return JSON.parse(readFileSync(CONFIG_FILE, "utf-8"));
  } catch {
    return {};
  }
}

export async function resolveOrProvisionAuth(
  apiUrl: string = "https://api.thecorporation.ai",
): Promise<McpAuthContext> {
  // 1. Env vars (set in claude_desktop_config.json or similar)
  const envKey = process.env.CORP_API_KEY || "";
  const envWs = process.env.CORP_WORKSPACE_ID || "";
  if (envKey && envWs) {
    return { workspaceId: envWs, apiKey: envKey, scopes: ["*"] };
  }

  // 2. Shared config from CLI (run `corp setup` to authenticate)
  const cfg = loadConfig();
  if (cfg.api_key && cfg.workspace_id) {
    return { workspaceId: cfg.workspace_id, apiKey: cfg.api_key, scopes: ["*"] };
  }

  // No credentials found — guide user to authenticate via CLI
  throw new Error(
    "No credentials found. Run `npx @thecorporation/cli setup` to authenticate, " +
    "or set CORP_API_KEY and CORP_WORKSPACE_ID environment variables."
  );
}
