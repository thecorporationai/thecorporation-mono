import { loadConfig, saveConfig } from "../config.js";
import { printError } from "../output.js";

export async function claimCommand(code: string): Promise<void> {
  const cfg = loadConfig();
  const apiUrl = (cfg.api_url || "https://api.thecorporation.ai").replace(/\/+$/, "");
  try {
    const resp = await fetch(`${apiUrl}/v1/workspaces/claim`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ code }),
    });
    if (!resp.ok) {
      let detail = "";
      try { const body = await resp.json() as Record<string, string>; detail = body.detail ?? ""; } catch { /* ignore */ }
      printError(detail || `${resp.status} ${resp.statusText}`);
      process.exit(1);
    }
    const data = await resp.json() as Record<string, string>;
    cfg.api_key = data.api_key;
    cfg.workspace_id = data.workspace_id;
    saveConfig(cfg);
    console.log(`Workspace joined: ${data.workspace_id}`);
    console.log("Credentials saved to ~/.corp/config.json");
  } catch (err) {
    printError(`${err}`);
    process.exit(1);
  }
}
