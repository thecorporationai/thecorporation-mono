import type { CommandDef, CommandContext } from "./types.js";
import {
  printServiceCatalogTable,
  printServiceRequestsTable,
  printDryRun,
  printError,
  printJson,
  printReferenceSummary,
  printSuccess,
  printWriteResult,
} from "../output.js";
import chalk from "chalk";

// ---------------------------------------------------------------------------
// Service registry entries
// ---------------------------------------------------------------------------

export const serviceCommands: CommandDef[] = [
  // --- services (alias to catalog) ---
  {
    name: "services",
    description: "Service catalog and fulfillment",
    route: { method: "GET", path: "/v1/services/catalog" },
    entity: true,
    display: {
      title: "Service Catalog",
      cols: ["name>Name", "slug>Slug", "$price_cents>Price", "#service_id|id>ID"],
    },
    handler: async (ctx) => {
      const items = await ctx.client.listServiceCatalog();
      if (ctx.opts.json) { ctx.writer.json(items); return; }
      printServiceCatalogTable(items);
    },
    examples: ["corp services", "corp services --json"],
  },

  // --- services catalog ---
  {
    name: "services catalog",
    description: "List the service catalog",
    route: { method: "GET", path: "/v1/services/catalog" },
    display: {
      title: "Service Catalog",
      cols: ["name>Name", "slug>Slug", "$price_cents>Price", "#service_id|id>ID"],
    },
    handler: async (ctx) => {
      const items = await ctx.client.listServiceCatalog();
      if (ctx.opts.json) { ctx.writer.json(items); return; }
      printServiceCatalogTable(items);
    },
    examples: ["corp services catalog"],
  },

  // --- services list ---
  {
    name: "services list",
    description: "List service requests for an entity",
    route: { method: "GET", path: "/v1/entities/{eid}/service-requests" },
    entity: true,
    display: {
      title: "Service Requests",
      cols: ["service_slug>Service", "status>Status", "@created_at>Created", "#request_id|id>ID"],
    },
    handler: async (ctx) => {
      const eid = await ctx.resolver.resolveEntity(ctx.opts.entityId as string | undefined);
      const requests = await ctx.client.listServiceRequests(eid);
      const stable = await ctx.resolver.stabilizeRecords("service_request", requests, eid);
      if (ctx.opts.json) { ctx.writer.json(stable); return; }
      printServiceRequestsTable(stable);
    },
    examples: ["corp services list", "corp services list --json"],
  },

  // --- services show <ref> ---
  {
    name: "services show",
    description: "Show service request detail",
    route: { method: "GET", path: "/v1/service-requests/{pos}" },
    entity: true,
    args: [{ name: "ref", required: true, description: "Service request reference" }],
    display: { title: "Service Request Detail" },
    handler: async (ctx) => {
      const ref_ = ctx.positional[0];
      const eid = await ctx.resolver.resolveEntity(ctx.opts.entityId as string | undefined);
      const requestId = await ctx.resolver.resolveServiceRequest(eid, ref_);
      const result = await ctx.client.getServiceRequest(requestId, eid);
      await ctx.resolver.stabilizeRecord("service_request", result, eid);
      ctx.resolver.rememberFromRecord("service_request", result, eid);
      if (ctx.opts.json) { ctx.writer.json(result); return; }
      printReferenceSummary("service_request", result);
      printJson(result);
    },
    examples: ["corp services show", "corp services show --json"],
  },

  // --- services buy <slug> ---
  {
    name: "services buy",
    description: "Purchase a service from the catalog",
    route: { method: "POST", path: "/v1/service-requests" },
    entity: true,
    dryRun: true,
    args: [{ name: "slug", required: true, description: "Service catalog slug" }],
    handler: async (ctx) => {
      const slug = ctx.positional[0];
      const eid = await ctx.resolver.resolveEntity(ctx.opts.entityId as string | undefined);
      const payload = { entity_id: eid, service_slug: slug };
      if (ctx.dryRun) {
        ctx.writer.dryRun("services.create_request", payload);
        return;
      }
      const result = await ctx.client.createServiceRequest(payload);
      await ctx.resolver.stabilizeRecord("service_request", result, eid);
      ctx.resolver.rememberFromRecord("service_request", result, eid);

      // Auto-begin checkout to get the URL.
      const requestId = String(result.request_id ?? result.id ?? "");
      if (requestId) {
        const checkout = await ctx.client.beginServiceCheckout(requestId, { entity_id: eid });
        if (ctx.opts.json) { ctx.writer.json(checkout); return; }
        ctx.writer.success(`Service request created: ${requestId}`);
        printReferenceSummary("service_request", result, { showReuseHint: true });
        if (checkout.checkout_url) {
          console.log(`\n  ${chalk.bold("Checkout URL:")} ${checkout.checkout_url}`);
        }
        console.log(chalk.dim("\n  Next steps:"));
        console.log(chalk.dim("    Complete payment at the checkout URL above"));
        console.log(chalk.dim("    corp services list --entity-id <id>"));
      } else {
        ctx.writer.writeResult(result, "Service request created", {
          referenceKind: "service_request",
          showReuseHint: true,
        });
      }
    },
    produces: { kind: "service_request" },
    successTemplate: "Service request created",
    examples: ["corp services buy <slug>"],
  },

  // --- services fulfill <ref> ---
  {
    name: "services fulfill",
    description: "Mark a service request as fulfilled (operator)",
    route: { method: "POST", path: "/v1/service-requests/{pos}/fulfill" },
    entity: true,
    args: [{ name: "ref", required: true, description: "Service request reference" }],
    options: [
      { flags: "--note <note>", description: "Fulfillment note" },
    ],
    handler: async (ctx) => {
      const ref_ = ctx.positional[0];
      const eid = await ctx.resolver.resolveEntity(ctx.opts.entityId as string | undefined);
      const requestId = await ctx.resolver.resolveServiceRequest(eid, ref_);
      const result = await ctx.client.fulfillServiceRequest(requestId, {
        entity_id: eid,
        note: ctx.opts.note as string | undefined,
      });
      await ctx.resolver.stabilizeRecord("service_request", result, eid);
      ctx.resolver.rememberFromRecord("service_request", result, eid);
      if (ctx.opts.json) { ctx.writer.json(result); return; }
      ctx.writer.success(`Service request fulfilled: ${requestId}`);
      printReferenceSummary("service_request", result, { showReuseHint: true });
      printJson(result);
    },
    examples: ["corp services fulfill <ref>", "corp services fulfill --json"],
  },

  // --- services cancel <ref> ---
  {
    name: "services cancel",
    description: "Cancel a service request",
    route: { method: "POST", path: "/v1/service-requests/{pos}/cancel" },
    entity: true,
    args: [{ name: "ref", required: true, description: "Service request reference" }],
    handler: async (ctx) => {
      const ref_ = ctx.positional[0];
      const eid = await ctx.resolver.resolveEntity(ctx.opts.entityId as string | undefined);
      const requestId = await ctx.resolver.resolveServiceRequest(eid, ref_);
      const result = await ctx.client.cancelServiceRequest(requestId, {
        entity_id: eid,
      });
      await ctx.resolver.stabilizeRecord("service_request", result, eid);
      ctx.resolver.rememberFromRecord("service_request", result, eid);
      if (ctx.opts.json) { ctx.writer.json(result); return; }
      ctx.writer.success(`Service request cancelled: ${requestId}`);
      printReferenceSummary("service_request", result, { showReuseHint: true });
      printJson(result);
    },
    examples: ["corp services cancel <ref>"],
  },

  // ── Auto-generated from OpenAPI ──────────────────────────────
  {
    name: "services requests",
    description: "Submit a new service request",
    route: { method: "POST", path: "/v1/services/requests" },
    options: [
      { flags: "--obligation-id <obligation-id>", description: "Obligation ID" },
      { flags: "--service-slug <service-slug>", description: "Service Slug", required: true },
    ],
    examples: ["corp services requests --service-slug 'service-slug'", "corp services requests --json"],
    successTemplate: "Requests created",
  },
  {
    name: "services requests",
    description: "Submit a new service request",
    route: { method: "GET", path: "/v1/services/requests/{pos}" },
    entity: true,
    args: [{ name: "request-id", required: true, description: "Document request ID" }],
    display: { title: "Services Requests", cols: ["amount_cents>Amount Cents", "checkout_url>Checkout Url", "failed_at>Failed At", "fulfilled_at>Fulfilled At", "@created_at>Created At", "#entity_id>ID"] },
    examples: ["corp services requests", "corp services requests --json"],
  },
  {
    name: "services requests-cancel",
    description: "Cancel a service request",
    route: { method: "POST", path: "/v1/services/requests/{pos}/cancel" },
    args: [{ name: "request-id", required: true, description: "Document request ID" }],
    examples: ["corp services requests-cancel <request-id>"],
    successTemplate: "Requests Cancel created",
  },
  {
    name: "services requests-checkout",
    description: "Start checkout for a service request",
    route: { method: "POST", path: "/v1/services/requests/{pos}/checkout" },
    args: [{ name: "request-id", required: true, description: "Document request ID" }],
    examples: ["corp services requests-checkout <request-id>"],
    successTemplate: "Requests Checkout created",
  },
  {
    name: "services requests-fulfill",
    description: "Fulfill a service request",
    route: { method: "POST", path: "/v1/services/requests/{pos}/fulfill" },
    args: [{ name: "request-id", required: true, description: "Document request ID" }],
    options: [
      { flags: "--note <note>", description: "Note" },
    ],
    examples: ["corp services requests-fulfill <request-id>", "corp services requests-fulfill --json"],
    successTemplate: "Requests Fulfill created",
  },

];