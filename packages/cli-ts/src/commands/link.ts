import { requireConfig } from "../config.js";
import { CorpAPIClient } from "../api-client.js";
import { printError } from "../output.js";

export async function linkCommand(): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    const data = await client.createLink();
    const code = data.code as string;
    const expires = (data.expires_in_seconds ?? 900) as number;
    console.log();
    console.log(`  ${code}`);
    console.log();
    console.log(`Run this on the other device (expires in ${Math.floor(expires / 60)} minutes):`);
    console.log(`  corp claim ${code}`);
    console.log();
  } catch (err) {
    printError(`${err}`);
    process.exit(1);
  }
}
