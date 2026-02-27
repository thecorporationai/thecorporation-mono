import { requireConfig } from "../config.js";
import { CorpAPIClient } from "../api-client.js";
import { printBillingPanel, printError, printSuccess, printJson } from "../output.js";
import { execSync } from "node:child_process";

function makeClient() {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  return new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
}

function openUrl(url: string) {
  try {
    const cmd = process.platform === "darwin" ? "open" : process.platform === "win32" ? "start" : "xdg-open";
    execSync(`${cmd} ${JSON.stringify(url)}`, { stdio: "ignore" });
  } catch { /* browser open is best-effort */ }
}

export async function billingCommand(opts: { json?: boolean }): Promise<void> {
  const client = makeClient();
  try {
    const [status, plans] = await Promise.all([client.getBillingStatus(), client.getBillingPlans()]);
    if (opts.json) printJson({ status, plans });
    else printBillingPanel(status, plans);
  } catch (err) { printError(`Failed to fetch billing info: ${err}`); process.exit(1); }
}

export async function billingPortalCommand(): Promise<void> {
  const client = makeClient();
  try {
    const result = await client.createBillingPortal();
    const url = result.portal_url as string;
    if (!url) { printError("No portal URL returned. Ensure you have an active subscription."); process.exit(1); }
    console.log(`Opening Stripe Customer Portal...\n${url}`);
    openUrl(url);
    printSuccess("Portal opened in your browser.");
  } catch (err) { printError(`Failed to create portal session: ${err}`); process.exit(1); }
}

export async function billingUpgradeCommand(opts: { tier: string }): Promise<void> {
  const client = makeClient();
  try {
    const result = await client.createBillingCheckout(opts.tier);
    const url = result.checkout_url as string;
    if (!url) { printError("No checkout URL returned."); process.exit(1); }
    console.log(`Opening Stripe Checkout for ${opts.tier}...\n${url}`);
    openUrl(url);
    printSuccess("Checkout opened in your browser.");
  } catch (err) { printError(`Failed to create checkout session: ${err}`); process.exit(1); }
}
