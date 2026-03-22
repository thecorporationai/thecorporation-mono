import type { CorpAPIClient } from "./api-client.js";
import { readFileSync, writeFileSync, mkdirSync, existsSync } from "node:fs";
import { join } from "node:path";
import { GENERATED_TOOL_DEFINITIONS } from "./tool-defs.generated.js";
import type { CapTableInstrument } from "./types.js";
import {
  resolveInstrumentForGrant,
  ensureIssuancePreflight,
} from "./workflows/equity-helpers.js";
export type { CapTableInstrument } from "./types.js";

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

// ---------------------------------------------------------------------------
// Sub-handlers grouped by consolidated tool
// ---------------------------------------------------------------------------

const workspaceActions: Record<string, ToolHandler> = {
  status: async (_args, client) => client.getStatus(),
  list_entities: async (_args, client) => client.listEntities(),
  obligations: async (args, client) => client.getObligations(args.tier as string | undefined),
  billing: async (_args, client) => {
    const [status, plans] = await Promise.all([client.getBillingStatus(), client.getBillingPlans()]);
    return { status, plans };
  },
  checkout: async (args, client) => {
    const planId = requiredString(args, "plan_id");
    return client.createBillingCheckout(planId);
  },
  portal: async (_args, client) => {
    return client.createBillingPortal();
  },
};

const entityActions: Record<string, ToolHandler> = {
  get_cap_table: async (args, client) => client.getCapTable(requiredString(args, "entity_id")),
  list_documents: async (args, client) => client.getEntityDocuments(requiredString(args, "entity_id")),
  list_safe_notes: async (args, client) => client.getSafeNotes(requiredString(args, "entity_id")),

  create: async (args, client) => {
    const entityType = args.entity_type as string;
    let jurisdiction = (args.jurisdiction as string) || "";
    if (!jurisdiction || jurisdiction.length === 2) {
      jurisdiction = entityType === "llc" ? "US-WY" : "US-DE";
    }
    return client.createPendingEntity({
      entity_type: entityType,
      legal_name: args.entity_name,
      jurisdiction,
      registered_agent_name: args.registered_agent_name,
      registered_agent_address: args.registered_agent_address,
      formation_date: args.formation_date,
      fiscal_year_end: args.fiscal_year_end,
      s_corp_election: args.s_corp_election,
      transfer_restrictions: args.transfer_restrictions,
      right_of_first_refusal: args.right_of_first_refusal,
      company_address: args.company_address,
    });
  },

  add_founder: async (args, client) => {
    const entityId = requiredString(args, "entity_id");
    return client.addFounder(entityId, {
      name: args.name,
      email: args.email,
      role: args.role,
      ownership_pct: args.ownership_pct,
      officer_title: args.officer_title,
      is_incorporator: args.is_incorporator,
      address: args.address,
    });
  },

  finalize: async (args, client, ctx) => {
    const entityId = requiredString(args, "entity_id");
    const result = await client.finalizeFormation(entityId, {
      authorized_shares: args.authorized_shares,
      par_value: args.par_value,
      registered_agent_name: args.registered_agent_name,
      registered_agent_address: args.registered_agent_address,
      formation_date: args.formation_date,
      fiscal_year_end: args.fiscal_year_end,
      s_corp_election: args.s_corp_election,
      transfer_restrictions: args.transfer_restrictions,
      right_of_first_refusal: args.right_of_first_refusal,
      company_address: args.company_address,
      incorporator_name: args.incorporator_name,
      incorporator_address: args.incorporator_address,
    });
    if (entityId && ctx.onEntityFormed) {
      ctx.onEntityFormed(entityId);
    }
    return result;
  },

  form: async (args, client, ctx) => {
    const entityType = args.entity_type as string;
    let jurisdiction = (args.jurisdiction as string) || "";
    if (!jurisdiction || jurisdiction.length === 2) {
      jurisdiction = entityType === "llc" ? "US-WY" : "US-DE";
    }
    const members = (args.members ?? []) as Record<string, unknown>[];
    if (!members.length) return { error: "Members are required." };
    for (const m of members) {
      if (!m.investor_type) m.investor_type = "natural_person";
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

  convert: async (args, client) => {
    const body: Record<string, unknown> = { target_type: args.target_type ?? args.to_type ?? args.new_entity_type };
    if (args.new_jurisdiction) body.new_jurisdiction = args.new_jurisdiction;
    return client.convertEntity(requiredString(args, "entity_id"), body);
  },
  dissolve: async (args, client) => client.dissolveEntity(requiredString(args, "entity_id"), args),
};

export async function ensureSafeInstrument(
  client: CorpAPIClient,
  entityId: string,
): Promise<CapTableInstrument> {
  const capTable = await client.getCapTable(entityId);
  const issuerLegalEntityId = capTable.issuer_legal_entity_id as string;
  if (!issuerLegalEntityId) {
    throw new Error("No issuer legal entity found.");
  }

  const instruments = (capTable.instruments ?? []) as CapTableInstrument[];
  const safeInstrument = instruments.find((instrument) => {
    const status = String(instrument.status ?? "active").toLowerCase();
    return instrument.kind.toLowerCase() === "safe" && status === "active";
  });
  if (safeInstrument) {
    return safeInstrument;
  }

  const created = await client.createInstrument({
    entity_id: entityId,
    issuer_legal_entity_id: issuerLegalEntityId,
    symbol: "SAFE",
    kind: "safe",
    terms: {},
  });
  return {
    instrument_id: String(created.instrument_id ?? ""),
    kind: String(created.kind ?? "safe"),
    symbol: String(created.symbol ?? "SAFE"),
    status: String(created.status ?? "active"),
  };
}

const equityActions: Record<string, ToolHandler> = {
  start_round: async (args, client) => client.startEquityRound(args),

  add_security: async (args, client) => {
    const roundId = requiredString(args, "round_id");
    return client.addRoundSecurity(roundId, args);
  },

  issue_round: async (args, client) => {
    const roundId = requiredString(args, "round_id");
    return client.issueRound(roundId, args);
  },

  issue: async (args, client) => {
    const entityId = requiredString(args, "entity_id");
    // Fetch cap table to auto-resolve issuer and instrument
    const capTable = await client.getCapTable(entityId);
    const issuerLegalEntityId = capTable.issuer_legal_entity_id as string;
    if (!issuerLegalEntityId) return { error: "No issuer legal entity found. Has this entity been formed with a cap table?" };

    const instruments = (capTable.instruments ?? []) as CapTableInstrument[];
    if (!instruments?.length) return { error: "No instruments found on cap table." };

    const grantType = String(args.grant_type ?? "");
    const instrument = resolveInstrumentForGrant(instruments, grantType, args.instrument_id as string | undefined);
    await ensureIssuancePreflight(
      client,
      entityId,
      grantType,
      instrument,
      args.meeting_id as string | undefined,
      args.resolution_id as string | undefined,
    );

    const round = await client.startEquityRound({
      entity_id: entityId,
      name: `${args.grant_type ?? "equity"} grant — ${args.recipient_name ?? "unknown"}`,
      issuer_legal_entity_id: issuerLegalEntityId,
    });
    const roundId = (round.round_id ?? round.equity_round_id) as string;

    const securityData: Record<string, unknown> = {
      entity_id: entityId,
      instrument_id: instrument.instrument_id,
      quantity: args.shares ?? args.quantity,
      recipient_name: args.recipient_name,
      grant_type: args.grant_type,
    };
    if (args.email) securityData.email = args.email;
    await client.addRoundSecurity(roundId, securityData);

    const issueBody: Record<string, unknown> = { entity_id: entityId };
    if (args.meeting_id) issueBody.meeting_id = args.meeting_id;
    if (args.resolution_id) issueBody.resolution_id = args.resolution_id;
    return client.issueRound(roundId, issueBody);
  },
  issue_safe: async (args, client) => {
    const entityId = requiredString(args, "entity_id");
    await ensureIssuancePreflight(
      client,
      entityId,
      String(args.safe_type ?? "post_money"),
      undefined,
      args.meeting_id as string | undefined,
      args.resolution_id as string | undefined,
    );

    const principalCents = (args.principal_amount_cents ?? args.amount_cents ?? 0) as number;
    const body: Record<string, unknown> = {
      entity_id: entityId,
      investor_name: args.investor_name,
      principal_amount_cents: principalCents,
      valuation_cap_cents: args.valuation_cap_cents,
      safe_type: args.safe_type ?? "post_money",
    };
    if (args.email) body.email = args.email;
    if (args.meeting_id) body.meeting_id = args.meeting_id;
    if (args.resolution_id) body.resolution_id = args.resolution_id;
    return client.createSafeNote(body);
  },

  transfer: async (args, client) => {
    if (args.skip_governance_review !== true) {
      return {
        error: "Transfer blocked: governance review required. Use the transfer workflow (create_transfer_workflow → submit-review → record-board-approval → execute) for governed transfers. To bypass governance and record a direct transfer, pass skip_governance_review: true.",
      };
    }
    return client.transferShares(args);
  },

  distribution: async (args, client) => client.calculateDistribution(args),
};

const valuationActions: Record<string, ToolHandler> = {
  create: async (args, client) => client.createValuation(args),

  submit: async (args, client) =>
    client.submitValuationForApproval(
      requiredString(args, "valuation_id"),
      requiredString(args, "entity_id"),
    ),

  approve: async (args, client) =>
    client.approveValuation(
      requiredString(args, "valuation_id"),
      requiredString(args, "entity_id"),
      args.resolution_id as string | undefined,
    ),
};

const meetingActions: Record<string, ToolHandler> = {
  schedule: async (args, client) => {
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

  notice: async (args, client) => client.sendNotice(
    requiredString(args, "meeting_id"),
    requiredString(args, "entity_id"),
  ),

  convene: async (args, client) => client.conveneMeeting(
    requiredString(args, "meeting_id"),
    requiredString(args, "entity_id"),
    {
      present_seat_ids: Array.isArray(args.present_seat_ids) ? args.present_seat_ids : [],
    },
  ),

  vote: async (args, client) =>
    client.castVote(
      requiredString(args, "entity_id"),
      requiredString(args, "meeting_id"),
      requiredString(args, "agenda_item_id"),
      {
        voter_id: requiredString(args, "voter_id"),
        vote_value: requiredString(args, "vote_value"),
      },
    ),

  resolve: async (args, client) => {
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

  finalize_item: async (args, client) => client.finalizeAgendaItem(
    requiredString(args, "meeting_id"),
    requiredString(args, "agenda_item_id"),
    {
      entity_id: requiredString(args, "entity_id"),
      status: requiredString(args, "status"),
    },
  ),

  adjourn: async (args, client) => client.adjournMeeting(
    requiredString(args, "meeting_id"),
    requiredString(args, "entity_id"),
  ),

  cancel: async (args, client) => client.cancelMeeting(
    requiredString(args, "meeting_id"),
    requiredString(args, "entity_id"),
  ),

  consent: async (args, client) => client.writtenConsent({
    entity_id: requiredString(args, "entity_id"),
    body_id: requiredString(args, "body_id"),
    title: requiredString(args, "title"),
    description: args.description as string ?? "",
  }),

  attach_document: async (args, client) => client.attachResolutionDocument(
    requiredString(args, "meeting_id"),
    requiredString(args, "resolution_id"),
    {
      entity_id: requiredString(args, "entity_id"),
      document_id: requiredString(args, "document_id"),
    },
  ),

  list_items: async (args, client) => client.listAgendaItems(
    requiredString(args, "meeting_id"),
    requiredString(args, "entity_id"),
  ),

  list_votes: async (args, client) => client.listVotes(
    requiredString(args, "meeting_id"),
    requiredString(args, "agenda_item_id"),
    requiredString(args, "entity_id"),
  ),
};

const financeActions: Record<string, ToolHandler> = {
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
    body.bank_name = args.bank_name ?? args.institution_name ?? "Mercury";
    return client.openBankAccount(body);
  },

  reconcile: async (args, client) => client.reconcileLedger(args),
};

const complianceActions: Record<string, ToolHandler> = {
  file_tax: async (args, client) => client.fileTaxDocument(args),
  track_deadline: async (args, client) => client.trackDeadline(args),
  classify_contractor: async (args, client) => client.classifyContractor(args),
  generate_contract: async (args, client) => {
    const data: Record<string, unknown> = {
      entity_id: requiredString(args, "entity_id"),
      template_type: requiredString(args, "template_type"),
      counterparty_name: args.counterparty_name ?? args.counterparty ?? "",
      effective_date: args.effective_date ?? new Date().toISOString().slice(0, 10),
    };
    if (args.parameters) data.parameters = args.parameters;
    return client.generateContract(data);
  },
};

const documentActions: Record<string, ToolHandler> = {
  signing_link: async (args, client) => client.getSigningLink(args.document_id as string, requiredString(args, "entity_id")),

  signer_link: async (args, client) => {
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

  download_link: async (args, client) => {
    const docId = args.document_id as string;
    return {
      document_id: docId,
      download_url: `${client.apiUrl}/v1/documents/${docId}/pdf`,
      note: "Use your API key to authenticate the download.",
    };
  },

  preview_pdf: async (args, client) => {
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
};

const checklistActions: Record<string, ToolHandler> = {
  get: async (_args, _client, ctx) => {
    const path = join(ctx.dataDir, "checklist.md");
    if (existsSync(path)) return { checklist: readFileSync(path, "utf-8") };
    return { checklist: null, message: "No checklist yet." };
  },

  update: async (args, _client, ctx) => {
    const path = join(ctx.dataDir, "checklist.md");
    mkdirSync(ctx.dataDir, { recursive: true });
    writeFileSync(path, args.checklist as string);
    return { status: "updated", checklist: args.checklist };
  },
};

const workItemActions: Record<string, ToolHandler> = {
  list: async (args, client) =>
    client.listWorkItems(requiredString(args, "entity_id"), args.status ? { status: args.status as string } : undefined),

  get: async (args, client) =>
    client.getWorkItem(requiredString(args, "entity_id"), requiredString(args, "work_item_id")),

  create: async (args, client) => {
    const data: Record<string, unknown> = {
      title: requiredString(args, "title"),
      category: requiredString(args, "category"),
    };
    if (args.description) data.description = args.description;
    if (args.deadline) data.deadline = args.deadline;
    if (args.asap != null) data.asap = args.asap;
    if (args.metadata) data.metadata = args.metadata;
    if (args.created_by) data.created_by = args.created_by;
    return client.createWorkItem(requiredString(args, "entity_id"), data);
  },

  claim: async (args, client) => {
    const data: Record<string, unknown> = {
      claimed_by: requiredString(args, "claimed_by"),
    };
    if (args.ttl_seconds != null) data.ttl_seconds = args.ttl_seconds;
    return client.claimWorkItem(
      requiredString(args, "entity_id"),
      requiredString(args, "work_item_id"),
      data,
    );
  },

  complete: async (args, client) => {
    const data: Record<string, unknown> = {
      completed_by: requiredString(args, "completed_by"),
    };
    if (args.result) data.result = args.result;
    return client.completeWorkItem(
      requiredString(args, "entity_id"),
      requiredString(args, "work_item_id"),
      data,
    );
  },

  release: async (args, client) =>
    client.releaseWorkItem(requiredString(args, "entity_id"), requiredString(args, "work_item_id")),

  cancel: async (args, client) =>
    client.cancelWorkItem(requiredString(args, "entity_id"), requiredString(args, "work_item_id")),
};

const agentActions: Record<string, ToolHandler> = {
  list: async (_args, client) => client.listAgents(),
  create: async (args, client) => client.createAgent(args),
  message: async (args, client) => client.sendAgentMessage(args.agent_id as string, (args.message ?? args.body) as string),
  update: async (args, client) => client.updateAgent(args.agent_id as string, args),
  add_skill: async (args, client) => client.addAgentSkill(args.agent_id as string, args),
};

// ---------------------------------------------------------------------------
// Dispatch table: consolidated tool name → action sub-handlers
// ---------------------------------------------------------------------------

const TOOL_DISPATCH: Record<string, Record<string, ToolHandler>> = {
  workspace: workspaceActions,
  entity: entityActions,
  equity: equityActions,
  valuation: valuationActions,
  meeting: meetingActions,
  finance: financeActions,
  compliance: complianceActions,
  document: documentActions,
  checklist: checklistActions,
  work_item: workItemActions,
  agent: agentActions,
};

// Tool definitions are generated from the backend OpenAPI spec.
// Regenerate: make generate-tools
export const TOOL_DEFINITIONS: Record<string, unknown>[] = GENERATED_TOOL_DEFINITIONS;

export const TOOL_DISPATCH_COUNT = Object.keys(TOOL_DISPATCH).length;

// Actions that are read-only (no user confirmation needed)
const READ_ONLY_ACTIONS = new Set([
  "workspace:status",
  "workspace:list_entities",
  "workspace:obligations",
  "workspace:billing",
  "entity:get_cap_table",
  "entity:list_documents",
  "entity:list_safe_notes",
  "document:signing_link",
  "document:signer_link",
  "document:download_link",
  "document:preview_pdf",
  "checklist:get",
  "meeting:list_items",
  "meeting:list_votes",
  "work_item:list",
  "work_item:get",
  "agent:list",
]);

export function isWriteTool(name: string, args?: Record<string, unknown>): boolean {
  if (args && typeof args.action === "string") {
    return !READ_ONLY_ACTIONS.has(`${name}:${args.action}`);
  }
  // Fallback: if no action provided, assume write
  return true;
}

export async function executeTool(
  name: string,
  args: Record<string, unknown>,
  client: CorpAPIClient,
  ctx: ToolContext,
): Promise<string> {
  // Handle non-dispatch tools
  if (name === "feedback") {
    try {
      const result = await client.submitFeedback(
        args.message as string,
        args.category as string | undefined,
        args.email as string | undefined,
      );
      return JSON.stringify(result, null, 0);
    } catch (err) {
      return JSON.stringify({ error: String(err) });
    }
  }

  const dispatch = TOOL_DISPATCH[name];
  if (!dispatch) return JSON.stringify({ error: `Unknown tool: ${name}` });

  const action = args.action as string;
  if (!action) return JSON.stringify({ error: `Missing required field: action` });

  const handler = dispatch[action];
  if (!handler) return JSON.stringify({ error: `Unknown action "${action}" for tool "${name}"` });

  try {
    const result = await handler(args, client, ctx);
    return JSON.stringify(result, null, 0);
  } catch (err) {
    return JSON.stringify({ error: String(err) });
  }
}
