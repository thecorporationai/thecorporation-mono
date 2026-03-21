import type { CommandDef, CommandContext } from "./types.js";
import { printDryRun, printError, printJson, printReferenceSummary, printSuccess } from "../output.js";
import { setActiveEntityId, saveConfig, requireConfig } from "../config.js";
import { activateFormationEntity } from "../formation-automation.js";
import chalk from "chalk";
import Table from "cli-table3";
import type { ApiRecord } from "../types.js";

// ── form (one-shot formation) handler ──────────────────────────

async function formHandler(ctx: CommandContext): Promise<void> {
  const { formCommand } = await import("../commands/form.js");
  // Map --entity-type / --legal-name aliases to --type / --name for formCommand
  const opts: Record<string, unknown> = { ...ctx.opts, quiet: ctx.quiet };
  if (opts.entityType && !opts.type) opts.type = opts.entityType;
  if (opts.legalName && !opts.name) opts.name = opts.legalName;
  await formCommand(opts as Parameters<typeof formCommand>[0]);
}

// ── form create handler ────────────────────────────────────────

const SUPPORTED_ENTITY_TYPES = ["llc", "c_corp", "s_corp", "corporation"];

function parseCsvAddress(raw?: string): { street: string; city: string; state: string; zip: string } | undefined {
  if (!raw) return undefined;
  const parts = raw.split(",").map((p) => p.trim()).filter(Boolean);
  if (parts.length !== 4) {
    throw new Error(`Invalid address format: ${raw}. Expected 'street,city,state,zip'`);
  }
  return { street: parts[0], city: parts[1], state: parts[2], zip: parts[3] };
}

function shouldResolveEntityRefForDryRun(entityRef: string): boolean {
  const trimmed = entityRef.trim().toLowerCase();
  return trimmed === "_" || trimmed === "@last" || trimmed.startsWith("@last:");
}

async function resolveEntityRefForFormCommand(
  resolver: CommandContext["resolver"],
  entityRef: string,
  dryRun?: boolean,
): Promise<string> {
  if (!dryRun || shouldResolveEntityRefForDryRun(entityRef)) {
    return resolver.resolveEntity(entityRef);
  }
  try {
    return await resolver.resolveEntity(entityRef);
  } catch {
    // In dry-run mode, fall back to the raw ref if resolution fails
    // (e.g. fetch failed, auth error, network error, etc.)
    return entityRef;
  }
}

async function formCreateHandler(ctx: CommandContext): Promise<void> {
  const opts = ctx.opts;
  const resolvedType = opts.type as string | undefined;
  const resolvedName = opts.name as string | undefined;
  if (!resolvedType) {
    ctx.writer.error("required option '--type <type>' not specified");
    process.exit(1);
  }
  if (!SUPPORTED_ENTITY_TYPES.includes(resolvedType)) {
    ctx.writer.error(`unsupported entity type '${resolvedType}'. Supported types: ${SUPPORTED_ENTITY_TYPES.join(", ")}`);
    process.exit(1);
  }
  if (!resolvedName) {
    ctx.writer.error("required option '--name <name>' not specified");
    process.exit(1);
  }
  if (!resolvedName.trim()) {
    ctx.writer.error("--name cannot be empty or whitespace");
    process.exit(1);
  }

  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  try {
    const entityType = resolvedType === "corporation" ? "c_corp" : resolvedType;
    const payload: ApiRecord = {
      entity_type: entityType,
      legal_name: resolvedName,
    };
    if (opts.jurisdiction) payload.jurisdiction = opts.jurisdiction;
    if (opts.registeredAgentName) payload.registered_agent_name = opts.registeredAgentName;
    if (opts.registeredAgentAddress) payload.registered_agent_address = opts.registeredAgentAddress;
    if (opts.formationDate) payload.formation_date = opts.formationDate;
    if (opts.fiscalYearEnd) payload.fiscal_year_end = opts.fiscalYearEnd;
    if (opts.sCorp !== undefined) payload.s_corp_election = opts.sCorp;
    if (opts.transferRestrictions !== undefined) payload.transfer_restrictions = opts.transferRestrictions;
    if (opts.rofr !== undefined) payload.right_of_first_refusal = opts.rofr;
    const companyAddress = parseCsvAddress(opts.companyAddress as string | undefined);
    if (companyAddress) payload.company_address = companyAddress;

    if (ctx.dryRun) {
      printDryRun("formation.create_pending", payload);
      return;
    }

    const result = await ctx.client.createPendingEntity(payload);
    await ctx.resolver.stabilizeRecord("entity", result);
    ctx.resolver.rememberFromRecord("entity", result);

    if (result.entity_id) {
      setActiveEntityId(cfg, String(result.entity_id));
      saveConfig(cfg);
    }

    if (ctx.quiet) {
      const id = result.entity_id;
      if (id) console.log(String(id));
      return;
    }
    if (opts.json) {
      printJson(result);
      return;
    }
    printSuccess(`Pending entity created: ${result.entity_id}`);
    printReferenceSummary("entity", result, { showReuseHint: true });
    console.log(`  Name: ${result.legal_name}`);
    console.log(`  Type: ${result.entity_type}`);
    console.log(`  Jurisdiction: ${result.jurisdiction}`);
    console.log(`  Status: ${result.formation_status}`);
    console.log(chalk.yellow(`\n  Next: corp form add-founder @last:entity --name "..." --email "..." --role member --pct 50`));
  } catch (err) {
    printError(`Failed to create pending entity: ${err}`);
    process.exit(1);
  }
}

// ── form add-founder handler ───────────────────────────────────

async function formAddFounderHandler(ctx: CommandContext): Promise<void> {
  const entityRef = ctx.positional[0];
  const opts = ctx.opts;
  try {
    const resolvedEntityId = await resolveEntityRefForFormCommand(ctx.resolver, entityRef, ctx.dryRun);
    const payload: ApiRecord = {
      name: opts.name as string,
      email: opts.email as string,
      role: opts.role as string,
      ownership_pct: parseFloat(opts.pct as string),
    };
    if (opts.officerTitle) payload.officer_title = (opts.officerTitle as string).toLowerCase();
    if (opts.incorporator) payload.is_incorporator = true;
    const address = parseCsvAddress(opts.address as string | undefined);
    if (address) payload.address = address;

    if (ctx.dryRun) {
      printDryRun("formation.add_founder", { entity_id: resolvedEntityId, ...payload });
      return;
    }

    const result = await ctx.client.addFounder(resolvedEntityId, payload);
    if (opts.json) {
      printJson(result);
      return;
    }
    printSuccess(`Founder added (${result.member_count} total)`);
    const members = (result.members ?? []) as ApiRecord[];
    for (const m of members) {
      const pct = typeof m.ownership_pct === "number" ? ` (${m.ownership_pct}%)` : "";
      console.log(`  - ${m.name} <${m.email ?? "no email"}> [${m.role ?? "member"}]${pct}`);
    }
    console.log(chalk.yellow(`\n  Next: add more founders or run: corp form finalize @last:entity`));
  } catch (err) {
    printError(`Failed to add founder: ${err}`);
    process.exit(1);
  }
}

// ── form finalize handler ──────────────────────────────────────

async function formFinalizeHandler(ctx: CommandContext): Promise<void> {
  const entityRef = ctx.positional[0];
  const opts = ctx.opts;
  try {
    const resolvedEntityId = await resolveEntityRefForFormCommand(ctx.resolver, entityRef, ctx.dryRun);
    const payload: ApiRecord = {};
    if (opts.authorizedShares) {
      const authorizedShares = parseInt(opts.authorizedShares as string, 10);
      if (!Number.isFinite(authorizedShares)) {
        throw new Error(`Invalid authorized shares: ${opts.authorizedShares}`);
      }
      payload.authorized_shares = authorizedShares;
    }
    if (opts.parValue) payload.par_value = opts.parValue;
    if (opts.boardSize) {
      const boardSize = parseInt(opts.boardSize as string, 10);
      if (!Number.isFinite(boardSize) || boardSize <= 0) {
        throw new Error(`Invalid board size: ${opts.boardSize}`);
      }
      payload.board_size = boardSize;
    }
    if (opts.principalName) payload.principal_name = opts.principalName;
    if (opts.registeredAgentName) payload.registered_agent_name = opts.registeredAgentName;
    if (opts.registeredAgentAddress) payload.registered_agent_address = opts.registeredAgentAddress;
    if (opts.formationDate) payload.formation_date = opts.formationDate;
    if (opts.fiscalYearEnd) payload.fiscal_year_end = opts.fiscalYearEnd;
    if (opts.sCorp !== undefined) payload.s_corp_election = opts.sCorp;
    if (opts.transferRestrictions !== undefined) payload.transfer_restrictions = opts.transferRestrictions;
    if (opts.rofr !== undefined) payload.right_of_first_refusal = opts.rofr;
    const companyAddress = parseCsvAddress(opts.companyAddress as string | undefined);
    if (companyAddress) payload.company_address = companyAddress;
    if (opts.incorporatorName) payload.incorporator_name = opts.incorporatorName;
    if (opts.incorporatorAddress) payload.incorporator_address = opts.incorporatorAddress;

    if (ctx.dryRun) {
      printDryRun("formation.finalize", { entity_id: resolvedEntityId, ...payload });
      return;
    }

    const result = await ctx.client.finalizeFormation(resolvedEntityId, payload);
    await ctx.resolver.stabilizeRecord("entity", result);
    ctx.resolver.rememberFromRecord("entity", result);
    if (opts.json) {
      printJson(result);
      return;
    }
    printSuccess(`Formation finalized: ${result.entity_id}`);
    printReferenceSummary("entity", result, { showReuseHint: true });
    if (result.legal_entity_id) console.log(`  Legal Entity ID: ${result.legal_entity_id}`);
    if (result.instrument_id) console.log(`  Instrument ID: ${result.instrument_id}`);

    const docIds = (result.document_ids ?? []) as string[];
    if (docIds.length > 0) {
      console.log(`  Documents: ${docIds.length} generated`);
    }

    const holders = (result.holders ?? []) as ApiRecord[];
    if (holders.length > 0) {
      console.log();
      const table = new Table({
        head: [chalk.dim("Holder"), chalk.dim("Shares"), chalk.dim("Ownership %")],
      });
      for (const h of holders) {
        const pct = typeof h.ownership_pct === "number" ? `${h.ownership_pct.toFixed(1)}%` : "\u2014";
        table.push([String(h.name ?? "?"), String(h.shares ?? 0), pct]);
      }
      console.log(chalk.bold("  Cap Table:"));
      console.log(table.toString());
    }

    if (result.next_action) {
      console.log(chalk.yellow(`\n  Next: ${result.next_action}`));
    }
  } catch (err) {
    const msg = String(err);
    if (msg.includes("officers_list") || msg.includes("officer")) {
      printError(
        `Finalization failed: ${msg}\n` +
        "  Hint: C-Corp entities require at least one founder with an officer_title.\n" +
        "  Add a founder with: corp form add-founder @last:entity --name '...' --email '...' --role director --pct 100 --officer-title ceo",
      );
    } else {
      printError(`Failed to finalize formation: ${err}`);
    }
    process.exit(1);
  }
}

// ── form activate handler ──────────────────────────────────────

async function formActivateHandler(ctx: CommandContext): Promise<void> {
  const entityRef = ctx.positional[0];
  const opts = ctx.opts;
  try {
    const resolvedEntityId = await resolveEntityRefForFormCommand(ctx.resolver, entityRef, ctx.dryRun);
    const payload: ApiRecord = { entity_id: resolvedEntityId };
    if (opts.evidenceUri) payload.evidence_uri = opts.evidenceUri;
    if (opts.evidenceType) payload.evidence_type = opts.evidenceType;
    if (opts.filingId) payload.filing_id = opts.filingId;
    if (opts.receiptReference) payload.receipt_reference = opts.receiptReference;
    if (opts.ein) payload.ein = opts.ein;

    if (ctx.dryRun) {
      printDryRun("formation.activate", payload);
      return;
    }

    const result = await activateFormationEntity(ctx.client, ctx.resolver, resolvedEntityId, {
      evidenceUri: opts.evidenceUri as string | undefined,
      evidenceType: opts.evidenceType as string | undefined,
      filingId: opts.filingId as string | undefined,
      receiptReference: opts.receiptReference as string | undefined,
      ein: opts.ein as string | undefined,
    });
    const formation = await ctx.client.getFormation(resolvedEntityId);
    await ctx.resolver.stabilizeRecord("entity", formation as ApiRecord);
    ctx.resolver.rememberFromRecord("entity", formation as ApiRecord);

    if (opts.json) {
      printJson({
        ...result,
        formation,
      });
      return;
    }

    printSuccess(`Formation advanced to ${result.final_status}.`);
    printReferenceSummary("entity", formation as ApiRecord, { showReuseHint: true });
    if (result.steps.length > 0) {
      console.log("  Steps:");
      for (const step of result.steps) {
        console.log(`    - ${step}`);
      }
    }
    console.log(`  Signatures added: ${result.signatures_added}`);
    console.log(`  Documents updated: ${result.documents_signed}`);
  } catch (err) {
    printError(`Failed to activate formation: ${err}`);
    process.exit(1);
  }
}

// ── Command definitions ────────────────────────────────────────

export const formationCommands: CommandDef[] = [
  {
    name: "form",
    description: "Form a new entity with founders and cap table",
    passThroughOptions: true,
    dryRun: true,
    options: [
      { flags: "--type <type>", description: "Entity type (llc, c_corp)" },
      { flags: "--entity-type <type>", description: "Entity type (alias for --type)" },
      { flags: "--name <name>", description: "Legal name" },
      { flags: "--legal-name <name>", description: "Legal name (alias for --name)" },
      { flags: "--jurisdiction <jurisdiction>", description: "Jurisdiction (e.g. US-DE, US-WY)" },
      { flags: "--member <member>", description: "Founder as 'name,email,role[,pct[,address[,officer_title[,is_incorporator]]]]' (repeatable)", type: "array", default: [] },
      { flags: "--member-json <json>", description: "Founder JSON object (repeatable)", type: "array", default: [] },
      { flags: "--members-file <path>", description: "Path to a JSON array of founders or {\"members\": [...]}" },
      { flags: "--address <address>", description: "Company address as 'street,city,state,zip'" },
      { flags: "--fiscal-year-end <date>", description: "Fiscal year end (MM-DD)", default: "12-31" },
      { flags: "--s-corp", description: "Elect S-Corp status" },
      { flags: "--transfer-restrictions", description: "Enable transfer restrictions" },
      { flags: "--rofr", description: "Enable right of first refusal" },
    ],
    handler: formHandler,
    produces: { kind: "entity", trackEntity: true },
    successTemplate: "Entity formed: {legal_name}",
    examples: [
      'corp form --type llc --name "My LLC" --member "Alice,alice@co.com,member,100"',
      "corp form --type c_corp --name \"Acme Inc\" --jurisdiction US-DE --member-json '{\"name\":\"Bob\",\"email\":\"bob@acme.com\",\"role\":\"director\",\"pct\":100}'",
      'corp form create --type llc --name "My LLC"',
      'corp form add-founder @last:entity --name "Alice" --email "alice@co.com" --role member --pct 100',
      "corp form finalize @last:entity",
      "corp form activate @last:entity",
    ],
  },
  {
    name: "form create",
    description: "Create a pending entity (staged flow step 1)",
    dryRun: true,
    options: [
      { flags: "--type <type>", description: "Entity type (llc, c_corp)" },
      { flags: "--name <name>", description: "Legal name" },
      { flags: "--jurisdiction <jurisdiction>", description: "Jurisdiction (e.g. US-DE, US-WY)" },
      { flags: "--registered-agent-name <name>", description: "Registered agent legal name" },
      { flags: "--registered-agent-address <address>", description: "Registered agent address line" },
      { flags: "--formation-date <date>", description: "Formation date (RFC3339 or YYYY-MM-DD)" },
      { flags: "--fiscal-year-end <date>", description: "Fiscal year end (MM-DD)" },
      { flags: "--s-corp", description: "Elect S-Corp status" },
      { flags: "--transfer-restrictions", description: "Enable transfer restrictions" },
      { flags: "--rofr", description: "Enable right of first refusal" },
      { flags: "--company-address <address>", description: "Company address as 'street,city,state,zip'" },
    ],
    handler: formCreateHandler,
    produces: { kind: "entity", trackEntity: true },
    successTemplate: "Pending entity created: {legal_name}",
  },
  {
    name: "form add-founder",
    description: "Add a founder to a pending entity (staged flow step 2)",
    dryRun: true,
    args: [{ name: "entity-ref", required: true }],
    options: [
      { flags: "--name <name>", description: "Founder name", required: true },
      { flags: "--email <email>", description: "Founder email", required: true },
      { flags: "--role <role>", description: "Role: director|officer|manager|member|chair", required: true },
      { flags: "--pct <pct>", description: "Ownership percentage", required: true },
      { flags: "--officer-title <title>", description: "Officer title (corporations only)", choices: ["ceo", "cfo", "cto", "coo", "secretary", "treasurer", "president", "vp", "other"] },
      { flags: "--incorporator", description: "Mark as sole incorporator (corporations only)" },
      { flags: "--address <address>", description: "Founder address as 'street,city,state,zip'" },
    ],
    handler: formAddFounderHandler,
  },
  {
    name: "form finalize",
    description: "Finalize formation and generate documents + cap table (staged flow step 3)",
    dryRun: true,
    args: [{ name: "entity-ref", required: true }],
    options: [
      { flags: "--authorized-shares <count>", description: "Authorized shares for corporations" },
      { flags: "--par-value <value>", description: "Par value per share, e.g. 0.0001" },
      { flags: "--board-size <count>", description: "Board size for corporations" },
      { flags: "--principal-name <name>", description: "Principal or manager name for LLCs" },
      { flags: "--registered-agent-name <name>", description: "Registered agent legal name" },
      { flags: "--registered-agent-address <address>", description: "Registered agent address line" },
      { flags: "--formation-date <date>", description: "Formation date (RFC3339 or YYYY-MM-DD)" },
      { flags: "--fiscal-year-end <date>", description: "Fiscal year end (MM-DD)" },
      { flags: "--s-corp", description: "Elect S-Corp status" },
      { flags: "--transfer-restrictions", description: "Enable transfer restrictions" },
      { flags: "--rofr", description: "Enable right of first refusal" },
      { flags: "--company-address <address>", description: "Company address as 'street,city,state,zip'" },
      { flags: "--incorporator-name <name>", description: "Incorporator legal name (overrides founder)" },
      { flags: "--incorporator-address <address>", description: "Incorporator mailing address (overrides founder)" },
    ],
    handler: formFinalizeHandler,
  },
  {
    name: "form activate",
    description: "Programmatically sign formation documents and advance an entity to active",
    dryRun: true,
    args: [{ name: "entity-ref", required: true }],
    options: [
      { flags: "--evidence-uri <uri>", description: "Registered-agent consent evidence URI placeholder" },
      { flags: "--evidence-type <type>", description: "Registered-agent consent evidence type", default: "generated" },
      { flags: "--filing-id <id>", description: "External filing identifier to record" },
      { flags: "--receipt-reference <ref>", description: "External receipt reference to record" },
      { flags: "--ein <ein>", description: "EIN to confirm (defaults to a deterministic simulated EIN)" },
    ],
    handler: formActivateHandler,
  },

  // ── Auto-generated from OpenAPI ──────────────────────────────
  {
    name: "contracts",
    description: "/v1/contracts",
    route: { method: "POST", path: "/v1/contracts" },
    options: [
      { flags: "--counterparty-name <counterparty-name>", description: "Counterparty Name", required: true },
      { flags: "--effective-date <effective-date>", description: "Effective Date", required: true },
      { flags: "--parameters <parameters>", description: "Parameters" },
      { flags: "--template-type <template-type>", description: "Template type for generated contracts.", required: true, choices: ["consulting_agreement", "employment_offer", "contractor_agreement", "nda", "safe_agreement", "custom"] },
    ],
  },
  {
    name: "documents",
    description: "/v1/documents/{document_id}",
    route: { method: "GET", path: "/v1/documents/{pos}" },
    entity: true,
    args: [{ name: "document-id", required: true, description: "Document Id" }],
    display: { title: "Documents", cols: ["content>Content", "content_hash>Content Hash", "@created_at>Created At", "#document_id>ID", "document_type>Document Type", "#entity_id>ID", "signatures>Signatures", "status>Status"] },
  },
  {
    name: "documents amendment-history",
    description: "/v1/documents/{document_id}/amendment-history",
    route: { method: "GET", path: "/v1/documents/{pos}/amendment-history" },
    entity: true,
    args: [{ name: "document-id", required: true, description: "Document Id" }],
    display: { title: "Documents Amendment History", cols: ["amended_at>Amended At", "description>Description", "version>Version"] },
  },
  {
    name: "documents pdf",
    description: "/v1/documents/{document_id}/pdf",
    route: { method: "GET", path: "/v1/documents/{pos}/pdf" },
    entity: true,
    args: [{ name: "document-id", required: true, description: "Document Id" }],
  },
  {
    name: "documents request-copy",
    description: "/v1/documents/{document_id}/request-copy",
    route: { method: "POST", path: "/v1/documents/{pos}/request-copy" },
    args: [{ name: "document-id", required: true, description: "Document Id" }],
    options: [
      { flags: "--recipient-email <recipient-email>", description: "Recipient Email" },
    ],
  },
  {
    name: "entities",
    description: "/v1/entities",
    route: { method: "GET", path: "/v1/entities" },
    display: { title: "Entities", cols: ["#entity_id>ID", "entity_type>Entity Type", "formation_date>Formation Date", "formation_state>Formation State", "formation_status>Formation Status", "jurisdiction>Jurisdiction", "legal_name>Legal Name", "next_action>Next Action"] },
  },
  {
    name: "entities governance-documents",
    description: "/v1/entities/{entity_id}/governance-documents",
    route: { method: "GET", path: "/v1/entities/{eid}/governance-documents" },
    entity: true,
    display: { title: "Entities Governance Documents", cols: ["@created_at>Created At", "#document_id>ID", "document_type>Document Type", "signature_count>Signature Count", "status>Status", "title>Title"] },
  },
  {
    name: "entities governance-documents-current",
    description: "/v1/entities/{entity_id}/governance-documents/current",
    route: { method: "GET", path: "/v1/entities/{eid}/governance-documents/current" },
    entity: true,
    display: { title: "Entities Governance Documents Current", cols: ["@created_at>Created At", "#document_id>ID", "document_type>Document Type", "signature_count>Signature Count", "status>Status", "title>Title"] },
  },
  {
    name: "formations",
    description: "/v1/formations",
    route: { method: "POST", path: "/v1/formations" },
    options: [
      { flags: "--authorized-shares <authorized-shares>", description: "Authorized Shares" },
      { flags: "--company-address <company-address>", description: "Company Address" },
      { flags: "--entity-type <entity-type>", description: "The legal structure of a business entity.", required: true, choices: ["c_corp", "llc"] },
      { flags: "--fiscal-year-end <fiscal-year-end>", description: "Fiscal year end, e.g. \"12-31\". Defaults to \"12-31\"." },
      { flags: "--formation-date <formation-date>", description: "Optional formation date for importing pre-formed entities." },
      { flags: "--jurisdiction <jurisdiction>", description: "Jurisdiction (e.g. Delaware, US-DE)", required: true },
      { flags: "--legal-name <legal-name>", description: "Legal Name", required: true },
      { flags: "--members <members>", description: "Members", required: true, type: "array" },
      { flags: "--par-value <par-value>", description: "Par Value" },
      { flags: "--registered-agent-address <registered-agent-address>", description: "Registered Agent Address" },
      { flags: "--registered-agent-name <registered-agent-name>", description: "Registered Agent Name" },
      { flags: "--right-of-first-refusal <right-of-first-refusal>", description: "Include right of first refusal in bylaws (corp). Default true." },
      { flags: "--s-corp-election <s-corp-election>", description: "Whether the company will elect S-Corp tax treatment." },
      { flags: "--transfer-restrictions <transfer-restrictions>", description: "Include transfer restrictions in bylaws (corp). Default true." },
    ],
  },
  {
    name: "formations pending",
    description: "/v1/formations/pending",
    route: { method: "POST", path: "/v1/formations/pending" },
    options: [
      { flags: "--company-address <company-address>", description: "Company Address" },
      { flags: "--entity-type <entity-type>", description: "The legal structure of a business entity.", required: true, choices: ["c_corp", "llc"] },
      { flags: "--fiscal-year-end <fiscal-year-end>", description: "Fiscal Year End" },
      { flags: "--formation-date <formation-date>", description: "Formation Date" },
      { flags: "--jurisdiction <jurisdiction>", description: "Jurisdiction" },
      { flags: "--legal-name <legal-name>", description: "Legal Name", required: true },
      { flags: "--registered-agent-address <registered-agent-address>", description: "Registered Agent Address" },
      { flags: "--registered-agent-name <registered-agent-name>", description: "Registered Agent Name" },
      { flags: "--right-of-first-refusal <right-of-first-refusal>", description: "Right Of First Refusal" },
      { flags: "--s-corp-election <s-corp-election>", description: "S Corp Election" },
      { flags: "--transfer-restrictions <transfer-restrictions>", description: "Transfer Restrictions" },
    ],
  },
  {
    name: "formations with-cap-table",
    description: "/v1/formations/with-cap-table",
    route: { method: "POST", path: "/v1/formations/with-cap-table" },
    options: [
      { flags: "--authorized-shares <authorized-shares>", description: "Authorized Shares" },
      { flags: "--company-address <company-address>", description: "Company Address" },
      { flags: "--entity-type <entity-type>", description: "The legal structure of a business entity.", required: true, choices: ["c_corp", "llc"] },
      { flags: "--fiscal-year-end <fiscal-year-end>", description: "Fiscal year end, e.g. \"12-31\". Defaults to \"12-31\"." },
      { flags: "--formation-date <formation-date>", description: "Optional formation date for importing pre-formed entities." },
      { flags: "--jurisdiction <jurisdiction>", description: "Jurisdiction (e.g. Delaware, US-DE)", required: true },
      { flags: "--legal-name <legal-name>", description: "Legal Name", required: true },
      { flags: "--members <members>", description: "Members", required: true, type: "array" },
      { flags: "--par-value <par-value>", description: "Par Value" },
      { flags: "--registered-agent-address <registered-agent-address>", description: "Registered Agent Address" },
      { flags: "--registered-agent-name <registered-agent-name>", description: "Registered Agent Name" },
      { flags: "--right-of-first-refusal <right-of-first-refusal>", description: "Include right of first refusal in bylaws (corp). Default true." },
      { flags: "--s-corp-election <s-corp-election>", description: "Whether the company will elect S-Corp tax treatment." },
      { flags: "--transfer-restrictions <transfer-restrictions>", description: "Include transfer restrictions in bylaws (corp). Default true." },
    ],
  },
  {
    name: "formations",
    description: "/v1/formations/{entity_id}",
    route: { method: "GET", path: "/v1/formations/{eid}" },
    entity: true,
    display: { title: "Formations", cols: ["#entity_id>ID", "entity_type>Entity Type", "formation_date>Formation Date", "formation_state>Formation State", "formation_status>Formation Status", "jurisdiction>Jurisdiction", "legal_name>Legal Name", "next_action>Next Action"] },
  },
  {
    name: "formations apply-ein",
    description: "/v1/formations/{entity_id}/apply-ein",
    route: { method: "POST", path: "/v1/formations/{eid}/apply-ein" },
    entity: true,
  },
  {
    name: "formations ein-confirmation",
    description: "/v1/formations/{entity_id}/ein-confirmation",
    route: { method: "POST", path: "/v1/formations/{eid}/ein-confirmation" },
    entity: true,
    options: [
      { flags: "--ein <ein>", description: "Ein", required: true },
    ],
  },
  {
    name: "formations filing-attestation",
    description: "/v1/formations/{entity_id}/filing-attestation",
    route: { method: "POST", path: "/v1/formations/{eid}/filing-attestation" },
    entity: true,
    options: [
      { flags: "--consent-text <consent-text>", description: "Consent Text" },
      { flags: "--notes <notes>", description: "Notes" },
      { flags: "--signer-email <signer-email>", description: "Signer Email", required: true },
      { flags: "--signer-name <signer-name>", description: "Signer Name", required: true },
      { flags: "--signer-role <signer-role>", description: "Signer Role", required: true },
    ],
  },
  {
    name: "formations filing-confirmation",
    description: "/v1/formations/{entity_id}/filing-confirmation",
    route: { method: "POST", path: "/v1/formations/{eid}/filing-confirmation" },
    entity: true,
    options: [
      { flags: "--external-filing-id <external-filing-id>", description: "External Filing Id", required: true },
      { flags: "--receipt-reference <receipt-reference>", description: "Receipt Reference" },
    ],
  },
  {
    name: "formations finalize",
    description: "/v1/formations/{entity_id}/finalize",
    route: { method: "POST", path: "/v1/formations/{eid}/finalize" },
    entity: true,
    options: [
      { flags: "--authorized-shares <authorized-shares>", description: "Authorized Shares" },
      { flags: "--board-size <board-size>", description: "Board Size" },
      { flags: "--company-address <company-address>", description: "Company Address" },
      { flags: "--fiscal-year-end <fiscal-year-end>", description: "Fiscal Year End" },
      { flags: "--formation-date <formation-date>", description: "Formation Date" },
      { flags: "--incorporator-address <incorporator-address>", description: "Incorporator Address" },
      { flags: "--incorporator-name <incorporator-name>", description: "Incorporator Name" },
      { flags: "--par-value <par-value>", description: "Par Value" },
      { flags: "--principal-name <principal-name>", description: "Principal Name" },
      { flags: "--registered-agent-address <registered-agent-address>", description: "Registered Agent Address" },
      { flags: "--registered-agent-name <registered-agent-name>", description: "Registered Agent Name" },
      { flags: "--right-of-first-refusal <right-of-first-refusal>", description: "Right Of First Refusal" },
      { flags: "--s-corp-election <s-corp-election>", description: "S Corp Election" },
      { flags: "--transfer-restrictions <transfer-restrictions>", description: "Transfer Restrictions" },
    ],
  },
  {
    name: "formations founders",
    description: "/v1/formations/{entity_id}/founders",
    route: { method: "POST", path: "/v1/formations/{eid}/founders" },
    entity: true,
    options: [
      { flags: "--address <address>", description: "Address" },
      { flags: "--email <email>", description: "Email" },
      { flags: "--is-incorporator <is-incorporator>", description: "Is Incorporator" },
      { flags: "--name <name>", description: "Name", required: true },
      { flags: "--officer-title <officer-title>", description: "Officer Title", choices: ["ceo", "cfo", "cto", "coo", "secretary", "treasurer", "president", "vp", "other"] },
      { flags: "--ownership-pct <ownership-pct>", description: "Ownership Pct" },
      { flags: "--role <role>", description: "Role", choices: ["director", "officer", "manager", "member", "chair"] },
    ],
  },
  {
    name: "formations gates",
    description: "/v1/formations/{entity_id}/gates",
    route: { method: "GET", path: "/v1/formations/{eid}/gates" },
    entity: true,
    display: { title: "Formations Gates", cols: ["attestation_recorded>Attestation Recorded", "designated_attestor_email>Designated Attestor Email", "designated_attestor_name>Designated Attestor Name", "designated_attestor_role>Designated Attestor Role", "#entity_id>ID", "filing_submission_blockers>Filing Submission Blockers", "registered_agent_consent_evidence_count>Registered Agent Consent Evidence Count", "requires_natural_person_attestation>Requires Natural Person Attestation"] },
  },
  {
    name: "formations mark-documents-signed",
    description: "/v1/formations/{entity_id}/mark-documents-signed",
    route: { method: "POST", path: "/v1/formations/{eid}/mark-documents-signed" },
    entity: true,
  },
  {
    name: "formations registered-agent-consent-evidence",
    description: "/v1/formations/{entity_id}/registered-agent-consent-evidence",
    route: { method: "POST", path: "/v1/formations/{eid}/registered-agent-consent-evidence" },
    entity: true,
    options: [
      { flags: "--evidence-type <evidence-type>", description: "Evidence Type" },
      { flags: "--evidence-uri <evidence-uri>", description: "Evidence Uri", required: true },
      { flags: "--notes <notes>", description: "Notes" },
    ],
  },
  {
    name: "formations service-agreement-execute",
    description: "/v1/formations/{entity_id}/service-agreement/execute",
    route: { method: "POST", path: "/v1/formations/{eid}/service-agreement/execute" },
    entity: true,
    options: [
      { flags: "--contract-id <contract-id>", description: "Contract Id" },
      { flags: "--document-id <document-id>", description: "Document Id" },
      { flags: "--notes <notes>", description: "Notes" },
    ],
  },
  {
    name: "formations submit-filing",
    description: "/v1/formations/{entity_id}/submit-filing",
    route: { method: "POST", path: "/v1/formations/{eid}/submit-filing" },
    entity: true,
  },
  {
    name: "governance catalog",
    description: "/v1/governance/catalog",
    route: { method: "GET", path: "/v1/governance/catalog" },
    display: { title: "Governance Catalog", cols: ["categories>Categories"] },
  },
  {
    name: "governance catalog-markdown",
    description: "/v1/governance/catalog/{document_id}/markdown",
    route: { method: "GET", path: "/v1/governance/catalog/{pos}/markdown" },
    args: [{ name: "document-id", required: true, description: "Document Id" }],
    display: { title: "Governance Catalog Markdown", cols: ["category>Category", "entity_scope>Entity Scope", "id>Id", "markdown>Markdown", "title>Title", "variants>Variants"] },
  },

];