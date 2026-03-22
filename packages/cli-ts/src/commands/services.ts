import { requireConfig } from "../config.js";
import { CorpAPIClient } from "../api-client.js";
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
import { ReferenceResolver } from "../references.js";
import chalk from "chalk";

export async function servicesCatalogCommand(opts: {
  json?: boolean;
}): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    const items = await client.listServiceCatalog();
    if (opts.json) {
      printJson(items);
      return;
    }
    printServiceCatalogTable(items);
  } catch (err) {
    printError(`Failed to list service catalog: ${err}`);
    process.exit(1);
  }
}

export async function servicesBuyCommand(
  slug: string,
  opts: { entityId?: string; json?: boolean; dryRun?: boolean },
): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const eid = await resolver.resolveEntity(opts.entityId);
    const payload = { entity_id: eid, service_slug: slug };
    if (opts.dryRun) {
      printDryRun("services.create_request", payload);
      return;
    }
    const result = await client.createServiceRequest(payload);
    await resolver.stabilizeRecord("service_request", result, eid);
    resolver.rememberFromRecord("service_request", result, eid);

    // Auto-begin checkout to get the URL.
    const requestId = String(result.request_id ?? result.id ?? "");
    if (requestId) {
      const checkout = await client.beginServiceCheckout(requestId, { entity_id: eid });
      if (opts.json) {
        printJson(checkout);
        return;
      }
      printSuccess(`Service request created: ${requestId}`);
      printReferenceSummary("service_request", result, { showReuseHint: true });
      if (checkout.checkout_url) {
        console.log(`\n  ${chalk.bold("Checkout URL:")} ${checkout.checkout_url}`);
      }
      console.log(chalk.dim("\n  Next steps:"));
      console.log(chalk.dim("    Complete payment at the checkout URL above"));
      console.log(chalk.dim("    corp services list --entity-id <id>"));
    } else {
      printWriteResult(result, "Service request created", {
        referenceKind: "service_request",
        showReuseHint: true,
      });
    }
  } catch (err) {
    printError(`Failed to create service request: ${err}`);
    process.exit(1);
  }
}

export async function servicesListCommand(opts: {
  entityId?: string;
  json?: boolean;
}): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const eid = await resolver.resolveEntity(opts.entityId);
    const requests = await client.listServiceRequests(eid);
    const stable = await resolver.stabilizeRecords("service_request", requests, eid);
    if (opts.json) {
      printJson(stable);
      return;
    }
    printServiceRequestsTable(stable);
  } catch (err) {
    printError(`Failed to list service requests: ${err}`);
    process.exit(1);
  }
}

export async function servicesShowCommand(
  ref_: string,
  opts: { entityId?: string; json?: boolean },
): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const eid = await resolver.resolveEntity(opts.entityId);
    const requestId = await resolver.resolveServiceRequest(eid, ref_);
    const result = await client.getServiceRequest(requestId, eid);
    await resolver.stabilizeRecord("service_request", result, eid);
    resolver.rememberFromRecord("service_request", result, eid);
    if (opts.json) {
      printJson(result);
      return;
    }
    printReferenceSummary("service_request", result);
  } catch (err) {
    printError(`Failed to show service request: ${err}`);
    process.exit(1);
  }
}

export async function servicesFulfillCommand(
  ref_: string,
  opts: { entityId?: string; note?: string; json?: boolean },
): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const eid = await resolver.resolveEntity(opts.entityId);
    const requestId = await resolver.resolveServiceRequest(eid, ref_);
    const result = await client.fulfillServiceRequest(requestId, {
      entity_id: eid,
      note: opts.note,
    });
    await resolver.stabilizeRecord("service_request", result, eid);
    resolver.rememberFromRecord("service_request", result, eid);
    if (opts.json) {
      printJson(result);
      return;
    }
    printSuccess(`Service request fulfilled: ${requestId}`);
    printReferenceSummary("service_request", result, { showReuseHint: true });
  } catch (err) {
    printError(`Failed to fulfill service request: ${err}`);
    process.exit(1);
  }
}

export async function servicesCancelCommand(
  ref_: string,
  opts: { entityId?: string; json?: boolean },
): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const eid = await resolver.resolveEntity(opts.entityId);
    const requestId = await resolver.resolveServiceRequest(eid, ref_);
    const result = await client.cancelServiceRequest(requestId, {
      entity_id: eid,
    });
    await resolver.stabilizeRecord("service_request", result, eid);
    resolver.rememberFromRecord("service_request", result, eid);
    if (opts.json) {
      printJson(result);
      return;
    }
    printSuccess(`Service request cancelled: ${requestId}`);
    printReferenceSummary("service_request", result, { showReuseHint: true });
  } catch (err) {
    printError(`Failed to cancel service request: ${err}`);
    process.exit(1);
  }
}
