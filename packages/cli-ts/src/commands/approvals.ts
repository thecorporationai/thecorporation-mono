import { requireConfig } from "../config.js";
import { CorpAPIClient } from "../api-client.js";
import { printApprovalsTable, printError, printSuccess, printJson } from "../output.js";

export async function approvalsListCommand(opts: { json?: boolean }): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    const approvals = await client.listPendingApprovals();
    if (opts.json) printJson(approvals);
    else if (approvals.length === 0) console.log("No pending approvals.");
    else printApprovalsTable(approvals);
  } catch (err) { printError(`Failed to fetch approvals: ${err}`); process.exit(1); }
}

export async function approvalsRespondCommand(
  approvalId: string,
  decision: string,
  opts: { message?: string }
): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    await client.respondApproval(approvalId, decision, opts.message);
    printSuccess(`Approval ${approvalId} ${decision}d.`);
  } catch (err) { printError(`Failed to respond to approval: ${err}`); process.exit(1); }
}
