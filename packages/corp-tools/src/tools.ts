import type { CorpAPIClient } from "./api-client.js";
import { readFileSync, writeFileSync, mkdirSync, existsSync } from "node:fs";
import { join } from "node:path";

export interface ToolContext {
  dataDir: string;
  onEntityFormed?: (entityId: string) => void;
}

type ToolHandler = (args: Record<string, unknown>, client: CorpAPIClient, ctx: ToolContext) => Promise<unknown>;

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

  form_entity: async (args, client, ctx) => {
    const entityType = args.entity_type as string;
    let jurisdiction = (args.jurisdiction as string) || "";
    if (!jurisdiction || jurisdiction.length === 2) {
      jurisdiction = entityType === "llc" ? "US-WY" : "US-DE";
    }
    const members = (args.members ?? []) as Record<string, unknown>[];
    if (!members.length) return { error: "Members are required." };
    const result = await client.createFormation({
      entity_type: entityType, legal_name: args.entity_name, jurisdiction,
      members, workspace_id: client.workspaceId,
    });
    const entityId = result.entity_id as string;
    if (entityId && ctx.onEntityFormed) {
      ctx.onEntityFormed(entityId);
    }
    return result;
  },

  issue_equity: async (args, client) => client.issueEquity(args),
  issue_safe: async (args, client) => client.issueSafe(args),
  create_invoice: async (args, client) => {
    if (!("amount_cents" in args) && Array.isArray(args.line_items)) {
      args.amount_cents = (args.line_items as Record<string, number>[])
        .reduce((sum, item) => sum + (item.amount_cents ?? 0), 0);
    }
    if (!("amount_cents" in args)) args.amount_cents = 0;
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
      body_id: args.body_id, meeting_type: args.meeting_type,
      title: args.title, proposed_date: args.proposed_date,
    };
    if (args.agenda_items) body.agenda_item_titles = args.agenda_items;
    return client.conveneMeeting(body);
  },

  cast_vote: async (args, client) =>
    client.castVote(args.meeting_id as string, args.agenda_item_id as string, {
      voter_id: args.voter_id, vote_value: args.vote,
    }),

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

  get_signing_link: async (args, client) => client.getSigningLink(args.document_id as string),
  convert_entity: async (args, client) => client.convertEntity(args.entity_id as string, args),
  dissolve_entity: async (args, client) => client.dissolveEntity(args.entity_id as string, args),
  transfer_shares: async (args, client) => client.transferShares(args),
  calculate_distribution: async (args, client) => client.calculateDistribution(args),
  classify_contractor: async (args, client) => client.classifyContractor(args),
  reconcile_ledger: async (args, client) => client.reconcileLedger(args),
  track_deadline: async (args, client) => client.trackDeadline(args),
  convene_meeting: async (args, client) => client.conveneMeeting(args),

  create_agent: async (args, client) => client.createAgent(args),
  send_agent_message: async (args, client) => client.sendAgentMessage(args.agent_id as string, args.body as string),
  update_agent: async (args, client) => client.updateAgent(args.agent_id as string, args),
  add_agent_skill: async (args, client) => client.addAgentSkill(args.agent_id as string, args),
};

function def(
  name: string,
  description: string,
  properties: Record<string, unknown>,
  required?: string[],
): Record<string, unknown> {
  return {
    type: "function",
    function: {
      name,
      description,
      parameters: { type: "object", properties, required: required ?? [] },
    },
  };
}

export const TOOL_DEFINITIONS: Record<string, unknown>[] = [
  // Read tools
  def("get_workspace_status", "Get workspace status summary", {}),
  def("list_entities", "List all entities in the workspace", {}),
  def("get_cap_table", "Get cap table for an entity", { entity_id: { type: "string" } }, ["entity_id"]),
  def("list_documents", "List documents for an entity", { entity_id: { type: "string" } }, ["entity_id"]),
  def("list_safe_notes", "List SAFE notes for an entity", { entity_id: { type: "string" } }, ["entity_id"]),
  def("list_agents", "List all agents in the workspace", {}),
  def("get_checklist", "Get the user's onboarding checklist", {}),
  def("get_document_link", "Get a download link for a document", { document_id: { type: "string" } }, ["document_id"]),
  def("get_signing_link", "Get a signing link for a document", { document_id: { type: "string" } }, ["document_id"]),
  def("list_obligations", "List obligations with urgency tiers", {
    tier: { type: "string", description: "Filter by urgency tier" },
  }),
  def("get_billing_status", "Get billing status and plans", {}),

  // Write tools
  def("form_entity", "Form a new business entity", {
    entity_type: { type: "string" }, entity_name: { type: "string" },
    jurisdiction: { type: "string" },
    members: { type: "array", items: { type: "object", properties: {
      name: { type: "string" }, email: { type: "string" }, role: { type: "string" },
      investor_type: { type: "string" }, ownership_pct: { type: "number" }, share_count: { type: "integer" },
    }, required: ["name", "email", "role"] } },
  }, ["entity_type", "entity_name", "jurisdiction", "members"]),

  def("issue_equity", "Issue an equity grant", {
    entity_id: { type: "string" }, grant_type: { type: "string" }, shares: { type: "integer" },
    recipient_name: { type: "string" },
  }, ["entity_id", "grant_type", "shares", "recipient_name"]),

  def("issue_safe", "Issue a SAFE note", {
    entity_id: { type: "string" }, investor_name: { type: "string" },
    principal_amount_cents: { type: "integer" }, safe_type: { type: "string" }, valuation_cap_cents: { type: "integer" },
  }, ["entity_id", "investor_name", "principal_amount_cents", "safe_type", "valuation_cap_cents"]),

  def("transfer_shares", "Transfer shares between holders", {
    entity_id: { type: "string" }, from_holder: { type: "string" }, to_holder: { type: "string" },
    shares: { type: "integer" }, transfer_type: { type: "string" },
  }, ["entity_id", "from_holder", "to_holder", "shares"]),

  def("calculate_distribution", "Calculate a distribution", {
    entity_id: { type: "string" }, total_amount_cents: { type: "integer" }, distribution_type: { type: "string" },
  }, ["entity_id", "total_amount_cents"]),

  def("create_invoice", "Create an invoice", {
    entity_id: { type: "string" }, customer_name: { type: "string" }, amount_cents: { type: "integer" },
    due_date: { type: "string" }, description: { type: "string" },
  }, ["entity_id", "customer_name", "amount_cents", "due_date"]),

  def("run_payroll", "Run payroll", {
    entity_id: { type: "string" }, pay_period_start: { type: "string" }, pay_period_end: { type: "string" },
  }, ["entity_id", "pay_period_start", "pay_period_end"]),

  def("submit_payment", "Submit a payment", {
    entity_id: { type: "string" }, amount_cents: { type: "integer" }, recipient: { type: "string" },
  }, ["entity_id", "amount_cents", "recipient"]),

  def("open_bank_account", "Open a business bank account", {
    entity_id: { type: "string" }, institution_name: { type: "string" },
  }, ["entity_id"]),

  def("generate_contract", "Generate a contract from a template", {
    entity_id: { type: "string" }, template_type: { type: "string" }, parameters: { type: "object" },
  }, ["entity_id", "template_type"]),

  def("file_tax_document", "File a tax document", {
    entity_id: { type: "string" }, document_type: { type: "string" }, tax_year: { type: "integer" },
  }, ["entity_id", "document_type", "tax_year"]),

  def("track_deadline", "Track a compliance deadline", {
    entity_id: { type: "string" }, deadline_type: { type: "string" }, due_date: { type: "string" }, description: { type: "string" },
  }, ["entity_id", "deadline_type", "due_date", "description"]),

  def("classify_contractor", "Classify contractor risk", {
    entity_id: { type: "string" }, contractor_name: { type: "string" }, state: { type: "string" },
    hours_per_week: { type: "integer" }, exclusive_client: { type: "boolean" },
    duration_months: { type: "integer" }, provides_tools: { type: "boolean" },
  }, ["entity_id", "contractor_name", "state", "hours_per_week"]),

  def("reconcile_ledger", "Reconcile an entity's ledger", {
    entity_id: { type: "string" }, start_date: { type: "string" }, end_date: { type: "string" },
  }, ["entity_id", "start_date", "end_date"]),

  def("convene_meeting", "Convene a governance meeting", {
    entity_id: { type: "string" }, governance_body_id: { type: "string" }, meeting_type: { type: "string" },
    title: { type: "string" }, scheduled_date: { type: "string" },
  }, ["entity_id", "governance_body_id", "meeting_type", "title", "scheduled_date"]),

  def("cast_vote", "Cast a vote on an agenda item", {
    meeting_id: { type: "string" }, agenda_item_id: { type: "string" }, voter_id: { type: "string" },
    vote: { type: "string", description: "for, against, abstain, or recusal" },
  }, ["meeting_id", "agenda_item_id", "voter_id", "vote"]),

  def("schedule_meeting", "Schedule a board or member meeting", {
    body_id: { type: "string" }, meeting_type: { type: "string" }, title: { type: "string" },
    proposed_date: { type: "string" },
    agenda_items: { type: "array", items: { type: "string" } },
  }, ["body_id", "meeting_type", "title", "proposed_date"]),

  def("get_signer_link", "Generate a signing link for a human obligation", {
    obligation_id: { type: "string" },
  }, ["obligation_id"]),

  def("update_checklist", "Update the user's onboarding checklist", {
    checklist: { type: "string" },
  }, ["checklist"]),

  def("convert_entity", "Convert entity type", {
    entity_id: { type: "string" }, new_entity_type: { type: "string" },
  }, ["entity_id", "new_entity_type"]),

  def("dissolve_entity", "Dissolve an entity", {
    entity_id: { type: "string" }, reason: { type: "string" },
  }, ["entity_id", "reason"]),

  def("create_agent", "Create a new agent", {
    name: { type: "string" }, system_prompt: { type: "string" }, model: { type: "string" },
  }, ["name", "system_prompt"]),

  def("send_agent_message", "Send a message to an agent", {
    agent_id: { type: "string" }, body: { type: "string" },
  }, ["agent_id", "body"]),

  def("update_agent", "Update an agent", {
    agent_id: { type: "string" }, status: { type: "string" },
  }, ["agent_id"]),

  def("add_agent_skill", "Add a skill to an agent", {
    agent_id: { type: "string" }, skill_name: { type: "string" }, description: { type: "string" },
  }, ["agent_id", "skill_name", "description"]),
];

const READ_ONLY_TOOLS = new Set([
  "get_workspace_status", "list_entities", "get_cap_table", "list_documents",
  "list_safe_notes", "list_agents", "get_checklist", "get_document_link",
  "get_signing_link", "list_obligations", "get_billing_status",
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
