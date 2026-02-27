import { requireConfig, resolveEntityId } from "../config.js";
import { CorpAPIClient } from "../api-client.js";
import { printError, printSuccess, printJson } from "../output.js";

export async function taxFileCommand(opts: { entityId?: string; type: string; year: number }): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const eid = resolveEntityId(cfg, opts.entityId);
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    const result = await client.fileTaxDocument({ entity_id: eid, document_type: opts.type, tax_year: opts.year });
    printSuccess(`Tax document filed: ${result.filing_id ?? "OK"}`);
    printJson(result);
  } catch (err) { printError(`Failed to file tax document: ${err}`); process.exit(1); }
}

export async function taxDeadlineCommand(opts: {
  entityId?: string; type: string; dueDate: string; description: string; recurrence?: string;
}): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const eid = resolveEntityId(cfg, opts.entityId);
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    const result = await client.trackDeadline({
      entity_id: eid, deadline_type: opts.type, due_date: opts.dueDate,
      description: opts.description, recurrence: opts.recurrence ?? "",
    });
    printSuccess(`Deadline tracked: ${result.deadline_id ?? "OK"}`);
    printJson(result);
  } catch (err) { printError(`Failed to track deadline: ${err}`); process.exit(1); }
}
