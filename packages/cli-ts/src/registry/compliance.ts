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
    description: "List tax filing records",
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
      { flags: "--filer-contact-id <ref>", description: "Contact reference for per-person filings (e.g. 83(b) elections)" },
    ],
    handler: async (ctx) => {
      const eid = await ctx.resolver.resolveEntity(ctx.opts.entityId as string | undefined);
      const rawType = ctx.opts.type as string;
      const docType = TAX_DOCUMENT_TYPE_ALIASES[rawType] ?? rawType;
      const filerContactId = (ctx.opts.filerContactId as string | undefined)
        ? await ctx.resolver.resolveContact(eid, ctx.opts.filerContactId as string)
        : undefined;
      const payload: Record<string, unknown> = {
        entity_id: eid,
        document_type: docType,
        tax_year: ctx.opts.year as number,
      };
      if (filerContactId) payload.filer_contact_id = filerContactId;
      const result = await ctx.client.fileTaxDocument(payload);
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
    examples: [
      "corp tax file --type 1120 --year 2025",
      "corp tax file --type 83b --year 2025 --filer-contact-id @last:contact",
    ],
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
      { flags: "--description <desc>", description: "Description text", required: true },
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
    examples: [
      'corp tax deadline --type annual_report --due-date 2026-03-31 --description "Delaware annual report due"',
      'corp tax deadline --type annual_report --due-date 2026-03-31 --description "Delaware annual report" --recurrence annual',
    ],
  },

  // ── Auto-generated from OpenAPI ──────────────────────────────
  {
    name: "compliance escalations-scan",
    description: "Scan for compliance issues and create escalations",
    route: { method: "POST", path: "/v1/compliance/escalations/scan" },
    examples: [
      "corp compliance escalations-scan",
      "corp compliance escalations-scan --json",
    ],
    successTemplate: "Escalation scan complete",
  },
  {
    name: "compliance escalations-resolve-with-evidence",
    description: "Resolve a compliance escalation with evidence",
    route: { method: "POST", path: "/v1/compliance/escalations/{pos}/resolve-with-evidence" },
    args: [{ name: "escalation-id", required: true, description: "Escalation ID" }],
    options: [
      { flags: "--evidence-type <evidence-type>", description: "Evidence Type" },
      { flags: "--filing-reference <filing-reference>", description: "Filing Reference" },
      { flags: "--notes <notes>", description: "Additional notes" },
      { flags: "--packet-id <packet-id>", description: "Document packet ID" },
      { flags: "--resolve-incident", description: "Resolve Incident" },
      { flags: "--resolve-obligation", description: "Resolve Obligation" },
    ],
    examples: [
      'corp compliance escalations-resolve-with-evidence esc_01hx9k3n2p4q7r8s9t0uvwxyz --evidence-type filing --filing-reference DE-2026-0042',
      "corp compliance escalations-resolve-with-evidence esc_01hx9k3n2p4q7r8s9t0uvwxyz --resolve-incident --notes \"Filed on time\"",
    ],
    successTemplate: "Escalations Resolve With Evidence created",
  },
  {
    name: "contractors classify",
    description: "Classify a worker as employee or contractor",
    route: { method: "POST", path: "/v1/contractors/classify" },
    entity: true,
    options: [
      { flags: "--contractor-name <contractor-name>", description: "Contractor Name", required: true },
      { flags: "--duration-months <duration-months>", description: "Duration Months" },
      { flags: "--exclusive-client <exclusive-client>", description: "Exclusive Client" },
      { flags: "--factors <factors>", description: "Factors" },
      { flags: "--hours-per-week <hours-per-week>", description: "Hours Per Week" },
      { flags: "--provides-tools <provides-tools>", description: "Provides Tools" },
      { flags: "--state <state>", description: "State" },
    ],
    examples: [
      'corp contractors classify --contractor-name "Jane Doe" --state CA --hours-per-week 20',
      'corp contractors classify --contractor-name "Acme Services LLC" --duration-months 6 --exclusive-client false',
    ],
    successTemplate: "Classify created",
  },
  {
    name: "deadlines",
    description: "Create a compliance deadline",
    route: { method: "POST", path: "/v1/deadlines" },
    options: [
      { flags: "--deadline-type <deadline-type>", description: "Deadline Type", required: true },
      { flags: "--description <description>", description: "Description text", required: true },
      { flags: "--due-date <due-date>", description: "Due date (ISO 8601, e.g. 2026-06-30)", required: true },
      { flags: "--recurrence <recurrence>", description: "Recurrence pattern for a deadline.", choices: ["one_time", "monthly", "quarterly", "annual"] },
      { flags: "--severity <severity>", description: "Risk severity of missing a deadline.", choices: ["low", "medium", "high", "critical"] },
    ],
    examples: [
      'corp deadlines --deadline-type annual_report --description "Delaware annual report" --due-date 2026-03-31 --recurrence annual',
      'corp deadlines --deadline-type tax_filing --description "Q1 estimated taxes" --due-date 2026-04-15 --severity high',
    ],
    successTemplate: "Deadlines created",
  },
  {
    name: "entities compliance-escalations",
    description: "List compliance escalations for an entity",
    route: { method: "GET", path: "/v1/entities/{eid}/compliance/escalations" },
    entity: true,
    display: { title: "Entities Compliance Escalations", cols: ["action>Action", "authority>Authority", "milestone>Milestone", "@created_at>Created At", "#deadline_id>ID"] },
    examples: ["corp entities compliance-escalations", "corp entities compliance-escalations --json"],
  },
  {
    name: "tax create-filing",
    description: "Create a tax filing record",
    route: { method: "POST", path: "/v1/tax/filings" },
    entity: true,
    options: [
      { flags: "--document-type <document-type>", description: "Type of document required", required: true },
      { flags: "--tax-year <tax-year>", description: "Tax Year", required: true, type: "int" },
    ],
    examples: [
      "corp tax create-filing --document-type 1120 --tax-year 2025",
      "corp tax create-filing --document-type 1065 --tax-year 2025 --json",
    ],
    successTemplate: "Filings created",
  },

];