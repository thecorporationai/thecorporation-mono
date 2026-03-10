import { requireConfig, resolveEntityId } from "../config.js";
import { CorpAPIClient } from "../api-client.js";
import { printDocumentsTable, printError, printSuccess, printJson, printWriteResult } from "../output.js";

const HUMANS_APP_ORIGIN = "https://humans.thecorporation.ai";

function formatSigningLink(docId: string, result: { token?: unknown; signing_url?: unknown }): string {
  if (typeof result.token === "string" && result.token.length > 0) {
    return `${HUMANS_APP_ORIGIN}/sign/${docId}?token=${encodeURIComponent(result.token)}`;
  }
  if (typeof result.signing_url === "string" && result.signing_url.length > 0) {
    if (/^https?:\/\//.test(result.signing_url)) {
      return result.signing_url;
    }
    const normalizedPath = result.signing_url.startsWith("/human/sign/")
      ? result.signing_url.replace("/human/sign/", "/sign/")
      : result.signing_url;
    return `${HUMANS_APP_ORIGIN}${normalizedPath.startsWith("/") ? normalizedPath : `/${normalizedPath}`}`;
  }
  return `${HUMANS_APP_ORIGIN}/sign/${docId}`;
}

export async function documentsListCommand(opts: { entityId?: string; json?: boolean }): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const eid = resolveEntityId(cfg, opts.entityId);
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    const docs = await client.getEntityDocuments(eid);
    if (opts.json) printJson(docs);
    else if (docs.length === 0) console.log("No documents found.");
    else printDocumentsTable(docs);
  } catch (err) { printError(`Failed to fetch documents: ${err}`); process.exit(1); }
}

export async function documentsSigningLinkCommand(docId: string, opts: { entityId?: string }): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const eid = resolveEntityId(cfg, opts.entityId);
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    const result = await client.getSigningLink(docId, eid);
    const shareUrl = formatSigningLink(docId, result);
    if (process.stdout.isTTY) {
      printSuccess("Signing link generated.");
    }
    console.log(shareUrl);
  } catch (err) { printError(`Failed to get signing link: ${err}`); process.exit(1); }
}

export async function documentsGenerateCommand(opts: {
  entityId?: string;
  template: string;
  counterparty: string;
  effectiveDate?: string;
  baseSalary?: string;
  param?: string[];
  json?: boolean;
}): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const eid = resolveEntityId(cfg, opts.entityId);
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    const parameters: Record<string, unknown> = {};
    if (opts.baseSalary) {
      parameters.base_salary = opts.baseSalary;
    }
    for (const raw of opts.param ?? []) {
      const idx = raw.indexOf("=");
      if (idx <= 0) {
        throw new Error(`Invalid --param value: ${raw}. Expected key=value.`);
      }
      const key = raw.slice(0, idx).trim();
      const value = raw.slice(idx + 1).trim();
      if (!key) {
        throw new Error(`Invalid --param value: ${raw}. Expected key=value.`);
      }
      parameters[key] = coerceParamValue(value);
    }

    const result = await client.generateContract({
      entity_id: eid,
      template_type: opts.template,
      counterparty_name: opts.counterparty,
      effective_date: opts.effectiveDate ?? new Date().toISOString().slice(0, 10),
      parameters,
    });
    printWriteResult(result, `Contract generated: ${result.contract_id ?? "OK"}`, opts.json);
  } catch (err) { printError(`Failed to generate contract: ${err}`); process.exit(1); }
}

function coerceParamValue(raw: string): unknown {
  if (raw === "true") return true;
  if (raw === "false") return false;
  if (/^-?\d+(\.\d+)?$/.test(raw)) return Number(raw);
  if ((raw.startsWith("{") && raw.endsWith("}")) || (raw.startsWith("[") && raw.endsWith("]"))) {
    try {
      return JSON.parse(raw);
    } catch {
      return raw;
    }
  }
  return raw;
}

export async function documentsPreviewPdfCommand(opts: {
  entityId?: string; documentId: string;
}): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const eid = resolveEntityId(cfg, opts.entityId);
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    await client.validatePreviewPdf(eid, opts.documentId);
    const url = client.getPreviewPdfUrl(eid, opts.documentId);
    printSuccess(`Preview PDF URL: ${url}`);
    console.log("The document definition was validated successfully. Use your API key to download the PDF.");
  } catch (err) {
    printError(`Failed to validate preview PDF: ${err}`);
    process.exit(1);
  }
}
