import {
  getActiveEntityId,
  getLastReference,
  setLastReference,
  updateConfig,
} from "./config.js";
import { CorpAPIClient } from "./api-client.js";
import type { ApiRecord, CorpConfig } from "./types.js";

// Re-export types and pure functions from the shared core so existing
// consumers in cli-ts can keep importing from "./references.js".
export type { ResourceKind, MatchRecord, ReferenceMatch } from "@thecorporation/corp-tools";
export {
  shortId,
  slugify,
  describeReferenceRecord,
  getReferenceId,
  getReferenceLabel,
  getReferenceAlias,
  RESOURCE_KINDS,
} from "@thecorporation/corp-tools";

import {
  type ResourceKind,
  type MatchRecord,
  type ReferenceStorage,
  ReferenceTracker,
  shortId,
  normalize,
  validateReferenceInput,
  describeReferenceRecord,
  getReferenceAlias,
  isOpaqueUuid,
  isShortIdCandidate,
  parseLastReference,
  kindLabel,
  isEntityScopedKind,
  extractId,
  matchRank,
} from "@thecorporation/corp-tools";

// ---------------------------------------------------------------------------
// Node-specific storage adapter
// ---------------------------------------------------------------------------

class NodeReferenceStorage implements ReferenceStorage {
  constructor(private cfg: CorpConfig) {}

  getLastReference(kind: ResourceKind, entityId?: string): string | undefined {
    return getLastReference(this.cfg, kind, entityId);
  }

  setLastReference(kind: ResourceKind, id: string, entityId?: string): void {
    setLastReference(this.cfg, kind, id, entityId);
  }

  getActiveEntityId(): string | undefined {
    return getActiveEntityId(this.cfg);
  }
}

// ---------------------------------------------------------------------------
// ReferenceResolver — Node/CLI-specific resolver with API calls and caching
// ---------------------------------------------------------------------------

type Scope = { entityId?: string; bodyId?: string; meetingId?: string };

// Sentinel cache key used for global (non-entity-scoped) resource kinds.
const GLOBAL_KEY = "__global__";

export class ReferenceResolver {
  private readonly client: CorpAPIClient;
  private readonly cfg: CorpConfig;
  private readonly tracker: ReferenceTracker;

  // Single unified cache: ResourceKind → (cacheKey → records)
  private readonly recordsCache = new Map<ResourceKind, Map<string, ApiRecord[]>>();

  // Cap table is fetched as a single object; instruments/share_classes are
  // derived from it, so we keep its own dedicated cache.
  private readonly capTableCache = new Map<string, ApiRecord>();

  // Dispatch table: ResourceKind → fetcher that receives the scope and
  // returns the raw (uncached) records for that kind.
  private readonly fetchers: Map<ResourceKind, (scope: Scope) => Promise<{ key: string; records: ApiRecord[] }>>;

  constructor(client: CorpAPIClient, cfg: CorpConfig) {
    this.client = client;
    this.cfg = cfg;
    this.tracker = new ReferenceTracker(new NodeReferenceStorage(cfg));

    this.fetchers = new Map([
      ["entity", async (_scope) => ({
        key: GLOBAL_KEY,
        records: await this.client.listEntities(),
      })],

      ["agent", async (_scope) => ({
        key: GLOBAL_KEY,
        records: (await this.client.listAgents()) as ApiRecord[],
      })],

      ["contact", async ({ entityId }) => {
        if (!entityId) throw new Error("An entity context is required to resolve contacts.");
        return { key: entityId, records: await this.client.listContacts(entityId) };
      }],

      ["share_transfer", async ({ entityId }) => {
        if (!entityId) throw new Error("An entity context is required to resolve share transfers.");
        return { key: entityId, records: await this.client.listShareTransfers(entityId) };
      }],

      ["invoice", async ({ entityId }) => {
        if (!entityId) throw new Error("An entity context is required to resolve invoices.");
        return { key: entityId, records: await this.client.listInvoices(entityId) };
      }],

      ["bank_account", async ({ entityId }) => {
        if (!entityId) throw new Error("An entity context is required to resolve bank accounts.");
        return { key: entityId, records: await this.client.listBankAccounts(entityId) };
      }],

      ["payment", async ({ entityId }) => {
        if (!entityId) throw new Error("An entity context is required to resolve payments.");
        return { key: entityId, records: await this.client.listPayments(entityId) };
      }],

      ["payroll_run", async ({ entityId }) => {
        if (!entityId) throw new Error("An entity context is required to resolve payroll runs.");
        return { key: entityId, records: await this.client.listPayrollRuns(entityId) };
      }],

      ["distribution", async ({ entityId }) => {
        if (!entityId) throw new Error("An entity context is required to resolve distributions.");
        return { key: entityId, records: await this.client.listDistributions(entityId) };
      }],

      ["reconciliation", async ({ entityId }) => {
        if (!entityId) throw new Error("An entity context is required to resolve reconciliations.");
        return { key: entityId, records: await this.client.listReconciliations(entityId) };
      }],

      ["tax_filing", async ({ entityId }) => {
        if (!entityId) throw new Error("An entity context is required to resolve tax filings.");
        return { key: entityId, records: await this.client.listTaxFilings(entityId) };
      }],

      ["deadline", async ({ entityId }) => {
        if (!entityId) throw new Error("An entity context is required to resolve deadlines.");
        return { key: entityId, records: await this.client.listDeadlines(entityId) };
      }],

      ["classification", async ({ entityId }) => {
        if (!entityId) throw new Error("An entity context is required to resolve contractor classifications.");
        return { key: entityId, records: await this.client.listContractorClassifications(entityId) };
      }],

      ["body", async ({ entityId }) => {
        if (!entityId) throw new Error("An entity context is required to resolve governance bodies.");
        return { key: entityId, records: (await this.client.listGovernanceBodies(entityId)) as ApiRecord[] };
      }],

      ["meeting", async ({ entityId, bodyId }) => {
        if (!entityId) throw new Error("An entity context is required to resolve meetings.");
        const cacheKey = `${entityId}:${bodyId ?? "*"}`;
        const meetings: ApiRecord[] = [];
        if (bodyId) {
          meetings.push(...((await this.client.listMeetings(bodyId, entityId)) as ApiRecord[]));
        } else {
          const bodies = await this.getCachedRecords("body", { entityId });
          for (const body of bodies) {
            const resolvedBodyId = extractId(body, ["body_id", "id"]);
            if (!resolvedBodyId) continue;
            meetings.push(...((await this.client.listMeetings(resolvedBodyId, entityId)) as ApiRecord[]));
          }
        }
        return { key: cacheKey, records: meetings };
      }],

      ["seat", async ({ entityId, bodyId }) => {
        if (!entityId) throw new Error("An entity context is required to resolve seats.");
        const cacheKey = `${entityId}:${bodyId ?? "*"}`;
        const seats: ApiRecord[] = [];
        if (bodyId) {
          seats.push(...((await this.client.getGovernanceSeats(bodyId, entityId)) as ApiRecord[]));
        } else {
          const bodies = await this.getCachedRecords("body", { entityId });
          for (const body of bodies) {
            const resolvedBodyId = extractId(body, ["body_id", "id"]);
            if (!resolvedBodyId) continue;
            seats.push(...((await this.client.getGovernanceSeats(resolvedBodyId, entityId)) as ApiRecord[]));
          }
        }
        return { key: cacheKey, records: seats };
      }],

      ["agenda_item", async ({ entityId, meetingId }) => {
        if (!entityId || !meetingId) {
          throw new Error("Entity and meeting context are required to resolve agenda items.");
        }
        return {
          key: `${entityId}:${meetingId}`,
          records: (await this.client.listAgendaItems(meetingId, entityId)) as ApiRecord[],
        };
      }],

      ["resolution", async ({ entityId, meetingId }) => {
        if (!entityId) throw new Error("An entity context is required to resolve resolutions.");
        const cacheKey = `${entityId}:${meetingId ?? "*"}`;
        const resolutions: ApiRecord[] = [];
        if (meetingId) {
          resolutions.push(...((await this.client.getMeetingResolutions(meetingId, entityId)) as ApiRecord[]));
        } else {
          const meetings = await this.getCachedRecords("meeting", { entityId });
          for (const meeting of meetings) {
            const resolvedMeetingId = extractId(meeting, ["meeting_id", "id"]);
            if (!resolvedMeetingId) continue;
            resolutions.push(...((await this.client.getMeetingResolutions(resolvedMeetingId, entityId)) as ApiRecord[]));
          }
        }
        return { key: cacheKey, records: resolutions };
      }],

      ["document", async ({ entityId }) => {
        if (!entityId) throw new Error("An entity context is required to resolve documents.");
        return { key: entityId, records: (await this.client.getEntityDocuments(entityId)) as ApiRecord[] };
      }],

      ["work_item", async ({ entityId }) => {
        if (!entityId) throw new Error("An entity context is required to resolve work items.");
        return { key: entityId, records: (await this.client.listWorkItems(entityId)) as ApiRecord[] };
      }],

      ["valuation", async ({ entityId }) => {
        if (!entityId) throw new Error("An entity context is required to resolve valuations.");
        return { key: entityId, records: (await this.client.getValuations(entityId)) as ApiRecord[] };
      }],

      ["safe_note", async ({ entityId }) => {
        if (!entityId) throw new Error("An entity context is required to resolve SAFE notes.");
        return { key: entityId, records: (await this.client.getSafeNotes(entityId)) as ApiRecord[] };
      }],

      ["instrument", async ({ entityId }) => {
        if (!entityId) throw new Error("An entity context is required to resolve cap table resources.");
        const capTable = await this.getCapTable(entityId);
        return {
          key: entityId,
          records: Array.isArray(capTable.instruments) ? (capTable.instruments as ApiRecord[]) : [],
        };
      }],

      ["share_class", async ({ entityId }) => {
        if (!entityId) throw new Error("An entity context is required to resolve cap table resources.");
        const capTable = await this.getCapTable(entityId);
        return {
          key: entityId,
          records: Array.isArray(capTable.share_classes) ? (capTable.share_classes as ApiRecord[]) : [],
        };
      }],

      ["round", async ({ entityId }) => {
        if (!entityId) throw new Error("An entity context is required to resolve rounds.");
        return { key: entityId, records: (await this.client.listEquityRounds(entityId)) as ApiRecord[] };
      }],

      ["service_request", async ({ entityId }) => {
        if (!entityId) throw new Error("An entity context is required to resolve service requests.");
        return { key: entityId, records: (await this.client.listServiceRequests(entityId)) as ApiRecord[] };
      }],
    ]);
  }

  /**
   * Public generic resolver for any resource kind.
   * Used by the generic executor when a positional arg declares `posKind`.
   * Entity-scoped kinds require an entityId in the scope.
   */
  async resolveByKind(kind: ResourceKind, ref: string, entityId?: string): Promise<string> {
    return this.resolve(kind, ref, { entityId });
  }

  async resolveEntity(ref?: string): Promise<string> {
    if (ref !== undefined && ref !== null && !ref.trim()) {
      // An explicit but empty/whitespace-only ref is likely a bug in a script
      throw new Error(
        "Entity reference is empty or whitespace. If you want the active entity, omit --entity-id entirely.",
      );
    }
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

  async resolveServiceRequest(entityId: string, ref: string): Promise<string> {
    return this.resolve("service_request", ref, { entityId });
  }

  async find(
    kind: ResourceKind,
    query: string,
    scope: Scope = {},
  ): Promise<{ kind: ResourceKind; id: string; short_id: string; label: string; alias?: string; raw: ApiRecord }[]> {
    const records = await this.listRecords(kind, scope);
    return this.tracker.findMatches(kind, query, records);
  }

  getLastId(kind: ResourceKind, entityId?: string): string | undefined {
    return getLastReference(this.cfg, kind, entityId);
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

  private async listRecords(kind: ResourceKind, scope: Scope): Promise<ApiRecord[]> {
    const records = await this.getCachedRecords(kind, scope);
    return this.attachStableHandles(kind, records, scope.entityId);
  }

  /**
   * Returns the cached records for a given kind and scope, invoking the
   * fetcher on a cache miss. Does NOT attach stable handles — use
   * `listRecords` for that.
   */
  private async getCachedRecords(kind: ResourceKind, scope: Scope): Promise<ApiRecord[]> {
    const fetcher = this.fetchers.get(kind);
    if (!fetcher) {
      throw new Error(`No fetcher registered for resource kind "${kind}".`);
    }

    let kindCache = this.recordsCache.get(kind);
    if (!kindCache) {
      kindCache = new Map<string, ApiRecord[]>();
      this.recordsCache.set(kind, kindCache);
    }

    // We must compute the cache key before checking — let the fetcher derive
    // it by calling it once if no entry exists. To avoid redundant API calls
    // we check with a probe: if the fetcher has a deterministic key we can
    // pre-check. Instead, we just call the fetcher to get both key+records,
    // but only invoke it on a miss.
    //
    // For kinds whose key depends solely on the scope (all current kinds),
    // we can determine the key cheaply. We do this by invoking the fetcher
    // only when necessary.
    //
    // Strategy: call fetcher to get (key, records); store; return records.
    // On subsequent calls the kindCache will already have the key.
    //
    // Since we don't know the key ahead of time for composite-key kinds
    // (meeting, seat, agenda_item, resolution), we call the fetcher and
    // check after. For simple-key kinds this is equivalent to the old
    // per-field pattern.

    // Fast path: derive the expected cache key without calling the fetcher.
    const probeKey = this.probeKey(kind, scope);
    if (probeKey !== undefined) {
      const hit = kindCache.get(probeKey);
      if (hit) return hit;
      const { key, records } = await fetcher(scope);
      kindCache.set(key, records);
      return records;
    }

    // Fallback (should not be reached with current kinds, but defensive):
    const { key, records } = await fetcher(scope);
    const hit = kindCache.get(key);
    if (hit) return hit;
    kindCache.set(key, records);
    return records;
  }

  /**
   * Returns the expected cache key for a given kind and scope without
   * invoking the fetcher. Returns `undefined` if the key cannot be
   * determined without fetching (e.g., composite-key kinds that need to
   * enumerate sub-resources first).
   */
  private probeKey(kind: ResourceKind, scope: Scope): string | undefined {
    switch (kind) {
      case "entity":
      case "agent":
        return GLOBAL_KEY;
      case "meeting":
      case "seat":
        if (!scope.entityId) return GLOBAL_KEY; // will throw in fetcher
        return `${scope.entityId}:${scope.bodyId ?? "*"}`;
      case "agenda_item":
        if (!scope.entityId || !scope.meetingId) return GLOBAL_KEY; // will throw in fetcher
        return `${scope.entityId}:${scope.meetingId}`;
      case "resolution":
        if (!scope.entityId) return GLOBAL_KEY; // will throw in fetcher
        return `${scope.entityId}:${scope.meetingId ?? "*"}`;
      default:
        return scope.entityId ?? GLOBAL_KEY;
    }
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

    // Chunk into batches of 400 to stay under the API's 500-item limit
    const SYNC_BATCH_SIZE = 400;
    const handleById = new Map<string, string>();
    const scopeEntityId = isEntityScopedKind(kind) ? entityId : undefined;

    for (let i = 0; i < missing.length; i += SYNC_BATCH_SIZE) {
      const batch = missing.slice(i, i + SYNC_BATCH_SIZE);
      const response = await this.client.syncReferences(
        kind,
        batch.map(({ described }) => ({
          resource_id: described.id,
          label: described.label,
        })),
        scopeEntityId,
      );
      for (const reference of response.references) {
        if (typeof reference.resource_id === "string" && typeof reference.handle === "string") {
          handleById.set(reference.resource_id, reference.handle);
        }
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

  private async getCapTable(entityId?: string): Promise<ApiRecord> {
    if (!entityId) throw new Error("An entity context is required to resolve cap table resources.");
    const cached = this.capTableCache.get(entityId);
    if (cached) return cached;
    const capTable = (await this.client.getCapTable(entityId)) as ApiRecord;
    this.capTableCache.set(entityId, capTable);
    return capTable;
  }
}
