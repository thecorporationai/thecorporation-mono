import { requireConfig } from "../config.js";
import { CorpAPIClient } from "../api-client.js";
import { printBillingPanel, printError, printSuccess, printJson } from "../output.js";
import type { ApiRecord } from "../types.js";

function makeClient() {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  return new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
}

function enrichBillingStatus(status: ApiRecord): ApiRecord {
  if (typeof status.status_explanation === "string" && status.status_explanation.trim()) {
    return status;
  }

  const plan = String(status.plan ?? status.tier ?? "").trim();
  const subStatus = String(status.status ?? "").trim();
  if (subStatus !== "pending_checkout") {
    return status;
  }

  const statusExplanation = plan
    ? `Checkout for the ${plan} plan has started, but billing will not become active until Stripe checkout is completed.`
    : "Checkout has started, but billing will not become active until Stripe checkout is completed.";

  return { ...status, status_explanation: statusExplanation };
}

export async function billingCommand(opts: { json?: boolean }): Promise<void> {
  const client = makeClient();
  try {
    const [status, plans] = await Promise.all([client.getBillingStatus(), client.getBillingPlans()]);
    const enrichedStatus = enrichBillingStatus(status);
    if (opts.json) printJson({ status: enrichedStatus, plans });
    else printBillingPanel(enrichedStatus, plans);
  } catch (err) { printError(`Failed to fetch billing info: ${err}`); process.exit(1); }
}

export async function billingPortalCommand(): Promise<void> {
  const client = makeClient();
  try {
    const result = await client.createBillingPortal();
    const url = result.portal_url as string;
    if (!url) { printError("No portal URL returned. Ensure you have an active subscription."); process.exit(1); }
    printSuccess("Stripe Customer Portal URL:");
    console.log(url);
  } catch (err) { printError(`Failed to create portal session: ${err}`); process.exit(1); }
}

export async function billingUpgradeCommand(opts: { plan: string }): Promise<void> {
  const client = makeClient();
  try {
    const result = await client.createBillingCheckout(opts.plan);
    const url = result.checkout_url as string;
    if (!url) { printError("No checkout URL returned."); process.exit(1); }
    printSuccess(`Stripe Checkout URL for ${opts.plan}:`);
    console.log(url);
  } catch (err) { printError(`Failed to create checkout session: ${err}`); process.exit(1); }
}
