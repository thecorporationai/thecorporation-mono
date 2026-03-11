import { requireConfig } from "../config.js";
import { CorpAPIClient } from "../api-client.js";
import { printEntitiesTable, printError, printReferenceSummary, printSuccess, printJson } from "../output.js";
import { ReferenceResolver } from "../references.js";
import { withSpinner } from "../spinner.js";
import chalk from "chalk";

export async function entitiesCommand(opts: { json?: boolean }): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  const jsonOutput = Boolean(opts.json);
  try {
    const entities = await withSpinner("Loading", () => client.listEntities(), jsonOutput);
    await resolver.stabilizeRecords("entity", entities);
    if (jsonOutput) {
      printJson(entities);
    } else if (entities.length === 0) {
      console.log("No entities found.");
    } else {
      printEntitiesTable(entities);
    }
  } catch (err) {
    printError(`Failed to fetch entities: ${err}`);
    process.exit(1);
  }
}

export async function entitiesShowCommand(entityId: string, opts: { json?: boolean }): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  const jsonOutput = Boolean(opts.json);
  try {
    const resolvedEntityId = await resolver.resolveEntity(entityId);
    const entities = await client.listEntities();
    const entity = entities.find((e) => e.entity_id === resolvedEntityId);
    if (!entity) {
      printError(`Entity not found: ${entityId}`);
      process.exit(1);
    }
    await resolver.stabilizeRecord("entity", entity);
    if (jsonOutput) {
      printJson(entity);
    } else {
      console.log(chalk.blue("─".repeat(40)));
      console.log(chalk.blue.bold("  Entity Detail"));
      console.log(chalk.blue("─".repeat(40)));
      console.log(`  ${chalk.bold("Name:")} ${entity.legal_name ?? entity.name ?? "N/A"}`);
      console.log(`  ${chalk.bold("Type:")} ${entity.entity_type ?? "N/A"}`);
      console.log(`  ${chalk.bold("Jurisdiction:")} ${entity.jurisdiction ?? "N/A"}`);
      console.log(`  ${chalk.bold("Status:")} ${entity.formation_status ?? entity.status ?? "N/A"}`);
      console.log(`  ${chalk.bold("State:")} ${entity.formation_state ?? "N/A"}`);
      printReferenceSummary("entity", entity, { showReuseHint: true });
      if (entity.formation_date) console.log(`  ${chalk.bold("Formation Date:")} ${entity.formation_date}`);
      if (entity.ein) console.log(`  ${chalk.bold("EIN:")} ${entity.ein}`);
      console.log(chalk.blue("─".repeat(40)));
    }
  } catch (err) {
    printError(`Failed to fetch entities: ${err}`);
    process.exit(1);
  }
}

export async function entitiesConvertCommand(
  entityId: string,
  opts: { to: string; jurisdiction?: string }
): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const resolvedEntityId = await resolver.resolveEntity(entityId);
    const data: Record<string, string> = { target_type: opts.to };
    if (opts.jurisdiction) data.new_jurisdiction = opts.jurisdiction;
    const result = await client.convertEntity(resolvedEntityId, data);
    printSuccess(`Entity conversion initiated: ${result.conversion_id ?? "OK"}`);
    printJson(result);
  } catch (err) {
    printError(`Failed to convert entity: ${err}`);
    process.exit(1);
  }
}

export async function entitiesDissolveCommand(
  entityId: string,
  opts: { reason: string; effectiveDate?: string }
): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const resolvedEntityId = await resolver.resolveEntity(entityId);
    const data: Record<string, string> = { reason: opts.reason };
    if (opts.effectiveDate) data.effective_date = opts.effectiveDate;
    const result = await client.dissolveEntity(resolvedEntityId, data);
    printSuccess(`Dissolution initiated: ${result.dissolution_id ?? "OK"}`);
    printJson(result);
  } catch (err) {
    const msg = String(err);
    if (msg.includes("InvalidTransition") || msg.includes("422")) {
      printError(`Cannot dissolve entity: only entities with 'active' status can be dissolved. Check the entity's current status with: corp entities show ${entityId}`);
    } else {
      printError(`Failed to dissolve entity: ${err}`);
    }
    process.exit(1);
  }
}
