import { requireConfig, resolveEntityId } from "../config.js";
import { CorpAPIClient } from "../api-client.js";
import { printError, printWriteResult } from "../output.js";

function normalizeRecurrence(recurrence?: string): string | undefined {
  if (!recurrence) return undefined;
  if (recurrence === "yearly") return "annual";
  return recurrence;
}

export async function taxFileCommand(opts: {
  entityId?: string;
  type: string;
  year: number;
  json?: boolean;
}): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const eid = resolveEntityId(cfg, opts.entityId);
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    const result = await client.fileTaxDocument({ entity_id: eid, document_type: opts.type, tax_year: opts.year });
    printWriteResult(result, `Tax document filed: ${result.filing_id ?? "OK"}`, opts.json);
  } catch (err) { printError(`Failed to file tax document: ${err}`); process.exit(1); }
}

export async function taxDeadlineCommand(opts: {
  entityId?: string;
  type: string;
  dueDate: string;
  description: string;
  recurrence?: string;
  json?: boolean;
}): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const eid = resolveEntityId(cfg, opts.entityId);
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    const payload: Record<string, unknown> = {
      entity_id: eid, deadline_type: opts.type, due_date: opts.dueDate,
      description: opts.description,
    };
    const recurrence = normalizeRecurrence(opts.recurrence);
    if (recurrence) payload.recurrence = recurrence;
    const result = await client.trackDeadline(payload);
    printWriteResult(result, `Deadline tracked: ${result.deadline_id ?? "OK"}`, opts.json);
  } catch (err) { printError(`Failed to track deadline: ${err}`); process.exit(1); }
}
