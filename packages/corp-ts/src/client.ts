/**
 * @module client
 *
 * Typed HTTP client for the TheCorporation API.
 *
 * Uses the standard `fetch` API (Node 20+ or browser) — no extra deps.
 *
 * ```ts
 * import { CorpClient } from "@thecorporation/corp/client";
 *
 * const client = new CorpClient("http://localhost:8000", "corp_...");
 * const entities = await client.entities.list();
 * ```
 */

import type {
  Entity,
  EntityType,
  Document,
  Filing,
  TaxProfile,
  CapTable,
  ShareClass,
  EquityGrant,
  SafeNote,
  Valuation,
  GovernanceBody,
  GovernanceSeat,
  Meeting,
  Invoice,
  Payment,
  Contact,
  Agent,
  WorkItem,
  ApiKeyResponse,
  EntityId,
  DocumentId,
  ContactId,
  GovernanceBodyId,
  GovernanceSeatId,
  MeetingId,
  AgendaItemId,
  ValuationId,
  SafeNoteId,
  InvoiceId,
  BankAccountId,
  PayrollRunId,
  AgentId,
  WorkItemId,
  ServiceRequestId,
  ApiKeyId,
} from "./types.js";

// ── Error ────────────────────────────────────────────────────────────────────

export class CorpApiError extends Error {
  constructor(
    public readonly status: number,
    public readonly body: string,
    public readonly path: string,
  ) {
    super(`${status} ${path}: ${body}`);
    this.name = "CorpApiError";
  }
}

// ── Client ───────────────────────────────────────────────────────────────────

export class CorpClient {
  private readonly baseUrl: string;
  private readonly headers: Record<string, string>;

  /** Sub-clients for each domain. */
  readonly entities: EntitiesApi;
  readonly formation: FormationApi;
  readonly equity: EquityApi;
  readonly governance: GovernanceApi;
  readonly treasury: TreasuryApi;
  readonly contacts: ContactsApi;
  readonly agents: AgentsApi;
  readonly workItems: WorkItemsApi;
  readonly admin: AdminApi;

  constructor(baseUrl: string, apiKey?: string) {
    this.baseUrl = baseUrl.replace(/\/+$/, "");
    this.headers = {
      "Content-Type": "application/json",
      "User-Agent": "corp-ts/0.1.0",
    };
    if (apiKey) {
      this.headers["Authorization"] = `Bearer ${apiKey}`;
    }

    this.entities = new EntitiesApi(this);
    this.formation = new FormationApi(this);
    this.equity = new EquityApi(this);
    this.governance = new GovernanceApi(this);
    this.treasury = new TreasuryApi(this);
    this.contacts = new ContactsApi(this);
    this.agents = new AgentsApi(this);
    this.workItems = new WorkItemsApi(this);
    this.admin = new AdminApi(this);
  }

  // ── Raw HTTP ─────────────────────────────────────────────────────────────

  async get<T = unknown>(path: string): Promise<T> {
    return this.request("GET", path);
  }

  async post<T = unknown>(path: string, body?: unknown): Promise<T> {
    return this.request("POST", path, body);
  }

  async put<T = unknown>(path: string, body?: unknown): Promise<T> {
    return this.request("PUT", path, body);
  }

  async patch<T = unknown>(path: string, body?: unknown): Promise<T> {
    return this.request("PATCH", path, body);
  }

  async delete<T = unknown>(path: string): Promise<T> {
    return this.request("DELETE", path);
  }

  private async request<T>(method: string, path: string, body?: unknown): Promise<T> {
    const url = `${this.baseUrl}${path}`;
    const opts: RequestInit = { method, headers: this.headers };
    if (body !== undefined) {
      opts.body = JSON.stringify(body);
    }

    const resp = await fetch(url, opts);

    if (!resp.ok) {
      const text = await resp.text().catch(() => "");
      throw new CorpApiError(resp.status, text, path);
    }

    if (resp.status === 204) return {} as T;
    return resp.json() as Promise<T>;
  }
}

// ── Entities ─────────────────────────────────────────────────────────────────

class EntitiesApi {
  constructor(private c: CorpClient) {}

  list(): Promise<Entity[]> {
    return this.c.get("/v1/entities");
  }

  get(id: EntityId): Promise<Entity> {
    return this.c.get(`/v1/entities/${id}`);
  }

  create(opts: { legal_name: string; entity_type: EntityType; jurisdiction: string }): Promise<Entity> {
    return this.c.post("/v1/entities", opts);
  }

  dissolve(id: EntityId): Promise<Entity> {
    return this.c.post(`/v1/entities/${id}/dissolve`, {});
  }
}

// ── Formation ────────────────────────────────────────────────────────────────

class FormationApi {
  constructor(private c: CorpClient) {}

  advance(entityId: EntityId): Promise<Entity> {
    return this.c.post(`/v1/formations/${entityId}/advance`, {});
  }

  listDocuments(entityId: EntityId): Promise<Document[]> {
    return this.c.get(`/v1/formations/${entityId}/documents`);
  }

  getDocument(entityId: EntityId, documentId: DocumentId): Promise<Document> {
    return this.c.get(`/v1/formations/${entityId}/documents/${documentId}`);
  }

  signDocument(documentId: DocumentId, opts: {
    signer_name: string;
    signer_role: string;
    signer_email: string;
    signature_text: string;
    consent_text: string;
    signature_svg?: string;
  }): Promise<Document> {
    return this.c.post(`/v1/documents/${documentId}/sign`, opts);
  }

  getFiling(entityId: EntityId): Promise<Filing> {
    return this.c.get(`/v1/formations/${entityId}/filing`);
  }

  confirmFiling(entityId: EntityId, confirmationNumber?: string): Promise<Filing> {
    return this.c.post(`/v1/formations/${entityId}/filing/confirm`, {
      confirmation_number: confirmationNumber ?? null,
    });
  }

  getTaxProfile(entityId: EntityId): Promise<TaxProfile> {
    return this.c.get(`/v1/formations/${entityId}/tax`);
  }

  confirmEin(entityId: EntityId, ein: string): Promise<TaxProfile> {
    return this.c.post(`/v1/formations/${entityId}/tax/confirm-ein`, { ein });
  }
}

// ── Equity ───────────────────────────────────────────────────────────────────

class EquityApi {
  constructor(private c: CorpClient) {}

  getCapTable(entityId: EntityId): Promise<CapTable> {
    return this.c.get(`/v1/entities/${entityId}/cap-table`);
  }

  createCapTable(entityId: EntityId): Promise<CapTable> {
    return this.c.post(`/v1/entities/${entityId}/cap-table`, {});
  }

  listShareClasses(entityId: EntityId): Promise<ShareClass[]> {
    return this.c.get(`/v1/entities/${entityId}/share-classes`);
  }

  createShareClass(entityId: EntityId, opts: {
    cap_table_id: string;
    class_code: string;
    stock_type: string;
    par_value: string;
    authorized_shares: number;
    liquidation_preference?: string;
  }): Promise<ShareClass> {
    return this.c.post(`/v1/entities/${entityId}/share-classes`, opts);
  }

  listGrants(entityId: EntityId): Promise<EquityGrant[]> {
    return this.c.get(`/v1/entities/${entityId}/grants`);
  }

  createGrant(entityId: EntityId, opts: Record<string, unknown>): Promise<EquityGrant> {
    return this.c.post(`/v1/entities/${entityId}/grants`, opts);
  }

  listSafes(entityId: EntityId): Promise<SafeNote[]> {
    return this.c.get(`/v1/entities/${entityId}/safes`);
  }

  issueSafe(entityId: EntityId, opts: Record<string, unknown>): Promise<SafeNote> {
    return this.c.post(`/v1/entities/${entityId}/safes`, opts);
  }

  convertSafe(entityId: EntityId, safeId: SafeNoteId): Promise<SafeNote> {
    return this.c.post(`/v1/entities/${entityId}/safes/${safeId}/convert`, {});
  }

  listValuations(entityId: EntityId): Promise<Valuation[]> {
    return this.c.get(`/v1/entities/${entityId}/valuations`);
  }

  createValuation(entityId: EntityId, opts: Record<string, unknown>): Promise<Valuation> {
    return this.c.post(`/v1/entities/${entityId}/valuations`, opts);
  }

  submitValuation(entityId: EntityId, valuationId: ValuationId): Promise<Valuation> {
    return this.c.post(`/v1/entities/${entityId}/valuations/${valuationId}/submit`, {});
  }

  approveValuation(entityId: EntityId, valuationId: ValuationId, approvedBy?: string): Promise<Valuation> {
    return this.c.post(`/v1/entities/${entityId}/valuations/${valuationId}/approve`, { approved_by: approvedBy });
  }
}

// ── Governance ───────────────────────────────────────────────────────────────

class GovernanceApi {
  constructor(private c: CorpClient) {}

  listBodies(entityId: EntityId): Promise<GovernanceBody[]> {
    return this.c.get(`/v1/entities/${entityId}/governance/bodies`);
  }

  createBody(entityId: EntityId, opts: {
    name: string;
    body_type: string;
    quorum_rule: string;
    voting_method: string;
  }): Promise<GovernanceBody> {
    return this.c.post(`/v1/entities/${entityId}/governance/bodies`, opts);
  }

  listSeats(entityId: EntityId): Promise<GovernanceSeat[]> {
    return this.c.get(`/v1/entities/${entityId}/governance/seats`);
  }

  createSeat(entityId: EntityId, opts: {
    body_id: GovernanceBodyId;
    holder_id: ContactId;
    role: string;
    appointed_date: string;
    voting_power: number;
    term_expiration?: string;
  }): Promise<GovernanceSeat> {
    return this.c.post(`/v1/entities/${entityId}/governance/seats`, opts);
  }

  resignSeat(entityId: EntityId, seatId: GovernanceSeatId): Promise<GovernanceSeat> {
    return this.c.post(`/v1/entities/${entityId}/governance/seats/${seatId}/resign`, {});
  }

  listMeetings(entityId: EntityId): Promise<Meeting[]> {
    return this.c.get(`/v1/entities/${entityId}/governance/meetings`);
  }

  createMeeting(entityId: EntityId, opts: {
    body_id: GovernanceBodyId;
    meeting_type: string;
    title: string;
    scheduled_date?: string;
    location?: string;
    notice_days?: number;
  }): Promise<Meeting> {
    return this.c.post(`/v1/entities/${entityId}/governance/meetings`, opts);
  }

  sendNotice(entityId: EntityId, meetingId: MeetingId): Promise<Meeting> {
    return this.c.post(`/v1/entities/${entityId}/governance/meetings/${meetingId}/notice`, {});
  }

  convene(entityId: EntityId, meetingId: MeetingId): Promise<Meeting> {
    return this.c.post(`/v1/entities/${entityId}/governance/meetings/${meetingId}/convene`, {});
  }

  adjourn(entityId: EntityId, meetingId: MeetingId): Promise<Meeting> {
    return this.c.post(`/v1/entities/${entityId}/governance/meetings/${meetingId}/adjourn`, {});
  }

  cancel(entityId: EntityId, meetingId: MeetingId): Promise<Meeting> {
    return this.c.post(`/v1/entities/${entityId}/governance/meetings/${meetingId}/cancel`, {});
  }

  recordAttendance(entityId: EntityId, meetingId: MeetingId, seatIds: GovernanceSeatId[]): Promise<Meeting> {
    return this.c.post(`/v1/entities/${entityId}/governance/meetings/${meetingId}/attendance`, { seat_ids: seatIds });
  }

  castVote(entityId: EntityId, meetingId: MeetingId, opts: {
    agenda_item_id: AgendaItemId;
    seat_id: GovernanceSeatId;
    value: "for" | "against" | "abstain" | "recusal";
  }): Promise<unknown> {
    return this.c.post(`/v1/entities/${entityId}/governance/meetings/${meetingId}/votes`, opts);
  }

  quickApprove(entityId: EntityId, opts: {
    body_id: GovernanceBodyId;
    title: string;
    description: string;
  }): Promise<unknown> {
    return this.c.post(`/v1/entities/${entityId}/governance/quick-approve`, opts);
  }

  writtenConsent(entityId: EntityId, opts: {
    body_id: GovernanceBodyId;
    title: string;
    description?: string;
  }): Promise<Meeting> {
    return this.c.post(`/v1/entities/${entityId}/governance/written-consent`, opts);
  }
}

// ── Treasury ─────────────────────────────────────────────────────────────────

class TreasuryApi {
  constructor(private c: CorpClient) {}

  listInvoices(entityId: EntityId): Promise<Invoice[]> {
    return this.c.get(`/v1/entities/${entityId}/invoices`);
  }

  createInvoice(entityId: EntityId, opts: Record<string, unknown>): Promise<Invoice> {
    return this.c.post(`/v1/entities/${entityId}/invoices`, opts);
  }

  sendInvoice(entityId: EntityId, invoiceId: InvoiceId): Promise<Invoice> {
    return this.c.post(`/v1/entities/${entityId}/invoices/${invoiceId}/send`, {});
  }

  payInvoice(entityId: EntityId, invoiceId: InvoiceId): Promise<Invoice> {
    return this.c.post(`/v1/entities/${entityId}/invoices/${invoiceId}/pay`, {});
  }

  listPayments(entityId: EntityId): Promise<Payment[]> {
    return this.c.get(`/v1/entities/${entityId}/payments`);
  }

  createPayment(entityId: EntityId, opts: Record<string, unknown>): Promise<Payment> {
    return this.c.post(`/v1/entities/${entityId}/payments`, opts);
  }

  listBankAccounts(entityId: EntityId): Promise<unknown[]> {
    return this.c.get(`/v1/entities/${entityId}/bank-accounts`);
  }

  createBankAccount(entityId: EntityId, opts: Record<string, unknown>): Promise<unknown> {
    return this.c.post(`/v1/entities/${entityId}/bank-accounts`, opts);
  }

  activateBankAccount(entityId: EntityId, bankId: BankAccountId): Promise<unknown> {
    return this.c.post(`/v1/entities/${entityId}/bank-accounts/${bankId}/activate`, {});
  }

  listPayrollRuns(entityId: EntityId): Promise<unknown[]> {
    return this.c.get(`/v1/entities/${entityId}/payroll-runs`);
  }

  createPayrollRun(entityId: EntityId, opts: Record<string, unknown>): Promise<unknown> {
    return this.c.post(`/v1/entities/${entityId}/payroll-runs`, opts);
  }
}

// ── Contacts ─────────────────────────────────────────────────────────────────

class ContactsApi {
  constructor(private c: CorpClient) {}

  list(entityId: EntityId): Promise<Contact[]> {
    return this.c.get(`/v1/entities/${entityId}/contacts`);
  }

  get(entityId: EntityId, contactId: ContactId): Promise<Contact> {
    return this.c.get(`/v1/entities/${entityId}/contacts/${contactId}`);
  }

  create(entityId: EntityId, opts: {
    name: string;
    email?: string;
    contact_type?: string;
    category?: string;
  }): Promise<Contact> {
    return this.c.post(`/v1/entities/${entityId}/contacts`, opts);
  }

  update(entityId: EntityId, contactId: ContactId, opts: Record<string, unknown>): Promise<Contact> {
    return this.c.patch(`/v1/entities/${entityId}/contacts/${contactId}`, opts);
  }

  deactivate(entityId: EntityId, contactId: ContactId): Promise<Contact> {
    return this.c.post(`/v1/entities/${entityId}/contacts/${contactId}/deactivate`, {});
  }
}

// ── Agents ───────────────────────────────────────────────────────────────────

class AgentsApi {
  constructor(private c: CorpClient) {}

  list(): Promise<Agent[]> {
    return this.c.get("/v1/agents");
  }

  get(agentId: AgentId): Promise<Agent> {
    return this.c.get(`/v1/agents/${agentId}`);
  }

  create(opts: { name: string; system_prompt?: string; model?: string }): Promise<Agent> {
    return this.c.post("/v1/agents", opts);
  }

  pause(agentId: AgentId): Promise<Agent> {
    return this.c.post(`/v1/agents/${agentId}/pause`, {});
  }

  resume(agentId: AgentId): Promise<Agent> {
    return this.c.post(`/v1/agents/${agentId}/resume`, {});
  }

  delete(agentId: AgentId): Promise<void> {
    return this.c.delete(`/v1/agents/${agentId}`);
  }
}

// ── Work Items ───────────────────────────────────────────────────────────────

class WorkItemsApi {
  constructor(private c: CorpClient) {}

  list(entityId: EntityId): Promise<WorkItem[]> {
    return this.c.get(`/v1/entities/${entityId}/work-items`);
  }

  create(entityId: EntityId, opts: {
    title: string;
    description: string;
    category: string;
    deadline?: string;
    asap?: boolean;
  }): Promise<WorkItem> {
    return this.c.post(`/v1/entities/${entityId}/work-items`, opts);
  }

  claim(entityId: EntityId, itemId: WorkItemId, opts: { claimed_by: string; claim_ttl_seconds?: number }): Promise<WorkItem> {
    return this.c.post(`/v1/entities/${entityId}/work-items/${itemId}/claim`, opts);
  }

  complete(entityId: EntityId, itemId: WorkItemId, opts: { completed_by: string; result?: string }): Promise<WorkItem> {
    return this.c.post(`/v1/entities/${entityId}/work-items/${itemId}/complete`, opts);
  }

  cancel(entityId: EntityId, itemId: WorkItemId): Promise<WorkItem> {
    return this.c.post(`/v1/entities/${entityId}/work-items/${itemId}/cancel`, {});
  }
}

// ── Admin ────────────────────────────────────────────────────────────────────

class AdminApi {
  constructor(private c: CorpClient) {}

  health(): Promise<{ status: string }> {
    return this.c.get("/health");
  }

  listWorkspaces(): Promise<unknown[]> {
    return this.c.get("/v1/workspaces");
  }

  listApiKeys(): Promise<unknown[]> {
    return this.c.get("/v1/api-keys");
  }

  createApiKey(opts: { name: string; scopes?: string[] }): Promise<ApiKeyResponse> {
    return this.c.post("/v1/api-keys", opts);
  }

  revokeApiKey(keyId: ApiKeyId): Promise<void> {
    return this.c.post(`/v1/api-keys/${keyId}/revoke`, {});
  }
}
