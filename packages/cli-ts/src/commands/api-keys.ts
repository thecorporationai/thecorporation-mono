import { requireConfig, maskKey } from "../config.js";
import { CorpAPIClient } from "../api-client.js";
import { printError, printJson } from "../output.js";
import chalk from "chalk";
import Table from "cli-table3";

export async function apiKeysCommand(opts: { json?: boolean }): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    const keys = await client.listApiKeys();
    if (opts.json) { printJson(keys); return; }
    if (keys.length === 0) { console.log("No API keys found."); return; }
    console.log(`\n${chalk.bold("API Keys")}`);
    const table = new Table({
      head: [chalk.dim("ID"), chalk.dim("Name"), chalk.dim("Key"), chalk.dim("Created"), chalk.dim("Last Used")],
    });
    for (const k of keys) {
      table.push([
        String(k.id ?? k.api_key_id ?? "").slice(0, 12),
        String(k.name ?? ""),
        maskKey(String(k.key ?? k.api_key ?? "")),
        String(k.created_at ?? ""),
        String(k.last_used_at ?? ""),
      ]);
    }
    console.log(table.toString());
  } catch (err) { printError(`Failed to fetch API keys: ${err}`); process.exit(1); }
}
