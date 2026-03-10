import chalk from "chalk";
import { loadConfig, requireConfig } from "../config.js";
import { CorpAPIClient } from "../api-client.js";
import { printError, printJson } from "../output.js";

export async function contextCommand(opts: { json?: boolean }): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const rawCfg = loadConfig();
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);

  try {
    const [status, entities] = await Promise.all([
      client.getStatus(),
      client.listEntities(),
    ]);

    const activeEntity = rawCfg.active_entity_id
      ? entities.find((entity) => entity.entity_id === rawCfg.active_entity_id) ?? null
      : null;

    const [contactsResult, documentsResult, workItemsResult] = activeEntity
      ? await Promise.allSettled([
          client.listContacts(String(activeEntity.entity_id)),
          client.getEntityDocuments(String(activeEntity.entity_id)),
          client.listWorkItems(String(activeEntity.entity_id)),
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

    if (opts.json) {
      printJson(payload);
      return;
    }

    console.log(chalk.blue("─".repeat(50)));
    console.log(chalk.blue.bold("  Corp Context"));
    console.log(chalk.blue("─".repeat(50)));
    console.log(`  ${chalk.bold("User:")} ${payload.user.name || "N/A"} <${payload.user.email || "N/A"}>`);
    console.log(`  ${chalk.bold("Workspace:")} ${rawCfg.workspace_id}`);
    console.log(`  ${chalk.bold("API URL:")} ${rawCfg.api_url}`);
    console.log(`  ${chalk.bold("Entities:")} ${entities.length}`);
    if (payload.active_entity) {
      console.log(`  ${chalk.bold("Active Entity:")} ${payload.active_entity.entity.legal_name ?? payload.active_entity.entity.entity_id}`);
      console.log(`  ${chalk.bold("Active Entity ID:")} ${payload.active_entity.entity.entity_id}`);
      console.log(`  ${chalk.bold("Contacts:")} ${payload.active_entity.summary.contact_count ?? "N/A"}`);
      console.log(`  ${chalk.bold("Documents:")} ${payload.active_entity.summary.document_count ?? "N/A"}`);
      console.log(`  ${chalk.bold("Work Items:")} ${payload.active_entity.summary.work_item_count ?? "N/A"}`);
    } else {
      console.log(`  ${chalk.bold("Active Entity:")} none`);
    }
    if (status.next_deadline) {
      console.log(`  ${chalk.bold("Next Deadline:")} ${status.next_deadline}`);
    }
    console.log(chalk.blue("─".repeat(50)));
  } catch (err) {
    printError(`Failed to fetch context: ${err}`);
    process.exit(1);
  }
}
