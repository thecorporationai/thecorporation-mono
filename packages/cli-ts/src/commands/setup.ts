import { input, confirm } from "@inquirer/prompts";
import { loadConfig, saveConfig } from "../config.js";
import { provisionWorkspace } from "../api-client.js";
import { printSuccess, printError } from "../output.js";

const API_URL = "https://api.thecorporation.ai";

export async function setupCommand(): Promise<void> {
  const cfg = loadConfig();
  console.log("Welcome to corp — corporate governance from the terminal.\n");

  cfg.api_url = API_URL;

  console.log("--- User Info ---");
  const user = cfg.user ?? { name: "", email: "" };
  user.name = await input({ message: "Your name", default: user.name || undefined });
  user.email = await input({ message: "Your email", default: user.email || undefined });
  cfg.user = user;

  if (!cfg.api_key || !cfg.workspace_id) {
    console.log("\nProvisioning workspace...");
    try {
      const result = await provisionWorkspace(cfg.api_url, `${user.name}'s workspace`);
      cfg.api_key = result.api_key as string;
      cfg.workspace_id = result.workspace_id as string;
      console.log(`Workspace provisioned: ${result.workspace_id}`);
    } catch (err) {
      printError(`Auto-provision failed: ${err}`);
      console.log("You can manually set credentials with: corp config set api_key <key>");
    }
  } else {
    console.log("\nVerifying existing credentials...");
    let keyValid = false;
    try {
      const resp = await fetch(
        `${cfg.api_url.replace(/\/+$/, "")}/v1/workspaces/${cfg.workspace_id}/status`,
        { headers: { Authorization: `Bearer ${cfg.api_key}` } }
      );
      keyValid = resp.status !== 401;
    } catch {
      // network error — treat as potentially valid
    }

    if (keyValid) {
      console.log("Credentials OK.");
    } else {
      console.log("API key is no longer valid.");
      const reprovision = await confirm({
        message: "Provision a new workspace? (This will replace your current credentials)",
        default: false,
      });
      if (reprovision) {
        try {
          const result = await provisionWorkspace(cfg.api_url, `${user.name}'s workspace`);
          cfg.api_key = result.api_key as string;
          cfg.workspace_id = result.workspace_id as string;
          console.log(`Workspace provisioned: ${result.workspace_id}`);
        } catch (err) {
          printError(`Provisioning failed: ${err}`);
        }
      } else {
        console.log("Keeping existing credentials. You can manually update with: corp config set api_key <key>");
      }
    }
  }

  saveConfig(cfg);
  console.log("\nConfig saved to ~/.corp/config.json");
  console.log("Run 'corp status' to verify your connection.");
}
