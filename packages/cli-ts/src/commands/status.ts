import { requireConfig } from "../config.js";
import { CorpAPIClient } from "../api-client.js";
import { printError, printJson, printStatusPanel } from "../output.js";
import { withSpinner } from "../spinner.js";

export async function statusCommand(opts: { json?: boolean } = {}): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    const data = await withSpinner("Loading", () => client.getStatus());
    if (opts.json) {
      printJson(data);
    } else {
      printStatusPanel(data);
    }
  } catch (err) {
    printError(`Failed to fetch status: ${err}`);
    process.exit(1);
  }
}
