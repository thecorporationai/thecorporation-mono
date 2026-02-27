/**
 * MCP server authentication — auto-provisioning and shared config.
 *
 * Resolution order:
 * 1. CORP_API_KEY + CORP_WORKSPACE_ID env vars (explicit)
 * 2. ~/.corp/config.json (shared with TUI/CLI)
 * 3. Auto-provision via POST /v1/workspaces/provision
 */

import { readFileSync, writeFileSync, mkdirSync } from "node:fs";
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

function saveConfig(cfg: Record<string, string>): void {
  const dir = join(homedir(), ".corp");
  mkdirSync(dir, { recursive: true });
  writeFileSync(CONFIG_FILE, JSON.stringify(cfg, null, 2) + "\n");
}

async function provision(apiUrl: string): Promise<Record<string, string>> {
  const resp = await fetch(`${apiUrl.replace(/\/+$/, "")}/v1/workspaces/provision`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ name: "mcp-auto" }),
    signal: AbortSignal.timeout(15_000),
  });
  if (!resp.ok) throw new Error(`Provision failed: ${resp.status}`);
  return await resp.json() as Record<string, string>;
}

export async function resolveOrProvisionAuth(
  apiUrl: string = "https://api.thecorporation.ai",
): Promise<McpAuthContext> {
  // 1. Env vars
  const envKey = process.env.CORP_API_KEY || "";
  const envWs = process.env.CORP_WORKSPACE_ID || "";
  if (envKey && envWs) {
    return { workspaceId: envWs, apiKey: envKey, scopes: ["*"] };
  }

  // 2. Config file
  const cfg = loadConfig();
  if (cfg.api_key && cfg.workspace_id) {
    return { workspaceId: cfg.workspace_id, apiKey: cfg.api_key, scopes: ["*"] };
  }

  // 3. Auto-provision
  const result = await provision(apiUrl);
  cfg.api_key = result.api_key;
  cfg.workspace_id = result.workspace_id;
  if (!cfg.api_url) cfg.api_url = apiUrl;
  saveConfig(cfg);

  return { workspaceId: result.workspace_id, apiKey: result.api_key, scopes: ["*"] };
}
