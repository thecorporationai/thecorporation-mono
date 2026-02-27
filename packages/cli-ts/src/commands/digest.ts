import { requireConfig } from "../config.js";
import { CorpAPIClient } from "../api-client.js";
import { printError, printSuccess, printJson } from "../output.js";

export async function digestCommand(opts: { trigger?: boolean; key?: string }): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    if (opts.trigger) {
      const result = await client.triggerDigest();
      printSuccess("Digest triggered.");
      printJson(result);
    } else if (opts.key) {
      const result = await client.getDigest(opts.key);
      printJson(result);
    } else {
      const digests = await client.listDigests();
      if (digests.length === 0) console.log("No digest history found.");
      else printJson(digests);
    }
  } catch (err) { printError(`Failed: ${err}`); process.exit(1); }
}
