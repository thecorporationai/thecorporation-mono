import { requireConfig } from "../config.js";
import { CorpAPIClient } from "../api-client.js";
import { printError, printSuccess } from "../output.js";

export async function linkCommand(opts: { externalId: string; provider: string }): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    const data = await client.createLink(opts.externalId, opts.provider);
    printSuccess(`Workspace linked to ${opts.provider} (external ID: ${opts.externalId})`);
    if (data.workspace_id) console.log(`  Workspace: ${data.workspace_id}`);
  } catch (err) {
    printError(`${err}`);
    process.exit(1);
  }
}
