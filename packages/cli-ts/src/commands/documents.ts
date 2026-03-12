import { requireConfig } from "../config.js";
import { CorpAPIClient } from "../api-client.js";
import { printDocumentsTable, printError, printReferenceSummary, printSuccess, printJson, printWriteResult } from "../output.js";
import { ReferenceResolver } from "../references.js";
import { autoSignFormationDocument, autoSignFormationDocuments } from "../formation-automation.js";

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
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const eid = await resolver.resolveEntity(opts.entityId);
    const docs = await client.getEntityDocuments(eid);
    await resolver.stabilizeRecords("document", docs, eid);
    if (opts.json) printJson(docs);
    else if (docs.length === 0) console.log("No documents found.");
    else printDocumentsTable(docs);
  } catch (err) { printError(`Failed to fetch documents: ${err}`); process.exit(1); }
}

export async function documentsSigningLinkCommand(docId: string, opts: { entityId?: string }): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const eid = await resolver.resolveEntity(opts.entityId);
    const resolvedDocumentId = await resolver.resolveDocument(eid, docId);
    const result = await client.getSigningLink(resolvedDocumentId, eid);
    const shareUrl = formatSigningLink(resolvedDocumentId, result);
    if (process.stdout.isTTY) {
      printSuccess("Signing link generated.");
      const summaryRecord = await resolver.stabilizeRecord("document", { document_id: resolvedDocumentId, title: docId }, eid);
      printReferenceSummary("document", summaryRecord, { label: "Document Ref:" });
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
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const eid = await resolver.resolveEntity(opts.entityId);
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
    await resolver.stabilizeRecord("document", result, eid);
    resolver.rememberFromRecord("document", result, eid);
    printWriteResult(result, `Contract generated: ${result.contract_id ?? "OK"}`, {
      jsonOnly: opts.json,
      referenceKind: "document",
      showReuseHint: true,
    });
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

export async function documentsSignCommand(docId: string, opts: {
  entityId?: string;
  signerName?: string;
  signerRole?: string;
  signerEmail?: string;
  signatureText?: string;
  json?: boolean;
}): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const eid = await resolver.resolveEntity(opts.entityId);
    const resolvedDocumentId = await resolver.resolveDocument(eid, docId);

    if (opts.signerName || opts.signerRole || opts.signerEmail || opts.signatureText) {
      if (!opts.signerName || !opts.signerRole || !opts.signerEmail) {
        throw new Error("Manual signing requires --signer-name, --signer-role, and --signer-email.");
      }
      const result = await client.signDocument(resolvedDocumentId, eid, {
        signer_name: opts.signerName,
        signer_role: opts.signerRole,
        signer_email: opts.signerEmail,
        signature_text: opts.signatureText ?? opts.signerName,
      });
      await resolver.stabilizeRecord("document", { document_id: resolvedDocumentId, title: docId }, eid);
      printWriteResult(result, `Document ${resolvedDocumentId} signed.`, opts.json);
      return;
    }

    const result = await autoSignFormationDocument(client, eid, resolvedDocumentId);
    await resolver.stabilizeRecord("document", result.document, eid);
    resolver.rememberFromRecord("document", result.document, eid);
    if (opts.json) {
      printJson({
        document_id: resolvedDocumentId,
        signatures_added: result.signatures_added,
        document: result.document,
      });
      return;
    }
    printSuccess(
      result.signatures_added > 0
        ? `Applied ${result.signatures_added} signature(s) to ${resolvedDocumentId}.`
        : `No signatures were needed for ${resolvedDocumentId}.`,
    );
    printReferenceSummary("document", result.document, { showReuseHint: true });
    printJson(result.document);
  } catch (err) {
    printError(`Failed to sign document: ${err}`);
    process.exit(1);
  }
}

export async function documentsSignAllCommand(opts: {
  entityId?: string;
  json?: boolean;
}): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const eid = await resolver.resolveEntity(opts.entityId);
    const result = await autoSignFormationDocuments(client, resolver, eid);
    if (opts.json) {
      printJson(result);
      return;
    }
    printSuccess(
      `Processed ${result.documents_seen} formation document(s); added ${result.signatures_added} signature(s) across ${result.documents_signed} document(s).`,
    );
    printJson(result.documents.map((document) => ({
      document_id: document.document_id,
      title: document.title,
      status: document.status,
      signatures: Array.isArray(document.signatures) ? document.signatures.length : document.signatures,
    })));
  } catch (err) {
    printError(`Failed to sign formation documents: ${err}`);
    process.exit(1);
  }
}

export async function documentsPreviewPdfCommand(opts: {
  entityId?: string; documentId: string;
}): Promise<void> {
  if (!opts.documentId || opts.documentId.trim().length === 0) {
    printError("preview-pdf requires --definition-id (or deprecated alias --document-id)");
    process.exit(1);
  }
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const eid = await resolver.resolveEntity(opts.entityId);
    await client.validatePreviewPdf(eid, opts.documentId);
    const url = client.getPreviewPdfUrl(eid, opts.documentId);
    printSuccess(`Preview PDF URL: ${url}`);
    console.log("The document definition was validated successfully. Use your API key to download the PDF.");
  } catch (err) {
    printError(`Failed to validate preview PDF: ${err}`);
    process.exit(1);
  }
}
