import { requireConfig } from "../config.js";
import { CorpAPIClient } from "../api-client.js";
import { printReferenceSummary, printWorkItemsTable, printError, printJson, printWriteResult } from "../output.js";
import { ReferenceResolver } from "../references.js";
import chalk from "chalk";

function actorLabel(record: Record<string, unknown>, key: "claimed_by" | "completed_by" | "created_by"): string | undefined {
  const actor = record[`${key}_actor`];
  if (actor && typeof actor === "object" && !Array.isArray(actor)) {
    const label = (actor as Record<string, unknown>).label;
    const actorType = (actor as Record<string, unknown>).actor_type;
    if (typeof label === "string" && label.trim()) {
      return typeof actorType === "string" && actorType.trim()
        ? `${label} (${actorType})`
        : label;
    }
  }
  const legacy = record[key];
  return typeof legacy === "string" && legacy.trim() ? legacy : undefined;
}

export async function workItemsListCommand(opts: { entityId?: string; json?: boolean; status?: string; category?: string }): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const eid = await resolver.resolveEntity(opts.entityId);
    const params: Record<string, string> = {};
    if (opts.status) params.status = opts.status;
    if (opts.category) params.category = opts.category;
    const items = await client.listWorkItems(eid, Object.keys(params).length > 0 ? params : undefined);
    await resolver.stabilizeRecords("work_item", items, eid);
    if (opts.json) printJson(items);
    else if (items.length === 0) console.log("No work items found.");
    else printWorkItemsTable(items);
  } catch (err) { printError(`Failed to fetch work items: ${err}`); process.exit(1); }
}

export async function workItemsShowCommand(workItemId: string, opts: { entityId?: string; json?: boolean }): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const eid = await resolver.resolveEntity(opts.entityId);
    const resolvedWorkItemId = await resolver.resolveWorkItem(eid, workItemId);
    const w = await client.getWorkItem(eid, resolvedWorkItemId);
    await resolver.stabilizeRecord("work_item", w, eid);
    if (opts.json) { printJson(w); return; }
    console.log(chalk.cyan("─".repeat(40)));
    console.log(chalk.cyan.bold("  Work Item Detail"));
    console.log(chalk.cyan("─".repeat(40)));
    console.log(`  ${chalk.bold("Title:")} ${w.title ?? "N/A"}`);
    console.log(`  ${chalk.bold("Category:")} ${w.category ?? "N/A"}`);
    console.log(`  ${chalk.bold("Status:")} ${w.effective_status ?? w.status ?? "N/A"}`);
    printReferenceSummary("work_item", w, { showReuseHint: true });
    if (w.description) console.log(`  ${chalk.bold("Description:")} ${w.description}`);
    if (w.deadline) console.log(`  ${chalk.bold("Deadline:")} ${w.deadline}`);
    if (w.asap) console.log(`  ${chalk.bold("Priority:")} ${chalk.red.bold("ASAP")}`);
    const claimedBy = actorLabel(w, "claimed_by");
    const completedBy = actorLabel(w, "completed_by");
    const createdBy = actorLabel(w, "created_by");
    if (claimedBy) console.log(`  ${chalk.bold("Claimed by:")} ${claimedBy}`);
    if (w.claimed_at) console.log(`  ${chalk.bold("Claimed at:")} ${w.claimed_at}`);
    if (w.claim_ttl_seconds) console.log(`  ${chalk.bold("Claim TTL:")} ${w.claim_ttl_seconds}s`);
    if (completedBy) console.log(`  ${chalk.bold("Completed by:")} ${completedBy}`);
    if (w.completed_at) console.log(`  ${chalk.bold("Completed at:")} ${w.completed_at}`);
    if (w.result) console.log(`  ${chalk.bold("Result:")} ${w.result}`);
    if (createdBy) console.log(`  ${chalk.bold("Created by:")} ${createdBy}`);
    console.log(`  ${chalk.bold("Created at:")} ${w.created_at ?? "N/A"}`);
    console.log(chalk.cyan("─".repeat(40)));
  } catch (err) { printError(`Failed to fetch work item: ${err}`); process.exit(1); }
}

export async function workItemsCreateCommand(opts: {
  entityId?: string; title: string; category?: string;
  description?: string; deadline?: string; asap?: boolean; createdBy?: string; json?: boolean;
}): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const eid = await resolver.resolveEntity(opts.entityId);
    if (!opts.category) {
      printError("Missing required option: --category <category>");
      process.exit(1);
    }
    const data: Record<string, unknown> = { title: opts.title, category: opts.category };
    if (opts.description) data.description = opts.description;
    if (opts.deadline) data.deadline = opts.deadline;
    if (opts.asap) data.asap = true;
    if (opts.createdBy) data.created_by_actor = await resolver.resolveWorkItemActor(eid, opts.createdBy);
    const result = await client.createWorkItem(eid, data);
    await resolver.stabilizeRecord("work_item", result, eid);
    resolver.rememberFromRecord("work_item", result, eid);
    printWriteResult(
      result,
      `Work item created: ${result.work_item_id ?? result.id ?? "OK"}`,
      { jsonOnly: opts.json, referenceKind: "work_item", showReuseHint: true },
    );
  } catch (err) { printError(`Failed to create work item: ${err}`); process.exit(1); }
}

export async function workItemsClaimCommand(workItemId: string, opts: {
  entityId?: string; claimedBy: string; ttl?: number; json?: boolean;
}): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const eid = await resolver.resolveEntity(opts.entityId);
    const resolvedWorkItemId = await resolver.resolveWorkItem(eid, workItemId);
    const data: Record<string, unknown> = {
      claimed_by_actor: await resolver.resolveWorkItemActor(eid, opts.claimedBy),
    };
    if (opts.ttl != null) data.ttl_seconds = opts.ttl;
    const result = await client.claimWorkItem(eid, resolvedWorkItemId, data);
    printWriteResult(result, `Work item ${resolvedWorkItemId} claimed by ${opts.claimedBy}.`, opts.json);
  } catch (err) { printError(`Failed to claim work item: ${err}`); process.exit(1); }
}

export async function workItemsCompleteCommand(workItemId: string, opts: {
  entityId?: string; completedBy: string; result?: string; json?: boolean;
}): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const eid = await resolver.resolveEntity(opts.entityId);
    const resolvedWorkItemId = await resolver.resolveWorkItem(eid, workItemId);
    const data: Record<string, unknown> = {
      completed_by_actor: await resolver.resolveWorkItemActor(eid, opts.completedBy),
    };
    if (opts.result) data.result = opts.result;
    const result = await client.completeWorkItem(eid, resolvedWorkItemId, data);
    printWriteResult(result, `Work item ${resolvedWorkItemId} completed.`, opts.json);
  } catch (err) { printError(`Failed to complete work item: ${err}`); process.exit(1); }
}

export async function workItemsReleaseCommand(workItemId: string, opts: { entityId?: string; json?: boolean }): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const eid = await resolver.resolveEntity(opts.entityId);
    const resolvedWorkItemId = await resolver.resolveWorkItem(eid, workItemId);
    const result = await client.releaseWorkItem(eid, resolvedWorkItemId);
    printWriteResult(result, `Work item ${resolvedWorkItemId} claim released.`, opts.json);
  } catch (err) { printError(`Failed to release work item: ${err}`); process.exit(1); }
}

export async function workItemsCancelCommand(workItemId: string, opts: { entityId?: string; json?: boolean }): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const eid = await resolver.resolveEntity(opts.entityId);
    const resolvedWorkItemId = await resolver.resolveWorkItem(eid, workItemId);
    const result = await client.cancelWorkItem(eid, resolvedWorkItemId);
    printWriteResult(result, `Work item ${resolvedWorkItemId} cancelled.`, opts.json);
  } catch (err) { printError(`Failed to cancel work item: ${err}`); process.exit(1); }
}
