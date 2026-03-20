import type { CommandDef, CommandContext } from "./types.js";
import {
  printReferenceSummary,
  printWorkItemsTable,
  printError,
  printJson,
  printWriteResult,
} from "../output.js";
import { confirm } from "@inquirer/prompts";
import chalk from "chalk";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Work-item registry entries
// ---------------------------------------------------------------------------

export const workItemCommands: CommandDef[] = [
  // --- work-items (list) ---
  {
    name: "work-items",
    description: "Long-term work item coordination",
    route: { method: "GET", path: "/v1/entities/{eid}/work-items" },
    entity: true,
    optQP: ["status", "category"],
    display: {
      title: "Work Items",
      cols: ["title>Title", "category>Category", "effective_status|status>Status", "@deadline>Deadline", "#work_item_id|id>ID"],
    },
    options: [
      { flags: "--status <status>", description: "Filter by status (open, claimed, completed, cancelled)" },
      { flags: "--category <category>", description: "Filter by category" },
    ],
    handler: async (ctx) => {
      const eid = await ctx.resolver.resolveEntity(ctx.opts.entityId as string | undefined);
      const params: Record<string, string> = {};
      if (ctx.opts.status) params.status = ctx.opts.status as string;
      if (ctx.opts.category) params.category = ctx.opts.category as string;
      const items = await ctx.client.listWorkItems(eid, Object.keys(params).length > 0 ? params : undefined);
      await ctx.resolver.stabilizeRecords("work_item", items, eid);
      if (ctx.opts.json) { ctx.writer.json(items); return; }
      if (items.length === 0) { ctx.writer.writeln("No work items found."); return; }
      printWorkItemsTable(items);
    },
    examples: [
      "corp work-items",
      'corp work-items create --title "File Q1 taxes" --category compliance --deadline 2026-04-15',
      "corp work-items claim @last:work_item --by bookkeeper-agent",
      'corp work-items complete @last:work_item --by bookkeeper-agent --result "Filed 1120 for Q1"',
    ],
  },

  // --- work-items show <item-ref> ---
  {
    name: "work-items show",
    description: "Show work item detail",
    route: { method: "GET", path: "/v1/entities/{eid}/work-items/{pos}" },
    entity: true,
    args: [{ name: "item-ref", required: true, description: "Work item reference" }],
    display: { title: "Work Item Detail" },
    handler: async (ctx) => {
      const itemRef = ctx.positional[0];
      const eid = await ctx.resolver.resolveEntity(ctx.opts.entityId as string | undefined);
      const resolvedWorkItemId = await ctx.resolver.resolveWorkItem(eid, itemRef);
      const w = await ctx.client.getWorkItem(eid, resolvedWorkItemId);
      await ctx.resolver.stabilizeRecord("work_item", w, eid);
      if (ctx.opts.json) { ctx.writer.json(w); return; }
      console.log(chalk.cyan("\u2500".repeat(40)));
      console.log(chalk.cyan.bold("  Work Item Detail"));
      console.log(chalk.cyan("\u2500".repeat(40)));
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
      console.log(chalk.cyan("\u2500".repeat(40)));
    },
  },

  // --- work-items create ---
  {
    name: "work-items create",
    description: "Create a new work item",
    route: { method: "POST", path: "/v1/entities/{eid}/work-items" },
    entity: true,
    options: [
      { flags: "--title <title>", description: "Work item title", required: true },
      { flags: "--category <category>", description: "Work item category" },
      { flags: "--description <desc>", description: "Description" },
      { flags: "--deadline <date>", description: "Deadline (YYYY-MM-DD)" },
      { flags: "--asap", description: "Mark as ASAP priority" },
      { flags: "--created-by <name>", description: "Creator identifier" },
    ],
    handler: async (ctx) => {
      const eid = await ctx.resolver.resolveEntity(ctx.opts.entityId as string | undefined);
      const category = ctx.opts.category as string | undefined;
      if (!category) {
        printError("required option '--category <category>' not specified");
        process.exit(1);
      }
      const data: Record<string, unknown> = { title: ctx.opts.title as string, category };
      if (ctx.opts.description) data.description = ctx.opts.description as string;
      if (ctx.opts.deadline) data.deadline = ctx.opts.deadline as string;
      if (ctx.opts.asap) data.asap = true;
      if (ctx.opts.createdBy) data.created_by_actor = await ctx.resolver.resolveWorkItemActor(eid, ctx.opts.createdBy as string);
      const result = await ctx.client.createWorkItem(eid, data);
      await ctx.resolver.stabilizeRecord("work_item", result, eid);
      ctx.resolver.rememberFromRecord("work_item", result, eid);
      ctx.writer.writeResult(
        result,
        `Work item created: ${result.work_item_id ?? result.id ?? "OK"}`,
        { jsonOnly: ctx.opts.json, referenceKind: "work_item", showReuseHint: true },
      );
    },
    produces: { kind: "work_item" },
    successTemplate: "Work item created: {title}",
  },

  // --- work-items claim <item-ref> ---
  {
    name: "work-items claim",
    description: "Claim a work item",
    route: { method: "POST", path: "/v1/entities/{eid}/work-items/{pos}/claim" },
    entity: true,
    args: [{ name: "item-ref", required: true, description: "Work item reference" }],
    options: [
      { flags: "--by <name>", description: "Agent or user claiming the item (required)" },
      { flags: "--claimer <name>", description: "Alias for --by" },
      { flags: "--ttl <seconds>", description: "Auto-release TTL in seconds", type: "int" },
    ],
    handler: async (ctx) => {
      const itemRef = ctx.positional[0];
      const eid = await ctx.resolver.resolveEntity(ctx.opts.entityId as string | undefined);
      const resolvedWorkItemId = await ctx.resolver.resolveWorkItem(eid, itemRef);
      const claimedBy = (ctx.opts.by as string | undefined) ?? (ctx.opts.claimer as string | undefined);
      if (!claimedBy) {
        printError("required option '--by <name>' not specified");
        process.exit(1);
      }
      const data: Record<string, unknown> = {
        claimed_by_actor: await ctx.resolver.resolveWorkItemActor(eid, claimedBy),
      };
      if (ctx.opts.ttl != null) data.ttl_seconds = ctx.opts.ttl as number;
      const result = await ctx.client.claimWorkItem(eid, resolvedWorkItemId, data);
      ctx.writer.writeResult(result, `Work item ${resolvedWorkItemId} claimed by ${claimedBy}.`, { jsonOnly: ctx.opts.json });
    },
  },

  // --- work-items complete <item-ref> ---
  {
    name: "work-items complete",
    description: "Mark a work item as completed",
    route: { method: "POST", path: "/v1/entities/{eid}/work-items/{pos}/complete" },
    entity: true,
    args: [{ name: "item-ref", required: true, description: "Work item reference" }],
    options: [
      { flags: "--by <name>", description: "Agent or user completing the item (required)" },
      { flags: "--completed-by <name>", description: "Alias for --by" },
      { flags: "--result <text>", description: "Completion result or notes" },
      { flags: "--notes <text>", description: "Alias for --result" },
    ],
    handler: async (ctx) => {
      const itemRef = ctx.positional[0];
      const eid = await ctx.resolver.resolveEntity(ctx.opts.entityId as string | undefined);
      const resolvedWorkItemId = await ctx.resolver.resolveWorkItem(eid, itemRef);
      const completedBy = (ctx.opts.by as string | undefined) ?? (ctx.opts.completedBy as string | undefined);
      if (!completedBy) {
        printError("required option '--by <name>' not specified");
        process.exit(1);
      }
      const data: Record<string, unknown> = {
        completed_by_actor: await ctx.resolver.resolveWorkItemActor(eid, completedBy),
      };
      const resultText = (ctx.opts.result as string | undefined) ?? (ctx.opts.notes as string | undefined);
      if (resultText) data.result = resultText;
      const result = await ctx.client.completeWorkItem(eid, resolvedWorkItemId, data);
      ctx.writer.writeResult(result, `Work item ${resolvedWorkItemId} completed.`, { jsonOnly: ctx.opts.json });
    },
  },

  // --- work-items release <item-ref> ---
  {
    name: "work-items release",
    description: "Release a claimed work item",
    route: { method: "POST", path: "/v1/entities/{eid}/work-items/{pos}/release" },
    entity: true,
    args: [{ name: "item-ref", required: true, description: "Work item reference" }],
    handler: async (ctx) => {
      const itemRef = ctx.positional[0];
      const eid = await ctx.resolver.resolveEntity(ctx.opts.entityId as string | undefined);
      const resolvedWorkItemId = await ctx.resolver.resolveWorkItem(eid, itemRef);
      const result = await ctx.client.releaseWorkItem(eid, resolvedWorkItemId);
      ctx.writer.writeResult(result, `Work item ${resolvedWorkItemId} claim released.`, { jsonOnly: ctx.opts.json });
    },
  },

  // --- work-items cancel <item-ref> ---
  {
    name: "work-items cancel",
    description: "Cancel a work item",
    route: { method: "POST", path: "/v1/entities/{eid}/work-items/{pos}/cancel" },
    entity: true,
    args: [{ name: "item-ref", required: true, description: "Work item reference" }],
    options: [
      { flags: "--yes, -y", description: "Skip confirmation prompt" },
    ],
    handler: async (ctx) => {
      const itemRef = ctx.positional[0];
      const eid = await ctx.resolver.resolveEntity(ctx.opts.entityId as string | undefined);
      const resolvedWorkItemId = await ctx.resolver.resolveWorkItem(eid, itemRef);
      if (!ctx.opts.yes) {
        const ok = await confirm({
          message: `Cancel work item ${resolvedWorkItemId}?`,
          default: false,
        });
        if (!ok) { console.log("Cancelled."); return; }
      }
      const result = await ctx.client.cancelWorkItem(eid, resolvedWorkItemId);
      ctx.writer.writeResult(result, `Work item ${resolvedWorkItemId} cancelled.`, { jsonOnly: ctx.opts.json });
    },
  },
];
