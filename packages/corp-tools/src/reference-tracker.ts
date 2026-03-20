/**
 * Browser-compatible reference matching and tracking core.
 *
 * Pure functions for matching, ranking, and describing resource references,
 * plus the pluggable ReferenceTracker class that works in any JS runtime.
 */

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

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
  | "round"
  | "service_request";

export type MatchRecord = {
  id: string;
  label: string;
  tokens: Set<string>;
  raw: Record<string, unknown>;
};

export type ReferenceMatch = {
  kind: ResourceKind;
  id: string;
  short_id: string;
  label: string;
  alias?: string;
  raw: Record<string, unknown>;
};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

export const RESOURCE_KINDS = [
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
  "service_request",
] as const satisfies readonly ResourceKind[];

const VALID_RESOURCE_KINDS = new Set<ResourceKind>(RESOURCE_KINDS);
const MAX_REFERENCE_INPUT_LEN = 256;

// ---------------------------------------------------------------------------
// Pure helpers
// ---------------------------------------------------------------------------

export function normalize(value: string): string {
  return value.trim().toLowerCase();
}

export function validateReferenceInput(
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

export function isOpaqueUuid(value: string): boolean {
  return /^[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i.test(
    value.trim(),
  );
}

export function isShortIdCandidate(value: string): boolean {
  const trimmed = value.trim();
  return /^[0-9a-f-]{4,}$/i.test(trimmed) || /^[a-z]+_[a-z0-9_-]{3,}$/i.test(trimmed);
}

export function parseLastReference(value: string): { isLast: boolean; kind?: ResourceKind } {
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

export function uniqueStrings(values: Array<string | undefined | null>): Set<string> {
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

export function kindLabel(kind: ResourceKind): string {
  return kind.replaceAll("_", " ");
}

export function isEntityScopedKind(kind: ResourceKind): boolean {
  return kind !== "entity" && kind !== "agent";
}

export function extractId(record: Record<string, unknown>, fields: string[]): string | undefined {
  for (const field of fields) {
    const value = record[field];
    if (typeof value === "string" && value.trim()) {
      return value.trim();
    }
  }
  return undefined;
}

export function isValidResourceKind(kind: string): kind is ResourceKind {
  return VALID_RESOURCE_KINDS.has(kind as ResourceKind);
}

// ---------------------------------------------------------------------------
// describeReferenceRecord — extract id / label / tokens from a raw record
// ---------------------------------------------------------------------------

export function describeReferenceRecord(
  kind: ResourceKind,
  record: Record<string, unknown>,
): MatchRecord | null {
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
    service_request: {
      idFields: ["request_id", "service_request_id", "id"],
      labelFields: ["service_slug", "status"],
    },
  };
  const spec = specs[kind];
  const id = extractId(record, spec.idFields);
  if (!id) {
    return null;
  }
  const labels = spec.labelFields
    .map((field) => record[field])
    .filter((value): value is string => typeof value === "string" && value.trim().length > 0);
  const persistedHandle =
    typeof record.handle === "string" && record.handle.trim().length > 0
      ? record.handle.trim()
      : undefined;
  let label = labels[0] ?? id;
  if (kind === "share_transfer") {
    const fromHolder = typeof record.from_holder === "string" ? record.from_holder.trim() : "";
    const toHolder = typeof record.to_holder === "string" ? record.to_holder.trim() : "";
    const transferType =
      typeof record.transfer_type === "string" ? record.transfer_type.trim() : "";
    const composite = [
      fromHolder && toHolder ? `${fromHolder}-to-${toHolder}` : "",
      transferType,
    ]
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

// ---------------------------------------------------------------------------
// getReferenceId / getReferenceLabel / getReferenceAlias
// ---------------------------------------------------------------------------

export function getReferenceId(
  kind: ResourceKind,
  record: Record<string, unknown>,
): string | undefined {
  return describeReferenceRecord(kind, record)?.id;
}

export function getReferenceLabel(
  kind: ResourceKind,
  record: Record<string, unknown>,
): string | undefined {
  return describeReferenceRecord(kind, record)?.label;
}

export function getReferenceAlias(
  kind: ResourceKind,
  record: Record<string, unknown>,
): string | undefined {
  if (typeof record.handle === "string" && record.handle.trim().length > 0) {
    return record.handle.trim();
  }
  const described = describeReferenceRecord(kind, record);
  if (!described) return undefined;
  const alias = slugify(described.label);
  return alias || shortId(described.id);
}

// ---------------------------------------------------------------------------
// matchRank — rank how well a record matches a normalized query
// ---------------------------------------------------------------------------

export function matchRank(record: MatchRecord, normalizedQuery: string): number {
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

// ---------------------------------------------------------------------------
// ReferenceStorage — pluggable storage interface
// ---------------------------------------------------------------------------

export interface ReferenceStorage {
  getLastReference(kind: ResourceKind, entityId?: string): string | undefined;
  setLastReference(kind: ResourceKind, id: string, entityId?: string): void;
  getActiveEntityId(): string | undefined;
}

// ---------------------------------------------------------------------------
// ReferenceTracker — browser-compatible matching + @last tracking
// ---------------------------------------------------------------------------

export class ReferenceTracker {
  constructor(private storage: ReferenceStorage) {}

  /** Remember a reference for @last reuse. */
  remember(kind: ResourceKind, id: string, entityId?: string): void {
    this.storage.setLastReference(kind, id, entityId);
  }

  /** Resolve @last / @last:kind references. */
  resolveLastReference(
    ref: string,
    kind: ResourceKind,
    entityId?: string,
  ): { isLast: boolean; kind?: ResourceKind; id?: string } {
    const parsed = parseLastReference(ref);
    if (!parsed.isLast) return parsed;
    const lastKind = parsed.kind ?? kind;
    if (lastKind !== kind) {
      throw new Error(
        `@last:${lastKind} cannot be used where a ${kindLabel(kind)} reference is required.`,
      );
    }
    const id = this.storage.getLastReference(lastKind, entityId);
    return { ...parsed, id };
  }

  /** Find the single best match for a query against a list of records. */
  findBestMatch(
    kind: ResourceKind,
    query: string,
    records: Record<string, unknown>[],
  ): ReferenceMatch | null {
    const matches = this.findMatches(kind, query, records);
    return matches.length > 0 ? matches[0] : null;
  }

  /** Find all matching records ranked by relevance. */
  findMatches(
    kind: ResourceKind,
    query: string,
    records: Record<string, unknown>[],
  ): ReferenceMatch[] {
    const trimmedQuery = validateReferenceInput(query, "query", { allowEmpty: true });
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
      .sort(
        (left, right) =>
          matchRank(left, normalizedQuery) - matchRank(right, normalizedQuery) ||
          left.label.localeCompare(right.label) ||
          left.id.localeCompare(right.id),
      );

    return matches.map((record) => ({
      kind,
      id: record.id,
      short_id: shortId(record.id),
      label: record.label,
      alias: getReferenceAlias(kind, record.raw),
      raw: record.raw,
    }));
  }
}
