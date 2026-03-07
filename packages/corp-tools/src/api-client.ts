import type {
  AcceptEquityRoundRequest,
  ApiRecord,
  ApplyEquityRoundTermsRequest,
  BoardApproveEquityRoundRequest,
  CreateEquityRoundRequest,
  CreateExecutionIntentRequest,
  EquityRoundResponse,
  ExecuteRoundConversionRequest,
  IntentResponse,
  PreviewRoundConversionRequest,
} from "./types.js";

export class SessionExpiredError extends Error {
  constructor() {
    super("Your API key is no longer valid. Run 'corp setup' to re-authenticate.");
    this.name = "SessionExpiredError";
  }
}

async function extractErrorMessage(resp: Response): Promise<string> {
  try {
    const text = await resp.text();
    try {
      const json = JSON.parse(text);
      return json.error || json.message || json.detail || text;
    } catch {
      return text;
    }
  } catch {
    return resp.statusText;
  }
}

export async function provisionWorkspace(
  apiUrl: string,
  name?: string
): Promise<ApiRecord> {
  const url = `${apiUrl.replace(/\/+$/, "")}/v1/workspaces/provision`;
  const body: Record<string, string> = {};
  if (name) body.name = name;
  const resp = await fetch(url, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });
  if (!resp.ok) {
    const detail = await extractErrorMessage(resp);
    throw new Error(`Provision failed: ${resp.status} ${resp.statusText} — ${detail}`);
  }
  return resp.json() as Promise<ApiRecord>;
}

export class CorpAPIClient {
  readonly apiUrl: string;
  readonly apiKey: string;
  readonly workspaceId: string;

  constructor(apiUrl: string, apiKey: string, workspaceId: string) {
    this.apiUrl = apiUrl.replace(/\/+$/, "");
    this.apiKey = apiKey;
    this.workspaceId = workspaceId;
  }

  private headers(): Record<string, string> {
    return {
      Authorization: `Bearer ${this.apiKey}`,
      "Content-Type": "application/json",
      Accept: "application/json",
    };
  }

  private async request(method: string, path: string, body?: unknown, params?: Record<string, string>): Promise<Response> {
    let url = `${this.apiUrl}${path}`;
    if (params) {
      const qs = new URLSearchParams(params).toString();
      if (qs) url += `?${qs}`;
    }
    const opts: RequestInit = { method, headers: this.headers() };
    if (body !== undefined) opts.body = JSON.stringify(body);
    return fetch(url, opts);
  }

  private async throwIfError(resp: Response): Promise<void> {
    if (resp.status === 401) throw new SessionExpiredError();
    if (!resp.ok) {
      const detail = await extractErrorMessage(resp);
      throw new Error(`${resp.status} ${resp.statusText} — ${detail}`);
    }
  }

  private async get(path: string, params?: Record<string, string>): Promise<unknown> {
    const resp = await this.request("GET", path, undefined, params);
    await this.throwIfError(resp);
    return resp.json();
  }

  private async post(path: string, body?: unknown): Promise<unknown> {
    const resp = await this.request("POST", path, body);
    await this.throwIfError(resp);
    return resp.json();
  }

  private async postWithParams(path: string, body: unknown, params: Record<string, string>): Promise<unknown> {
    const resp = await this.request("POST", path, body, params);
    await this.throwIfError(resp);
    return resp.json();
  }

  private async patch(path: string, body?: unknown): Promise<unknown> {
    const resp = await this.request("PATCH", path, body);
    await this.throwIfError(resp);
    return resp.json();
  }

  private async del(path: string): Promise<void> {
    const resp = await this.request("DELETE", path);
    await this.throwIfError(resp);
  }

  // --- Workspace ---
  getStatus() { return this.get(`/v1/workspaces/${this.workspaceId}/status`) as Promise<ApiRecord>; }

  // --- Obligations ---
  getObligations(tier?: string) {
    const params: Record<string, string> = {};
    if (tier) params.tier = tier;
    return this.get("/v1/obligations/summary", params) as Promise<ApiRecord>;
  }

  // --- Digests ---
  listDigests() { return this.get("/v1/digests") as Promise<ApiRecord[]>; }
  triggerDigest() { return this.post("/v1/digests/trigger") as Promise<ApiRecord>; }
  getDigest(key: string) { return this.get(`/v1/digests/${key}`) as Promise<ApiRecord>; }

  // --- Entities ---
  listEntities() { return this.get(`/v1/workspaces/${this.workspaceId}/entities`) as Promise<ApiRecord[]>; }

  // --- Contacts ---
  listContacts(entityId: string) { return this.get(`/v1/entities/${entityId}/contacts`) as Promise<ApiRecord[]>; }
  getContact(id: string, entityId: string) { return this.get(`/v1/contacts/${id}`, { entity_id: entityId }) as Promise<ApiRecord>; }
  getContactProfile(id: string, entityId: string) { return this.get(`/v1/contacts/${id}/profile`, { entity_id: entityId }) as Promise<ApiRecord>; }
  createContact(data: ApiRecord) { return this.post("/v1/contacts", data) as Promise<ApiRecord>; }
  updateContact(id: string, data: ApiRecord) { return this.patch(`/v1/contacts/${id}`, data) as Promise<ApiRecord>; }
  getNotificationPrefs(contactId: string) { return this.get(`/v1/contacts/${contactId}/notification-prefs`) as Promise<ApiRecord>; }
  updateNotificationPrefs(contactId: string, prefs: ApiRecord) { return this.patch(`/v1/contacts/${contactId}/notification-prefs`, prefs) as Promise<ApiRecord>; }

  // --- Cap Table ---
  getCapTable(entityId: string) { return this.get(`/v1/entities/${entityId}/cap-table`) as Promise<ApiRecord>; }
  /** Extract SAFE instruments from the cap table (no dedicated list endpoint). */
  async getSafeNotes(entityId: string): Promise<ApiRecord[]> {
    const ct = await this.getCapTable(entityId);
    const instruments = (ct.instruments ?? []) as ApiRecord[];
    const positions = (ct.positions ?? []) as ApiRecord[];
    const safeIds = new Set(instruments.filter((i) => String(i.kind).toLowerCase() === "safe").map((i) => i.instrument_id));
    if (safeIds.size === 0) return [];
    return positions.filter((p) => safeIds.has(p.instrument_id));
  }
  /** Extract transfer-workflow info (no dedicated list endpoint for share transfers). */
  async getShareTransfers(entityId: string): Promise<ApiRecord[]> {
    // No list endpoint exists; return empty with a hint.
    return [{ _note: "Use transfer workflows: POST /v1/equity/transfer-workflows to initiate transfers.", entity_id: entityId }];
  }
  getValuations(entityId: string) { return this.get(`/v1/entities/${entityId}/valuations`) as Promise<ApiRecord[]>; }
  getCurrent409a(entityId: string) { return this.get(`/v1/entities/${entityId}/current-409a`) as Promise<ApiRecord>; }
  createValuation(data: ApiRecord) { return this.post("/v1/valuations", data) as Promise<ApiRecord>; }
  submitValuationForApproval(valuationId: string, entityId: string) {
    return this.post(`/v1/valuations/${valuationId}/submit-for-approval`, { entity_id: entityId }) as Promise<ApiRecord>;
  }
  approveValuation(valuationId: string, entityId: string, resolutionId?: string) {
    const body: ApiRecord = { entity_id: entityId };
    if (resolutionId) body.resolution_id = resolutionId;
    return this.post(`/v1/valuations/${valuationId}/approve`, body) as Promise<ApiRecord>;
  }
  transferShares(data: ApiRecord) { return this.post("/v1/equity/transfer-workflows", data) as Promise<ApiRecord>; }
  calculateDistribution(data: ApiRecord) { return this.post("/v1/distributions", data) as Promise<ApiRecord>; }

  // --- Equity rounds (v1) ---
  createEquityRound(data: CreateEquityRoundRequest) {
    return this.post("/v1/equity/rounds", data) as Promise<EquityRoundResponse>;
  }
  applyEquityRoundTerms(roundId: string, data: ApplyEquityRoundTermsRequest) {
    return this.post(`/v1/equity/rounds/${roundId}/apply-terms`, data) as Promise<ApiRecord>;
  }
  boardApproveEquityRound(roundId: string, data: BoardApproveEquityRoundRequest) {
    return this.post(`/v1/equity/rounds/${roundId}/board-approve`, data) as Promise<EquityRoundResponse>;
  }
  acceptEquityRound(roundId: string, data: AcceptEquityRoundRequest) {
    return this.post(`/v1/equity/rounds/${roundId}/accept`, data) as Promise<EquityRoundResponse>;
  }
  previewRoundConversion(data: PreviewRoundConversionRequest) {
    return this.post("/v1/equity/conversions/preview", data) as Promise<ApiRecord>;
  }
  executeRoundConversion(data: ExecuteRoundConversionRequest) {
    return this.post("/v1/equity/conversions/execute", data) as Promise<ApiRecord>;
  }

  // --- Staged equity rounds ---
  startEquityRound(data: ApiRecord) { return this.post("/v1/equity/rounds/staged", data) as Promise<ApiRecord>; }
  addRoundSecurity(roundId: string, data: ApiRecord) { return this.post(`/v1/equity/rounds/${roundId}/securities`, data) as Promise<ApiRecord>; }
  issueRound(roundId: string, data: ApiRecord) { return this.post(`/v1/equity/rounds/${roundId}/issue`, data) as Promise<ApiRecord>; }

  // --- Intent lifecycle helpers ---
  createExecutionIntent(data: CreateExecutionIntentRequest) {
    return this.post("/v1/execution/intents", data) as Promise<IntentResponse>;
  }
  evaluateIntent(intentId: string, entityId: string) {
    return this.postWithParams(`/v1/intents/${intentId}/evaluate`, {}, { entity_id: entityId }) as Promise<ApiRecord>;
  }
  authorizeIntent(intentId: string, entityId: string) {
    return this.postWithParams(`/v1/intents/${intentId}/authorize`, {}, { entity_id: entityId }) as Promise<ApiRecord>;
  }

  // --- Governance ---
  listGovernanceBodies(entityId: string) { return this.get(`/v1/entities/${entityId}/governance-bodies`) as Promise<ApiRecord[]>; }
  getGovernanceSeats(bodyId: string) { return this.get(`/v1/governance-bodies/${bodyId}/seats`) as Promise<ApiRecord[]>; }
  listMeetings(bodyId: string) { return this.get(`/v1/governance-bodies/${bodyId}/meetings`) as Promise<ApiRecord[]>; }
  getMeetingResolutions(meetingId: string) { return this.get(`/v1/meetings/${meetingId}/resolutions`) as Promise<ApiRecord[]>; }
  scheduleMeeting(data: ApiRecord) { return this.post("/v1/meetings", data) as Promise<ApiRecord>; }
  conveneMeeting(meetingId: string, entityId: string, data: ApiRecord) {
    return this.postWithParams(`/v1/meetings/${meetingId}/convene`, data, { entity_id: entityId }) as Promise<ApiRecord>;
  }
  castVote(entityId: string, meetingId: string, itemId: string, data: ApiRecord) {
    return this.postWithParams(`/v1/meetings/${meetingId}/agenda-items/${itemId}/vote`, data, { entity_id: entityId }) as Promise<ApiRecord>;
  }
  sendNotice(meetingId: string, entityId: string) {
    return this.postWithParams(`/v1/meetings/${meetingId}/notice`, {}, { entity_id: entityId }) as Promise<ApiRecord>;
  }
  adjournMeeting(meetingId: string, entityId: string) {
    return this.postWithParams(`/v1/meetings/${meetingId}/adjourn`, {}, { entity_id: entityId }) as Promise<ApiRecord>;
  }
  cancelMeeting(meetingId: string, entityId: string) {
    return this.postWithParams(`/v1/meetings/${meetingId}/cancel`, {}, { entity_id: entityId }) as Promise<ApiRecord>;
  }
  finalizeAgendaItem(meetingId: string, itemId: string, data: ApiRecord) {
    return this.post(`/v1/meetings/${meetingId}/agenda-items/${itemId}/finalize`, data) as Promise<ApiRecord>;
  }
  computeResolution(meetingId: string, itemId: string, entityId: string, data: ApiRecord) {
    return this.postWithParams(`/v1/meetings/${meetingId}/agenda-items/${itemId}/resolution`, data, { entity_id: entityId }) as Promise<ApiRecord>;
  }
  attachResolutionDocument(meetingId: string, resolutionId: string, data: ApiRecord) {
    return this.post(`/v1/meetings/${meetingId}/resolutions/${resolutionId}/attach-document`, data) as Promise<ApiRecord>;
  }
  writtenConsent(data: ApiRecord) {
    return this.post("/v1/meetings/written-consent", data) as Promise<ApiRecord>;
  }
  listAgendaItems(meetingId: string, entityId: string) {
    return this.get(`/v1/meetings/${meetingId}/agenda-items`, { entity_id: entityId }) as Promise<ApiRecord[]>;
  }
  listVotes(meetingId: string, itemId: string, entityId: string) {
    return this.get(`/v1/meetings/${meetingId}/agenda-items/${itemId}/votes`, { entity_id: entityId }) as Promise<ApiRecord[]>;
  }

  // --- Documents ---
  getEntityDocuments(entityId: string) { return this.get(`/v1/formations/${entityId}/documents`) as Promise<ApiRecord[]>; }
  generateContract(data: ApiRecord) { return this.post("/v1/contracts", data) as Promise<ApiRecord>; }
  getSigningLink(documentId: string, entityId: string) {
    return this.get(`/v1/sign/${documentId}`, { entity_id: entityId }) as Promise<ApiRecord>;
  }

  // --- Finance ---
  createInvoice(data: ApiRecord) { return this.post("/v1/treasury/invoices", data) as Promise<ApiRecord>; }
  runPayroll(data: ApiRecord) { return this.post("/v1/payroll/runs", data) as Promise<ApiRecord>; }
  submitPayment(data: ApiRecord) { return this.post("/v1/payments", data) as Promise<ApiRecord>; }
  openBankAccount(data: ApiRecord) { return this.post("/v1/treasury/bank-accounts", data) as Promise<ApiRecord>; }
  classifyContractor(data: ApiRecord) { return this.post("/v1/contractors/classify", data) as Promise<ApiRecord>; }
  reconcileLedger(data: ApiRecord) { return this.post("/v1/ledger/reconcile", data) as Promise<ApiRecord>; }

  // --- Tax ---
  fileTaxDocument(data: ApiRecord) { return this.post("/v1/tax/filings", data) as Promise<ApiRecord>; }
  trackDeadline(data: ApiRecord) { return this.post("/v1/deadlines", data) as Promise<ApiRecord>; }

  // --- Billing ---
  getBillingStatus() { return this.get("/v1/billing/status", { workspace_id: this.workspaceId }) as Promise<ApiRecord>; }
  getBillingPlans() {
    return (this.get("/v1/billing/plans") as Promise<unknown>).then((data) => {
      if (typeof data === "object" && data !== null && "plans" in data) {
        return (data as { plans: ApiRecord[] }).plans;
      }
      return data as ApiRecord[];
    });
  }
  createBillingPortal() { return this.post("/v1/billing/portal", { workspace_id: this.workspaceId }) as Promise<ApiRecord>; }
  createBillingCheckout(planId: string, entityId?: string) {
    const body: ApiRecord = { plan_id: planId };
    if (entityId) body.entity_id = entityId;
    return this.post("/v1/billing/checkout", body) as Promise<ApiRecord>;
  }

  // --- Formations ---
  getFormation(id: string) { return this.get(`/v1/formations/${id}`) as Promise<ApiRecord>; }
  getFormationDocuments(id: string) { return this.get(`/v1/formations/${id}/documents`) as Promise<ApiRecord[]>; }
  createFormation(data: ApiRecord) { return this.post("/v1/formations", data) as Promise<ApiRecord>; }
  createFormationWithCapTable(data: ApiRecord) { return this.post("/v1/formations/with-cap-table", data) as Promise<ApiRecord>; }
  createPendingEntity(data: ApiRecord) { return this.post("/v1/formations/pending", data) as Promise<ApiRecord>; }
  addFounder(entityId: string, data: ApiRecord) { return this.post(`/v1/formations/${entityId}/founders`, data) as Promise<ApiRecord>; }
  finalizeFormation(entityId: string) { return this.post(`/v1/formations/${entityId}/finalize`, {}) as Promise<ApiRecord>; }

  // --- Human obligations ---
  getHumanObligations() { return this.get(`/v1/workspaces/${this.workspaceId}/human-obligations`) as Promise<ApiRecord[]>; }
  getSignerToken(obligationId: string) { return this.post(`/v1/human-obligations/${obligationId}/signer-token`) as Promise<ApiRecord>; }

  // --- Demo ---
  seedDemo(name: string) { return this.post("/v1/demo/seed", { name, workspace_id: this.workspaceId }) as Promise<ApiRecord>; }

  // --- Entities writes ---
  convertEntity(entityId: string, data: ApiRecord) { return this.post(`/v1/entities/${entityId}/convert`, data) as Promise<ApiRecord>; }
  dissolveEntity(entityId: string, data: ApiRecord) { return this.post(`/v1/entities/${entityId}/dissolve`, data) as Promise<ApiRecord>; }

  // --- Agents ---
  listAgents() { return this.get("/v1/agents") as Promise<ApiRecord[]>; }
  getAgent(id: string) { return this.get(`/v1/agents/${id}/resolved`) as Promise<ApiRecord>; }
  createAgent(data: ApiRecord) { return this.post("/v1/agents", data) as Promise<ApiRecord>; }
  updateAgent(id: string, data: ApiRecord) { return this.patch(`/v1/agents/${id}`, data) as Promise<ApiRecord>; }
  deleteAgent(id: string) { return this.patch(`/v1/agents/${id}`, { status: "disabled" }) as Promise<ApiRecord>; }
  sendAgentMessage(id: string, message: string) { return this.post(`/v1/agents/${id}/messages`, { message }) as Promise<ApiRecord>; }
  addAgentSkill(id: string, data: ApiRecord) { return this.post(`/v1/agents/${id}/skills`, data) as Promise<ApiRecord>; }
  listSupportedModels() { return this.get("/v1/models") as Promise<ApiRecord[]>; }

  // --- Governance bodies ---
  createGovernanceBody(data: ApiRecord) { return this.post("/v1/governance-bodies", data) as Promise<ApiRecord>; }
  createGovernanceSeat(bodyId: string, data: ApiRecord) { return this.post(`/v1/governance-bodies/${bodyId}/seats`, data) as Promise<ApiRecord>; }

  // --- API Keys ---
  listApiKeys() { return this.get("/v1/api-keys", { workspace_id: this.workspaceId }) as Promise<ApiRecord[]>; }

  // --- Obligations ---
  assignObligation(obligationId: string, contactId: string) {
    return this.patch(`/v1/obligations/${obligationId}/assign`, { contact_id: contactId }) as Promise<ApiRecord>;
  }

  // --- Config ---
  getConfig() { return this.get("/v1/config") as Promise<ApiRecord>; }

  // --- Link/Claim ---
  async createLink(externalId: string, provider: string): Promise<ApiRecord> {
    const resp = await this.request("POST", "/v1/workspaces/link", { external_id: externalId, provider });
    if (!resp.ok) {
      const detail = await extractErrorMessage(resp);
      throw new Error(`${resp.status} ${resp.statusText} — ${detail}`);
    }
    return resp.json() as Promise<ApiRecord>;
  }
}
