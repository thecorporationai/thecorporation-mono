import type { CommandDef, CommandContext } from "./types.js";
import {
  printDocumentsTable,
  printError,
  printReferenceSummary,
  printSuccess,
  printJson,
  printWriteResult,
} from "../output.js";
import { autoSignFormationDocument, autoSignFormationDocuments } from "../formation-automation.js";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Document registry entries
// ---------------------------------------------------------------------------

export const documentCommands: CommandDef[] = [
  // --- documents (list) ---
  {
    name: "documents",
    description: "Documents and signing",
    route: { method: "GET", path: "/v1/formations/{eid}/documents" },
    entity: true,
    display: {
      title: "Documents",
      cols: ["title>Title", "document_type>Type", "status>Status", "@created_at>Date", "#document_id>ID"],
    },
    handler: async (ctx) => {
      const eid = await ctx.resolver.resolveEntity(ctx.opts.entityId as string | undefined);
      const docs = await ctx.client.getEntityDocuments(eid);
      await ctx.resolver.stabilizeRecords("document", docs, eid);
      if (ctx.opts.json) { ctx.writer.json(docs); return; }
      if (docs.length === 0) { ctx.writer.writeln("No documents found."); return; }
      printDocumentsTable(docs);
    },
  },

  // --- documents signing-link <doc-ref> ---
  {
    name: "documents signing-link",
    description: "Get a signing link for a document",
    route: { method: "GET", path: "/v1/sign/{pos}" },
    entity: "query",
    args: [{ name: "doc-ref", required: true, description: "Document reference" }],
    display: { title: "Signing Link" },
    handler: async (ctx) => {
      const docRef = ctx.positional[0];
      const eid = await ctx.resolver.resolveEntity(ctx.opts.entityId as string | undefined);
      let resolvedDocumentId: string;
      try {
        resolvedDocumentId = await ctx.resolver.resolveDocument(eid, docRef);
      } catch {
        printError(
          `Could not resolve '${docRef}' as a document. If you just generated a contract, ` +
          "use the document_id from the generate output, not @last (which may reference the contract_id).\n" +
          "  List documents with: corp documents",
        );
        process.exit(1);
        return;
      }
      const result = await ctx.client.getSigningLink(resolvedDocumentId, eid);
      const shareUrl = formatSigningLink(resolvedDocumentId, result);
      if (process.stdout.isTTY) {
        ctx.writer.success("Signing link generated.");
        const summaryRecord = await ctx.resolver.stabilizeRecord("document", { document_id: resolvedDocumentId, title: docRef }, eid);
        printReferenceSummary("document", summaryRecord, { label: "Document Ref:" });
      }
      console.log(shareUrl);
    },
  },

  // --- documents sign <doc-ref> ---
  {
    name: "documents sign",
    description: "Sign a formation document, or auto-sign all missing required signatures",
    route: { method: "POST", path: "/v1/documents/{pos}/sign" },
    entity: true,
    args: [{ name: "doc-ref", required: true, description: "Document reference" }],
    options: [
      { flags: "--signer-name <name>", description: "Manual signer name" },
      { flags: "--signer-role <role>", description: "Manual signer role" },
      { flags: "--signer-email <email>", description: "Manual signer email" },
      { flags: "--signature-text <text>", description: "Manual signature text (defaults to signer name)" },
    ],
    handler: async (ctx) => {
      const docRef = ctx.positional[0];
      const eid = await ctx.resolver.resolveEntity(ctx.opts.entityId as string | undefined);
      const resolvedDocumentId = await ctx.resolver.resolveDocument(eid, docRef);
      const signerName = ctx.opts.signerName as string | undefined;
      const signerRole = ctx.opts.signerRole as string | undefined;
      const signerEmail = ctx.opts.signerEmail as string | undefined;
      const signatureText = ctx.opts.signatureText as string | undefined;

      if (signerName || signerRole || signerEmail || signatureText) {
        if (!signerName || !signerRole || !signerEmail) {
          throw new Error("Manual signing requires --signer-name, --signer-role, and --signer-email.");
        }
        const result = await ctx.client.signDocument(resolvedDocumentId, eid, {
          signer_name: signerName,
          signer_role: signerRole,
          signer_email: signerEmail,
          signature_text: signatureText ?? signerName,
        });
        await ctx.resolver.stabilizeRecord("document", { document_id: resolvedDocumentId, title: docRef }, eid);
        ctx.writer.writeResult(result, `Document ${resolvedDocumentId} signed.`, { jsonOnly: ctx.opts.json });
        return;
      }

      const result = await autoSignFormationDocument(ctx.client, eid, resolvedDocumentId);
      await ctx.resolver.stabilizeRecord("document", result.document, eid);
      ctx.resolver.rememberFromRecord("document", result.document, eid);
      if (ctx.opts.json) {
        ctx.writer.json({
          document_id: resolvedDocumentId,
          signatures_added: result.signatures_added,
          document: result.document,
        });
        return;
      }
      ctx.writer.success(
        result.signatures_added > 0
          ? `Applied ${result.signatures_added} signature(s) to ${resolvedDocumentId}.`
          : `No signatures were needed for ${resolvedDocumentId}.`,
      );
      printReferenceSummary("document", result.document, { showReuseHint: true });
      printJson(result.document);
    },
  },

  // --- documents sign-all ---
  {
    name: "documents sign-all",
    description: "Auto-sign all outstanding formation documents for an entity",
    route: { method: "POST", path: "/v1/formations/{eid}/sign-all" },
    entity: true,
    handler: async (ctx) => {
      const eid = await ctx.resolver.resolveEntity(ctx.opts.entityId as string | undefined);
      const result = await autoSignFormationDocuments(ctx.client, ctx.resolver, eid);
      if (ctx.opts.json) {
        ctx.writer.json(result);
        return;
      }
      ctx.writer.success(
        `Processed ${result.documents_seen} formation document(s); added ${result.signatures_added} signature(s) across ${result.documents_signed} document(s).`,
      );
      printJson(result.documents.map((document: Record<string, unknown>) => ({
        document_id: document.document_id,
        title: document.title,
        status: document.status,
        signatures: Array.isArray(document.signatures) ? document.signatures.length : document.signatures,
      })));
    },
  },

  // --- documents generate ---
  {
    name: "documents generate",
    description: "Generate a contract from a template",
    route: { method: "POST", path: "/v1/contracts/generate" },
    entity: true,
    options: [
      { flags: "--template <type>", description: "Template type (consulting_agreement, employment_offer, contractor_agreement, nda, custom)", required: true },
      { flags: "--counterparty <name>", description: "Counterparty name", required: true },
      { flags: "--effective-date <date>", description: "Effective date (ISO 8601, defaults to today)" },
      { flags: "--base-salary <amount>", description: "Employment offer base salary (for employment_offer)" },
      { flags: "--param <key=value>", description: "Additional template parameter (repeatable)", type: "array" },
    ],
    handler: async (ctx) => {
      const eid = await ctx.resolver.resolveEntity(ctx.opts.entityId as string | undefined);
      const template = ctx.opts.template as string;
      const counterparty = ctx.opts.counterparty as string;
      const baseSalary = ctx.opts.baseSalary as string | undefined;
      const paramRaw = (ctx.opts.param as string[] | undefined) ?? [];

      const parameters: Record<string, unknown> = {};
      if (baseSalary) parameters.base_salary = baseSalary;
      for (const raw of paramRaw) {
        const idx = raw.indexOf("=");
        if (idx <= 0) throw new Error(`Invalid --param value: ${raw}. Expected key=value.`);
        const key = raw.slice(0, idx).trim();
        const value = raw.slice(idx + 1).trim();
        if (!key) throw new Error(`Invalid --param value: ${raw}. Expected key=value.`);
        parameters[key] = coerceParamValue(value);
      }

      try {
        const result = await ctx.client.generateContract({
          entity_id: eid,
          template_type: template,
          counterparty_name: counterparty,
          effective_date: (ctx.opts.effectiveDate as string | undefined) ?? new Date().toISOString().slice(0, 10),
          parameters,
        });
        await ctx.resolver.stabilizeRecord("document", result, eid);
        ctx.resolver.rememberFromRecord("document", result, eid);
        if (result.document_id) {
          ctx.resolver.remember("document", String(result.document_id), eid);
        }
        ctx.writer.writeResult(result, `Contract generated: ${result.contract_id ?? "OK"}`, {
          jsonOnly: ctx.opts.json,
          referenceKind: "document",
          showReuseHint: true,
        });
      } catch (err) {
        const msg = String(err);
        if (template === "employment_offer" && (msg.includes("base_salary") || msg.includes("required"))) {
          printError(
            `Failed to generate employment_offer: ${msg}\n` +
            "  Hint: employment_offer requires base_salary. Use:\n" +
            "    --base-salary 150000\n" +
            "  Or: --param base_salary=150000\n" +
            "  Optional params: position_title, start_date, work_location, classification,\n" +
            "    bonus_terms, equity_terms, benefits_summary, governing_law",
          );
        } else if (template === "safe_agreement" && (msg.includes("purchase_amount") || msg.includes("investment_amount") || msg.includes("valuation_cap") || msg.includes("investor_notice") || msg.includes("required"))) {
          printError(
            `Failed to generate safe_agreement: ${msg}\n` +
            "  Hint: safe_agreement requires purchase_amount, valuation_cap, and investor_notice_address. Use:\n" +
            "    --param purchase_amount=50000000 --param valuation_cap=10000000\n" +
            '    --param investor_notice_address="123 Main St, City, ST 12345"',
          );
        } else {
          printError(`Failed to generate contract: ${err}`);
        }
        process.exit(1);
      }
    },
  },

  // --- documents preview-pdf ---
  {
    name: "documents preview-pdf",
    description: "Validate and print the authenticated PDF preview URL for a governance document",
    route: { method: "GET", path: "/v1/documents/preview/pdf" },
    entity: true,
    options: [
      { flags: "--definition-id <id>", description: "AST document definition ID (e.g. 'bylaws')" },
      { flags: "--document-id <id>", description: "Deprecated alias for --definition-id" },
    ],
    handler: async (ctx) => {
      const documentId = (ctx.opts.definitionId as string | undefined) ?? (ctx.opts.documentId as string | undefined);
      if (!documentId || documentId.trim().length === 0) {
        printError("preview-pdf requires --definition-id (or deprecated alias --document-id)");
        process.exit(1);
      }
      const eid = await ctx.resolver.resolveEntity(ctx.opts.entityId as string | undefined);
      await ctx.client.validatePreviewPdf(eid, documentId);
      const url = ctx.client.getPreviewPdfUrl(eid, documentId);
      ctx.writer.success(`Preview PDF URL: ${url}`);
      console.log("The document definition was validated successfully. Use your API key to download the PDF.");
    },
  },
];
