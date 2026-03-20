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
import type {
  WorkspaceStatusResponse,
  FormationResponse,
  FormationWithCapTableResponse,
  PendingFormationResponse,
  AddFounderResponse,
  GovernanceBodyResponse,
  GovernanceSeatResponse,
  MeetingResponse,
  ResolutionResponse,
  AgendaItemResponse,
  VoteResponse,
  WrittenConsentResponse,
  ObligationResponse,
  DocumentResponse,
  DigestSummary,
  DigestTriggerResponse,
  NextStepsResponse,
} from "./api-schemas.js";
import { processRequest } from "./process-transport.js";

export class SessionExpiredError extends Error {
  constructor(detail?: string) {
    super(detail || "Your API key is no longer valid. Run 'corp setup' to re-authenticate.");
    this.name = "SessionExpiredError";
  }
}

const MAX_ERROR_DETAIL_LEN = 500;

function sanitizeErrorDetail(value: string): string {
  const sanitized = value
    .replace(/[\u0000-\u001f\u007f-\u009f\u001b]/g, " ")
    .replace(/\s+/g, " ")
    .trim();
  if (!sanitized) {
    return "request failed";
  }
  return sanitized.length > MAX_ERROR_DETAIL_LEN
    ? `${sanitized.slice(0, MAX_ERROR_DETAIL_LEN)}...`
    : sanitized;
}

function pathSegment(value: string): string {
  return encodeURIComponent(String(value));
}

async function extractErrorMessage(resp: Response): Promise<string> {
  try {
    const text = await resp.text();
    try {
      const json = JSON.parse(text);
      const val = json.error || json.message || json.detail;
      if (val == null) return sanitizeErrorDetail(text);
      return sanitizeErrorDetail(typeof val === "string" ? val : JSON.stringify(val));
    } catch {
      return sanitizeErrorDetail(text);
    }
  } catch {
    return sanitizeErrorDetail(resp.statusText);
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
    const prefix = resp.status >= 500
      ? "Server error"
      : resp.status === 404
        ? "Not found"
        : resp.status === 422
          ? "Validation error"
          : `HTTP ${resp.status}`;
    throw new Error(`Provision failed (${prefix}): ${detail}`);
  }
  return resp.json() as Promise<ApiRecord>;
}

export class CorpAPIClient {
  readonly apiUrl: string;
  readonly apiKey: string;
  readonly workspaceId: string;

  constructor(apiUrl: string, apiKey: string, workspaceId: string) {
    this.apiUrl = apiUrl.startsWith("process://") ? apiUrl : apiUrl.replace(/\/+$/, "");
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
    let fullPath = path;
    if (params) {
      const qs = new URLSearchParams(params).toString();
      if (qs) fullPath += `?${qs}`;
    }

    if (this.apiUrl.startsWith("process://")) {
      const hdrs = this.headers();
      const bodyStr = body !== undefined ? JSON.stringify(body) : undefined;
      return processRequest(this.apiUrl, method, fullPath, hdrs, bodyStr);
    }

    const url = `${this.apiUrl}${fullPath}`;
    const opts: RequestInit = { method, headers: this.headers() };
    if (body !== undefined) opts.body = JSON.stringify(body);
    return fetch(url, opts);
  }

  private async throwIfError(resp: Response): Promise<void> {
    if (!resp.ok) {
      const detail = await extractErrorMessage(resp);
      if (resp.status === 401) throw new SessionExpiredError(detail);
      const prefix = resp.status >= 500
        ? "Server error"
        : resp.status === 404
          ? "Not found"
          : resp.status === 422
            ? "Validation error"
            : `HTTP ${resp.status}`;
      throw new Error(`${prefix}: ${detail}`);
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

  /** Public generic GET for declarative/registry-driven commands. */
  async fetchJSON(path: string, params?: Record<string, string>): Promise<unknown> {
    return this.get(path, params);
  }

  // --- Workspace ---
  getStatus() { return this.get(`/v1/workspaces/${pathSegment(this.workspaceId)}/status`) as Promise<WorkspaceStatusResponse>; }

  // --- Obligations ---
  getObligations(tier?: string) {
    const params: Record<string, string> = {};
    if (tier) params.tier = tier;
    return this.get("/v1/obligations/summary", params) as Promise<ApiRecord>;
  }

  // --- Digests ---
  listDigests() { return this.get("/v1/digests") as Promise<DigestSummary[]>; }
  triggerDigest() { return this.post("/v1/digests/trigger") as Promise<DigestTriggerResponse>; }
  getDigest(key: string) { return this.get(`/v1/digests/${pathSegment(key)}`) as Promise<DigestSummary>; }

  // --- References ---
  syncReferences(kind: string, items: Array<{ resource_id: string; label: string }>, entityId?: string) {
    const body: ApiRecord = { kind, items };
    if (entityId) body.entity_id = entityId;
    return this.post("/v1/references/sync", body) as Promise<{ references: ApiRecord[] }>;
  }

  // --- Entities ---
  listEntities() { return this.get("/v1/entities") as Promise<ApiRecord[]>; }

  // --- Contacts ---
  listContacts(entityId: string) { return this.get(`/v1/entities/${pathSegment(entityId)}/contacts`) as Promise<ApiRecord[]>; }
  getContact(id: string, entityId: string) { return this.get(`/v1/contacts/${pathSegment(id)}`, { entity_id: entityId }) as Promise<ApiRecord>; }
  getContactProfile(id: string, entityId: string) { return this.get(`/v1/contacts/${pathSegment(id)}/profile`, { entity_id: entityId }) as Promise<ApiRecord>; }
  createContact(data: ApiRecord) { return this.post("/v1/contacts", data) as Promise<ApiRecord>; }
  updateContact(id: string, data: ApiRecord) { return this.patch(`/v1/contacts/${pathSegment(id)}`, data) as Promise<ApiRecord>; }
  getNotificationPrefs(contactId: string) { return this.get(`/v1/contacts/${pathSegment(contactId)}/notification-prefs`) as Promise<ApiRecord>; }
  updateNotificationPrefs(contactId: string, prefs: ApiRecord) { return this.patch(`/v1/contacts/${pathSegment(contactId)}/notification-prefs`, prefs) as Promise<ApiRecord>; }

  // --- Cap Table ---
  getCapTable(entityId: string) { return this.get(`/v1/entities/${pathSegment(entityId)}/cap-table`) as Promise<ApiRecord>; }
  async getSafeNotes(entityId: string): Promise<ApiRecord[]> {
    return this.get(`/v1/entities/${pathSegment(entityId)}/safe-notes`) as Promise<ApiRecord[]>;
  }
  createSafeNote(data: ApiRecord) { return this.post("/v1/safe-notes", data) as Promise<ApiRecord>; }
  listShareTransfers(entityId: string) { return this.get(`/v1/entities/${pathSegment(entityId)}/share-transfers`) as Promise<ApiRecord[]>; }
  getShareTransfers(entityId: string) { return this.listShareTransfers(entityId); }
  getValuations(entityId: string) { return this.get(`/v1/entities/${pathSegment(entityId)}/valuations`) as Promise<ApiRecord[]>; }
  getCurrent409a(entityId: string) { return this.get(`/v1/entities/${pathSegment(entityId)}/current-409a`) as Promise<ApiRecord>; }
  createValuation(data: ApiRecord) { return this.post("/v1/valuations", data) as Promise<ApiRecord>; }
  createInstrument(data: ApiRecord) { return this.post("/v1/equity/instruments", data) as Promise<ApiRecord>; }
  submitValuationForApproval(valuationId: string, entityId: string) {
    return this.post(`/v1/valuations/${pathSegment(valuationId)}/submit-for-approval`, { entity_id: entityId }) as Promise<ApiRecord>;
  }
  approveValuation(valuationId: string, entityId: string, resolutionId?: string) {
    const body: ApiRecord = { entity_id: entityId };
    if (resolutionId) body.resolution_id = resolutionId;
    return this.post(`/v1/valuations/${pathSegment(valuationId)}/approve`, body) as Promise<ApiRecord>;
  }
  transferShares(data: ApiRecord) { return this.post("/v1/equity/transfer-workflows", data) as Promise<ApiRecord>; }
  calculateDistribution(data: ApiRecord) { return this.post("/v1/distributions", data) as Promise<ApiRecord>; }

  // --- Equity rounds (v1) ---
  createEquityRound(data: CreateEquityRoundRequest) {
    return this.post("/v1/equity/rounds", data) as Promise<EquityRoundResponse>;
  }
  applyEquityRoundTerms(roundId: string, data: ApplyEquityRoundTermsRequest) {
    return this.post(`/v1/equity/rounds/${pathSegment(roundId)}/apply-terms`, data) as Promise<ApiRecord>;
  }
  boardApproveEquityRound(roundId: string, data: BoardApproveEquityRoundRequest) {
    return this.post(`/v1/equity/rounds/${pathSegment(roundId)}/board-approve`, data) as Promise<EquityRoundResponse>;
  }
  acceptEquityRound(roundId: string, data: AcceptEquityRoundRequest) {
    return this.post(`/v1/equity/rounds/${pathSegment(roundId)}/accept`, data) as Promise<EquityRoundResponse>;
  }
  previewRoundConversion(data: PreviewRoundConversionRequest) {
    return this.post("/v1/equity/conversions/preview", data) as Promise<ApiRecord>;
  }
  executeRoundConversion(data: ExecuteRoundConversionRequest) {
    return this.post("/v1/equity/conversions/execute", data) as Promise<ApiRecord>;
  }

  // --- Staged equity rounds ---
  listEquityRounds(entityId: string) { return this.get(`/v1/entities/${pathSegment(entityId)}/equity-rounds`) as Promise<ApiRecord[]>; }
  startEquityRound(data: ApiRecord) { return this.post("/v1/equity/rounds/staged", data) as Promise<ApiRecord>; }
  addRoundSecurity(roundId: string, data: ApiRecord) { return this.post(`/v1/equity/rounds/${pathSegment(roundId)}/securities`, data) as Promise<ApiRecord>; }
  issueRound(roundId: string, data: ApiRecord) { return this.post(`/v1/equity/rounds/${pathSegment(roundId)}/issue`, data) as Promise<ApiRecord>; }

  // --- Intent lifecycle helpers ---
  createExecutionIntent(data: CreateExecutionIntentRequest) {
    return this.post("/v1/execution/intents", data) as Promise<IntentResponse>;
  }
  evaluateIntent(intentId: string, entityId: string) {
    return this.postWithParams(`/v1/intents/${pathSegment(intentId)}/evaluate`, {}, { entity_id: entityId }) as Promise<ApiRecord>;
  }
  authorizeIntent(intentId: string, entityId: string) {
    return this.postWithParams(`/v1/intents/${pathSegment(intentId)}/authorize`, {}, { entity_id: entityId }) as Promise<ApiRecord>;
  }

  // --- Governance ---
  listGovernanceBodies(entityId: string) { return this.get(`/v1/entities/${pathSegment(entityId)}/governance-bodies`) as Promise<GovernanceBodyResponse[]>; }
  getGovernanceSeats(bodyId: string, entityId: string) {
    return this.get(`/v1/governance-bodies/${pathSegment(bodyId)}/seats`, { entity_id: entityId }) as Promise<GovernanceSeatResponse[]>;
  }
  listMeetings(bodyId: string, entityId: string) {
    return this.get(`/v1/governance-bodies/${pathSegment(bodyId)}/meetings`, { entity_id: entityId }) as Promise<MeetingResponse[]>;
  }
  getMeetingResolutions(meetingId: string, entityId: string) {
    return this.get(`/v1/meetings/${pathSegment(meetingId)}/resolutions`, { entity_id: entityId }) as Promise<ResolutionResponse[]>;
  }
  scheduleMeeting(data: ApiRecord) { return this.post("/v1/meetings", data) as Promise<MeetingResponse>; }
  conveneMeeting(meetingId: string, entityId: string, data: ApiRecord) {
    return this.postWithParams(`/v1/meetings/${pathSegment(meetingId)}/convene`, data, { entity_id: entityId }) as Promise<MeetingResponse>;
  }
  castVote(entityId: string, meetingId: string, itemId: string, data: ApiRecord) {
    return this.postWithParams(`/v1/meetings/${pathSegment(meetingId)}/agenda-items/${pathSegment(itemId)}/vote`, data, { entity_id: entityId }) as Promise<VoteResponse>;
  }
  sendNotice(meetingId: string, entityId: string) {
    return this.postWithParams(`/v1/meetings/${pathSegment(meetingId)}/notice`, {}, { entity_id: entityId }) as Promise<MeetingResponse>;
  }
  adjournMeeting(meetingId: string, entityId: string) {
    return this.postWithParams(`/v1/meetings/${pathSegment(meetingId)}/adjourn`, {}, { entity_id: entityId }) as Promise<MeetingResponse>;
  }
  reopenMeeting(meetingId: string, entityId: string) {
    return this.postWithParams(`/v1/meetings/${pathSegment(meetingId)}/reopen`, {}, { entity_id: entityId }) as Promise<MeetingResponse>;
  }
  cancelMeeting(meetingId: string, entityId: string) {
    return this.postWithParams(`/v1/meetings/${pathSegment(meetingId)}/cancel`, {}, { entity_id: entityId }) as Promise<MeetingResponse>;
  }
  finalizeAgendaItem(meetingId: string, itemId: string, data: ApiRecord) {
    return this.post(`/v1/meetings/${pathSegment(meetingId)}/agenda-items/${pathSegment(itemId)}/finalize`, data) as Promise<AgendaItemResponse>;
  }
  computeResolution(meetingId: string, itemId: string, entityId: string, data: ApiRecord) {
    return this.postWithParams(`/v1/meetings/${pathSegment(meetingId)}/agenda-items/${pathSegment(itemId)}/resolution`, data, { entity_id: entityId }) as Promise<ResolutionResponse>;
  }
  attachResolutionDocument(meetingId: string, resolutionId: string, data: ApiRecord) {
    return this.post(`/v1/meetings/${pathSegment(meetingId)}/resolutions/${pathSegment(resolutionId)}/attach-document`, data) as Promise<ResolutionResponse>;
  }
  writtenConsent(data: ApiRecord) {
    return this.post("/v1/meetings/written-consent", data) as Promise<WrittenConsentResponse>;
  }
  getGovernanceMode(entityId: string) {
    return this.get("/v1/governance/mode", { entity_id: entityId }) as Promise<ApiRecord>;
  }
  setGovernanceMode(data: ApiRecord) {
    return this.post("/v1/governance/mode", data) as Promise<ApiRecord>;
  }
  resignSeat(seatId: string, entityId: string) {
    return this.post(`/v1/governance-seats/${pathSegment(seatId)}/resign`, { entity_id: entityId }) as Promise<ApiRecord>;
  }
  createGovernanceIncident(data: ApiRecord) {
    return this.post("/v1/governance/incidents", data) as Promise<ApiRecord>;
  }
  listGovernanceIncidents(entityId: string) {
    return this.get(`/v1/entities/${pathSegment(entityId)}/governance/incidents`) as Promise<ApiRecord[]>;
  }
  resolveGovernanceIncident(incidentId: string, data: ApiRecord) {
    return this.post(`/v1/governance/incidents/${pathSegment(incidentId)}/resolve`, data) as Promise<ApiRecord>;
  }
  getGovernanceProfile(entityId: string) {
    return this.get(`/v1/entities/${pathSegment(entityId)}/governance/profile`) as Promise<ApiRecord>;
  }
  listAgendaItems(meetingId: string, entityId: string) {
    return this.get(`/v1/meetings/${pathSegment(meetingId)}/agenda-items`, { entity_id: entityId }) as Promise<AgendaItemResponse[]>;
  }
  listVotes(meetingId: string, itemId: string, entityId: string) {
    return this.get(`/v1/meetings/${pathSegment(meetingId)}/agenda-items/${pathSegment(itemId)}/votes`, { entity_id: entityId }) as Promise<VoteResponse[]>;
  }

  // --- Documents ---
  getEntityDocuments(entityId: string) { return this.get(`/v1/formations/${pathSegment(entityId)}/documents`) as Promise<DocumentResponse[]>; }
  getDocument(documentId: string, entityId: string) {
    return this.get(`/v1/documents/${pathSegment(documentId)}`, { entity_id: entityId }) as Promise<DocumentResponse>;
  }
  signDocument(documentId: string, entityId: string, data: ApiRecord) {
    return this.postWithParams(`/v1/documents/${pathSegment(documentId)}/sign`, data, { entity_id: entityId }) as Promise<ApiRecord>;
  }
  generateContract(data: ApiRecord) { return this.post("/v1/contracts", data) as Promise<ApiRecord>; }
  getSigningLink(documentId: string, entityId: string) {
    return this.get(`/v1/sign/${pathSegment(documentId)}`, { entity_id: entityId }) as Promise<ApiRecord>;
  }
  async validatePreviewPdf(entityId: string, documentId: string): Promise<ApiRecord> {
    const resp = await this.request("GET", "/v1/documents/preview/pdf/validate", undefined, { entity_id: entityId, document_id: documentId });
    await this.throwIfError(resp);
    return { entity_id: entityId, document_id: documentId };
  }
  getPreviewPdfUrl(entityId: string, documentId: string): string {
    if (this.apiUrl.startsWith("process://")) {
      throw new Error(
        "PDF preview is not available in local process transport mode.\n" +
        "  Use cloud mode (npx corp setup) or start a local HTTP server (npx corp serve) instead.",
      );
    }
    const qs = new URLSearchParams({ entity_id: entityId, document_id: documentId }).toString();
    return `${this.apiUrl}/v1/documents/preview/pdf?${qs}`;
  }

  // --- Finance ---
  listInvoices(entityId: string) { return this.get(`/v1/entities/${pathSegment(entityId)}/invoices`) as Promise<ApiRecord[]>; }
  listBankAccounts(entityId: string) { return this.get(`/v1/entities/${pathSegment(entityId)}/bank-accounts`) as Promise<ApiRecord[]>; }
  listPayments(entityId: string) { return this.get(`/v1/entities/${pathSegment(entityId)}/payments`) as Promise<ApiRecord[]>; }
  listPayrollRuns(entityId: string) { return this.get(`/v1/entities/${pathSegment(entityId)}/payroll-runs`) as Promise<ApiRecord[]>; }
  listDistributions(entityId: string) { return this.get(`/v1/entities/${pathSegment(entityId)}/distributions`) as Promise<ApiRecord[]>; }
  listReconciliations(entityId: string) { return this.get(`/v1/entities/${pathSegment(entityId)}/reconciliations`) as Promise<ApiRecord[]>; }
  createInvoice(data: ApiRecord) { return this.post("/v1/treasury/invoices", data) as Promise<ApiRecord>; }
  runPayroll(data: ApiRecord) { return this.post("/v1/payroll/runs", data) as Promise<ApiRecord>; }
  submitPayment(data: ApiRecord) { return this.post("/v1/payments", data) as Promise<ApiRecord>; }
  openBankAccount(data: ApiRecord) { return this.post("/v1/treasury/bank-accounts", data) as Promise<ApiRecord>; }
  activateBankAccount(bankAccountId: string, entityId: string) { return this.postWithParams(`/v1/bank-accounts/${pathSegment(bankAccountId)}/activate`, {}, { entity_id: entityId }) as Promise<ApiRecord>; }
  classifyContractor(data: ApiRecord) { return this.post("/v1/contractors/classify", data) as Promise<ApiRecord>; }
  reconcileLedger(data: ApiRecord) { return this.post("/v1/ledger/reconcile", data) as Promise<ApiRecord>; }
  getFinancialStatements(entityId: string, params?: Record<string, string>) {
    return this.get("/v1/treasury/financial-statements", { entity_id: entityId, ...(params ?? {}) }) as Promise<ApiRecord>;
  }

  // --- Equity analytics ---
  getDilutionPreview(entityId: string, roundId: string) {
    return this.get("/v1/equity/dilution/preview", { entity_id: entityId, round_id: roundId }) as Promise<ApiRecord>;
  }
  getControlMap(entityId: string, rootEntityId: string) {
    return this.get("/v1/equity/control-map", { entity_id: entityId, root_entity_id: rootEntityId }) as Promise<ApiRecord>;
  }

  // --- Tax ---
  listTaxFilings(entityId: string) { return this.get(`/v1/entities/${pathSegment(entityId)}/tax-filings`) as Promise<ApiRecord[]>; }
  listDeadlines(entityId: string) { return this.get(`/v1/entities/${pathSegment(entityId)}/deadlines`) as Promise<ApiRecord[]>; }
  listContractorClassifications(entityId: string) {
    return this.get(`/v1/entities/${pathSegment(entityId)}/contractor-classifications`) as Promise<ApiRecord[]>;
  }
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
  getFormation(id: string) { return this.get(`/v1/formations/${pathSegment(id)}`) as Promise<FormationResponse>; }
  getFormationDocuments(id: string) { return this.get(`/v1/formations/${pathSegment(id)}/documents`) as Promise<DocumentResponse[]>; }
  createFormation(data: ApiRecord) { return this.post("/v1/formations", data) as Promise<FormationResponse>; }
  createFormationWithCapTable(data: ApiRecord) { return this.post("/v1/formations/with-cap-table", data) as Promise<FormationWithCapTableResponse>; }
  createPendingEntity(data: ApiRecord) { return this.post("/v1/formations/pending", data) as Promise<PendingFormationResponse>; }
  addFounder(entityId: string, data: ApiRecord) { return this.post(`/v1/formations/${pathSegment(entityId)}/founders`, data) as Promise<AddFounderResponse>; }
  finalizeFormation(entityId: string, data: ApiRecord = {}) { return this.post(`/v1/formations/${pathSegment(entityId)}/finalize`, data) as Promise<FormationWithCapTableResponse>; }
  markFormationDocumentsSigned(entityId: string) {
    return this.post(`/v1/formations/${pathSegment(entityId)}/mark-documents-signed`) as Promise<ApiRecord>;
  }
  getFormationGates(entityId: string) {
    return this.get(`/v1/formations/${pathSegment(entityId)}/gates`) as Promise<ApiRecord>;
  }
  recordFilingAttestation(entityId: string, data: ApiRecord) {
    return this.post(`/v1/formations/${pathSegment(entityId)}/filing-attestation`, data) as Promise<ApiRecord>;
  }
  addRegisteredAgentConsentEvidence(entityId: string, data: ApiRecord) {
    return this.post(`/v1/formations/${pathSegment(entityId)}/registered-agent-consent-evidence`, data) as Promise<ApiRecord>;
  }
  submitFiling(entityId: string) {
    return this.post(`/v1/formations/${pathSegment(entityId)}/submit-filing`) as Promise<ApiRecord>;
  }
  confirmFiling(entityId: string, data: ApiRecord) {
    return this.post(`/v1/formations/${pathSegment(entityId)}/filing-confirmation`, data) as Promise<ApiRecord>;
  }
  applyEin(entityId: string) {
    return this.post(`/v1/formations/${pathSegment(entityId)}/apply-ein`) as Promise<ApiRecord>;
  }
  confirmEin(entityId: string, data: ApiRecord) {
    return this.post(`/v1/formations/${pathSegment(entityId)}/ein-confirmation`, data) as Promise<ApiRecord>;
  }

  // --- Human obligations ---
  getHumanObligations() { return this.get(`/v1/workspaces/${pathSegment(this.workspaceId)}/human-obligations`) as Promise<ObligationResponse[]>; }
  getSignerToken(obligationId: string) { return this.post(`/v1/human-obligations/${pathSegment(obligationId)}/signer-token`) as Promise<ApiRecord>; }

  // --- Next Steps ---
  getEntityNextSteps(entityId: string) {
    return this.get(`/v1/entities/${pathSegment(entityId)}/next-steps`) as Promise<NextStepsResponse>;
  }

  getWorkspaceNextSteps() {
    return this.get(`/v1/workspaces/${pathSegment(this.workspaceId)}/next-steps`) as Promise<NextStepsResponse>;
  }

  // --- Demo ---
  seedDemo(data: ApiRecord) { return this.post("/v1/demo/seed", data) as Promise<ApiRecord>; }

  // --- Entities writes ---
  convertEntity(entityId: string, data: ApiRecord) {
    const body: ApiRecord = {
      target_type: data.target_type ?? data.new_entity_type,
    };
    const jurisdiction = data.jurisdiction ?? data.new_jurisdiction;
    if (jurisdiction) body.jurisdiction = jurisdiction;
    return this.post(`/v1/entities/${pathSegment(entityId)}/convert`, body) as Promise<ApiRecord>;
  }
  dissolveEntity(entityId: string, data: ApiRecord) { return this.post(`/v1/entities/${pathSegment(entityId)}/dissolve`, data) as Promise<ApiRecord>; }

  // --- Agents ---
  listAgents() { return this.get("/v1/agents") as Promise<ApiRecord[]>; }
  getAgent(id: string) { return this.get(`/v1/agents/${pathSegment(id)}/resolved`) as Promise<ApiRecord>; }
  createAgent(data: ApiRecord) { return this.post("/v1/agents", data) as Promise<ApiRecord>; }
  updateAgent(id: string, data: ApiRecord) { return this.patch(`/v1/agents/${pathSegment(id)}`, data) as Promise<ApiRecord>; }
  deleteAgent(id: string) { return this.patch(`/v1/agents/${pathSegment(id)}`, { status: "disabled" }) as Promise<ApiRecord>; }
  sendAgentMessage(id: string, message: string) { return this.post(`/v1/agents/${pathSegment(id)}/messages`, { message }) as Promise<ApiRecord>; }
  addAgentSkill(id: string, data: ApiRecord) { return this.post(`/v1/agents/${pathSegment(id)}/skills`, data) as Promise<ApiRecord>; }
  listSupportedModels() { return this.get("/v1/models") as Promise<ApiRecord[]>; }
  async getAgentExecution(agentId: string, executionId: string): Promise<ApiRecord> {
    return this.get(`/v1/agents/${pathSegment(agentId)}/executions/${pathSegment(executionId)}`) as Promise<ApiRecord>;
  }
  async getAgentExecutionResult(agentId: string, executionId: string): Promise<ApiRecord> {
    return this.get(`/v1/agents/${pathSegment(agentId)}/executions/${pathSegment(executionId)}/result`) as Promise<ApiRecord>;
  }
  async getAgentExecutionLogs(agentId: string, executionId: string): Promise<ApiRecord> {
    return this.get(`/v1/agents/${pathSegment(agentId)}/executions/${pathSegment(executionId)}/logs`) as Promise<ApiRecord>;
  }
  async killAgentExecution(agentId: string, executionId: string): Promise<ApiRecord> {
    return this.post(`/v1/agents/${pathSegment(agentId)}/executions/${pathSegment(executionId)}/kill`, {}) as Promise<ApiRecord>;
  }

  // --- Governance bodies ---
  createGovernanceBody(data: ApiRecord) { return this.post("/v1/governance-bodies", data) as Promise<ApiRecord>; }
  createGovernanceSeat(bodyId: string, entityId: string, data: ApiRecord) { return this.postWithParams(`/v1/governance-bodies/${pathSegment(bodyId)}/seats`, data, { entity_id: entityId }) as Promise<ApiRecord>; }

  // --- Work Items ---
  listWorkItems(entityId: string, params?: Record<string, string>) { return this.get(`/v1/entities/${pathSegment(entityId)}/work-items`, params) as Promise<ApiRecord[]>; }
  getWorkItem(entityId: string, workItemId: string) { return this.get(`/v1/entities/${pathSegment(entityId)}/work-items/${pathSegment(workItemId)}`) as Promise<ApiRecord>; }
  createWorkItem(entityId: string, data: ApiRecord) { return this.post(`/v1/entities/${pathSegment(entityId)}/work-items`, data) as Promise<ApiRecord>; }
  claimWorkItem(entityId: string, workItemId: string, data: ApiRecord) { return this.post(`/v1/entities/${pathSegment(entityId)}/work-items/${pathSegment(workItemId)}/claim`, data) as Promise<ApiRecord>; }
  completeWorkItem(entityId: string, workItemId: string, data: ApiRecord) { return this.post(`/v1/entities/${pathSegment(entityId)}/work-items/${pathSegment(workItemId)}/complete`, data) as Promise<ApiRecord>; }
  releaseWorkItem(entityId: string, workItemId: string) { return this.post(`/v1/entities/${pathSegment(entityId)}/work-items/${pathSegment(workItemId)}/release`, {}) as Promise<ApiRecord>; }
  cancelWorkItem(entityId: string, workItemId: string) { return this.post(`/v1/entities/${pathSegment(entityId)}/work-items/${pathSegment(workItemId)}/cancel`, {}) as Promise<ApiRecord>; }

  // --- API Keys ---
  listApiKeys() { return this.get("/v1/api-keys", { workspace_id: this.workspaceId }) as Promise<ApiRecord[]>; }
  async createApiKey(data: ApiRecord): Promise<ApiRecord> {
    return this.post("/v1/api-keys", data) as Promise<ApiRecord>;
  }
  async revokeApiKey(keyId: string): Promise<void> {
    return this.del(`/v1/api-keys/${pathSegment(keyId)}`);
  }
  async rotateApiKey(keyId: string): Promise<ApiRecord> {
    return this.post(`/v1/api-keys/${pathSegment(keyId)}/rotate`, {}) as Promise<ApiRecord>;
  }

  // --- Obligations ---
  assignObligation(obligationId: string, contactId: string) {
    return this.patch(`/v1/obligations/${pathSegment(obligationId)}/assign`, { contact_id: contactId }) as Promise<ApiRecord>;
  }

  // --- Config ---
  getConfig() { return this.get("/v1/config") as Promise<ApiRecord>; }

  // --- Services ---
  listServiceCatalog() { return this.get("/v1/services/catalog") as Promise<ApiRecord[]>; }
  createServiceRequest(data: ApiRecord) { return this.post("/v1/services/requests", data) as Promise<ApiRecord>; }
  getServiceRequest(id: string, entityId: string) { return this.get(`/v1/services/requests/${pathSegment(id)}`, { entity_id: entityId }) as Promise<ApiRecord>; }
  listServiceRequests(entityId: string) { return this.get(`/v1/entities/${pathSegment(entityId)}/service-requests`) as Promise<ApiRecord[]>; }
  beginServiceCheckout(id: string, data: ApiRecord) { return this.post(`/v1/services/requests/${pathSegment(id)}/checkout`, data) as Promise<ApiRecord>; }
  fulfillServiceRequest(id: string, data: ApiRecord) { return this.post(`/v1/services/requests/${pathSegment(id)}/fulfill`, data) as Promise<ApiRecord>; }
  cancelServiceRequest(id: string, data: ApiRecord) { return this.post(`/v1/services/requests/${pathSegment(id)}/cancel`, data) as Promise<ApiRecord>; }

  // --- Feedback ---
  submitFeedback(message: string, category?: string, email?: string) {
    return this.post("/v1/feedback", { message, category, email }) as Promise<{ feedback_id: string; submitted_at: string }>;
  }

  // --- Link/Claim ---
  async createLink(externalId: string, provider: string): Promise<ApiRecord> {
    const resp = await this.request("POST", "/v1/workspaces/link", { external_id: externalId, provider });
    if (!resp.ok) {
      const detail = await extractErrorMessage(resp);
      if (resp.status === 401) throw new SessionExpiredError(detail);
      const prefix = resp.status >= 500
        ? "Server error"
        : resp.status === 404
          ? "Not found"
          : resp.status === 422
            ? "Validation error"
            : `HTTP ${resp.status}`;
      throw new Error(`${prefix}: ${detail}`);
    }
    return resp.json() as Promise<ApiRecord>;
  }
}
