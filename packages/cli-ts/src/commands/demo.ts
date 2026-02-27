import { requireConfig } from "../config.js";
import { CorpAPIClient } from "../api-client.js";
import { printError, printSuccess, printJson } from "../output.js";
import { withSpinner } from "../spinner.js";

export async function demoCommand(opts: { name: string }): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    const result = await withSpinner("Loading", () => client.seedDemo(opts.name));
    printSuccess(`Demo seeded: ${result.entity_id ?? "OK"}`);
    printJson(result);
  } catch (err) { printError(`Failed to seed demo: ${err}`); process.exit(1); }
}
