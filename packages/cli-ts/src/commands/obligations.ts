import { requireConfig } from "../config.js";
import { CorpAPIClient } from "../api-client.js";
import { printObligationsTable, printError, printJson } from "../output.js";

export async function obligationsCommand(opts: { tier?: string; json?: boolean }): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    const data = await client.getObligations(opts.tier);
    const obligations = (data.obligations ?? []) as Record<string, unknown>[];
    if (opts.json) printJson(obligations);
    else if (obligations.length === 0) console.log("No obligations found.");
    else printObligationsTable(obligations);
  } catch (err) { printError(`Failed to fetch obligations: ${err}`); process.exit(1); }
}
