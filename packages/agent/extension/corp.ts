/**
 * TheCorporation — Pi Extension
 *
 * Registers 30 corporate operations tools, slash commands, and write-tool
 * confirmation gating for the pi coding agent.
 */

import { Type, type Static } from "@sinclair/typebox";
import { readFileSync, existsSync, writeFileSync, mkdirSync } from "fs";
import { join } from "path";
import { homedir } from "os";
import { Text } from "@mariozechner/pi-tui";
import type { ExtensionAPI } from "@mariozechner/pi-coding-agent";

// ── Config & API Client ─────────────────────────────────────────

interface CorpConfig {
  api_url: string;
  api_key: string;
  workspace_id: string;
  active_entity_id?: string;
  user?: { name?: string; email?: string };
}

function loadConfig(): CorpConfig {
  const configPath = join(process.env.CORP_CONFIG_DIR || join(homedir(), ".corp"), "config.json");
  const defaults: CorpConfig = {
    api_url: process.env.CORP_API_URL || "https://api.thecorporation.ai",
    api_key: "",
    workspace_id: "",
  };
  if (!existsSync(configPath)) return defaults;
  try {
    const saved = JSON.parse(readFileSync(configPath, "utf-8"));
    return { ...defaults, ...saved };
  } catch {
    return defaults;
  }
}

async function apiCall(method: string, path: string, body?: unknown): Promise<any> {
  const cfg = loadConfig();
  const url = cfg.api_url.replace(/\/+$/, "") + path;
  const headers: Record<string, string> = {
    Authorization: `Bearer ${cfg.api_key}`,
    "Content-Type": "application/json",
    Accept: "application/json",
  };
  const opts: RequestInit = { method, headers, signal: AbortSignal.timeout(30_000) };
  if (body !== undefined) opts.body = JSON.stringify(body);
  const resp = await fetch(url, opts);
  if (!resp.ok) {
    const text = await resp.text().catch(() => "");
    throw new Error(`API ${resp.status}: ${text || resp.statusText}`);
  }
  return resp.json();
}

function wsId(): string {
  return loadConfig().workspace_id;
}

// ── Helpers ──────────────────────────────────────────────────────

function centsToUsd(cents: number): string {
  return "$" + (cents / 100).toLocaleString("en-US", { minimumFractionDigits: 0, maximumFractionDigits: 0 });
}

/** Tools that are auto-approved (no confirmation needed). */
const READ_ONLY_TOOLS = new Set([
  "get_workspace_status",
  "list_entities",
  "get_cap_table",
  "list_documents",
  "list_safe_notes",
  "list_agents",
  "get_checklist",
  "get_signing_link",
  "list_obligations",
  "get_billing_status",
]);

/** Human-readable description of a write tool call for the confirmation dialog. */
function describeToolCall(name: string, args: Record<string, any>): string {
  const a = { ...args };
  // Convert cents to dollars for display
  for (const k of ["amount_cents", "principal_amount_cents", "total_amount_cents", "valuation_cap_cents"]) {
    if (k in a) {
      try { a._amount = centsToUsd(Number(a[k])); } catch { a._amount = String(a[k]); }
    }
  }
  a._amount ??= "?";
  a.institution_name ??= "Mercury";
  a.payment_method ??= "ach";

  const fmts: Record<string, string> = {
    form_entity: 'Form a new {entity_type} named "{entity_name}" in {jurisdiction}',
    convert_entity: "Convert entity to {new_entity_type}",
    dissolve_entity: "Dissolve entity — {dissolution_reason}",
    issue_equity: "Issue {shares} {grant_type} shares to {recipient_name}",
    transfer_shares: "Transfer {shares} shares to {to_recipient_name}",
    issue_safe: "Issue SAFE note to {investor_name} for {_amount}",
    calculate_distribution: "Calculate {distribution_type} distribution of {_amount}",
    create_invoice: "Create invoice for {customer_name} — {_amount}",
    run_payroll: "Run payroll for {pay_period_start} to {pay_period_end}",
    submit_payment: "Submit {_amount} payment to {recipient} via {payment_method}",
    open_bank_account: "Open bank account at {institution_name}",
    reconcile_ledger: "Reconcile ledger from {start_date} to {end_date}",
    generate_contract: "Generate {template_type} contract for {counterparty_name}",
    file_tax_document: "File {document_type} for tax year {tax_year}",
    track_deadline: "Track {deadline_type} deadline — {description}",
    classify_contractor: "Classify contractor {contractor_name} in {state}",
    convene_meeting: "Convene {meeting_type} meeting",
    cast_vote: "Cast {vote} vote",
    schedule_meeting: "Schedule {meeting_type} meeting: {title}",
    update_checklist: "Update workspace checklist",
    create_agent: 'Create agent "{name}"',
    send_agent_message: "Send message to agent",
    update_agent: "Update agent configuration",
    add_agent_skill: 'Add skill "{skill_name}" to agent',
  };

  const fmt = fmts[name];
  if (fmt) {
    try {
      return fmt.replace(/\{(\w+)\}/g, (_, k) => String(a[k] ?? "?"));
    } catch { /* fall through */ }
  }
  return name.replace(/_/g, " ");
}

// ── Tool Definitions ────────────────────────────────────────────

// Shared render helpers
function renderCallText(label: string, lines: string[], theme: any): Text {
  let text = theme.fg("toolTitle", theme.bold(label + " "));
  text += lines.map((l) => theme.fg("muted", l)).join(theme.fg("dim", " | "));
  return new Text(text, 0, 0);
}

function renderResultOk(headline: string, details: string[], theme: any): Text {
  let text = theme.fg("success", "✓ " + headline);
  for (const d of details) text += "\n  " + theme.fg("dim", d);
  return new Text(text, 0, 0);
}

function renderResultErr(result: any, theme: any): Text | null {
  const content = result?.content?.[0]?.text ?? result?.details?.error;
  if (!content) return null;
  try {
    const d = typeof content === "string" ? JSON.parse(content) : content;
    if (d?.error) return new Text(theme.fg("error", "✗ " + d.error), 0, 0);
  } catch { /* not JSON */ }
  return null;
}

function parseResult(result: any): any {
  try {
    const raw = result?.content?.[0]?.text ?? result?.details;
    return typeof raw === "string" ? JSON.parse(raw) : raw ?? {};
  } catch { return {}; }
}

// -- Read tools --

const getWorkspaceStatus = {
  name: "get_workspace_status",
  label: "Get Workspace Status",
  description: "Get a summary of the current workspace: counts of entities, documents, grants, etc.",
  parameters: Type.Object({}),
  async execute(toolCallId: string, params: any, signal: any) {
    const data = await apiCall("GET", `/v1/workspaces/${wsId()}/status`);
    return { content: [{ type: "text" as const, text: JSON.stringify(data) }], details: data };
  },
  renderCall(args: any, theme: any) {
    return renderCallText("workspace_status", ["Fetching summary"], theme);
  },
  renderResult(result: any, opts: any, theme: any) {
    const err = renderResultErr(result, theme);
    if (err) return err;
    const d = parseResult(result);
    return renderResultOk("Workspace Status", [
      `Entities: ${d.entity_count ?? d.entities ?? "?"}`,
      `Documents: ${d.document_count ?? d.documents ?? "?"}`,
      `Grants: ${d.grant_count ?? d.grants ?? "?"}`,
    ], theme);
  },
};

const listEntities = {
  name: "list_entities",
  label: "List Entities",
  description: "List all entities (companies) in the workspace. Optionally filter by type.",
  parameters: Type.Object({
    entity_type: Type.Optional(Type.String({ description: "Filter by type: 'llc', 'corporation', or empty for all" })),
  }),
  async execute(toolCallId: string, params: any, signal: any) {
    const data = await apiCall("GET", `/v1/workspaces/${wsId()}/entities`);
    return { content: [{ type: "text" as const, text: JSON.stringify(data) }], details: data };
  },
  renderCall(args: any, theme: any) {
    const filter = args.entity_type ? `type=${args.entity_type}` : "all";
    return renderCallText("list_entities", [filter], theme);
  },
  renderResult(result: any, opts: any, theme: any) {
    const err = renderResultErr(result, theme);
    if (err) return err;
    const d = parseResult(result);
    const entities = Array.isArray(d) ? d : d.entities ?? [];
    const lines = entities.slice(0, 10).map((e: any) =>
      `${e.legal_name || e.entity_name || e.name} (${e.entity_type}) — ${e.jurisdiction ?? ""} [${e.status ?? "active"}]`
    );
    if (entities.length > 10) lines.push(`... and ${entities.length - 10} more`);
    return renderResultOk(`${entities.length} entities`, lines, theme);
  },
};

const getCapTable = {
  name: "get_cap_table",
  label: "Get Cap Table",
  description: "Get the full cap table for an entity including grants and SAFE notes.",
  parameters: Type.Object({
    entity_id: Type.String(),
  }),
  async execute(toolCallId: string, params: any, signal: any) {
    const data = await apiCall("GET", `/v1/entities/${params.entity_id}/cap-table`);
    return { content: [{ type: "text" as const, text: JSON.stringify(data) }], details: data };
  },
  renderCall(args: any, theme: any) {
    return renderCallText("get_cap_table", [`entity=${args.entity_id}`], theme);
  },
  renderResult(result: any, opts: any, theme: any) {
    const err = renderResultErr(result, theme);
    if (err) return err;
    const d = parseResult(result);
    const grants = d.grants ?? d.rows ?? [];
    const lines = grants.slice(0, 10).map((g: any) =>
      `${g.holder_name ?? g.recipient_name ?? "?"}: ${g.shares ?? "?"} ${g.grant_type ?? ""} shares (${g.percentage != null ? (g.percentage * 100).toFixed(1) + "%" : "?"})`
    );
    return renderResultOk("Cap Table", lines, theme);
  },
};

const listDocuments = {
  name: "list_documents",
  label: "List Documents",
  description: "List all documents for an entity.",
  parameters: Type.Object({
    entity_id: Type.String(),
  }),
  async execute(toolCallId: string, params: any, signal: any) {
    const data = await apiCall("GET", `/v1/formations/${params.entity_id}/documents`);
    return { content: [{ type: "text" as const, text: JSON.stringify(data) }], details: data };
  },
  renderCall(args: any, theme: any) {
    return renderCallText("list_documents", [`entity=${args.entity_id}`], theme);
  },
  renderResult(result: any, opts: any, theme: any) {
    const err = renderResultErr(result, theme);
    if (err) return err;
    const d = parseResult(result);
    const docs = Array.isArray(d) ? d : d.documents ?? [];
    const statusBadge = (s: string) => {
      if (s === "signed") return "✓signed";
      if (s === "pending" || s === "pending_signature") return "⏳pending";
      return s || "draft";
    };
    const lines = docs.slice(0, 10).map((doc: any) =>
      `${doc.title ?? doc.document_type ?? "doc"} [${statusBadge(doc.status)}]`
    );
    return renderResultOk(`${docs.length} documents`, lines, theme);
  },
};

const listSafeNotes = {
  name: "list_safe_notes",
  label: "List SAFE Notes",
  description: "List SAFE notes for an entity.",
  parameters: Type.Object({
    entity_id: Type.String(),
  }),
  async execute(toolCallId: string, params: any, signal: any) {
    const data = await apiCall("GET", `/v1/entities/${params.entity_id}/safe-notes`);
    return { content: [{ type: "text" as const, text: JSON.stringify(data) }], details: data };
  },
  renderCall(args: any, theme: any) {
    return renderCallText("list_safe_notes", [`entity=${args.entity_id}`], theme);
  },
  renderResult(result: any, opts: any, theme: any) {
    const err = renderResultErr(result, theme);
    if (err) return err;
    const d = parseResult(result);
    const notes = Array.isArray(d) ? d : d.safe_notes ?? [];
    const lines = notes.map((n: any) =>
      `${n.investor_name ?? "?"}: ${centsToUsd(n.principal_amount_cents ?? 0)} (cap ${centsToUsd(n.valuation_cap_cents ?? 0)}) ${n.safe_type ?? ""}`
    );
    return renderResultOk(`${notes.length} SAFE notes`, lines, theme);
  },
};

const listAgents = {
  name: "list_agents",
  label: "List Agents",
  description: "List all AI agents in the workspace.",
  parameters: Type.Object({}),
  async execute(toolCallId: string, params: any, signal: any) {
    const data = await apiCall("GET", "/v1/agents");
    return { content: [{ type: "text" as const, text: JSON.stringify(data) }], details: data };
  },
  renderCall(args: any, theme: any) {
    return renderCallText("list_agents", ["Fetching agents"], theme);
  },
  renderResult(result: any, opts: any, theme: any) {
    const err = renderResultErr(result, theme);
    if (err) return err;
    const d = parseResult(result);
    const agents = Array.isArray(d) ? d : d.agents ?? [];
    const lines = agents.map((a: any) => `${a.name} [${a.status ?? "active"}]`);
    return renderResultOk(`${agents.length} agents`, lines, theme);
  },
};

const getChecklist = {
  name: "get_checklist",
  label: "Get Checklist",
  description: "Get the current workspace checklist.",
  parameters: Type.Object({}),
  async execute(toolCallId: string, params: any, signal: any) {
    const checklistPath = join(process.env.CORP_CONFIG_DIR || join(homedir(), ".corp"), "checklist.md");
    if (existsSync(checklistPath)) {
      const checklist = readFileSync(checklistPath, "utf-8");
      return { content: [{ type: "text" as const, text: JSON.stringify({ checklist }) }], details: { checklist } };
    }
    return { content: [{ type: "text" as const, text: JSON.stringify({ checklist: null, message: "No checklist yet." }) }], details: {} };
  },
  renderCall(args: any, theme: any) {
    return renderCallText("get_checklist", ["Reading checklist"], theme);
  },
  renderResult(result: any, opts: any, theme: any) {
    const d = parseResult(result);
    if (!d.checklist) return new Text(theme.fg("muted", "No checklist yet"), 0, 0);
    return renderResultOk("Checklist", d.checklist.split("\n").slice(0, 15), theme);
  },
};

const getDocumentLink = {
  name: "get_document_link",
  label: "Get Document Link",
  description: "Get a temporary download link (PDF) for a signed document. The link expires in 24 hours.",
  parameters: Type.Object({
    document_id: Type.String({ description: "The document ID to generate a download link for" }),
  }),
  async execute(toolCallId: string, params: any, signal: any) {
    const cfg = loadConfig();
    try {
      const data = await apiCall("POST", `/v1/documents/${params.document_id}/request-copy`, { email: "owner@workspace" });
      let url = data.download_url ?? "";
      if (url.startsWith("/")) url = cfg.api_url.replace(/\/+$/, "") + url;
      const result = { document_id: params.document_id, download_url: url, expires_in: "24 hours" };
      return { content: [{ type: "text" as const, text: JSON.stringify(result) }], details: result };
    } catch {
      const result = { document_id: params.document_id, download_url: `${cfg.api_url.replace(/\/+$/, "")}/v1/documents/${params.document_id}/pdf`, note: "Use your API key to authenticate the download." };
      return { content: [{ type: "text" as const, text: JSON.stringify(result) }], details: result };
    }
  },
  renderCall(args: any, theme: any) {
    return renderCallText("get_document_link", [`doc=${args.document_id}`], theme);
  },
  renderResult(result: any, opts: any, theme: any) {
    const d = parseResult(result);
    return renderResultOk("Document Link", [d.download_url ?? "?"], theme);
  },
};

const getSigningLink = {
  name: "get_signing_link",
  label: "Get Signing Link",
  description: "Generate a signing link for a document. Returns a URL with a signing token the user must open to sign. Documents can ONLY be signed through this link.",
  parameters: Type.Object({
    document_id: Type.String({ description: "The document ID to generate a signing link for" }),
    entity_id: Type.String({ description: "The entity ID the document belongs to" }),
  }),
  async execute(toolCallId: string, params: any, signal: any) {
    const qs = new URLSearchParams({ entity_id: params.entity_id }).toString();
    const data = await apiCall("GET", `/v1/sign/${params.document_id}?${qs}`);
    const signingUrl = data.token
      ? `https://humans.thecorporation.ai/sign/${params.document_id}?token=${data.token}`
      : data.signing_url ?? `https://humans.thecorporation.ai/sign/${params.document_id}`;
    const result = {
      document_id: data.document_id ?? params.document_id,
      signing_url: signingUrl,
      token: data.token,
      message: "Open this link to sign the document. Only the authorized signer can complete the signature.",
    };
    return { content: [{ type: "text" as const, text: JSON.stringify(result) }], details: result };
  },
  renderCall(args: any, theme: any) {
    return renderCallText("get_signing_link", [`doc=${args.document_id}`], theme);
  },
  renderResult(result: any, opts: any, theme: any) {
    const d = parseResult(result);
    return renderResultOk("Signing Link", [d.signing_url ?? "?"], theme);
  },
};

const listObligations = {
  name: "list_obligations",
  label: "List Obligations",
  description: "List obligations with urgency tiers. Optionally filter by tier.",
  parameters: Type.Object({
    tier: Type.Optional(Type.String({ description: "Filter: overdue, due_today, d1, d7, d14, d30, upcoming" })),
  }),
  async execute(toolCallId: string, params: any, signal: any) {
    const q = params.tier ? `?tier=${encodeURIComponent(params.tier)}` : "";
    const data = await apiCall("GET", `/v1/obligations/summary${q}`);
    return { content: [{ type: "text" as const, text: JSON.stringify(data) }], details: data };
  },
  renderCall(args: any, theme: any) {
    return renderCallText("list_obligations", [args.tier ?? "all tiers"], theme);
  },
  renderResult(result: any, opts: any, theme: any) {
    const err = renderResultErr(result, theme);
    if (err) return err;
    const d = parseResult(result);
    const items = d.obligations ?? d.items ?? [];
    const tierColor = (t: string) => {
      if (t === "overdue") return "error";
      if (t === "due_today" || t === "d1") return "warning";
      return "dim";
    };
    const lines = items.slice(0, 10).map((o: any) =>
      theme.fg(tierColor(o.tier ?? ""), `[${o.tier ?? "?"}] ${o.description ?? o.title ?? "obligation"}`)
    );
    return renderResultOk(`${items.length} obligations`, lines, theme);
  },
};

const getBillingStatus = {
  name: "get_billing_status",
  label: "Get Billing Status",
  description: "Get current billing status: tier, subscriptions, usage, and available plans.",
  parameters: Type.Object({}),
  async execute(toolCallId: string, params: any, signal: any) {
    const [status, plans] = await Promise.all([
      apiCall("GET", `/v1/billing/status?workspace_id=${wsId()}`),
      apiCall("GET", "/v1/billing/plans"),
    ]);
    const result = { status, plans: plans?.plans ?? plans };
    return { content: [{ type: "text" as const, text: JSON.stringify(result) }], details: result };
  },
  renderCall(args: any, theme: any) {
    return renderCallText("billing_status", ["Fetching billing info"], theme);
  },
  renderResult(result: any, opts: any, theme: any) {
    const d = parseResult(result);
    const s = d.status ?? {};
    return renderResultOk("Billing", [
      `Tier: ${s.tier ?? s.plan ?? "?"}`,
      `Tool calls: ${s.tool_call_count ?? "?"}/${s.tool_call_limit ?? "?"}`,
    ], theme);
  },
};

// -- Entity lifecycle tools --

const formEntity = {
  name: "form_entity",
  label: "Form Entity",
  description: "Form a new business entity. LLC uses US-WY, corporation uses US-DE. Collect member details and ownership allocations before calling.",
  parameters: Type.Object({
    entity_type: Type.String({ description: "Must be 'llc' or 'corporation'" }),
    entity_name: Type.String({ description: "Legal name of the entity" }),
    jurisdiction: Type.String({ description: "Jurisdiction code (e.g. US-WY for LLC, US-DE for corporation)" }),
    members: Type.Array(Type.Object({
      name: Type.String({ description: "Member's full name" }),
      email: Type.String({ description: "Member's email" }),
      role: Type.String({ description: "Role (e.g. Member, Manager, Incorporator)" }),
      investor_type: Type.Optional(Type.String({ description: "natural_person or entity" })),
      ownership_pct: Type.Optional(Type.Number({ description: "Ownership fraction 0.0-1.0 (for LLCs). Must total 1.0." })),
      share_count: Type.Optional(Type.Integer({ description: "Number of shares (for corporations)" })),
    }), { description: "Founding members with ownership allocations" }),
  }),
  async execute(toolCallId: string, params: any, signal: any) {
    const body = {
      entity_type: params.entity_type,
      legal_name: params.entity_name,
      jurisdiction: params.jurisdiction,
      members: params.members,
      workspace_id: wsId(),
    };
    const data = await apiCall("POST", "/v1/formations", body);
    return { content: [{ type: "text" as const, text: JSON.stringify(data) }], details: data };
  },
  renderCall(args: any, theme: any) {
    return renderCallText("form_entity", [
      args.entity_name,
      `${args.entity_type} in ${args.jurisdiction}`,
      `${args.members?.length ?? 0} members`,
    ], theme);
  },
  renderResult(result: any, opts: any, theme: any) {
    const err = renderResultErr(result, theme);
    if (err) return err;
    const d = parseResult(result);
    return renderResultOk(`Entity formed: ${d.legal_name ?? d.entity_name ?? "?"}`, [
      `ID: ${d.entity_id ?? d.id ?? "?"}`,
      `Documents: ${d.documents?.length ?? 0} generated`,
    ], theme);
  },
};

const convertEntity = {
  name: "convert_entity",
  label: "Convert Entity",
  description: "Convert an entity from one type to another (e.g. LLC to C-Corp).",
  parameters: Type.Object({
    entity_id: Type.String(),
    new_entity_type: Type.String({ description: "'llc' or 'corporation'" }),
    new_jurisdiction: Type.Optional(Type.String({ description: "New jurisdiction (defaults to current)" })),
  }),
  async execute(toolCallId: string, params: any, signal: any) {
    const body: any = { new_entity_type: params.new_entity_type };
    if (params.new_jurisdiction) body.new_jurisdiction = params.new_jurisdiction;
    const data = await apiCall("POST", `/v1/entities/${params.entity_id}/convert`, body);
    return { content: [{ type: "text" as const, text: JSON.stringify(data) }], details: data };
  },
  renderCall(args: any, theme: any) {
    return renderCallText("convert_entity", [`→ ${args.new_entity_type}`, args.entity_id], theme);
  },
  renderResult(result: any, opts: any, theme: any) {
    const err = renderResultErr(result, theme);
    if (err) return err;
    const d = parseResult(result);
    return renderResultOk("Entity converted", [`New type: ${d.entity_type ?? d.new_entity_type ?? "?"}`], theme);
  },
};

const dissolveEntity = {
  name: "dissolve_entity",
  label: "Dissolve Entity",
  description: "Initiate entity dissolution and wind-down workflow.",
  parameters: Type.Object({
    entity_id: Type.String(),
    dissolution_reason: Type.String(),
    effective_date: Type.Optional(Type.String({ description: "ISO 8601 date (defaults to today)" })),
  }),
  async execute(toolCallId: string, params: any, signal: any) {
    const body: any = { dissolution_reason: params.dissolution_reason };
    if (params.effective_date) body.effective_date = params.effective_date;
    const data = await apiCall("POST", `/v1/entities/${params.entity_id}/dissolve`, body);
    return { content: [{ type: "text" as const, text: JSON.stringify(data) }], details: data };
  },
  renderCall(args: any, theme: any) {
    return renderCallText("dissolve_entity", [args.entity_id, args.dissolution_reason], theme);
  },
  renderResult(result: any, opts: any, theme: any) {
    const err = renderResultErr(result, theme);
    if (err) return err;
    return renderResultOk("Entity dissolution initiated", [], theme);
  },
};

// -- Equity tools --

const issueEquity = {
  name: "issue_equity",
  label: "Issue Equity",
  description: "Issue equity (shares or membership units) to a recipient.",
  parameters: Type.Object({
    entity_id: Type.String(),
    grant_type: Type.String({ description: "'common', 'preferred', 'option', or 'unit'" }),
    shares: Type.Integer(),
    recipient_name: Type.String(),
  }),
  async execute(toolCallId: string, params: any, signal: any) {
    const data = await apiCall("POST", "/v1/equity/grants", params);
    return { content: [{ type: "text" as const, text: JSON.stringify(data) }], details: data };
  },
  renderCall(args: any, theme: any) {
    return renderCallText("issue_equity", [
      `${args.shares} ${args.grant_type}`,
      `→ ${args.recipient_name}`,
    ], theme);
  },
  renderResult(result: any, opts: any, theme: any) {
    const err = renderResultErr(result, theme);
    if (err) return err;
    const d = parseResult(result);
    return renderResultOk(`Equity issued: ${d.shares ?? "?"} ${d.grant_type ?? ""} shares`, [
      `Recipient: ${d.recipient_name ?? "?"}`,
      `Grant ID: ${d.grant_id ?? d.id ?? "?"}`,
    ], theme);
  },
};

const transferShares = {
  name: "transfer_shares",
  label: "Transfer Shares",
  description: "Transfer shares from one grant to a new recipient.",
  parameters: Type.Object({
    entity_id: Type.String(),
    from_grant_id: Type.String(),
    to_recipient_name: Type.String(),
    shares: Type.Integer(),
    transfer_type: Type.Optional(Type.String({ description: "'sale', 'gift', or 'rofr_exercise'" })),
  }),
  async execute(toolCallId: string, params: any, signal: any) {
    const data = await apiCall("POST", "/v1/share-transfers", params);
    return { content: [{ type: "text" as const, text: JSON.stringify(data) }], details: data };
  },
  renderCall(args: any, theme: any) {
    return renderCallText("transfer_shares", [
      `${args.shares} shares`,
      `→ ${args.to_recipient_name}`,
      args.transfer_type ?? "transfer",
    ], theme);
  },
  renderResult(result: any, opts: any, theme: any) {
    const err = renderResultErr(result, theme);
    if (err) return err;
    return renderResultOk("Shares transferred", [`${parseResult(result).shares ?? "?"} shares`], theme);
  },
};

const issueSafe = {
  name: "issue_safe",
  label: "Issue SAFE",
  description: "Issue a SAFE note to an investor.",
  parameters: Type.Object({
    entity_id: Type.String(),
    investor_name: Type.String(),
    principal_amount_cents: Type.Integer({ description: "Investment amount in cents" }),
    valuation_cap_cents: Type.Integer({ description: "Valuation cap in cents" }),
    safe_type: Type.Optional(Type.String({ description: "'pre_money', 'post_money', or 'mfn'" })),
  }),
  async execute(toolCallId: string, params: any, signal: any) {
    const data = await apiCall("POST", "/v1/safe-notes", params);
    return { content: [{ type: "text" as const, text: JSON.stringify(data) }], details: data };
  },
  renderCall(args: any, theme: any) {
    return renderCallText("issue_safe", [
      args.investor_name,
      centsToUsd(args.principal_amount_cents),
      `cap ${centsToUsd(args.valuation_cap_cents)}`,
    ], theme);
  },
  renderResult(result: any, opts: any, theme: any) {
    const err = renderResultErr(result, theme);
    if (err) return err;
    const d = parseResult(result);
    return renderResultOk(`SAFE issued to ${d.investor_name ?? "?"}`, [
      `Amount: ${centsToUsd(d.principal_amount_cents ?? 0)}`,
      `Cap: ${centsToUsd(d.valuation_cap_cents ?? 0)}`,
    ], theme);
  },
};

const calculateDistribution = {
  name: "calculate_distribution",
  label: "Calculate Distribution",
  description: "Calculate and record a distribution to equity holders.",
  parameters: Type.Object({
    entity_id: Type.String(),
    total_amount_cents: Type.Integer(),
    distribution_type: Type.String({ description: "'pro_rata', 'waterfall', or 'guaranteed_payment'" }),
  }),
  async execute(toolCallId: string, params: any, signal: any) {
    const data = await apiCall("POST", "/v1/distributions", params);
    return { content: [{ type: "text" as const, text: JSON.stringify(data) }], details: data };
  },
  renderCall(args: any, theme: any) {
    return renderCallText("calculate_distribution", [
      args.distribution_type,
      centsToUsd(args.total_amount_cents),
    ], theme);
  },
  renderResult(result: any, opts: any, theme: any) {
    const err = renderResultErr(result, theme);
    if (err) return err;
    const d = parseResult(result);
    return renderResultOk(`Distribution: ${centsToUsd(d.total_amount_cents ?? 0)}`, [
      `Type: ${d.distribution_type ?? "?"}`,
    ], theme);
  },
};

// -- Finance tools --

const createInvoice = {
  name: "create_invoice",
  label: "Create Invoice",
  description: "Create an invoice for a customer.",
  parameters: Type.Object({
    entity_id: Type.String(),
    customer_name: Type.String(),
    amount_cents: Type.Integer(),
    due_date: Type.String({ description: "ISO 8601 date" }),
    description: Type.Optional(Type.String()),
  }),
  async execute(toolCallId: string, params: any, signal: any) {
    const data = await apiCall("POST", "/v1/invoices", params);
    return { content: [{ type: "text" as const, text: JSON.stringify(data) }], details: data };
  },
  renderCall(args: any, theme: any) {
    return renderCallText("create_invoice", [
      args.customer_name,
      centsToUsd(args.amount_cents),
      `due ${args.due_date}`,
    ], theme);
  },
  renderResult(result: any, opts: any, theme: any) {
    const err = renderResultErr(result, theme);
    if (err) return err;
    const d = parseResult(result);
    return renderResultOk(`Invoice created: ${centsToUsd(d.amount_cents ?? 0)}`, [
      `Customer: ${d.customer_name ?? "?"}`,
      `Due: ${d.due_date ?? "?"}`,
    ], theme);
  },
};

const runPayroll = {
  name: "run_payroll",
  label: "Run Payroll",
  description: "Run payroll for an entity over a pay period.",
  parameters: Type.Object({
    entity_id: Type.String(),
    pay_period_start: Type.String(),
    pay_period_end: Type.String(),
  }),
  async execute(toolCallId: string, params: any, signal: any) {
    const data = await apiCall("POST", "/v1/payroll/runs", params);
    return { content: [{ type: "text" as const, text: JSON.stringify(data) }], details: data };
  },
  renderCall(args: any, theme: any) {
    return renderCallText("run_payroll", [`${args.pay_period_start} → ${args.pay_period_end}`], theme);
  },
  renderResult(result: any, opts: any, theme: any) {
    const err = renderResultErr(result, theme);
    if (err) return err;
    return renderResultOk("Payroll run completed", [], theme);
  },
};

const submitPayment = {
  name: "submit_payment",
  label: "Submit Payment",
  description: "Submit a payment from an entity to a recipient.",
  parameters: Type.Object({
    entity_id: Type.String(),
    amount_cents: Type.Integer(),
    recipient: Type.String(),
    payment_method: Type.Optional(Type.String({ description: "'ach', 'wire', or 'check'" })),
  }),
  async execute(toolCallId: string, params: any, signal: any) {
    const data = await apiCall("POST", "/v1/payments", params);
    return { content: [{ type: "text" as const, text: JSON.stringify(data) }], details: data };
  },
  renderCall(args: any, theme: any) {
    return renderCallText("submit_payment", [
      centsToUsd(args.amount_cents),
      `→ ${args.recipient}`,
      args.payment_method ?? "ach",
    ], theme);
  },
  renderResult(result: any, opts: any, theme: any) {
    const err = renderResultErr(result, theme);
    if (err) return err;
    const d = parseResult(result);
    return renderResultOk(`Payment: ${centsToUsd(d.amount_cents ?? 0)}`, [
      `Recipient: ${d.recipient ?? "?"}`,
      `Method: ${d.payment_method ?? "ach"}`,
    ], theme);
  },
};

const openBankAccount = {
  name: "open_bank_account",
  label: "Open Bank Account",
  description: "Open a business bank account for an entity.",
  parameters: Type.Object({
    entity_id: Type.String(),
    institution_name: Type.Optional(Type.String({ description: "Bank name (defaults to Mercury)" })),
  }),
  async execute(toolCallId: string, params: any, signal: any) {
    const body: any = { entity_id: params.entity_id };
    if (params.institution_name) body.institution_name = params.institution_name;
    const data = await apiCall("POST", "/v1/bank-accounts", body);
    return { content: [{ type: "text" as const, text: JSON.stringify(data) }], details: data };
  },
  renderCall(args: any, theme: any) {
    return renderCallText("open_bank_account", [args.institution_name ?? "Mercury"], theme);
  },
  renderResult(result: any, opts: any, theme: any) {
    const err = renderResultErr(result, theme);
    if (err) return err;
    const d = parseResult(result);
    return renderResultOk("Bank account opened", [
      `Institution: ${d.institution_name ?? "Mercury"}`,
    ], theme);
  },
};

const reconcileLedger = {
  name: "reconcile_ledger",
  label: "Reconcile Ledger",
  description: "Reconcile an entity's ledger for a given period.",
  parameters: Type.Object({
    entity_id: Type.String(),
    start_date: Type.String(),
    end_date: Type.String(),
  }),
  async execute(toolCallId: string, params: any, signal: any) {
    const data = await apiCall("POST", "/v1/ledger/reconcile", params);
    return { content: [{ type: "text" as const, text: JSON.stringify(data) }], details: data };
  },
  renderCall(args: any, theme: any) {
    return renderCallText("reconcile_ledger", [`${args.start_date} → ${args.end_date}`], theme);
  },
  renderResult(result: any, opts: any, theme: any) {
    const err = renderResultErr(result, theme);
    if (err) return err;
    return renderResultOk("Ledger reconciled", [], theme);
  },
};

// -- Documents & compliance tools --

const generateContract = {
  name: "generate_contract",
  label: "Generate Contract",
  description: "Generate a contract from a template (NDA, contractor agreement, offer letter, etc).",
  parameters: Type.Object({
    entity_id: Type.String(),
    template_type: Type.String({ description: "'nda', 'contractor_agreement', 'employment_offer', 'consulting_agreement'" }),
    counterparty_name: Type.String(),
    effective_date: Type.Optional(Type.String({ description: "ISO 8601 date (defaults to today)" })),
  }),
  async execute(toolCallId: string, params: any, signal: any) {
    const data = await apiCall("POST", "/v1/contracts", params);
    return { content: [{ type: "text" as const, text: JSON.stringify(data) }], details: data };
  },
  renderCall(args: any, theme: any) {
    return renderCallText("generate_contract", [
      args.template_type,
      `for ${args.counterparty_name}`,
    ], theme);
  },
  renderResult(result: any, opts: any, theme: any) {
    const err = renderResultErr(result, theme);
    if (err) return err;
    const d = parseResult(result);
    return renderResultOk(`Contract generated: ${d.template_type ?? "?"}`, [
      `Counterparty: ${d.counterparty_name ?? "?"}`,
      `Status: ${d.status ?? "draft"}`,
    ], theme);
  },
};

const fileTaxDocument = {
  name: "file_tax_document",
  label: "File Tax Document",
  description: "Generate and file a tax document for an entity.",
  parameters: Type.Object({
    entity_id: Type.String(),
    document_type: Type.String({ description: "'1099-NEC', 'K-1', '941', 'W-2', 'estimated_tax'" }),
    tax_year: Type.Integer(),
  }),
  async execute(toolCallId: string, params: any, signal: any) {
    const data = await apiCall("POST", "/v1/tax/filings", params);
    return { content: [{ type: "text" as const, text: JSON.stringify(data) }], details: data };
  },
  renderCall(args: any, theme: any) {
    return renderCallText("file_tax_document", [args.document_type, `TY ${args.tax_year}`], theme);
  },
  renderResult(result: any, opts: any, theme: any) {
    const err = renderResultErr(result, theme);
    if (err) return err;
    const d = parseResult(result);
    return renderResultOk(`Tax document filed: ${d.document_type ?? "?"}`, [
      `Tax year: ${d.tax_year ?? "?"}`,
    ], theme);
  },
};

const trackDeadline = {
  name: "track_deadline",
  label: "Track Deadline",
  description: "Track a compliance or filing deadline.",
  parameters: Type.Object({
    entity_id: Type.String(),
    deadline_type: Type.String({ description: "'tax_filing', 'annual_report', 'sales_tax', 'franchise_tax'" }),
    due_date: Type.String(),
    description: Type.String(),
    recurrence: Type.Optional(Type.String({ description: "'monthly', 'quarterly', 'annually', or empty" })),
  }),
  async execute(toolCallId: string, params: any, signal: any) {
    const data = await apiCall("POST", "/v1/deadlines", params);
    return { content: [{ type: "text" as const, text: JSON.stringify(data) }], details: data };
  },
  renderCall(args: any, theme: any) {
    return renderCallText("track_deadline", [
      args.deadline_type,
      `due ${args.due_date}`,
      args.recurrence ?? "",
    ].filter(Boolean), theme);
  },
  renderResult(result: any, opts: any, theme: any) {
    const err = renderResultErr(result, theme);
    if (err) return err;
    return renderResultOk("Deadline tracked", [
      `${parseResult(result).deadline_type ?? "?"}: ${parseResult(result).due_date ?? "?"}`,
    ], theme);
  },
};

const classifyContractor = {
  name: "classify_contractor",
  label: "Classify Contractor",
  description: "Analyze contractor classification risk (employee vs independent contractor).",
  parameters: Type.Object({
    entity_id: Type.String(),
    contractor_name: Type.String(),
    state: Type.String({ description: "US state code" }),
    hours_per_week: Type.Integer(),
    exclusive_client: Type.Boolean(),
    duration_months: Type.Integer(),
    provides_tools: Type.Optional(Type.Boolean()),
  }),
  async execute(toolCallId: string, params: any, signal: any) {
    const data = await apiCall("POST", "/v1/contractors/classify", params);
    return { content: [{ type: "text" as const, text: JSON.stringify(data) }], details: data };
  },
  renderCall(args: any, theme: any) {
    return renderCallText("classify_contractor", [
      args.contractor_name,
      args.state,
      `${args.hours_per_week}h/wk`,
    ], theme);
  },
  renderResult(result: any, opts: any, theme: any) {
    const err = renderResultErr(result, theme);
    if (err) return err;
    const d = parseResult(result);
    const riskColor = (d.risk_level === "high" || d.classification === "employee") ? "error" : "success";
    return new Text(
      theme.fg(riskColor, `✓ ${d.contractor_name ?? "?"}: ${d.classification ?? d.risk_level ?? "?"}`) +
      (d.recommendation ? "\n  " + theme.fg("dim", d.recommendation) : ""),
      0, 0
    );
  },
};

// -- Governance tools --

const conveneMeeting = {
  name: "convene_meeting",
  label: "Convene Meeting",
  description: "Convene a governance meeting (board, shareholder, or member vote).",
  parameters: Type.Object({
    entity_id: Type.String(),
    meeting_type: Type.String({ description: "'board', 'shareholder', or 'member'" }),
    agenda_items: Type.Array(Type.String()),
  }),
  async execute(toolCallId: string, params: any, signal: any) {
    const data = await apiCall("POST", "/v1/meetings", params);
    return { content: [{ type: "text" as const, text: JSON.stringify(data) }], details: data };
  },
  renderCall(args: any, theme: any) {
    return renderCallText("convene_meeting", [
      args.meeting_type,
      `${args.agenda_items?.length ?? 0} agenda items`,
    ], theme);
  },
  renderResult(result: any, opts: any, theme: any) {
    const err = renderResultErr(result, theme);
    if (err) return err;
    const d = parseResult(result);
    return renderResultOk(`Meeting convened: ${d.meeting_type ?? "?"}`, [
      `Meeting ID: ${d.meeting_id ?? d.id ?? "?"}`,
    ], theme);
  },
};

const castVote = {
  name: "cast_vote",
  label: "Cast Vote",
  description: "Cast a vote on an agenda item in a governance meeting.",
  parameters: Type.Object({
    meeting_id: Type.String(),
    agenda_item_id: Type.String(),
    voter_id: Type.String(),
    vote: Type.String({ description: "'for', 'against', 'abstain', or 'recusal'" }),
  }),
  async execute(toolCallId: string, params: any, signal: any) {
    const data = await apiCall(
      "POST",
      `/v1/meetings/${params.meeting_id}/agenda-items/${params.agenda_item_id}/vote`,
      { voter_id: params.voter_id, vote_value: params.vote }
    );
    return { content: [{ type: "text" as const, text: JSON.stringify(data) }], details: data };
  },
  renderCall(args: any, theme: any) {
    const voteColor = args.vote === "for" ? "success" : args.vote === "against" ? "error" : "warning";
    return new Text(
      theme.fg("toolTitle", theme.bold("cast_vote ")) +
      theme.fg(voteColor, args.vote) +
      theme.fg("muted", ` on ${args.agenda_item_id}`),
      0, 0
    );
  },
  renderResult(result: any, opts: any, theme: any) {
    const err = renderResultErr(result, theme);
    if (err) return err;
    return renderResultOk("Vote recorded", [], theme);
  },
};

const scheduleMeeting = {
  name: "schedule_meeting",
  label: "Schedule Meeting",
  description: "Schedule a board or member meeting.",
  parameters: Type.Object({
    body_id: Type.String({ description: "Governance body ID" }),
    meeting_type: Type.String({ description: "regular, special, annual, or written_consent" }),
    title: Type.String({ description: "Meeting title" }),
    proposed_date: Type.String({ description: "ISO datetime for the meeting" }),
    agenda_items: Type.Optional(Type.Array(Type.String(), { description: "Agenda item titles" })),
  }),
  async execute(toolCallId: string, params: any, signal: any) {
    const body: any = {
      body_id: params.body_id,
      meeting_type: params.meeting_type,
      title: params.title,
      proposed_date: params.proposed_date,
    };
    if (params.agenda_items) body.agenda_item_titles = params.agenda_items;
    const data = await apiCall("POST", "/v1/meetings", body);
    return { content: [{ type: "text" as const, text: JSON.stringify(data) }], details: data };
  },
  renderCall(args: any, theme: any) {
    return renderCallText("schedule_meeting", [args.title, args.meeting_type, args.proposed_date], theme);
  },
  renderResult(result: any, opts: any, theme: any) {
    const err = renderResultErr(result, theme);
    if (err) return err;
    const d = parseResult(result);
    return renderResultOk(`Meeting scheduled: ${d.title ?? "?"}`, [
      `Date: ${d.proposed_date ?? "?"}`,
    ], theme);
  },
};

// -- Workspace tools --

const updateChecklist = {
  name: "update_checklist",
  label: "Update Checklist",
  description: "Create or replace the workspace checklist. Use markdown checkbox format.",
  parameters: Type.Object({
    checklist: Type.String({ description: "Full checklist in markdown checkbox format" }),
  }),
  async execute(toolCallId: string, params: any, signal: any) {
    const configDir = process.env.CORP_CONFIG_DIR || join(homedir(), ".corp");
    mkdirSync(configDir, { recursive: true });
    writeFileSync(join(configDir, "checklist.md"), params.checklist);
    return { content: [{ type: "text" as const, text: JSON.stringify({ status: "updated", checklist: params.checklist }) }], details: { status: "updated" } };
  },
  renderCall(args: any, theme: any) {
    const lineCount = args.checklist?.split("\n").length ?? 0;
    return renderCallText("update_checklist", [`${lineCount} lines`], theme);
  },
  renderResult(result: any, opts: any, theme: any) {
    return renderResultOk("Checklist updated", [], theme);
  },
};

// -- Agent tools --

const createAgent = {
  name: "create_agent",
  label: "Create Agent",
  description: "Create a new autonomous AI agent for a corporate operations task. Requires paid plan.",
  parameters: Type.Object({
    name: Type.String(),
    system_prompt: Type.String(),
    model: Type.Optional(Type.String({ description: "LLM model (default: anthropic/claude-sonnet-4-6)" })),
    entity_id: Type.Optional(Type.String({ description: "Optional entity to bind the agent to" })),
  }),
  async execute(toolCallId: string, params: any, signal: any) {
    const data = await apiCall("POST", "/v1/agents", params);
    return { content: [{ type: "text" as const, text: JSON.stringify(data) }], details: data };
  },
  renderCall(args: any, theme: any) {
    return renderCallText("create_agent", [`"${args.name}"`], theme);
  },
  renderResult(result: any, opts: any, theme: any) {
    const err = renderResultErr(result, theme);
    if (err) return err;
    const d = parseResult(result);
    return renderResultOk(`Agent created: ${d.name ?? "?"}`, [
      `ID: ${d.agent_id ?? d.id ?? "?"}`,
    ], theme);
  },
};

const sendAgentMessage = {
  name: "send_agent_message",
  label: "Send Agent Message",
  description: "Send a message to an agent, triggering an execution.",
  parameters: Type.Object({
    agent_id: Type.String(),
    message: Type.String(),
  }),
  async execute(toolCallId: string, params: any, signal: any) {
    const data = await apiCall("POST", `/v1/agents/${params.agent_id}/messages`, { body: params.message });
    return { content: [{ type: "text" as const, text: JSON.stringify(data) }], details: data };
  },
  renderCall(args: any, theme: any) {
    const preview = args.message?.length > 40 ? args.message.slice(0, 40) + "…" : args.message;
    return renderCallText("send_agent_message", [preview], theme);
  },
  renderResult(result: any, opts: any, theme: any) {
    const err = renderResultErr(result, theme);
    if (err) return err;
    return renderResultOk("Message sent to agent", [], theme);
  },
};

const updateAgent = {
  name: "update_agent",
  label: "Update Agent",
  description: "Update an agent's configuration (status, name, prompt).",
  parameters: Type.Object({
    agent_id: Type.String(),
    status: Type.Optional(Type.String({ description: "'active', 'paused', or 'disabled'" })),
    system_prompt: Type.Optional(Type.String()),
    name: Type.Optional(Type.String()),
  }),
  async execute(toolCallId: string, params: any, signal: any) {
    const { agent_id, ...body } = params;
    const data = await apiCall("PATCH", `/v1/agents/${agent_id}`, body);
    return { content: [{ type: "text" as const, text: JSON.stringify(data) }], details: data };
  },
  renderCall(args: any, theme: any) {
    const changes = [args.status, args.name].filter(Boolean).join(", ") || "config update";
    return renderCallText("update_agent", [changes], theme);
  },
  renderResult(result: any, opts: any, theme: any) {
    const err = renderResultErr(result, theme);
    if (err) return err;
    return renderResultOk("Agent updated", [], theme);
  },
};

const addAgentSkill = {
  name: "add_agent_skill",
  label: "Add Agent Skill",
  description: "Add a skill to an agent.",
  parameters: Type.Object({
    agent_id: Type.String(),
    skill_name: Type.String(),
    description: Type.String(),
    instructions: Type.Optional(Type.String()),
  }),
  async execute(toolCallId: string, params: any, signal: any) {
    const { agent_id, ...body } = params;
    const data = await apiCall("POST", `/v1/agents/${agent_id}/skills`, body);
    return { content: [{ type: "text" as const, text: JSON.stringify(data) }], details: data };
  },
  renderCall(args: any, theme: any) {
    return renderCallText("add_agent_skill", [`"${args.skill_name}"`], theme);
  },
  renderResult(result: any, opts: any, theme: any) {
    const err = renderResultErr(result, theme);
    if (err) return err;
    return renderResultOk(`Skill added: ${parseResult(result).skill_name ?? "?"}`, [], theme);
  },
};

// ── All Tools ───────────────────────────────────────────────────

const allTools = [
  // Read tools
  getWorkspaceStatus,
  listEntities,
  getCapTable,
  listDocuments,
  listSafeNotes,
  listAgents,
  getChecklist,
  getDocumentLink,
  getSigningLink,
  listObligations,
  getBillingStatus,
  // Entity lifecycle
  formEntity,
  convertEntity,
  dissolveEntity,
  // Equity
  issueEquity,
  transferShares,
  issueSafe,
  calculateDistribution,
  // Finance
  createInvoice,
  runPayroll,
  submitPayment,
  openBankAccount,
  reconcileLedger,
  // Documents & compliance
  generateContract,
  fileTaxDocument,
  trackDeadline,
  classifyContractor,
  // Governance
  conveneMeeting,
  castVote,
  scheduleMeeting,
  // Workspace
  updateChecklist,
  // Agents
  createAgent,
  sendAgentMessage,
  updateAgent,
  addAgentSkill,
];

// ── Extension Entry Point ───────────────────────────────────────

export default function (pi: ExtensionAPI) {
  // Register all 30+ tools
  for (const tool of allTools) {
    pi.registerTool(tool as any);
  }

  // ── Block built-in file write & bash tools in hosted web terminal ─
  // When running in the hosted web terminal (CORP_TERMINAL=web), Pi's
  // built-in bash/write/edit are inappropriate. In other contexts
  // (services/agents, local dev) they remain available.
  const BLOCKED_BUILTIN_TOOLS = new Set(["bash", "write", "edit"]);
  const isHostedWeb = process.env.CORP_TERMINAL === "web";
  const corpToolNames = new Set(allTools.map((t) => t.name));
  const headlessWritePolicy = (process.env.CORP_HEADLESS_WRITE_POLICY || "deny").toLowerCase();

  // ── Write Tool Confirmation ─────────────────────────────────
  pi.on("tool_call", async (event: any, ctx: any) => {
    const name = event.toolName ?? event.toolCall?.name;
    if (!name) return;

    const isHeadless = process.env.CORP_AUTO_APPROVE === "1" || !ctx?.hasUI;

    // Always block risky built-in mutation tools in hosted and headless contexts.
    if ((isHostedWeb || isHeadless) && BLOCKED_BUILTIN_TOOLS.has(name)) {
      return {
        block: true,
        reason: `The ${name} tool is not available. Use corporate operations tools instead.`,
      };
    }

    // Headless mode: fail closed for write actions unless explicitly allowed.
    if (isHeadless) {
      if (!corpToolNames.has(name)) return;
      if (READ_ONLY_TOOLS.has(name)) return;
      if (headlessWritePolicy === "allow") return;
      return {
        block: true,
        reason: "Approval required: mutating actions are blocked in unattended mode.",
      };
    }

    if (READ_ONLY_TOOLS.has(name)) return; // auto-approve reads

    // Only intercept our corp tools
    if (!corpToolNames.has(name)) return;

    const args = event.input ?? event.toolCall?.args ?? {};
    const desc = describeToolCall(name, args);
    const approved = await ctx.ui.confirm("Confirm action", desc);
    if (!approved) return { block: true, reason: "User declined" };
  });

  // ── System Prompt Injection ─────────────────────────────────
  pi.on("before_agent_start", async (event: any, ctx: any) => {
    const cfg = loadConfig();
    const contextLines: string[] = [];
    if (cfg.workspace_id) contextLines.push(`Workspace ID: ${cfg.workspace_id}`);
    if (cfg.active_entity_id) contextLines.push(`Active entity ID: ${cfg.active_entity_id}`);
    if (cfg.user?.name) contextLines.push(`User: ${cfg.user.name}`);

    const corpPrompt = [
      "\n## TheCorporation Context",
      "You have corporate operations tools registered. Use them to help with entity formation, equity, finance, compliance, governance, and agent management.",
      contextLines.length > 0 ? contextLines.join("\n") : "",
      "",
      "Tool categories:",
      "- Read (auto-approved): get_workspace_status, list_entities, get_cap_table, list_documents, list_safe_notes, list_agents, get_checklist, list_obligations, get_billing_status",
      "- Entity lifecycle: form_entity, convert_entity, dissolve_entity",
      "- Equity: issue_equity, transfer_shares, issue_safe, calculate_distribution",
      "- Finance: create_invoice, run_payroll, submit_payment, open_bank_account, reconcile_ledger",
      "- Documents: generate_contract, file_tax_document, track_deadline, classify_contractor, get_signing_link, get_document_link",
      "- Governance: convene_meeting, cast_vote, schedule_meeting",
      "- Agents: create_agent, send_agent_message, update_agent, add_agent_skill",
      "",
      isHostedWeb ? "IMPORTANT: Do NOT use the bash, write, or edit tools. They are disabled. You are a corporate operations agent — use only the corporate tools listed above." : "",
      "Rules: All monetary values in cents. LLC→US-WY, Corporation→US-DE. Documents can ONLY be signed through signing links. Always suggest next steps after actions.",
    ].filter(Boolean).join("\n");

    return {
      systemPrompt: (event.systemPrompt ?? "") + corpPrompt,
    };
  });

  // ── Boot Screen ─────────────────────────────────────────────
  pi.on("session_start", async (_event: any, ctx: any) => {
    if (!ctx.hasUI || process.env.CORP_AUTO_APPROVE === "1") return;
    const cfg = loadConfig();
    const ws = cfg.workspace_id ? cfg.workspace_id.slice(0, 8) + "…" : "not configured";
    const api = cfg.api_url || "not configured";
    const user = cfg.user?.name || "anonymous";
    const entity = cfg.active_entity_id ? cfg.active_entity_id.slice(0, 8) + "…" : "none";

    const lines = [
      "",
      "  ╔══════════════════════════════════════════╗",
      "  ║                                          ║",
      "  ║    T H E   C O R P O R A T I O N        ║",
      "  ║    Corporate Operations Agent             ║",
      "  ║                                          ║",
      "  ╚══════════════════════════════════════════╝",
      "",
      `    Workspace:  ${ws}`,
      `    API:        ${api}`,
      `    User:       ${user}`,
      `    Entity:     ${entity}`,
      "",
      "    Commands:  /status  /entities  /cap-table",
      "               /obligations  /documents  /billing",
      "",
      "    35 corporate tools loaded. Write tools require approval.",
      "",
    ];
    ctx.ui.notify(lines.join("\n"), "info");
  });

  // ── Headless Result Output ──────────────────────────────────
  // When running in headless mode (CORP_AUTO_APPROVE=1), write an
  // ExecutionResult-compatible JSON file so the orchestrator can collect it.
  pi.on("session_end", async (event: any, _ctx: any) => {
    if (process.env.CORP_AUTO_APPROVE !== "1") return;
    const resultPath = join(process.env.WORKSPACE || "/workspace", ".result.json");
    const result = {
      success: !event.error,
      reason: event.error?.message ?? null,
      final_response: event.lastAssistantMessage ?? "",
      tool_calls_count: event.stats?.toolCallCount ?? 0,
      turns: event.stats?.turnCount ?? 0,
      input_tokens: event.stats?.inputTokens ?? 0,
      output_tokens: event.stats?.outputTokens ?? 0,
      transcript: [],
      tasks: [],
    };
    writeFileSync(resultPath, JSON.stringify(result));
  });

  // ── LLM Provider Config (Headless) ────────────────────────
  // When running headless with a secrets proxy, configure Pi to route
  // LLM calls through the proxy URL with the opaque API key token.
  if (process.env.CORP_AUTO_APPROVE === "1" && process.env.CORP_LLM_PROXY_URL) {
    const piDir = join(process.env.PI_CODING_AGENT_DIR || join(homedir(), ".pi", "agent"));
    mkdirSync(piDir, { recursive: true });
    writeFileSync(join(piDir, "models.json"), JSON.stringify({
      providers: {
        openrouter: {
          baseUrl: process.env.CORP_LLM_PROXY_URL,
          apiKey: process.env.OPENROUTER_API_KEY || "",
        },
      },
    }));
  }

  // ── Slash Commands ──────────────────────────────────────────

  pi.registerCommand("status", {
    description: "Show workspace status summary",
    handler: async (args: string, ctx: any) => {
      try {
        const data = await apiCall("GET", `/v1/workspaces/${wsId()}/status`);
        const lines = [
          `Workspace: ${data.workspace_name ?? data.name ?? wsId()}`,
          `Entities: ${data.entity_count ?? data.entities ?? "?"}`,
          `Documents: ${data.document_count ?? data.documents ?? "?"}`,
          `Grants: ${data.grant_count ?? data.grants ?? "?"}`,
          `Tier: ${data.billing_tier ?? data.tier ?? "free"}`,
        ];
        ctx.ui.notify(lines.join("\n"), "info");
      } catch (e: any) {
        ctx.ui.notify(`Failed to fetch status: ${e.message}`, "error");
      }
    },
  });

  pi.registerCommand("entities", {
    description: "List entities in the workspace",
    handler: async (args: string, ctx: any) => {
      try {
        const data = await apiCall("GET", `/v1/workspaces/${wsId()}/entities`);
        const entities = Array.isArray(data) ? data : data.entities ?? [];
        if (entities.length === 0) {
          ctx.ui.notify("No entities found. Use form_entity to create one.", "info");
          return;
        }
        const items = entities.map((e: any) =>
          `${e.legal_name ?? e.entity_name ?? e.name} | ${e.entity_type} | ${e.jurisdiction ?? "?"} | ${e.status ?? "active"}`
        );
        const choice = await ctx.ui.select("Entities:", items);
        if (choice != null && entities[choice]) {
          const eid = entities[choice].entity_id ?? entities[choice].id;
          ctx.ui.notify(`Selected entity: ${eid}`, "info");
        }
      } catch (e: any) {
        ctx.ui.notify(`Failed to list entities: ${e.message}`, "error");
      }
    },
  });

  pi.registerCommand("cap-table", {
    description: "Show cap table for the active entity",
    handler: async (args: string, ctx: any) => {
      const cfg = loadConfig();
      const entityId = args.trim() || cfg.active_entity_id;
      if (!entityId) {
        ctx.ui.notify("No active entity. Pass an entity ID or set active_entity_id in ~/.corp/config.json", "warning");
        return;
      }
      try {
        const data = await apiCall("GET", `/v1/entities/${entityId}/cap-table`);
        const grants = data.grants ?? data.rows ?? [];
        if (grants.length === 0) {
          ctx.ui.notify("Cap table is empty.", "info");
          return;
        }
        const lines = grants.map((g: any) => {
          const pct = g.percentage != null ? (g.percentage * 100).toFixed(1) + "%" : "?";
          return `${g.holder_name ?? g.recipient_name ?? "?"} | ${g.shares ?? "?"} ${g.grant_type ?? ""} | ${pct}`;
        });
        ctx.ui.notify("Cap Table:\n" + lines.join("\n"), "info");
      } catch (e: any) {
        ctx.ui.notify(`Failed to fetch cap table: ${e.message}`, "error");
      }
    },
  });

  pi.registerCommand("obligations", {
    description: "Show obligations with urgency coloring",
    handler: async (args: string, ctx: any) => {
      try {
        const data = await apiCall("GET", "/v1/obligations/summary");
        const items = data.obligations ?? data.items ?? [];
        if (items.length === 0) {
          ctx.ui.notify("No obligations found.", "info");
          return;
        }
        const lines = items.map((o: any) => {
          const prefix = o.tier === "overdue" ? "🔴" : o.tier === "due_today" ? "🟡" : "🟢";
          return `${prefix} [${o.tier ?? "?"}] ${o.description ?? o.title ?? "obligation"} — due ${o.due_date ?? "?"}`;
        });
        ctx.ui.notify("Obligations:\n" + lines.join("\n"), "info");
      } catch (e: any) {
        ctx.ui.notify(`Failed to fetch obligations: ${e.message}`, "error");
      }
    },
  });

  pi.registerCommand("documents", {
    description: "List documents for the active entity",
    handler: async (args: string, ctx: any) => {
      const cfg = loadConfig();
      const entityId = args.trim() || cfg.active_entity_id;
      if (!entityId) {
        ctx.ui.notify("No active entity. Pass an entity ID or set active_entity_id.", "warning");
        return;
      }
      try {
        const data = await apiCall("GET", `/v1/formations/${entityId}/documents`);
        const docs = Array.isArray(data) ? data : data.documents ?? [];
        if (docs.length === 0) {
          ctx.ui.notify("No documents found.", "info");
          return;
        }
        const badge = (s: string) =>
          s === "signed" ? "✓" : s === "pending_signature" || s === "pending" ? "⏳" : "📝";
        const lines = docs.map((d: any) =>
          `${badge(d.status)} ${d.title ?? d.document_type ?? "doc"} [${d.status ?? "draft"}]`
        );
        ctx.ui.notify("Documents:\n" + lines.join("\n"), "info");
      } catch (e: any) {
        ctx.ui.notify(`Failed to fetch documents: ${e.message}`, "error");
      }
    },
  });

  pi.registerCommand("billing", {
    description: "Show billing status and plan info",
    handler: async (args: string, ctx: any) => {
      try {
        const [status, plansData] = await Promise.all([
          apiCall("GET", `/v1/billing/status?workspace_id=${wsId()}`),
          apiCall("GET", "/v1/billing/plans"),
        ]);
        const plans = plansData?.plans ?? plansData ?? [];
        const lines = [
          `Current tier: ${status.tier ?? status.plan ?? "free"}`,
          `Tool calls: ${status.tool_call_count ?? "?"}/${status.tool_call_limit ?? "?"}`,
        ];
        if (Array.isArray(plans) && plans.length > 0) {
          lines.push("", "Available plans:");
          for (const p of plans) {
            lines.push(`  ${p.name ?? p.tier}: ${p.price ?? "?"}`);
          }
        }
        ctx.ui.notify(lines.join("\n"), "info");
      } catch (e: any) {
        ctx.ui.notify(`Failed to fetch billing: ${e.message}`, "error");
      }
    },
  });
}
