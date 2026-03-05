import type { CorpAPIClient } from "./api-client.js";
import { readFileSync, writeFileSync, mkdirSync, existsSync } from "node:fs";
import { join } from "node:path";
import { GENERATED_TOOL_DEFINITIONS } from "./tool-defs.generated.js";

export interface ToolContext {
  dataDir: string;
  onEntityFormed?: (entityId: string) => void;
}

type ToolHandler = (args: Record<string, unknown>, client: CorpAPIClient, ctx: ToolContext) => Promise<unknown>;

function requiredString(args: Record<string, unknown>, key: string): string {
  const value = args[key];
  if (typeof value !== "string" || value.trim().length === 0) {
    throw new Error(`Missing required field: ${key}`);
  }
  return value;
}

const TOOL_HANDLERS: Record<string, ToolHandler> = {
  get_workspace_status: async (_args, client) => client.getStatus(),
  list_obligations: async (args, client) => client.getObligations(args.tier as string | undefined),
  list_entities: async (_args, client) => client.listEntities(),
  get_cap_table: async (args, client) => client.getCapTable(args.entity_id as string),
  list_documents: async (args, client) => client.getEntityDocuments(args.entity_id as string),
  list_safe_notes: async (args, client) => client.getSafeNotes(args.entity_id as string),
  list_agents: async (_args, client) => client.listAgents(),
  get_billing_status: async (_args, client) => {
    const [status, plans] = await Promise.all([client.getBillingStatus(), client.getBillingPlans()]);
    return { status, plans };
  },

  create_entity: async (args, client) => {
    const entityType = args.entity_type as string;
    let jurisdiction = (args.jurisdiction as string) || "";
    if (!jurisdiction || jurisdiction.length === 2) {
      jurisdiction = entityType === "llc" ? "US-WY" : "US-DE";
    }
    return client.createPendingEntity({
      entity_type: entityType,
      legal_name: args.entity_name,
      jurisdiction,
    });
  },

  add_founder: async (args, client) => {
    const entityId = requiredString(args, "entity_id");
    let ownershipPct = args.ownership_pct as number | undefined;
    if (typeof ownershipPct === "number" && ownershipPct > 1) {
      ownershipPct = ownershipPct / 100;
    }
    return client.addFounder(entityId, {
      name: args.name,
      email: args.email,
      role: args.role,
      ownership_pct: ownershipPct,
      officer_title: args.officer_title,
      is_incorporator: args.is_incorporator,
    });
  },

  finalize_formation: async (args, client, ctx) => {
    const entityId = requiredString(args, "entity_id");
    const result = await client.finalizeFormation(entityId);
    if (entityId && ctx.onEntityFormed) {
      ctx.onEntityFormed(entityId);
    }
    return result;
  },

  form_entity: async (args, client, ctx) => {
    const entityType = args.entity_type as string;
    let jurisdiction = (args.jurisdiction as string) || "";
    if (!jurisdiction || jurisdiction.length === 2) {
      jurisdiction = entityType === "llc" ? "US-WY" : "US-DE";
    }
    const members = (args.members ?? []) as Record<string, unknown>[];
    if (!members.length) return { error: "Members are required." };
    // Normalize: ensure investor_type defaults, convert ownership_pct > 1 to 0-1 scale
    for (const m of members) {
      if (!m.investor_type) m.investor_type = "natural_person";
      if (typeof m.ownership_pct === "number" && (m.ownership_pct as number) > 1) {
        m.ownership_pct = (m.ownership_pct as number) / 100;
      }
    }
    const result = await client.createFormationWithCapTable({
      entity_type: entityType, legal_name: args.entity_name, jurisdiction,
      members, workspace_id: client.workspaceId,
      fiscal_year_end: args.fiscal_year_end,
      s_corp_election: args.s_corp_election,
      transfer_restrictions: args.transfer_restrictions,
      right_of_first_refusal: args.right_of_first_refusal,
      company_address: args.company_address,
    });
    const entityId = result.entity_id as string;
    if (entityId && ctx.onEntityFormed) {
      ctx.onEntityFormed(entityId);
    }
    return result;
  },

  start_equity_round: async (args, client) => client.startEquityRound(args),
  add_security: async (args, client) => {
    const roundId = requiredString(args, "round_id");
    return client.addRoundSecurity(roundId, args);
  },
  issue_round: async (args, client) => {
    const roundId = requiredString(args, "round_id");
    return client.issueRound(roundId, args);
  },

  create_valuation: async (args, client) => client.createValuation(args),
  submit_valuation_for_approval: async (args, client) =>
    client.submitValuationForApproval(
      requiredString(args, "valuation_id"),
      requiredString(args, "entity_id"),
    ),
  approve_valuation: async (args, client) =>
    client.approveValuation(
      requiredString(args, "valuation_id"),
      requiredString(args, "entity_id"),
      args.resolution_id as string | undefined,
    ),

  issue_equity: async (args, client) => client.issueEquity(args),
  issue_safe: async (args, client) => client.issueSafe(args),
  create_invoice: async (args, client) => {
    if (!("amount_cents" in args) && Array.isArray(args.line_items)) {
      args.amount_cents = (args.line_items as Record<string, number>[])
        .reduce((sum, item) => sum + (item.amount_cents ?? 0), 0);
    }
    if (!("amount_cents" in args)) args.amount_cents = 0;
    if (!("description" in args) || typeof args.description !== "string" || args.description.trim().length === 0) {
      args.description = "Invoice";
    }
    return client.createInvoice(args);
  },
  run_payroll: async (args, client) => client.runPayroll(args),
  submit_payment: async (args, client) => client.submitPayment(args),
  open_bank_account: async (args, client) => {
    const body: Record<string, unknown> = { entity_id: args.entity_id };
    if (args.institution_name) body.institution_name = args.institution_name;
    return client.openBankAccount(body);
  },
  generate_contract: async (args, client) => client.generateContract(args),
  file_tax_document: async (args, client) => client.fileTaxDocument(args),

  get_signer_link: async (args, client) => {
    const result = await client.getSignerToken(args.obligation_id as string);
    const token = result.token as string ?? "";
    const obligationId = args.obligation_id as string;
    const humansBase = client.apiUrl.replace("://api.", "://humans.");
    return {
      signer_url: `${humansBase}/human/${obligationId}?token=${token}`,
      obligation_id: obligationId,
      expires_in_seconds: result.expires_in ?? 900,
      message: "Share this link with the signer. Link expires in 15 minutes.",
    };
  },

  schedule_meeting: async (args, client) => {
    const body: Record<string, unknown> = {
      entity_id: requiredString(args, "entity_id"),
      body_id: requiredString(args, "body_id"),
      meeting_type: requiredString(args, "meeting_type"),
      title: requiredString(args, "title"),
    };
    const scheduledDate = args.scheduled_date ?? args.proposed_date;
    if (typeof scheduledDate === "string" && scheduledDate.trim().length > 0) {
      body.scheduled_date = scheduledDate;
    }
    const agendaItems = args.agenda_item_titles ?? args.agenda_items;
    if (Array.isArray(agendaItems)) body.agenda_item_titles = agendaItems;
    return client.scheduleMeeting(body);
  },

  cast_vote: async (args, client) =>
    client.castVote(
      requiredString(args, "entity_id"),
      requiredString(args, "meeting_id"),
      requiredString(args, "agenda_item_id"),
      {
        voter_id: requiredString(args, "voter_id"),
        vote_value: requiredString(args, "vote_value"),
      },
    ),

  update_checklist: async (args, _client, ctx) => {
    const path = join(ctx.dataDir, "checklist.md");
    mkdirSync(ctx.dataDir, { recursive: true });
    writeFileSync(path, args.checklist as string);
    return { status: "updated", checklist: args.checklist };
  },

  get_checklist: async (_args, _client, ctx) => {
    const path = join(ctx.dataDir, "checklist.md");
    if (existsSync(path)) return { checklist: readFileSync(path, "utf-8") };
    return { checklist: null, message: "No checklist yet." };
  },

  get_document_link: async (args, client) => {
    const docId = args.document_id as string;
    try {
      const resp = await fetch(`${client.apiUrl}/v1/documents/${docId}/request-copy`, {
        method: "POST",
        headers: { Authorization: `Bearer ${client.apiKey}`, "Content-Type": "application/json" },
        body: JSON.stringify({ email: "owner@workspace" }),
      });
      if (!resp.ok) throw new Error("request-copy failed");
      const result = await resp.json() as Record<string, string>;
      let downloadUrl = result.download_url ?? "";
      if (downloadUrl.startsWith("/")) downloadUrl = client.apiUrl + downloadUrl;
      return { document_id: docId, download_url: downloadUrl, expires_in: "24 hours" };
    } catch {
      return {
        document_id: docId,
        download_url: `${client.apiUrl}/v1/documents/${docId}/pdf`,
        note: "Use your API key to authenticate the download.",
      };
    }
  },

  preview_document_pdf: async (args, client) => {
    const entityId = args.entity_id as string;
    const documentId = args.document_id as string;
    const qs = new URLSearchParams({ entity_id: entityId, document_id: documentId }).toString();
    return {
      entity_id: entityId,
      document_id: documentId,
      download_url: `${client.apiUrl}/v1/documents/preview/pdf?${qs}`,
      note: "Use your API key to authenticate the download.",
    };
  },

  get_signing_link: async (args, client) => client.getSigningLink(args.document_id as string),
  convert_entity: async (args, client) => client.convertEntity(args.entity_id as string, args),
  dissolve_entity: async (args, client) => client.dissolveEntity(args.entity_id as string, args),
  transfer_shares: async (args, client) => {
    if (args.skip_governance_review !== true) {
      return {
        error: "Transfer blocked: governance review required. Use the transfer workflow (create_transfer_workflow → submit-review → record-board-approval → execute) for governed transfers. To bypass governance and record a direct transfer, pass skip_governance_review: true.",
      };
    }
    return client.transferShares(args);
  },
  calculate_distribution: async (args, client) => client.calculateDistribution(args),
  classify_contractor: async (args, client) => client.classifyContractor(args),
  reconcile_ledger: async (args, client) => client.reconcileLedger(args),
  track_deadline: async (args, client) => client.trackDeadline(args),
  convene_meeting: async (args, client) => client.conveneMeeting(
    requiredString(args, "meeting_id"),
    requiredString(args, "entity_id"),
    {
      present_seat_ids: Array.isArray(args.present_seat_ids) ? args.present_seat_ids : [],
    },
  ),

  send_notice: async (args, client) => client.sendNotice(
    requiredString(args, "meeting_id"),
    requiredString(args, "entity_id"),
  ),

  adjourn_meeting: async (args, client) => client.adjournMeeting(
    requiredString(args, "meeting_id"),
    requiredString(args, "entity_id"),
  ),

  cancel_meeting: async (args, client) => client.cancelMeeting(
    requiredString(args, "meeting_id"),
    requiredString(args, "entity_id"),
  ),

  finalize_agenda_item: async (args, client) => client.finalizeAgendaItem(
    requiredString(args, "meeting_id"),
    requiredString(args, "agenda_item_id"),
    {
      entity_id: requiredString(args, "entity_id"),
      status: requiredString(args, "status"),
    },
  ),

  compute_resolution: async (args, client) => {
    const data: Record<string, unknown> = {
      resolution_text: requiredString(args, "resolution_text"),
    };
    if (typeof args.effective_date === "string") data.effective_date = args.effective_date;
    return client.computeResolution(
      requiredString(args, "meeting_id"),
      requiredString(args, "agenda_item_id"),
      requiredString(args, "entity_id"),
      data,
    );
  },

  attach_resolution_document: async (args, client) => client.attachResolutionDocument(
    requiredString(args, "meeting_id"),
    requiredString(args, "resolution_id"),
    {
      entity_id: requiredString(args, "entity_id"),
      document_id: requiredString(args, "document_id"),
    },
  ),

  written_consent: async (args, client) => client.writtenConsent({
    entity_id: requiredString(args, "entity_id"),
    body_id: requiredString(args, "body_id"),
    title: requiredString(args, "title"),
    description: args.description as string ?? "",
  }),

  list_agenda_items: async (args, client) => client.listAgendaItems(
    requiredString(args, "meeting_id"),
    requiredString(args, "entity_id"),
  ),

  list_votes: async (args, client) => client.listVotes(
    requiredString(args, "meeting_id"),
    requiredString(args, "agenda_item_id"),
    requiredString(args, "entity_id"),
  ),

  create_agent: async (args, client) => client.createAgent(args),
  send_agent_message: async (args, client) => client.sendAgentMessage(args.agent_id as string, args.body as string),
  update_agent: async (args, client) => client.updateAgent(args.agent_id as string, args),
  add_agent_skill: async (args, client) => client.addAgentSkill(args.agent_id as string, args),
};

// Tool definitions are generated from the backend OpenAPI spec.
// Regenerate: make generate-tools
export const TOOL_DEFINITIONS: Record<string, unknown>[] = GENERATED_TOOL_DEFINITIONS;

const READ_ONLY_TOOLS = new Set([
  "get_workspace_status", "list_entities", "get_cap_table", "list_documents",
  "list_safe_notes", "list_agents", "get_checklist",
  "get_signing_link", "list_obligations", "get_billing_status",
  "list_agenda_items", "list_votes",
  "preview_document_pdf",
]);

export function isWriteTool(name: string): boolean {
  return !READ_ONLY_TOOLS.has(name);
}

export async function executeTool(
  name: string,
  args: Record<string, unknown>,
  client: CorpAPIClient,
  ctx: ToolContext,
): Promise<string> {
  const handler = TOOL_HANDLERS[name];
  if (!handler) return JSON.stringify({ error: `Unknown tool: ${name}` });
  try {
    const result = await handler(args, client, ctx);
    return JSON.stringify(result, null, 0);
  } catch (err) {
    return JSON.stringify({ error: String(err) });
  }
}
