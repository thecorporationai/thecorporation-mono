import chalk from "chalk";
import type { CommandDef, CommandContext } from "./types.js";
import {
  loadConfig,
  requireConfig,
  resolveEntityId,
  getActiveEntityId,
  saveConfig,
  setActiveEntityId,
} from "../config.js";
import { CorpAPIClient } from "../api-client.js";
import { ReferenceResolver, getReferenceAlias } from "../references.js";
import {
  printError,
  printJson,
  printStatusPanel,
  printNextSteps,
  printBillingPanel,
  printReferenceSummary,
  printSuccess,
  printWarning,
} from "../output.js";
import { withSpinner } from "../spinner.js";
import type { NextStepsResponse, NextStepItem } from "@thecorporation/corp-tools";
import type { ApiRecord } from "../types.js";

// ---------------------------------------------------------------------------
// Local helpers (relocated from individual command files)
// ---------------------------------------------------------------------------

/** Pre-flight local checks for `next` (runs before any API call). */
function localChecks(): NextStepItem[] {
  const items: NextStepItem[] = [];
  let cfg;
  try {
    cfg = loadConfig();
  } catch {
    items.push({
      category: "setup",
      title: "Run initial setup",
      description: "No configuration found",
      command: "npx corp setup",
      urgency: "critical",
    });
    return items;
  }

  if (!cfg.api_key) {
    items.push({
      category: "setup",
      title: "Run setup to configure API key",
      description: "No API key configured",
      command: "npx corp setup",
      urgency: "critical",
    });
    return items;
  }

  if (!cfg.workspace_id) {
    items.push({
      category: "setup",
      title: "Claim a workspace",
      description: "No workspace configured",
      command: "npx corp claim <code>",
      urgency: "critical",
    });
    return items;
  }

  if (!getActiveEntityId(cfg)) {
    items.push({
      category: "setup",
      title: "Set an active entity",
      description: "No active entity — set one to get entity-specific recommendations",
      command: "npx corp use <entity-name>",
      urgency: "high",
    });
  }

  return items;
}

/** Enrich billing status with explanation when pending_checkout. */
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

// ---------------------------------------------------------------------------
// Registry entries
// ---------------------------------------------------------------------------

export const workspaceCommands: CommandDef[] = [
  // --- status ---
  {
    name: "status",
    description: "Workspace summary",
    route: { method: "GET", path: "/v1/workspaces/{wid}/status" },
    display: { title: "Corp Status" },
    handler: async (ctx) => {
      try {
        const data = await withSpinner("Loading", () => ctx.client.getStatus());
        if (ctx.opts.json) {
          ctx.writer.json(data);
        } else {
          printStatusPanel(data);
        }
      } catch (err) {
        ctx.writer.error(`Failed to fetch status: ${err}`);
        process.exit(1);
      }
    },
  },

  // --- context / whoami ---
  {
    name: "context",
    description: "Show the active workspace, user, and entity context",
    aliases: ["whoami"],
    display: { title: "Corp Context" },
    handler: async (ctx) => {
      const rawCfg = loadConfig();
      const resolver = new ReferenceResolver(ctx.client, rawCfg);

      try {
        const [status, entities] = await Promise.all([
          ctx.client.getStatus(),
          ctx.client.listEntities(),
        ]);
        await resolver.stabilizeRecords("entity", entities);

        const activeEntityId = getActiveEntityId(rawCfg);
        const activeEntity = activeEntityId
          ? entities.find((entity) => entity.entity_id === activeEntityId) ?? null
          : null;

        const [contactsResult, documentsResult, workItemsResult] = activeEntity
          ? await Promise.allSettled([
              ctx.client.listContacts(String(activeEntity.entity_id)),
              ctx.client.getEntityDocuments(String(activeEntity.entity_id)),
              ctx.client.listWorkItems(String(activeEntity.entity_id)),
            ])
          : [null, null, null];

        const payload = {
          user: {
            name: rawCfg.user?.name ?? "",
            email: rawCfg.user?.email ?? "",
          },
          workspace: {
            workspace_id: rawCfg.workspace_id,
            api_url: rawCfg.api_url,
            status,
          },
          active_entity: activeEntity
            ? {
                entity: activeEntity,
                summary: {
                  contact_count:
                    contactsResult && contactsResult.status === "fulfilled"
                      ? contactsResult.value.length
                      : null,
                  document_count:
                    documentsResult && documentsResult.status === "fulfilled"
                      ? documentsResult.value.length
                      : null,
                  work_item_count:
                    workItemsResult && workItemsResult.status === "fulfilled"
                      ? workItemsResult.value.length
                      : null,
                },
              }
            : null,
          entity_count: entities.length,
        };

        if (ctx.opts.json) {
          ctx.writer.json(payload);
          return;
        }

        console.log(chalk.blue("\u2500".repeat(50)));
        console.log(chalk.blue.bold("  Corp Context"));
        console.log(chalk.blue("\u2500".repeat(50)));
        console.log(`  ${chalk.bold("User:")} ${payload.user.name || "N/A"} <${payload.user.email || "N/A"}>`);
        console.log(`  ${chalk.bold("Workspace:")} ${rawCfg.workspace_id}`);
        console.log(`  ${chalk.bold("API URL:")} ${rawCfg.api_url}`);
        console.log(`  ${chalk.bold("Entities:")} ${entities.length}`);
        if (payload.active_entity) {
          console.log(`  ${chalk.bold("Active Entity:")} ${payload.active_entity.entity.legal_name ?? payload.active_entity.entity.entity_id}`);
          printReferenceSummary("entity", payload.active_entity.entity, { label: "Active Entity Ref:" });
          console.log(`  ${chalk.bold("Contacts:")} ${payload.active_entity.summary.contact_count ?? "N/A"}`);
          console.log(`  ${chalk.bold("Documents:")} ${payload.active_entity.summary.document_count ?? "N/A"}`);
          console.log(`  ${chalk.bold("Work Items:")} ${payload.active_entity.summary.work_item_count ?? "N/A"}`);
        } else {
          console.log(`  ${chalk.bold("Active Entity:")} none`);
        }
        if ((status as Record<string, unknown>).next_deadline) {
          console.log(`  ${chalk.bold("Next Deadline:")} ${(status as Record<string, unknown>).next_deadline}`);
        }
        console.log(chalk.blue("\u2500".repeat(50)));
      } catch (err) {
        ctx.writer.error(`Failed to fetch context: ${err}`);
        process.exit(1);
      }
    },
  },

  // --- use <entity-ref> ---
  {
    name: "use",
    description: "Set the active entity by name, short ID, or reference",
    args: [{ name: "entity-ref", required: true, description: "Entity name, short ID, or reference" }],
    handler: async (ctx) => {
      const entityRef = ctx.positional[0];
      const cfg = requireConfig("api_url", "api_key", "workspace_id");
      const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
      const resolver = new ReferenceResolver(client, cfg);
      try {
        const entityId = await resolver.resolveEntity(entityRef);
        setActiveEntityId(cfg, entityId);
        saveConfig(cfg);
        const alias = getReferenceAlias("entity", { entity_id: entityId }) ?? entityId;
        ctx.writer.success(`Active entity set to ${alias} (${entityId})`);
      } catch (err) {
        ctx.writer.error(`Failed to resolve entity: ${err}`);
        process.exit(1);
      }
    },
  },

  // --- next ---
  {
    name: "next",
    description: "See what to do next — your recommended actions",
    options: [
      { flags: "--entity-id <ref>", description: "Entity to check (default: active entity)" },
      { flags: "--workspace", description: "Show recommendations across all entities" },
    ],
    examples: [
      "$ corp next                          # Next steps for active entity",
      "$ corp next --workspace              # Next steps across all entities",
      "$ corp next --entity-id ent_abc123   # Next steps for specific entity",
      "$ corp next --json                   # JSON output for scripting",
    ],
    handler: async (ctx) => {
      const opts = ctx.opts as { entityId?: string; workspace?: boolean; json?: boolean };

      if (opts.entityId && opts.workspace) {
        ctx.writer.error("--entity-id and --workspace are mutually exclusive");
        process.exit(1);
      }

      const localItems = localChecks();
      const hasCriticalLocal = localItems.some((i) => i.urgency === "critical");

      if (hasCriticalLocal) {
        const top = localItems[0];
        const backlog = localItems.slice(1);
        const summary = { critical: 0, high: 0, medium: 0, low: 0 };
        for (const item of [top, ...backlog]) {
          const key = item.urgency as keyof typeof summary;
          if (key in summary) summary[key]++;
        }
        const response = { top, backlog, summary };
        if (opts.json) {
          ctx.writer.json(response);
        } else {
          printNextSteps(response);
        }
        return;
      }

      const cfg = requireConfig("api_url", "api_key", "workspace_id");
      const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);

      try {
        let data: NextStepsResponse;
        if (opts.workspace) {
          data = await withSpinner("Loading", () => client.getWorkspaceNextSteps(), opts.json);
        } else {
          const entityId = resolveEntityId(cfg, opts.entityId);
          data = await withSpinner("Loading", () => client.getEntityNextSteps(entityId), opts.json);
        }

        // Merge non-critical local items into backlog
        if (localItems.length > 0) {
          data.backlog.push(...localItems);
          const all = [data.top, ...data.backlog].filter(Boolean) as NextStepItem[];
          data.summary = { critical: 0, high: 0, medium: 0, low: 0 };
          for (const item of all) {
            const key = item.urgency as keyof typeof data.summary;
            if (key in data.summary) data.summary[key]++;
          }
        }

        if (opts.json) {
          ctx.writer.json(data);
        } else {
          printNextSteps(data);
        }
      } catch (err) {
        ctx.writer.error(`Failed to fetch next steps: ${err}`);
        process.exit(1);
      }
    },
  },

  // --- obligations (pure read) ---
  {
    name: "obligations",
    description: "List obligations with urgency tiers",
    route: { method: "GET", path: "/v1/obligations/summary" },
    display: {
      title: "Obligations",
      listKey: "obligations",
      cols: [
        "obligation_type>Type",
        "urgency>Urgency",
        "@due_at>Due",
        "status>Status",
        "#obligation_id>ID",
      ],
    },
    optQP: ["tier"],
    options: [{ flags: "--tier <tier>", description: "Filter by urgency tier" }],
  },

  // --- digest ---
  {
    name: "digest",
    description: "View or trigger daily digests",
    route: { method: "GET", path: "/v1/digests" },
    display: { title: "Digests" },
    options: [
      { flags: "--trigger", description: "Trigger digest now" },
      { flags: "--key <key>", description: "Get specific digest by key" },
      { flags: "--entity-id <ref>", description: "Entity reference (ID, short ID, @last, or unique name)" },
    ],
    handler: async (ctx) => {
      const opts = ctx.opts as { trigger?: boolean; key?: string; entityId?: string; json?: boolean };
      try {
        if (opts.trigger) {
          const result = await ctx.client.triggerDigest();
          const message = (() => {
            const value = (result as Record<string, unknown>).message;
            return typeof value === "string" && value.trim() ? value : null;
          })();
          if (!opts.json) {
            ctx.writer.success(result.digest_count > 0 ? "Digest triggered." : "Digest trigger accepted.");
          }
          if (message && !opts.json) {
            ctx.writer.warning(message);
          }
          ctx.writer.json(result);
        } else if (opts.key) {
          const result = await ctx.client.getDigest(opts.key);
          ctx.writer.json(result);
        } else {
          const digests = await ctx.client.listDigests();
          if (digests.length === 0) {
            if (opts.json) {
              ctx.writer.json([]);
            } else {
              ctx.writer.writeln("No digest history found.");
            }
          } else {
            ctx.writer.json(digests);
          }
        }
      } catch (err) {
        ctx.writer.error(`Failed: ${err}`);
        process.exit(1);
      }
    },
  },

  // --- billing ---
  {
    name: "billing",
    description: "Billing status, plans, and subscription management",
    display: { title: "Billing" },
    handler: async (ctx) => {
      try {
        const [status, plans] = await Promise.all([
          ctx.client.getBillingStatus(),
          ctx.client.getBillingPlans(),
        ]);
        const enrichedStatus = enrichBillingStatus(status);
        if (ctx.opts.json) {
          ctx.writer.json({ status: enrichedStatus, plans });
        } else {
          printBillingPanel(enrichedStatus, plans);
        }
      } catch (err) {
        ctx.writer.error(`Failed to fetch billing info: ${err}`);
        process.exit(1);
      }
    },
  },

  // --- billing portal ---
  {
    name: "billing portal",
    description: "Open Stripe Customer Portal",
    route: { method: "POST", path: "/v1/billing/portal" },
    handler: async (ctx) => {
      try {
        const result = await ctx.client.createBillingPortal();
        const url = result.portal_url as string;
        if (!url) {
          ctx.writer.error("No portal URL returned. Ensure you have an active subscription.");
          process.exit(1);
        }
        ctx.writer.success("Stripe Customer Portal URL:");
        ctx.writer.writeln(url);
      } catch (err) {
        ctx.writer.error(`Failed to create portal session: ${err}`);
        process.exit(1);
      }
    },
  },

  // --- billing upgrade ---
  {
    name: "billing upgrade",
    description: "Open Stripe Checkout to upgrade your plan",
    route: { method: "POST", path: "/v1/billing/checkout" },
    options: [
      {
        flags: "--plan <plan>",
        description: "Plan ID to upgrade to (free, pro, enterprise)",
        default: "pro",
        choices: ["free", "pro", "enterprise"],
      },
    ],
    handler: async (ctx) => {
      const opts = ctx.opts as { plan: string };
      try {
        const result = await ctx.client.createBillingCheckout(opts.plan);
        const url = result.checkout_url as string;
        if (!url) {
          ctx.writer.error("No checkout URL returned.");
          process.exit(1);
        }
        ctx.writer.success(`Stripe Checkout URL for ${opts.plan}:`);
        ctx.writer.writeln(url);
      } catch (err) {
        ctx.writer.error(`Failed to create checkout session: ${err}`);
        process.exit(1);
      }
    },
  },
];
