import { requireConfig, resolveEntityId } from "../config.js";
import { CorpAPIClient } from "../api-client.js";
import { printDocumentsTable, printError, printSuccess, printJson } from "../output.js";

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

export async function documentsSigningLinkCommand(docId: string): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    const result = client.getSigningLink(docId);
    printSuccess(`Signing link: ${result.signing_url}`);
    console.log("Open this link in your browser to sign the document.");
  } catch (err) { printError(`Failed to get signing link: ${err}`); process.exit(1); }
}

export async function documentsGenerateCommand(opts: {
  entityId?: string; template: string; counterparty: string; effectiveDate?: string;
}): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const eid = resolveEntityId(cfg, opts.entityId);
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    const result = await client.generateContract({
      entity_id: eid, template_type: opts.template,
      parameters: { counterparty_name: opts.counterparty, effective_date: opts.effectiveDate ?? "" },
    });
    printSuccess(`Contract generated: ${result.contract_id ?? "OK"}`);
    printJson(result);
  } catch (err) { printError(`Failed to generate contract: ${err}`); process.exit(1); }
}

export async function documentsPreviewPdfCommand(opts: {
  entityId?: string; documentId: string;
}): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const eid = resolveEntityId(cfg, opts.entityId);
  const apiUrl = cfg.api_url.replace(/\/+$/, "");
  const qs = new URLSearchParams({ entity_id: eid, document_id: opts.documentId }).toString();
  const url = `${apiUrl}/v1/documents/preview/pdf?${qs}`;
  printSuccess(`Preview PDF URL: ${url}`);
  console.log("Use your API key to authenticate the download.");
}
