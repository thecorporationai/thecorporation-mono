import type { CommandDef, CommandContext } from "./registry/types.js";
import type { ResourceKind } from "./references.js";
import { withSpinner } from "./spinner.js";

// ── Formatting helpers (local versions matching output.ts private helpers) ──

function s(val: unknown, maxLen?: number): string {
  const str = val == null ? "" : String(val);
  if (maxLen && str.length > maxLen) return str.slice(0, maxLen);
  return str;
}

function money(val: unknown): string {
  if (typeof val === "number") {
    const dollars = val / 100;
    return `$${dollars.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })}`;
  }
  return s(val);
}

function fmtDate(val: unknown): string {
  const str = s(val);
  if (!str) return "";
  const parsed = new Date(str);
  return Number.isNaN(parsed.getTime()) ? str : parsed.toISOString().slice(0, 10);
}

function shortId(val: string): string {
  return val.length > 8 ? val.slice(0, 8) + "\u2026" : val;
}

// ── Column spec parsing ──

interface ColSpec {
  keys: string[];
  label: string;
  fmt: "money" | "date" | "id" | null;
}

function parseCol(spec: string): ColSpec {
  let fmt: ColSpec["fmt"] = null;
  let rest = spec;
  if (rest[0] === "$") {
    fmt = "money";
    rest = rest.slice(1);
  } else if (rest[0] === "@") {
    fmt = "date";
    rest = rest.slice(1);
  } else if (rest[0] === "#") {
    fmt = "id";
    rest = rest.slice(1);
  }
  const [fieldPart, label] = rest.split(">");
  return { keys: fieldPart.split("|"), label: label || fieldPart, fmt };
}

function getField(obj: Record<string, unknown>, keys: string[]): unknown {
  for (const k of keys) {
    if (obj[k] != null) return obj[k];
  }
  return null;
}

function fmtField(val: unknown, fmt: ColSpec["fmt"]): string {
  if (val == null) return "";
  if (fmt === "money") return money(val);
  if (fmt === "date") return fmtDate(val);
  if (fmt === "id") return shortId(String(val));
  return String(val);
}

// ── Auto-detect columns from first item ──

function autoCols(items: unknown[]): ColSpec[] {
  if (!items.length) return [];
  const sample = items[0];
  if (typeof sample !== "object" || sample === null) return [];

  const keys = Object.keys(sample as Record<string, unknown>);
  const priority = [
    "name", "legal_name", "title", "slug", "symbol", "type", "kind",
    "entity_type", "body_type", "status", "effective_status", "category", "email",
    "description", "amount_cents", "total_cents", "due_date", "due_at", "created_at", "date",
  ];

  const picked: ColSpec[] = [];
  for (const p of priority) {
    if (keys.includes(p) && picked.length < 5) {
      let fmt: ColSpec["fmt"] = null;
      if (p.endsWith("_cents")) fmt = "money";
      else if (p.includes("date") || p.endsWith("_at")) fmt = "date";
      const label = p
        .replace(/_cents$/, "")
        .replace(/_/g, " ")
        .replace(/\b\w/g, (ch) => ch.toUpperCase());
      picked.push({ keys: [p], label, fmt });
    }
  }

  // Add an ID column if available
  const idKeys = keys.filter((k) => k.endsWith("_id") && k !== "workspace_id" && k !== "entity_id");
  if (idKeys.length && picked.length < 6) {
    picked.push({ keys: [idKeys[0]], label: "ID", fmt: "id" });
  }

  return picked;
}

// ── Panel display for single objects ──

function displayPanel(data: Record<string, unknown>, title: string, ctx: CommandContext): void {
  const entries = Object.entries(data).filter(
    ([k, v]) => v != null && typeof v !== "object" && k !== "workspace_id",
  );
  const lines = entries.slice(0, 15).map(([k, v]) => {
    const label = k
      .replace(/_/g, " ")
      .replace(/\b\w/g, (ch) => ch.toUpperCase());
    let formatted: string;
    if (k.endsWith("_cents") && typeof v === "number") formatted = money(v);
    else if ((k.includes("date") || k.endsWith("_at")) && v) formatted = fmtDate(v);
    else if (k.endsWith("_id")) formatted = shortId(String(v));
    else formatted = String(v);
    return `${label}: ${formatted}`;
  });
  ctx.writer.panel(title, "blue", lines);
}

// ── Positional arg resolution ──

/**
 * Look up the posKind for the positional arg at the given index.
 * Returns undefined when no posKind is declared (backwards-compatible passthrough).
 */
function getPosKind(def: CommandDef, posIndex: number): ResourceKind | undefined {
  if (!def.args) return undefined;
  // posIndex counts only positional args that have been consumed so far.
  // def.args is ordered, and positional args correspond in order.
  let argIdx = 0;
  for (const arg of def.args) {
    if (argIdx === posIndex) {
      return arg.posKind as ResourceKind | undefined;
    }
    argIdx++;
  }
  return undefined;
}

/**
 * Resolve a positional arg through the reference resolver if posKind is set.
 * Otherwise returns the raw value unchanged.
 */
async function resolvePositional(
  def: CommandDef,
  posIndex: number,
  rawValue: string,
  ctx: CommandContext,
): Promise<string> {
  const kind = getPosKind(def, posIndex);
  if (!kind) return rawValue;
  return ctx.resolver.resolveByKind(kind, rawValue, ctx.entityId);
}

// ── Main executor ──

export async function executeGenericRead(def: CommandDef, ctx: CommandContext): Promise<void> {
  if (!def.route?.path) {
    ctx.writer.error("No route defined for this command");
    return;
  }

  let path = def.route.path;
  const qp: Record<string, string> = {};
  let posIdx = 0;

  // Resolve {eid}
  if (def.entity) {
    let eid: string | undefined;
    const explicitEid = ctx.opts["entity-id"] as string | undefined;

    if (explicitEid) {
      eid = await ctx.resolver.resolveEntity(explicitEid);
    } else if (def.entity === true && !path.includes("{pos}") && ctx.positional[posIdx]) {
      eid = await ctx.resolver.resolveEntity(ctx.positional[posIdx++]);
    } else {
      eid = ctx.entityId; // active entity from config
    }

    if (eid) {
      path = path.replace("{eid}", encodeURIComponent(eid));
      if (def.entity === "query") qp.entity_id = eid;
    } else if (path.includes("{eid}")) {
      ctx.writer.error("Entity ID required. Use --entity-id or set active entity with 'corp use <name>'.");
      return;
    }
  }

  // Resolve {pos} — use reference resolver when posKind is declared
  if (path.includes("{pos}")) {
    if (!ctx.positional[posIdx]) {
      ctx.writer.error("Missing required argument (ID or reference).");
      return;
    }
    const resolved = await resolvePositional(def, posIdx, ctx.positional[posIdx], ctx);
    posIdx++;
    path = path.replace("{pos}", encodeURIComponent(resolved));
  }

  // Resolve workspace ID placeholders
  path = path.replace("{wid}", encodeURIComponent(ctx.client.workspaceId));
  path = path.replace("{workspace_id}", encodeURIComponent(ctx.client.workspaceId));

  // Forward optQP options as query params
  if (def.optQP) {
    for (const optName of def.optQP) {
      const val = ctx.opts[optName];
      if (val) qp[optName] = String(val);
    }
  }

  // Fetch
  const data = await withSpinner(
    "Loading",
    () => ctx.client.fetchJSON(path, Object.keys(qp).length ? qp : undefined),
    ctx.opts.json as boolean,
  );

  // JSON output
  if (ctx.opts.json) {
    ctx.writer.json(data);
    return;
  }

  // Unwrap listKey
  let items = data;
  if (def.display?.listKey && data && !Array.isArray(data)) {
    items = (data as Record<string, unknown>)[def.display.listKey] || [];
  }

  // Display
  const title = def.display?.title || def.name;

  if (Array.isArray(items)) {
    const cols = def.display?.cols ? def.display.cols.map(parseCol) : autoCols(items);
    if (!cols.length && items.length) {
      ctx.writer.json(items); // fallback when no columns can be determined
      return;
    }
    const headers = cols.map((c) => c.label);
    const rows = items.map((item) =>
      cols.map((col) => fmtField(getField(item as Record<string, unknown>, col.keys), col.fmt)),
    );
    ctx.writer.table(title, headers, rows);
  } else if (data && typeof data === "object") {
    // Single object -> panel
    displayPanel(data as Record<string, unknown>, title, ctx);
  } else {
    ctx.writer.json(data);
  }
}

/**
 * Generic write executor for POST/PATCH/DELETE commands that lack a custom handler.
 * Resolves path placeholders, collects option values as the body, and calls submitJSON.
 */
export async function executeGenericWrite(def: CommandDef, ctx: CommandContext): Promise<void> {
  if (!def.route?.path || !def.route?.method) {
    ctx.writer.error("No route defined for this command");
    return;
  }

  let path = def.route.path;
  let posIdx = 0;

  // Resolve {eid}
  if (def.entity) {
    let eid: string | undefined;
    const explicitEid = ctx.opts["entity-id"] as string | undefined;
    if (explicitEid) {
      eid = await ctx.resolver.resolveEntity(explicitEid);
    } else if (def.entity === true && !path.includes("{pos}") && ctx.positional[posIdx]) {
      eid = await ctx.resolver.resolveEntity(ctx.positional[posIdx++]);
    } else {
      eid = ctx.entityId;
    }
    if (eid) {
      path = path.replace("{eid}", encodeURIComponent(eid));
    } else if (path.includes("{eid}")) {
      ctx.writer.error("Entity ID required. Use --entity-id or set active entity with 'corp use <name>'.");
      return;
    }
  }

  // Resolve {pos} and {pos2} — use reference resolver when posKind is declared
  if (path.includes("{pos}")) {
    if (!ctx.positional[posIdx]) {
      ctx.writer.error("Missing required argument (ID or reference).");
      return;
    }
    const resolved = await resolvePositional(def, posIdx, ctx.positional[posIdx], ctx);
    posIdx++;
    path = path.replace("{pos}", encodeURIComponent(resolved));
  }
  if (path.includes("{pos2}")) {
    if (!ctx.positional[posIdx]) {
      ctx.writer.error("Missing required second argument (ID or reference).");
      return;
    }
    const resolved = await resolvePositional(def, posIdx, ctx.positional[posIdx], ctx);
    posIdx++;
    path = path.replace("{pos2}", encodeURIComponent(resolved));
  }

  // Resolve workspace placeholders
  path = path.replace("{wid}", encodeURIComponent(ctx.client.workspaceId));
  path = path.replace("{workspace_id}", encodeURIComponent(ctx.client.workspaceId));

  // Build body from defined options
  const body: Record<string, unknown> = {};
  if (def.entity && ctx.entityId) {
    body.entity_id = ctx.entityId;
  }
  for (const opt of def.options ?? []) {
    // Extract camelCase key from flags like "--foo-bar <val>"
    const match = opt.flags.match(/^--([a-z0-9-]+)/);
    if (!match) continue;
    const camelKey = match[1].replace(/-([a-z])/g, (_, c: string) => c.toUpperCase());
    const val = ctx.opts[camelKey];
    if (val != null && camelKey !== "entityId" && camelKey !== "json") {
      body[match[1].replace(/-/g, "_")] = val;
    }
  }

  if (ctx.dryRun) {
    ctx.writer.dryRun(def.name.replace(/ /g, "."), body);
    return;
  }

  const data = await withSpinner(
    "Submitting",
    () => ctx.client.submitJSON(def.route!.method, path, Object.keys(body).length ? body : undefined),
    ctx.opts.json as boolean,
  );

  if (ctx.opts.json) {
    ctx.writer.json(data);
    return;
  }

  // Try to extract an ID from the response for a friendly message
  const result = data as Record<string, unknown> | null;
  const idKey = result ? Object.keys(result).find((k) => k.endsWith("_id")) : undefined;
  const idVal = idKey && result ? result[idKey] : undefined;
  ctx.writer.success(`${def.description ?? def.name}: ${idVal ?? "OK"}`);
}
