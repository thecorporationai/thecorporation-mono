import { requireConfig, resolveEntityId } from "../config.js";
import { CorpAPIClient } from "../api-client.js";
import { printWorkItemsTable, printError, printSuccess, printJson } from "../output.js";
import chalk from "chalk";

export async function workItemsListCommand(opts: { entityId?: string; json?: boolean; status?: string; category?: string }): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const eid = resolveEntityId(cfg, opts.entityId);
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    const params: Record<string, string> = {};
    if (opts.status) params.status = opts.status;
    if (opts.category) params.category = opts.category;
    const items = await client.listWorkItems(eid, Object.keys(params).length > 0 ? params : undefined);
    if (opts.json) printJson(items);
    else if (items.length === 0) console.log("No work items found.");
    else printWorkItemsTable(items);
  } catch (err) { printError(`Failed to fetch work items: ${err}`); process.exit(1); }
}

export async function workItemsShowCommand(workItemId: string, opts: { entityId?: string; json?: boolean }): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const eid = resolveEntityId(cfg, opts.entityId);
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    const w = await client.getWorkItem(eid, workItemId);
    if (opts.json) { printJson(w); return; }
    console.log(chalk.cyan("─".repeat(40)));
    console.log(chalk.cyan.bold("  Work Item Detail"));
    console.log(chalk.cyan("─".repeat(40)));
    console.log(`  ${chalk.bold("Title:")} ${w.title ?? "N/A"}`);
    console.log(`  ${chalk.bold("Category:")} ${w.category ?? "N/A"}`);
    console.log(`  ${chalk.bold("Status:")} ${w.effective_status ?? w.status ?? "N/A"}`);
    if (w.description) console.log(`  ${chalk.bold("Description:")} ${w.description}`);
    if (w.deadline) console.log(`  ${chalk.bold("Deadline:")} ${w.deadline}`);
    if (w.asap) console.log(`  ${chalk.bold("Priority:")} ${chalk.red.bold("ASAP")}`);
    if (w.claimed_by) console.log(`  ${chalk.bold("Claimed by:")} ${w.claimed_by}`);
    if (w.claimed_at) console.log(`  ${chalk.bold("Claimed at:")} ${w.claimed_at}`);
    if (w.claim_ttl_seconds) console.log(`  ${chalk.bold("Claim TTL:")} ${w.claim_ttl_seconds}s`);
    if (w.completed_by) console.log(`  ${chalk.bold("Completed by:")} ${w.completed_by}`);
    if (w.completed_at) console.log(`  ${chalk.bold("Completed at:")} ${w.completed_at}`);
    if (w.result) console.log(`  ${chalk.bold("Result:")} ${w.result}`);
    if (w.created_by) console.log(`  ${chalk.bold("Created by:")} ${w.created_by}`);
    console.log(`  ${chalk.bold("Created at:")} ${w.created_at ?? "N/A"}`);
    console.log(chalk.cyan("─".repeat(40)));
  } catch (err) { printError(`Failed to fetch work item: ${err}`); process.exit(1); }
}

export async function workItemsCreateCommand(opts: {
  entityId?: string; title: string; category: string;
  description?: string; deadline?: string; asap?: boolean; createdBy?: string;
}): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const eid = resolveEntityId(cfg, opts.entityId);
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    const data: Record<string, unknown> = { title: opts.title, category: opts.category };
    if (opts.description) data.description = opts.description;
    if (opts.deadline) data.deadline = opts.deadline;
    if (opts.asap) data.asap = true;
    if (opts.createdBy) data.created_by = opts.createdBy;
    const result = await client.createWorkItem(eid, data);
    printSuccess(`Work item created: ${result.work_item_id ?? result.id ?? "OK"}`);
  } catch (err) { printError(`Failed to create work item: ${err}`); process.exit(1); }
}

export async function workItemsClaimCommand(workItemId: string, opts: {
  entityId?: string; claimedBy: string; ttl?: number;
}): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const eid = resolveEntityId(cfg, opts.entityId);
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    const data: Record<string, unknown> = { claimed_by: opts.claimedBy };
    if (opts.ttl != null) data.ttl_seconds = opts.ttl;
    await client.claimWorkItem(eid, workItemId, data);
    printSuccess(`Work item ${workItemId} claimed by ${opts.claimedBy}.`);
  } catch (err) { printError(`Failed to claim work item: ${err}`); process.exit(1); }
}

export async function workItemsCompleteCommand(workItemId: string, opts: {
  entityId?: string; completedBy: string; result?: string;
}): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const eid = resolveEntityId(cfg, opts.entityId);
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    const data: Record<string, unknown> = { completed_by: opts.completedBy };
    if (opts.result) data.result = opts.result;
    await client.completeWorkItem(eid, workItemId, data);
    printSuccess(`Work item ${workItemId} completed.`);
  } catch (err) { printError(`Failed to complete work item: ${err}`); process.exit(1); }
}

export async function workItemsReleaseCommand(workItemId: string, opts: { entityId?: string }): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const eid = resolveEntityId(cfg, opts.entityId);
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    await client.releaseWorkItem(eid, workItemId);
    printSuccess(`Work item ${workItemId} claim released.`);
  } catch (err) { printError(`Failed to release work item: ${err}`); process.exit(1); }
}

export async function workItemsCancelCommand(workItemId: string, opts: { entityId?: string }): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const eid = resolveEntityId(cfg, opts.entityId);
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    await client.cancelWorkItem(eid, workItemId);
    printSuccess(`Work item ${workItemId} cancelled.`);
  } catch (err) { printError(`Failed to cancel work item: ${err}`); process.exit(1); }
}
