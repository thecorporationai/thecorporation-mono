import {
  getActiveEntityId,
  getLastReference,
  setLastReference,
  updateConfig,
} from "./config.js";
import { CorpAPIClient } from "./api-client.js";
import type { ApiRecord, CorpConfig } from "./types.js";

export type ResourceKind =
  | "entity"
  | "contact"
  | "share_transfer"
  | "invoice"
  | "bank_account"
  | "payment"
  | "payroll_run"
  | "distribution"
  | "reconciliation"
  | "tax_filing"
  | "deadline"
  | "classification"
  | "body"
  | "meeting"
  | "seat"
  | "agenda_item"
  | "resolution"
  | "document"
  | "work_item"
  | "agent"
  | "valuation"
  | "safe_note"
  | "instrument"
  | "share_class"
  | "round";

export type MatchRecord = {
  id: string;
  label: string;
  tokens: Set<string>;
  raw: ApiRecord;
};

export type ReferenceMatch = {
  kind: ResourceKind;
  id: string;
  short_id: string;
  label: string;
  alias?: string;
  raw: ApiRecord;
};

const RESOURCE_KINDS = [
  "entity",
  "contact",
  "share_transfer",
  "invoice",
  "bank_account",
  "payment",
  "payroll_run",
  "distribution",
  "reconciliation",
  "tax_filing",
  "deadline",
  "classification",
  "body",
  "meeting",
  "seat",
  "agenda_item",
  "resolution",
  "document",
  "work_item",
  "agent",
  "valuation",
  "safe_note",
  "instrument",
  "share_class",
  "round",
] as const satisfies readonly ResourceKind[];

const VALID_RESOURCE_KINDS = new Set<ResourceKind>(RESOURCE_KINDS);
const MAX_REFERENCE_INPUT_LEN = 256;

function normalize(value: string): string {
  return value.trim().toLowerCase();
}

function validateReferenceInput(
  value: string,
  field: string,
  options: { allowEmpty?: boolean } = {},
): string {
  const trimmed = value.trim();
  if (!options.allowEmpty && trimmed.length === 0) {
    throw new Error(`${field} cannot be empty.`);
  }
  if (trimmed.length > MAX_REFERENCE_INPUT_LEN) {
    throw new Error(`${field} must be at most ${MAX_REFERENCE_INPUT_LEN} characters.`);
  }
  return trimmed;
}

export function shortId(value: string | undefined): string {
  return String(value ?? "").slice(0, 8);
}

export function slugify(value: string | undefined): string {
  return String(value ?? "")
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");
}

function isOpaqueUuid(value: string): boolean {
  return /^[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i.test(
    value.trim(),
  );
}

function isShortIdCandidate(value: string): boolean {
  const trimmed = value.trim();
  return /^[0-9a-f-]{4,}$/i.test(trimmed) || /^[a-z]+_[a-z0-9_-]{3,}$/i.test(trimmed);
}

function parseLastReference(value: string): { isLast: boolean; kind?: ResourceKind } {
  const trimmed = validateReferenceInput(value, "reference", { allowEmpty: false }).toLowerCase();
  if (trimmed === "_" || trimmed === "@last") {
    return { isLast: true };
  }
  if (trimmed.startsWith("@last:")) {
    const kind = trimmed.slice(6);
    if (!VALID_RESOURCE_KINDS.has(kind as ResourceKind)) {
      throw new Error(`Unknown reference kind: ${kind}`);
    }
    return { isLast: true, kind: kind as ResourceKind };
  }
  return { isLast: false };
}

function uniqueStrings(values: Array<string | undefined | null>): Set<string> {
  const out = new Set<string>();
  for (const value of values) {
    if (!value) continue;
    const trimmed = value.trim();
    if (!trimmed) continue;
    out.add(normalize(trimmed));
    const slug = slugify(trimmed);
    if (slug) out.add(slug);
  }
  return out;
}

function kindLabel(kind: ResourceKind): string {
  return kind.replaceAll("_", " ");
}

function isEntityScopedKind(kind: ResourceKind): boolean {
  return kind !== "entity" && kind !== "agent";
}

function extractId(record: ApiRecord, fields: string[]): string | undefined {
  for (const field of fields) {
    const value = record[field];
    if (typeof value === "string" && value.trim()) {
      return value.trim();
    }
  }
  return undefined;
}

export function describeReferenceRecord(kind: ResourceKind, record: ApiRecord): MatchRecord | null {
  const specs: Record<ResourceKind, { idFields: string[]; labelFields: string[] }> = {
    entity: { idFields: ["entity_id", "id"], labelFields: ["legal_name", "name"] },
    contact: { idFields: ["contact_id", "id"], labelFields: ["name", "email"] },
    share_transfer: {
      idFields: ["transfer_id", "id"],
      labelFields: ["from_holder", "to_holder", "transfer_type", "status"],
    },
    invoice: {
      idFields: ["invoice_id", "id"],
      labelFields: ["customer_name", "description", "due_date"],
    },
    bank_account: {
      idFields: ["bank_account_id", "account_id", "id"],
      labelFields: ["bank_name", "account_type"],
    },
    payment: {
      idFields: ["payment_id", "id"],
      labelFields: ["recipient", "description", "payment_method"],
    },
    payroll_run: {
      idFields: ["payroll_run_id", "id"],
      labelFields: ["pay_period_start", "pay_period_end"],
    },
    distribution: {
      idFields: ["distribution_id", "id"],
      labelFields: ["description", "distribution_type"],
    },
    reconciliation: {
      idFields: ["reconciliation_id", "id"],
      labelFields: ["as_of_date", "status"],
    },
    tax_filing: {
      idFields: ["filing_id", "id"],
      labelFields: ["document_type", "tax_year"],
    },
    deadline: {
      idFields: ["deadline_id", "id"],
      labelFields: ["deadline_type", "description", "due_date"],
    },
    classification: {
      idFields: ["classification_id", "id"],
      labelFields: ["contractor_name", "state", "risk_level"],
    },
    body: { idFields: ["body_id", "id"], labelFields: ["name"] },
    meeting: { idFields: ["meeting_id", "id"], labelFields: ["title", "name"] },
    seat: {
      idFields: ["seat_id", "id"],
      labelFields: ["seat_name", "title", "holder_name", "holder", "holder_email"],
    },
    agenda_item: { idFields: ["agenda_item_id", "item_id", "id"], labelFields: ["title"] },
    resolution: { idFields: ["resolution_id", "id"], labelFields: ["title"] },
    document: { idFields: ["document_id", "id"], labelFields: ["title", "name"] },
    work_item: { idFields: ["work_item_id", "id"], labelFields: ["title"] },
    agent: { idFields: ["agent_id", "id"], labelFields: ["name"] },
    valuation: {
      idFields: ["valuation_id", "id"],
      labelFields: ["valuation_type", "effective_date", "date"],
    },
    safe_note: {
      idFields: ["safe_note_id", "safe_id", "id"],
      labelFields: ["investor_name", "investor", "safe_type"],
    },
    instrument: { idFields: ["instrument_id", "id"], labelFields: ["symbol", "kind", "name"] },
    share_class: {
      idFields: ["share_class_id", "id"],
      labelFields: ["class_code", "name", "share_class"],
    },
    round: { idFields: ["round_id", "equity_round_id", "id"], labelFields: ["name"] },
  };
  const spec = specs[kind];
  const id = extractId(record, spec.idFields);
  if (!id) {
    return null;
  }
  const labels = spec.labelFields
    .map((field) => record[field])
    .filter((value): value is string => typeof value === "string" && value.trim().length > 0);
  const persistedHandle = typeof record.handle === "string" && record.handle.trim().length > 0
    ? record.handle.trim()
    : undefined;
  let label = labels[0] ?? id;
  if (kind === "share_transfer") {
    const fromHolder = typeof record.from_holder === "string" ? record.from_holder.trim() : "";
    const toHolder = typeof record.to_holder === "string" ? record.to_holder.trim() : "";
    const transferType = typeof record.transfer_type === "string" ? record.transfer_type.trim() : "";
    const composite = [fromHolder && toHolder ? `${fromHolder}-to-${toHolder}` : "", transferType]
      .filter(Boolean)
      .join("-");
    if (composite) {
      label = composite;
    }
  }
  return {
    id,
    label,
    tokens: uniqueStrings([id, persistedHandle, ...labels]),
    raw: record,
  };
}

export function getReferenceId(kind: ResourceKind, record: ApiRecord): string | undefined {
  return describeReferenceRecord(kind, record)?.id;
}

export function getReferenceLabel(kind: ResourceKind, record: ApiRecord): string | undefined {
  return describeReferenceRecord(kind, record)?.label;
}

export function getReferenceAlias(kind: ResourceKind, record: ApiRecord): string | undefined {
  if (typeof record.handle === "string" && record.handle.trim().length > 0) {
    return record.handle.trim();
  }
  const described = describeReferenceRecord(kind, record);
  if (!described) return undefined;
  const alias = slugify(described.label);
  return alias || shortId(described.id);
}

type Scope = { entityId?: string; bodyId?: string; meetingId?: string };

export class ReferenceResolver {
  private readonly client: CorpAPIClient;
  private readonly cfg: CorpConfig;
  private entityCache?: ApiRecord[];
  private readonly contactsCache = new Map<string, ApiRecord[]>();
  private readonly shareTransfersCache = new Map<string, ApiRecord[]>();
  private readonly invoicesCache = new Map<string, ApiRecord[]>();
  private readonly bankAccountsCache = new Map<string, ApiRecord[]>();
  private readonly paymentsCache = new Map<string, ApiRecord[]>();
  private readonly payrollRunsCache = new Map<string, ApiRecord[]>();
  private readonly distributionsCache = new Map<string, ApiRecord[]>();
  private readonly reconciliationsCache = new Map<string, ApiRecord[]>();
  private readonly taxFilingsCache = new Map<string, ApiRecord[]>();
  private readonly deadlinesCache = new Map<string, ApiRecord[]>();
  private readonly classificationsCache = new Map<string, ApiRecord[]>();
  private readonly bodiesCache = new Map<string, ApiRecord[]>();
  private readonly meetingsCache = new Map<string, ApiRecord[]>();
  private readonly seatsCache = new Map<string, ApiRecord[]>();
  private readonly agendaCache = new Map<string, ApiRecord[]>();
  private readonly resolutionsCache = new Map<string, ApiRecord[]>();
  private readonly documentsCache = new Map<string, ApiRecord[]>();
  private readonly workItemsCache = new Map<string, ApiRecord[]>();
  private readonly valuationsCache = new Map<string, ApiRecord[]>();
  private readonly safeNotesCache = new Map<string, ApiRecord[]>();
  private readonly roundsCache = new Map<string, ApiRecord[]>();
  private readonly capTableCache = new Map<string, ApiRecord>();
  private agentsCache?: ApiRecord[];

  constructor(client: CorpAPIClient, cfg: CorpConfig) {
    this.client = client;
    this.cfg = cfg;
  }

  async resolveEntity(ref?: string): Promise<string> {
    if (!ref || !ref.trim()) {
      const activeEntityId = getActiveEntityId(this.cfg);
      if (!activeEntityId) {
        throw new Error(
          "No entity specified. Use --entity-id or set active_entity_id via 'corp config set active_entity_id <ref>'.",
        );
      }
      this.remember("entity", activeEntityId);
      return activeEntityId;
    }
    return this.resolve("entity", ref);
  }

  async resolveContact(entityId: string, ref: string): Promise<string> {
    return this.resolve("contact", ref, { entityId });
  }

  async resolveWorkItemActor(
    entityId: string,
    ref: string,
  ): Promise<{ actor_type: "contact" | "agent"; actor_id: string }> {
    const trimmed = validateReferenceInput(ref, "actor reference");
    const [contactResult, agentResult] = await Promise.allSettled([
      this.resolveContact(entityId, trimmed),
      this.resolveAgent(trimmed),
    ]);

    const contactId =
      contactResult.status === "fulfilled" ? contactResult.value : undefined;
    const agentId =
      agentResult.status === "fulfilled" ? agentResult.value : undefined;

    if (contactId && agentId && contactId !== agentId) {
      throw new Error(
        `Actor reference '${trimmed}' is ambiguous between a contact and an agent. Use a unique ref or explicit @last:contact / @last:agent.`,
      );
    }
    if (contactId) {
      return { actor_type: "contact", actor_id: contactId };
    }
    if (agentId) {
      return { actor_type: "agent", actor_id: agentId };
    }

    throw new Error(
      `No matching contact or agent found for '${trimmed}'. Try 'corp find contact <query>' or 'corp find agent <query>'.`,
    );
  }

  async resolveShareTransfer(entityId: string, ref: string): Promise<string> {
    return this.resolve("share_transfer", ref, { entityId });
  }

  async resolveInvoice(entityId: string, ref: string): Promise<string> {
    return this.resolve("invoice", ref, { entityId });
  }

  async resolveBankAccount(entityId: string, ref: string): Promise<string> {
    return this.resolve("bank_account", ref, { entityId });
  }

  async resolvePayment(entityId: string, ref: string): Promise<string> {
    return this.resolve("payment", ref, { entityId });
  }

  async resolvePayrollRun(entityId: string, ref: string): Promise<string> {
    return this.resolve("payroll_run", ref, { entityId });
  }

  async resolveDistribution(entityId: string, ref: string): Promise<string> {
    return this.resolve("distribution", ref, { entityId });
  }

  async resolveReconciliation(entityId: string, ref: string): Promise<string> {
    return this.resolve("reconciliation", ref, { entityId });
  }

  async resolveTaxFiling(entityId: string, ref: string): Promise<string> {
    return this.resolve("tax_filing", ref, { entityId });
  }

  async resolveDeadline(entityId: string, ref: string): Promise<string> {
    return this.resolve("deadline", ref, { entityId });
  }

  async resolveClassification(entityId: string, ref: string): Promise<string> {
    return this.resolve("classification", ref, { entityId });
  }

  async resolveBody(entityId: string, ref: string): Promise<string> {
    return this.resolve("body", ref, { entityId });
  }

  async resolveMeeting(entityId: string, ref: string, bodyId?: string): Promise<string> {
    return this.resolve("meeting", ref, { entityId, bodyId });
  }

  async resolveSeat(entityId: string, ref: string, bodyId?: string): Promise<string> {
    return this.resolve("seat", ref, { entityId, bodyId });
  }

  async resolveAgendaItem(entityId: string, meetingId: string, ref: string): Promise<string> {
    return this.resolve("agenda_item", ref, { entityId, meetingId });
  }

  async resolveResolution(
    entityId: string,
    ref: string,
    meetingId?: string,
  ): Promise<string> {
    return this.resolve("resolution", ref, { entityId, meetingId });
  }

  async resolveDocument(entityId: string, ref: string): Promise<string> {
    return this.resolve("document", ref, { entityId });
  }

  async resolveWorkItem(entityId: string, ref: string): Promise<string> {
    return this.resolve("work_item", ref, { entityId });
  }

  async resolveAgent(ref: string): Promise<string> {
    return this.resolve("agent", ref);
  }

  async resolveValuation(entityId: string, ref: string): Promise<string> {
    return this.resolve("valuation", ref, { entityId });
  }

  async resolveSafeNote(entityId: string, ref: string): Promise<string> {
    return this.resolve("safe_note", ref, { entityId });
  }

  async resolveInstrument(entityId: string, ref: string): Promise<string> {
    return this.resolve("instrument", ref, { entityId });
  }

  async resolveShareClass(entityId: string, ref: string): Promise<string> {
    return this.resolve("share_class", ref, { entityId });
  }

  async resolveRound(entityId: string, ref: string): Promise<string> {
    return this.resolve("round", ref, { entityId });
  }

  async find(
    kind: ResourceKind,
    query: string,
    scope: Scope = {},
  ): Promise<ReferenceMatch[]> {
    const trimmedQuery = validateReferenceInput(query, "query", { allowEmpty: true });
    const records = await this.listRecords(kind, scope);
    const described = records
      .map((record) => describeReferenceRecord(kind, record))
      .filter((record): record is MatchRecord => record !== null);
    const normalizedQuery = normalize(trimmedQuery);

    const matches = described
      .filter((record) => {
        if (!normalizedQuery || normalizedQuery === "*") {
          return true;
        }
        if (normalize(record.id).startsWith(normalizedQuery)) {
          return true;
        }
        if (record.tokens.has(normalizedQuery)) {
          return true;
        }
        return Array.from(record.tokens).some((token) => token.includes(normalizedQuery));
      })
      .sort((left, right) => this.matchRank(left, normalizedQuery) - this.matchRank(right, normalizedQuery)
        || left.label.localeCompare(right.label)
        || left.id.localeCompare(right.id));

    return matches.map((record) => ({
      kind,
      id: record.id,
      short_id: shortId(record.id),
      label: record.label,
      alias: this.referenceAlias(kind, record.raw),
      raw: record.raw,
    }));
  }

  remember(kind: ResourceKind, referenceId: string, entityId?: string): void {
    setLastReference(this.cfg, kind, referenceId, entityId);
    updateConfig((cfg) => {
      setLastReference(cfg, kind, referenceId, entityId);
    });
  }

  rememberFromRecord(kind: ResourceKind, record: ApiRecord, entityId?: string): void {
    const described = describeReferenceRecord(kind, record);
    if (described) {
      this.remember(kind, described.id, entityId);
    }
  }

  async stabilizeRecord(kind: ResourceKind, record: ApiRecord, entityId?: string): Promise<ApiRecord> {
    const described = describeReferenceRecord(kind, record);
    if (!described) return record;
    if (typeof record.handle === "string" && record.handle.trim().length > 0) {
      return record;
    }
    const response = await this.client.syncReferences(
      kind,
      [{ resource_id: described.id, label: described.label }],
      isEntityScopedKind(kind) ? entityId : undefined,
    );
    const handle = response.references[0]?.handle;
    if (typeof handle === "string" && handle.trim().length > 0) {
      record.handle = handle.trim();
    }
    return record;
  }

  async stabilizeRecords(kind: ResourceKind, records: ApiRecord[], entityId?: string): Promise<ApiRecord[]> {
    return this.attachStableHandles(kind, records, entityId);
  }

  referenceAlias(kind: ResourceKind, record: ApiRecord): string | undefined {
    return getReferenceAlias(kind, record);
  }

  private async resolve(kind: ResourceKind, ref: string, scope: Scope = {}): Promise<string> {
    const last = parseLastReference(ref);
    if (last.isLast) {
      const lastKind = last.kind ?? kind;
      if (lastKind !== kind) {
        throw new Error(`@last:${lastKind} cannot be used where a ${kindLabel(kind)} reference is required.`);
      }
      const remembered = getLastReference(this.cfg, lastKind, scope.entityId);
      if (!remembered) {
        throw new Error(`No ${kindLabel(lastKind)} is recorded for @last.`);
      }
      this.remember(kind, remembered, scope.entityId);
      return remembered;
    }

    const trimmed = validateReferenceInput(ref, `${kindLabel(kind)} reference`);
    if (isOpaqueUuid(trimmed)) {
      this.remember(kind, trimmed, scope.entityId);
      return trimmed;
    }

    const records = await this.listRecords(kind, scope);
    const match = this.matchRecords(kind, trimmed, records);
    this.remember(kind, match.id, scope.entityId);
    return match.id;
  }

  private matchRecords(kind: ResourceKind, ref: string, records: ApiRecord[]): MatchRecord {
    const described = records
      .map((record) => describeReferenceRecord(kind, record))
      .filter((record): record is MatchRecord => record !== null);
    const normalizedRef = normalize(ref);

    const exactIdMatches = described.filter((record) => normalize(record.id) === normalizedRef);
    if (exactIdMatches.length === 1) {
      return exactIdMatches[0];
    }

    const exactTokenMatches = described.filter((record) => record.tokens.has(normalizedRef));
    if (exactTokenMatches.length === 1) {
      return exactTokenMatches[0];
    }
    if (exactTokenMatches.length > 1) {
      throw new Error(this.ambiguousMessage(kind, ref, exactTokenMatches));
    }

    if (isShortIdCandidate(ref)) {
      const prefixMatches = described.filter((record) => normalize(record.id).startsWith(normalizedRef));
      if (prefixMatches.length === 1) {
        return prefixMatches[0];
      }
      if (prefixMatches.length > 1) {
        throw new Error(this.ambiguousMessage(kind, ref, prefixMatches));
      }
    }

    throw new Error(
      `No ${kindLabel(kind)} found for reference "${ref}". Try: corp find ${kind} ${JSON.stringify(ref)}`,
    );
  }

  private ambiguousMessage(kind: ResourceKind, ref: string, matches: MatchRecord[]): string {
    const previews = matches
      .slice(0, 5)
      .map((match) => `${match.label} [${shortId(match.id)}]`)
      .join(", ");
    return `Ambiguous ${kindLabel(kind)} reference "${ref}". Matches: ${previews}. Try: corp find ${kind} ${JSON.stringify(ref)}`;
  }

  private matchRank(record: MatchRecord, normalizedQuery: string): number {
    if (!normalizedQuery || normalizedQuery === "*") {
      return 5;
    }
    if (normalize(record.id) === normalizedQuery) {
      return 0;
    }
    if (record.tokens.has(normalizedQuery)) {
      return 1;
    }
    if (normalize(record.id).startsWith(normalizedQuery)) {
      return 2;
    }
    if (Array.from(record.tokens).some((token) => token.startsWith(normalizedQuery))) {
      return 3;
    }
    return 4;
  }

  private async listRecords(kind: ResourceKind, scope: Scope): Promise<ApiRecord[]> {
    const records = await (async () => {
      switch (kind) {
      case "entity":
        return this.listEntities();
      case "contact":
        return this.listContacts(scope.entityId);
      case "share_transfer":
        return this.listShareTransfers(scope.entityId);
      case "invoice":
        return this.listInvoices(scope.entityId);
      case "bank_account":
        return this.listBankAccounts(scope.entityId);
      case "payment":
        return this.listPayments(scope.entityId);
      case "payroll_run":
        return this.listPayrollRuns(scope.entityId);
      case "distribution":
        return this.listDistributions(scope.entityId);
      case "reconciliation":
        return this.listReconciliations(scope.entityId);
      case "tax_filing":
        return this.listTaxFilings(scope.entityId);
      case "deadline":
        return this.listDeadlines(scope.entityId);
      case "classification":
        return this.listClassifications(scope.entityId);
      case "body":
        return this.listBodies(scope.entityId);
      case "meeting":
        return this.listMeetings(scope.entityId, scope.bodyId);
      case "seat":
        return this.listSeats(scope.entityId, scope.bodyId);
      case "agenda_item":
        return this.listAgendaItems(scope.entityId, scope.meetingId);
      case "resolution":
        return this.listResolutions(scope.entityId, scope.meetingId);
      case "document":
        return this.listDocuments(scope.entityId);
      case "work_item":
        return this.listWorkItems(scope.entityId);
      case "agent":
        return this.listAgents();
      case "valuation":
        return this.listValuations(scope.entityId);
      case "safe_note":
        return this.listSafeNotes(scope.entityId);
      case "instrument":
        return this.listInstruments(scope.entityId);
      case "share_class":
        return this.listShareClasses(scope.entityId);
      case "round":
        return this.listRounds(scope.entityId);
      }
    })();
    return this.attachStableHandles(kind, records, scope.entityId);
  }

  private async attachStableHandles(
    kind: ResourceKind,
    records: ApiRecord[],
    entityId?: string,
  ): Promise<ApiRecord[]> {
    const missing = records
      .map((record) => ({ record, described: describeReferenceRecord(kind, record) }))
      .filter(
        (entry): entry is { record: ApiRecord; described: MatchRecord } =>
          entry.described !== null
            && !(typeof entry.record.handle === "string" && entry.record.handle.trim().length > 0),
      );
    if (missing.length === 0) {
      return records;
    }

    const response = await this.client.syncReferences(
      kind,
      missing.map(({ described }) => ({
        resource_id: described.id,
        label: described.label,
      })),
      isEntityScopedKind(kind) ? entityId : undefined,
    );
    const handleById = new Map<string, string>();
    for (const reference of response.references) {
      if (typeof reference.resource_id === "string" && typeof reference.handle === "string") {
        handleById.set(reference.resource_id, reference.handle);
      }
    }
    for (const { record, described } of missing) {
      const handle = handleById.get(described.id);
      if (handle) {
        record.handle = handle;
      }
    }
    return records;
  }

  private async listEntities(): Promise<ApiRecord[]> {
    if (!this.entityCache) {
      this.entityCache = await this.client.listEntities();
    }
    return this.entityCache;
  }

  private async listContacts(entityId?: string): Promise<ApiRecord[]> {
    if (!entityId) throw new Error("An entity context is required to resolve contacts.");
    const cached = this.contactsCache.get(entityId);
    if (cached) return cached;
    const contacts = await this.client.listContacts(entityId);
    this.contactsCache.set(entityId, contacts);
    return contacts;
  }

  private async listShareTransfers(entityId?: string): Promise<ApiRecord[]> {
    if (!entityId) throw new Error("An entity context is required to resolve share transfers.");
    const cached = this.shareTransfersCache.get(entityId);
    if (cached) return cached;
    const transfers = await this.client.listShareTransfers(entityId);
    this.shareTransfersCache.set(entityId, transfers);
    return transfers;
  }

  private async listInvoices(entityId?: string): Promise<ApiRecord[]> {
    if (!entityId) throw new Error("An entity context is required to resolve invoices.");
    const cached = this.invoicesCache.get(entityId);
    if (cached) return cached;
    const invoices = await this.client.listInvoices(entityId);
    this.invoicesCache.set(entityId, invoices);
    return invoices;
  }

  private async listBankAccounts(entityId?: string): Promise<ApiRecord[]> {
    if (!entityId) throw new Error("An entity context is required to resolve bank accounts.");
    const cached = this.bankAccountsCache.get(entityId);
    if (cached) return cached;
    const bankAccounts = await this.client.listBankAccounts(entityId);
    this.bankAccountsCache.set(entityId, bankAccounts);
    return bankAccounts;
  }

  private async listPayments(entityId?: string): Promise<ApiRecord[]> {
    if (!entityId) throw new Error("An entity context is required to resolve payments.");
    const cached = this.paymentsCache.get(entityId);
    if (cached) return cached;
    const payments = await this.client.listPayments(entityId);
    this.paymentsCache.set(entityId, payments);
    return payments;
  }

  private async listPayrollRuns(entityId?: string): Promise<ApiRecord[]> {
    if (!entityId) throw new Error("An entity context is required to resolve payroll runs.");
    const cached = this.payrollRunsCache.get(entityId);
    if (cached) return cached;
    const payrollRuns = await this.client.listPayrollRuns(entityId);
    this.payrollRunsCache.set(entityId, payrollRuns);
    return payrollRuns;
  }

  private async listDistributions(entityId?: string): Promise<ApiRecord[]> {
    if (!entityId) throw new Error("An entity context is required to resolve distributions.");
    const cached = this.distributionsCache.get(entityId);
    if (cached) return cached;
    const distributions = await this.client.listDistributions(entityId);
    this.distributionsCache.set(entityId, distributions);
    return distributions;
  }

  private async listReconciliations(entityId?: string): Promise<ApiRecord[]> {
    if (!entityId) throw new Error("An entity context is required to resolve reconciliations.");
    const cached = this.reconciliationsCache.get(entityId);
    if (cached) return cached;
    const reconciliations = await this.client.listReconciliations(entityId);
    this.reconciliationsCache.set(entityId, reconciliations);
    return reconciliations;
  }

  private async listTaxFilings(entityId?: string): Promise<ApiRecord[]> {
    if (!entityId) throw new Error("An entity context is required to resolve tax filings.");
    const cached = this.taxFilingsCache.get(entityId);
    if (cached) return cached;
    const filings = await this.client.listTaxFilings(entityId);
    this.taxFilingsCache.set(entityId, filings);
    return filings;
  }

  private async listDeadlines(entityId?: string): Promise<ApiRecord[]> {
    if (!entityId) throw new Error("An entity context is required to resolve deadlines.");
    const cached = this.deadlinesCache.get(entityId);
    if (cached) return cached;
    const deadlines = await this.client.listDeadlines(entityId);
    this.deadlinesCache.set(entityId, deadlines);
    return deadlines;
  }

  private async listClassifications(entityId?: string): Promise<ApiRecord[]> {
    if (!entityId) throw new Error("An entity context is required to resolve contractor classifications.");
    const cached = this.classificationsCache.get(entityId);
    if (cached) return cached;
    const classifications = await this.client.listContractorClassifications(entityId);
    this.classificationsCache.set(entityId, classifications);
    return classifications;
  }

  private async listBodies(entityId?: string): Promise<ApiRecord[]> {
    if (!entityId) throw new Error("An entity context is required to resolve governance bodies.");
    const cached = this.bodiesCache.get(entityId);
    if (cached) return cached;
    const bodies = await this.client.listGovernanceBodies(entityId);
    this.bodiesCache.set(entityId, bodies as ApiRecord[]);
    return bodies as ApiRecord[];
  }

  private async listMeetings(entityId?: string, bodyId?: string): Promise<ApiRecord[]> {
    if (!entityId) throw new Error("An entity context is required to resolve meetings.");
    const cacheKey = `${entityId}:${bodyId ?? "*"}`;
    const cached = this.meetingsCache.get(cacheKey);
    if (cached) return cached;

    const meetings: ApiRecord[] = [];
    if (bodyId) {
      meetings.push(...((await this.client.listMeetings(bodyId, entityId)) as ApiRecord[]));
    } else {
      const bodies = await this.listBodies(entityId);
      for (const body of bodies) {
        const resolvedBodyId = extractId(body, ["body_id", "id"]);
        if (!resolvedBodyId) continue;
        meetings.push(...((await this.client.listMeetings(resolvedBodyId, entityId)) as ApiRecord[]));
      }
    }
    this.meetingsCache.set(cacheKey, meetings);
    return meetings;
  }

  private async listSeats(entityId?: string, bodyId?: string): Promise<ApiRecord[]> {
    if (!entityId) throw new Error("An entity context is required to resolve seats.");
    const cacheKey = `${entityId}:${bodyId ?? "*"}`;
    const cached = this.seatsCache.get(cacheKey);
    if (cached) return cached;

    const seats: ApiRecord[] = [];
    if (bodyId) {
      seats.push(...((await this.client.getGovernanceSeats(bodyId, entityId)) as ApiRecord[]));
    } else {
      const bodies = await this.listBodies(entityId);
      for (const body of bodies) {
        const resolvedBodyId = extractId(body, ["body_id", "id"]);
        if (!resolvedBodyId) continue;
        seats.push(...((await this.client.getGovernanceSeats(resolvedBodyId, entityId)) as ApiRecord[]));
      }
    }
    this.seatsCache.set(cacheKey, seats);
    return seats;
  }

  private async listAgendaItems(entityId?: string, meetingId?: string): Promise<ApiRecord[]> {
    if (!entityId || !meetingId) {
      throw new Error("Entity and meeting context are required to resolve agenda items.");
    }
    const cached = this.agendaCache.get(`${entityId}:${meetingId}`);
    if (cached) return cached;
    const items = (await this.client.listAgendaItems(meetingId, entityId)) as ApiRecord[];
    this.agendaCache.set(`${entityId}:${meetingId}`, items);
    return items;
  }

  private async listResolutions(entityId?: string, meetingId?: string): Promise<ApiRecord[]> {
    if (!entityId) throw new Error("An entity context is required to resolve resolutions.");
    const cacheKey = `${entityId}:${meetingId ?? "*"}`;
    const cached = this.resolutionsCache.get(cacheKey);
    if (cached) return cached;

    const resolutions: ApiRecord[] = [];
    if (meetingId) {
      resolutions.push(...((await this.client.getMeetingResolutions(meetingId, entityId)) as ApiRecord[]));
    } else {
      const meetings = await this.listMeetings(entityId);
      for (const meeting of meetings) {
        const resolvedMeetingId = extractId(meeting, ["meeting_id", "id"]);
        if (!resolvedMeetingId) continue;
        resolutions.push(...((await this.client.getMeetingResolutions(resolvedMeetingId, entityId)) as ApiRecord[]));
      }
    }
    this.resolutionsCache.set(cacheKey, resolutions);
    return resolutions;
  }

  private async listDocuments(entityId?: string): Promise<ApiRecord[]> {
    if (!entityId) throw new Error("An entity context is required to resolve documents.");
    const cached = this.documentsCache.get(entityId);
    if (cached) return cached;
    const docs = (await this.client.getEntityDocuments(entityId)) as ApiRecord[];
    this.documentsCache.set(entityId, docs);
    return docs;
  }

  private async listWorkItems(entityId?: string): Promise<ApiRecord[]> {
    if (!entityId) throw new Error("An entity context is required to resolve work items.");
    const cached = this.workItemsCache.get(entityId);
    if (cached) return cached;
    const items = (await this.client.listWorkItems(entityId)) as ApiRecord[];
    this.workItemsCache.set(entityId, items);
    return items;
  }

  private async listAgents(): Promise<ApiRecord[]> {
    if (!this.agentsCache) {
      this.agentsCache = (await this.client.listAgents()) as ApiRecord[];
    }
    return this.agentsCache;
  }

  private async listValuations(entityId?: string): Promise<ApiRecord[]> {
    if (!entityId) throw new Error("An entity context is required to resolve valuations.");
    const cached = this.valuationsCache.get(entityId);
    if (cached) return cached;
    const valuations = (await this.client.getValuations(entityId)) as ApiRecord[];
    this.valuationsCache.set(entityId, valuations);
    return valuations;
  }

  private async listSafeNotes(entityId?: string): Promise<ApiRecord[]> {
    if (!entityId) throw new Error("An entity context is required to resolve SAFE notes.");
    const cached = this.safeNotesCache.get(entityId);
    if (cached) return cached;
    const safeNotes = (await this.client.getSafeNotes(entityId)) as ApiRecord[];
    this.safeNotesCache.set(entityId, safeNotes);
    return safeNotes;
  }

  private async listRounds(entityId?: string): Promise<ApiRecord[]> {
    if (!entityId) throw new Error("An entity context is required to resolve rounds.");
    const cached = this.roundsCache.get(entityId);
    if (cached) return cached;
    const rounds = (await this.client.listEquityRounds(entityId)) as ApiRecord[];
    this.roundsCache.set(entityId, rounds);
    return rounds;
  }

  private async getCapTable(entityId?: string): Promise<ApiRecord> {
    if (!entityId) throw new Error("An entity context is required to resolve cap table resources.");
    const cached = this.capTableCache.get(entityId);
    if (cached) return cached;
    const capTable = (await this.client.getCapTable(entityId)) as ApiRecord;
    this.capTableCache.set(entityId, capTable);
    return capTable;
  }

  private async listInstruments(entityId?: string): Promise<ApiRecord[]> {
    const capTable = await this.getCapTable(entityId);
    return Array.isArray(capTable.instruments) ? (capTable.instruments as ApiRecord[]) : [];
  }

  private async listShareClasses(entityId?: string): Promise<ApiRecord[]> {
    const capTable = await this.getCapTable(entityId);
    return Array.isArray(capTable.share_classes) ? (capTable.share_classes as ApiRecord[]) : [];
  }
}
