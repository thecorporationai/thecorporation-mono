import type { CommandDef, CommandContext } from "./types.js";
import {
  printDeadlinesTable,
  printError,
  printJson,
  printTaxFilingsTable,
  printWriteResult,
} from "../output.js";

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const TAX_DOCUMENT_TYPE_DISPLAY = [
  "1120", "1120s", "1065", "franchise_tax", "annual_report", "83b",
  "1099_nec", "k1", "941", "w2",
] as const;

const TAX_DOCUMENT_TYPE_ALIASES: Record<string, string> = {
  form_1120: "1120", form_1120s: "1120s", form_1065: "1065",
  form_1099_nec: "1099_nec", form_k1: "k1", form_941: "941", form_w2: "w2",
};

const TAX_DOCUMENT_TYPE_CHOICES = [
  ...TAX_DOCUMENT_TYPE_DISPLAY,
  ...Object.keys(TAX_DOCUMENT_TYPE_ALIASES),
];

function normalizeRecurrence(recurrence?: string): string | undefined {
  if (!recurrence) return undefined;
  if (recurrence === "yearly") return "annual";
  return recurrence;
}

// ---------------------------------------------------------------------------
// Tax / Compliance registry entries
// ---------------------------------------------------------------------------

export const complianceCommands: CommandDef[] = [
  // --- tax (summary: filings + deadlines) ---
  {
    name: "tax",
    description: "Tax filings and deadline tracking",
    route: { method: "GET", path: "/v1/entities/{eid}/tax-filings" },
    entity: true,
    display: { title: "Tax Summary" },
    handler: async (ctx) => {
      const eid = await ctx.resolver.resolveEntity(ctx.opts.entityId as string | undefined);
      const [filings, deadlines] = await Promise.all([
        ctx.client.listTaxFilings(eid),
        ctx.client.listDeadlines(eid),
      ]);
      if (ctx.opts.json) { ctx.writer.json({ filings, deadlines }); return; }
      if (filings.length === 0 && deadlines.length === 0) {
        ctx.writer.writeln("No tax filings or deadlines found.");
        return;
      }
      if (filings.length > 0) printTaxFilingsTable(filings);
      if (deadlines.length > 0) printDeadlinesTable(deadlines);
    },
    examples: ["corp tax", "corp tax --json"],
  },

  // --- tax filings ---
  {
    name: "tax filings",
    description: "List tax filings",
    route: { method: "GET", path: "/v1/entities/{eid}/tax-filings" },
    entity: true,
    display: {
      title: "Tax Filings",
      cols: ["document_type>Type", "tax_year>Year", "status>Status", "#filing_id>ID"],
    },
    handler: async (ctx) => {
      const eid = await ctx.resolver.resolveEntity(ctx.opts.entityId as string | undefined);
      const filings = await ctx.client.listTaxFilings(eid);
      await ctx.resolver.stabilizeRecords("tax_filing", filings, eid);
      if (ctx.opts.json) { ctx.writer.json(filings); return; }
      if (filings.length === 0) { ctx.writer.writeln("No tax filings found."); return; }
      printTaxFilingsTable(filings);
    },
    examples: ["corp tax filings", "corp tax filings --json"],
  },

  // --- tax file ---
  {
    name: "tax file",
    description: "File a tax document",
    route: { method: "POST", path: "/v1/entities/{eid}/tax-filings" },
    entity: true,
    options: [
      {
        flags: "--type <type>",
        description: `Document type (${TAX_DOCUMENT_TYPE_DISPLAY.join(", ")})`,
        required: true,
        choices: [...TAX_DOCUMENT_TYPE_CHOICES],
      },
      { flags: "--year <year>", description: "Tax year", required: true, type: "int" },
    ],
    handler: async (ctx) => {
      const eid = await ctx.resolver.resolveEntity(ctx.opts.entityId as string | undefined);
      const rawType = ctx.opts.type as string;
      const docType = TAX_DOCUMENT_TYPE_ALIASES[rawType] ?? rawType;
      const result = await ctx.client.fileTaxDocument({
        entity_id: eid,
        document_type: docType,
        tax_year: ctx.opts.year as number,
      });
      await ctx.resolver.stabilizeRecord("tax_filing", result, eid);
      ctx.resolver.rememberFromRecord("tax_filing", result, eid);
      ctx.writer.writeResult(result, `Tax document filed: ${result.filing_id ?? "OK"}`, {
        jsonOnly: ctx.opts.json,
        referenceKind: "tax_filing",
        showReuseHint: true,
      });
    },
    produces: { kind: "tax_filing" },
    successTemplate: "Tax filing created",
    examples: ["corp tax file --year 'year'"],
  },

  // --- tax deadlines ---
  {
    name: "tax deadlines",
    description: "List tracked deadlines",
    route: { method: "GET", path: "/v1/entities/{eid}/deadlines" },
    entity: true,
    display: {
      title: "Deadlines",
      cols: ["deadline_type>Type", "@due_date>Due", "description>Description", "#deadline_id>ID"],
    },
    handler: async (ctx) => {
      const eid = await ctx.resolver.resolveEntity(ctx.opts.entityId as string | undefined);
      const deadlines = await ctx.client.listDeadlines(eid);
      await ctx.resolver.stabilizeRecords("deadline", deadlines, eid);
      if (ctx.opts.json) { ctx.writer.json(deadlines); return; }
      if (deadlines.length === 0) { ctx.writer.writeln("No deadlines found."); return; }
      printDeadlinesTable(deadlines);
    },
    examples: ["corp tax deadlines", "corp tax deadlines --json"],
  },

  // --- tax deadline ---
  {
    name: "tax deadline",
    description: "Track a compliance deadline",
    route: { method: "POST", path: "/v1/entities/{eid}/deadlines" },
    entity: true,
    options: [
      { flags: "--type <type>", description: "Deadline type", required: true },
      { flags: "--due-date <date>", description: "Due date (ISO 8601)", required: true },
      { flags: "--description <desc>", description: "Description", required: true },
      { flags: "--recurrence <recurrence>", description: "Recurrence (e.g. annual; 'yearly' is normalized). Required for annual_report type." },
    ],
    handler: async (ctx) => {
      const eid = await ctx.resolver.resolveEntity(ctx.opts.entityId as string | undefined);
      let recurrence = normalizeRecurrence(ctx.opts.recurrence as string | undefined);
      if (!recurrence && (ctx.opts.type as string) === "annual_report") {
        recurrence = "annual";
      }
      const payload: Record<string, unknown> = {
        entity_id: eid,
        deadline_type: ctx.opts.type as string,
        due_date: ctx.opts.dueDate as string,
        description: ctx.opts.description as string,
      };
      if (recurrence) payload.recurrence = recurrence;
      const result = await ctx.client.trackDeadline(payload);
      await ctx.resolver.stabilizeRecord("deadline", result, eid);
      ctx.resolver.rememberFromRecord("deadline", result, eid);
      ctx.writer.writeResult(result, `Deadline tracked: ${result.deadline_id ?? "OK"}`, {
        jsonOnly: ctx.opts.json,
        referenceKind: "deadline",
        showReuseHint: true,
      });
    },
    produces: { kind: "deadline" },
    successTemplate: "Deadline tracked",
    examples: ["corp tax deadline --type 'type' --due-date 'date' --description 'desc'", "corp tax deadline --json"],
  },

  // ── Auto-generated from OpenAPI ──────────────────────────────
  {
    name: "compliance escalations-scan",
    description: "Compliance Escalations Scan",
    route: { method: "POST", path: "/v1/compliance/escalations/scan" },
    examples: ["corp compliance escalations-scan"],
  },
  {
    name: "compliance escalations-resolve-with-evidence",
    description: "Compliance Escalations Resolve With Evidence",
    route: { method: "POST", path: "/v1/compliance/escalations/{pos}/resolve-with-evidence" },
    args: [{ name: "escalation-id", required: true, description: "Escalation Id" }],
    options: [
      { flags: "--evidence-type <evidence-type>", description: "Evidence Type" },
      { flags: "--filing-reference <filing-reference>", description: "Filing Reference" },
      { flags: "--notes <notes>", description: "Notes" },
      { flags: "--packet-id <packet-id>", description: "Packet Id" },
      { flags: "--resolve-incident", description: "Resolve Incident" },
      { flags: "--resolve-obligation", description: "Resolve Obligation" },
    ],
    examples: ["corp compliance escalations-resolve-with-evidence <escalation-id>", "corp compliance escalations-resolve-with-evidence --json"],
  },
  {
    name: "contractors classify",
    description: "Contractors Classify",
    route: { method: "POST", path: "/v1/contractors/classify" },
    options: [
      { flags: "--contractor-name <contractor-name>", description: "Contractor Name", required: true },
      { flags: "--duration-months <duration-months>", description: "Duration Months" },
      { flags: "--exclusive-client <exclusive-client>", description: "Exclusive Client" },
      { flags: "--factors <factors>", description: "Factors" },
      { flags: "--hours-per-week <hours-per-week>", description: "Hours Per Week" },
      { flags: "--provides-tools <provides-tools>", description: "Provides Tools" },
      { flags: "--state <state>", description: "State" },
    ],
    examples: ["corp contractors classify --contractor-name 'contractor-name'", "corp contractors classify --json"],
  },
  {
    name: "deadlines",
    description: "Deadlines",
    route: { method: "POST", path: "/v1/deadlines" },
    options: [
      { flags: "--deadline-type <deadline-type>", description: "Deadline Type", required: true },
      { flags: "--description <description>", description: "Description", required: true },
      { flags: "--due-date <due-date>", description: "Due Date", required: true },
      { flags: "--recurrence <recurrence>", description: "Recurrence pattern for a deadline.", choices: ["one_time", "monthly", "quarterly", "annual"] },
      { flags: "--severity <severity>", description: "Risk severity of missing a deadline.", choices: ["low", "medium", "high", "critical"] },
    ],
    examples: ["corp deadlines --deadline-type 'deadline-type' --description 'description' --due-date one_time", "corp deadlines --json"],
  },
  {
    name: "entities compliance-escalations",
    description: "Entities Compliance Escalations",
    route: { method: "GET", path: "/v1/entities/{eid}/compliance/escalations" },
    entity: true,
    display: { title: "Entities Compliance Escalations", cols: ["action>Action", "authority>Authority", "@created_at>Created At", "#deadline_id>ID", "#entity_id>ID", "#escalation_id>ID", "#incident_id>ID", "milestone>Milestone"] },
    examples: ["corp entities compliance-escalations", "corp entities compliance-escalations --json"],
  },
  {
    name: "tax filings",
    description: "Tax Filings",
    route: { method: "POST", path: "/v1/tax/filings" },
    options: [
      { flags: "--document-type <document-type>", description: "Document Type", required: true },
      { flags: "--tax-year <tax-year>", description: "Tax Year", required: true, type: "int" },
    ],
    examples: ["corp tax filings --document-type 'document-type' --tax-year 'tax-year'"],
  },

];