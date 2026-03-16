import { confirm } from "@inquirer/prompts";
import { requireConfig } from "../config.js";
import { CorpAPIClient } from "../api-client.js";
import { printError, printJson, printSuccess, printWriteResult } from "../output.js";

export async function apiKeysListCommand(opts: { json?: boolean }): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    const keys = await client.listApiKeys();
    if (opts.json) { printJson(keys); return; }
    if (keys.length === 0) { console.log("No API keys found."); return; }
    for (const k of keys) {
      const name = k.name ?? k.label ?? "unnamed";
      const id = k.key_id ?? k.id;
      const scopes = Array.isArray(k.scopes) ? (k.scopes as string[]).join(", ") : "all";
      console.log(`  ${name} [${id}] scopes: ${scopes}`);
    }
  } catch (err) { printError(`Failed to list API keys: ${err}`); process.exit(1); }
}

export async function apiKeysCreateCommand(opts: {
  name: string; scopes?: string; json?: boolean;
}): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    const data: Record<string, unknown> = { name: opts.name };
    if (opts.scopes) data.scopes = opts.scopes.split(",").map((s) => s.trim());
    const result = await client.createApiKey(data);
    printWriteResult(result, `API key created: ${result.key_id ?? "OK"}`, opts.json);
    if (!opts.json && result.api_key) {
      printSuccess(`Key: ${result.api_key}`);
      console.log("  Save this key — it will not be shown again.");
    }
  } catch (err) { printError(`Failed to create API key: ${err}`); process.exit(1); }
}

export async function apiKeysRevokeCommand(keyId: string, opts: {
  yes?: boolean; json?: boolean;
}): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    if (!opts.yes) {
      const ok = await confirm({ message: `Revoke API key ${keyId}? This cannot be undone.`, default: false });
      if (!ok) { console.log("Cancelled."); return; }
    }
    await client.revokeApiKey(keyId);
    if (opts.json) { printJson({ revoked: true, key_id: keyId }); return; }
    printSuccess(`API key ${keyId} revoked.`);
  } catch (err) { printError(`Failed to revoke API key: ${err}`); process.exit(1); }
}

export async function apiKeysRotateCommand(keyId: string, opts: {
  json?: boolean;
}): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    const result = await client.rotateApiKey(keyId);
    printWriteResult(result, `API key ${keyId} rotated.`, opts.json);
    if (!opts.json && result.api_key) {
      printSuccess(`New key: ${result.api_key}`);
      console.log("  Save this key — it will not be shown again.");
    }
  } catch (err) { printError(`Failed to rotate API key: ${err}`); process.exit(1); }
}
