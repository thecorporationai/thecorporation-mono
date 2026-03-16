import { requireConfig } from "../config.js";
import { CorpAPIClient } from "../api-client.js";
import { printDeadlinesTable, printError, printJson, printTaxFilingsTable, printWriteResult } from "../output.js";
import { ReferenceResolver } from "../references.js";

export async function taxSummaryCommand(opts: { entityId?: string; json?: boolean }): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const eid = await resolver.resolveEntity(opts.entityId);
    const [filings, deadlines] = await Promise.all([
      client.listTaxFilings(eid),
      client.listDeadlines(eid),
    ]);
    if (opts.json) {
      printJson({ filings, deadlines });
      return;
    }
    if (filings.length === 0 && deadlines.length === 0) {
      console.log("No tax filings or deadlines found.");
      return;
    }
    if (filings.length > 0) printTaxFilingsTable(filings);
    if (deadlines.length > 0) printDeadlinesTable(deadlines);
  } catch (err) {
    printError(`Failed to fetch tax summary: ${err}`);
    process.exit(1);
  }
}

function normalizeRecurrence(recurrence?: string): string | undefined {
  if (!recurrence) return undefined;
  if (recurrence === "yearly") return "annual";
  return recurrence;
}

export async function taxFilingsCommand(opts: { entityId?: string; json?: boolean }): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const eid = await resolver.resolveEntity(opts.entityId);
    const filings = await client.listTaxFilings(eid);
    await resolver.stabilizeRecords("tax_filing", filings, eid);
    if (opts.json) printJson(filings);
    else if (filings.length === 0) console.log("No tax filings found.");
    else printTaxFilingsTable(filings);
  } catch (err) { printError(`Failed to fetch tax filings: ${err}`); process.exit(1); }
}

export async function taxDeadlinesCommand(opts: { entityId?: string; json?: boolean }): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const eid = await resolver.resolveEntity(opts.entityId);
    const deadlines = await client.listDeadlines(eid);
    await resolver.stabilizeRecords("deadline", deadlines, eid);
    if (opts.json) printJson(deadlines);
    else if (deadlines.length === 0) console.log("No deadlines found.");
    else printDeadlinesTable(deadlines);
  } catch (err) { printError(`Failed to fetch deadlines: ${err}`); process.exit(1); }
}

const TAX_TYPE_ALIASES: Record<string, string> = {
  form_1120: "1120", form_1120s: "1120s", form_1065: "1065",
  form_1099_nec: "1099_nec", form_k1: "k1", form_941: "941", form_w2: "w2",
};

export async function taxFileCommand(opts: {
  entityId?: string;
  type: string;
  year: number;
  json?: boolean;
}): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const docType = TAX_TYPE_ALIASES[opts.type] ?? opts.type;
    const eid = await resolver.resolveEntity(opts.entityId);
    const result = await client.fileTaxDocument({ entity_id: eid, document_type: docType, tax_year: opts.year });
    await resolver.stabilizeRecord("tax_filing", result, eid);
    resolver.rememberFromRecord("tax_filing", result, eid);
    printWriteResult(result, `Tax document filed: ${result.filing_id ?? "OK"}`, {
      jsonOnly: opts.json,
      referenceKind: "tax_filing",
      showReuseHint: true,
    });
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
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const eid = await resolver.resolveEntity(opts.entityId);
    const payload: Record<string, unknown> = {
      entity_id: eid, deadline_type: opts.type, due_date: opts.dueDate,
      description: opts.description,
    };
    const recurrence = normalizeRecurrence(opts.recurrence);
    if (recurrence) payload.recurrence = recurrence;
    const result = await client.trackDeadline(payload);
    await resolver.stabilizeRecord("deadline", result, eid);
    resolver.rememberFromRecord("deadline", result, eid);
    printWriteResult(result, `Deadline tracked: ${result.deadline_id ?? "OK"}`, {
      jsonOnly: opts.json,
      referenceKind: "deadline",
      showReuseHint: true,
    });
  } catch (err) { printError(`Failed to track deadline: ${err}`); process.exit(1); }
}
