import chalk from "chalk";
import Table from "cli-table3";
import type { ApiRecord } from "./types.js";

const URGENCY_COLORS: Record<string, (s: string) => string> = {
  overdue: chalk.red.bold,
  due_today: chalk.yellow.bold,
  d1: chalk.yellow,
  d7: chalk.cyan,
  d14: chalk.blue,
  d30: chalk.dim,
  upcoming: chalk.dim,
};

export function printError(msg: string): void {
  console.error(chalk.red("Error:"), msg);
}

export function printSuccess(msg: string): void {
  console.log(chalk.green(msg));
}

export function printWarning(msg: string): void {
  console.log(chalk.yellow(msg));
}

export function printJson(data: unknown): void {
  console.log(JSON.stringify(data, null, 2));
}

// --- Status Panel ---

export function printStatusPanel(data: ApiRecord): void {
  console.log(chalk.blue("─".repeat(50)));
  console.log(chalk.blue.bold("  Corp Status"));
  console.log(chalk.blue("─".repeat(50)));
  console.log(`  ${chalk.bold("Workspace:")} ${data.workspace_id ?? "N/A"}`);
  console.log(`  ${chalk.bold("Entities:")}  ${data.entity_count ?? 0}`);

  const urgency = (data.urgency_counts ?? {}) as Record<string, number>;
  if (Object.keys(urgency).length > 0) {
    console.log(`\n  ${chalk.bold("Obligations:")}`);
    for (const [tier, count] of Object.entries(urgency)) {
      const colorFn = URGENCY_COLORS[tier] ?? ((s: string) => s);
      console.log(`    ${colorFn(`${tier}:`)} ${count}`);
    }
  }

  if (data.next_deadline) {
    console.log(`\n  ${chalk.bold("Next deadline:")} ${data.next_deadline}`);
  }
  console.log(chalk.blue("─".repeat(50)));
}

// --- Generic table helper ---

function makeTable(title: string, columns: string[]): Table.Table {
  console.log(`\n${chalk.bold(title)}`);
  return new Table({ head: columns.map((c) => chalk.dim(c)) });
}

function s(val: unknown, maxLen?: number): string {
  const str = val == null ? "" : String(val);
  if (maxLen && str.length > maxLen) return str.slice(0, maxLen);
  return str;
}

function money(val: unknown, cents = true): string {
  if (typeof val === "number") {
    const dollars = cents ? val / 100 : val;
    return `$${dollars.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })}`;
  }
  return String(val ?? "");
}

function date(val: unknown): string {
  const str = s(val);
  if (!str) return "";
  const parsed = new Date(str);
  return Number.isNaN(parsed.getTime()) ? str : parsed.toISOString().slice(0, 10);
}

// --- Domain tables ---

export function printEntitiesTable(entities: ApiRecord[]): void {
  const table = makeTable("Entities", ["ID", "Name", "Type", "Jurisdiction", "Status"]);
  for (const e of entities) {
    table.push([s(e.entity_id, 12), s(e.legal_name ?? e.name), s(e.entity_type), s(e.jurisdiction), s(e.formation_status ?? e.status)]);
  }
  console.log(table.toString());
}

export function printObligationsTable(obligations: ApiRecord[]): void {
  const table = makeTable("Obligations", ["ID", "Type", "Urgency", "Due", "Status"]);
  for (const o of obligations) {
    const urg = s(o.urgency) || "upcoming";
    const colorFn = URGENCY_COLORS[urg] ?? ((x: string) => x);
    table.push([s(o.obligation_id, 12), s(o.obligation_type), colorFn(urg), s(o.due_at), s(o.status)]);
  }
  console.log(table.toString());
}

export function printContactsTable(contacts: ApiRecord[]): void {
  const table = makeTable("Contacts", ["ID", "Name", "Email", "Category", "Entity"]);
  for (const c of contacts) {
    table.push([
      s(c.contact_id ?? c.id, 12),
      s(c.name),
      s(c.email),
      s(c.category),
      s(c.entity_name ?? c.entity_id),
    ]);
  }
  console.log(table.toString());
}

export function printCapTable(data: ApiRecord): void {
  const accessLevel = s(data.access_level) || "admin";
  const shareClasses = (data.share_classes ?? []) as ApiRecord[];
  if (shareClasses.length > 0) {
    const cols = ["Class", "Authorized", "Outstanding"];
    if (accessLevel !== "summary") cols.push("Holders");
    const table = makeTable("Cap Table — Share Classes", cols);
    for (const sc of shareClasses) {
      const row = [s(sc.class_code ?? sc.name), s(sc.authorized), s(sc.outstanding)];
      if (accessLevel !== "summary") {
        const holders = (sc.holders ?? []) as ApiRecord[];
        row.push(holders.map((h) => `${h.name ?? "?"}(${h.percentage ?? "?"}%)`).join(", "));
      }
      table.push(row);
    }
    console.log(table.toString());
  }

  const ownership = (data.ownership ?? []) as ApiRecord[];
  if (ownership.length > 0 && accessLevel !== "summary") {
    const table = makeTable("Ownership Breakdown", ["Holder", "Shares", "Percentage", "Class"]);
    for (const o of ownership) {
      table.push([s(o.holder_name ?? o.name), s(o.shares), `${o.percentage ?? ""}%`, s(o.share_class)]);
    }
    console.log(table.toString());
  }

  const pools = (data.option_pools ?? []) as ApiRecord[];
  if (pools.length > 0) {
    const table = makeTable("Option Pools", ["Name", "Authorized", "Granted", "Available"]);
    for (const p of pools) {
      table.push([s(p.name), s(p.authorized), s(p.granted), s(p.available)]);
    }
    console.log(table.toString());
  }

  if (data.fully_diluted_shares != null) {
    const fd = data.fully_diluted_shares;
    console.log(`\n${chalk.bold("Fully Diluted Shares:")} ${typeof fd === "number" ? fd.toLocaleString() : fd}`);
  }
}

export function printSafesTable(safes: ApiRecord[]): void {
  const table = makeTable("SAFE Notes", ["ID", "Investor", "Amount", "Cap", "Discount", "Date"]);
  for (const s_ of safes) {
    table.push([
      s(s_.safe_id ?? s_.id, 12),
      s(s_.investor_name ?? s_.investor),
      money(s_.investment_amount ?? s_.amount, false),
      s(s_.valuation_cap ?? s_.cap),
      s(s_.discount_rate ?? s_.discount),
      s(s_.date ?? s_.created_at),
    ]);
  }
  console.log(table.toString());
}

export function printTransfersTable(transfers: ApiRecord[]): void {
  const table = makeTable("Share Transfers", ["ID", "From", "To", "Shares", "Class", "Date"]);
  for (const t of transfers) {
    table.push([
      s(t.transfer_id ?? t.id, 12),
      s(t.from_holder ?? t.from),
      s(t.to_holder ?? t.to),
      s(t.shares),
      s(t.share_class),
      s(t.date ?? t.transfer_date),
    ]);
  }
  console.log(table.toString());
}

export function printValuationsTable(valuations: ApiRecord[]): void {
  const table = makeTable("Valuations", ["Date", "Type", "Valuation", "PPS"]);
  for (const v of valuations) {
    table.push([
      date(v.effective_date ?? v.valuation_date ?? v.date),
      s(v.valuation_type ?? v.type),
      money(v.enterprise_value_cents ?? v.enterprise_value ?? v.valuation),
      money(v.fmv_per_share_cents ?? v.price_per_share ?? v.pps ?? v.fmv_per_share),
    ]);
  }
  console.log(table.toString());
}

export function printGovernanceTable(bodies: ApiRecord[]): void {
  const table = makeTable("Governance Bodies", ["ID", "Body", "Type", "Seats", "Meetings"]);
  for (const b of bodies) {
    table.push([
      s(b.body_id ?? b.id, 12),
      s(b.name),
      s(b.body_type ?? b.type),
      s(b.seat_count ?? b.seats),
      s(b.meeting_count ?? b.meetings),
    ]);
  }
  console.log(table.toString());
}

export function printSeatsTable(seats: ApiRecord[]): void {
  const table = makeTable("Seats", ["Seat", "Holder", "Role", "Status"]);
  for (const st of seats) {
    table.push([s(st.seat_name ?? st.title), s(st.holder_name ?? st.holder), s(st.role), s(st.status)]);
  }
  console.log(table.toString());
}

export function printMeetingsTable(meetings: ApiRecord[]): void {
  const table = makeTable("Meetings", ["ID", "Title", "Date", "Status", "Resolutions"]);
  for (const m of meetings) {
    table.push([
      s(m.meeting_id ?? m.id, 12),
      s(m.title ?? m.name),
      s(m.scheduled_date ?? m.meeting_date ?? m.date),
      s(m.status),
      s(m.resolution_count ?? m.resolutions),
    ]);
  }
  console.log(table.toString());
}

export function printResolutionsTable(resolutions: ApiRecord[]): void {
  const table = makeTable("Resolutions", ["ID", "Title", "Type", "Status", "For", "Against"]);
  for (const r of resolutions) {
    table.push([
      s(r.resolution_id ?? r.id, 12),
      s(r.title),
      s(r.resolution_type ?? r.type),
      s(r.status),
      s(r.votes_for),
      s(r.votes_against),
    ]);
  }
  console.log(table.toString());
}

export function printDocumentsTable(docs: ApiRecord[]): void {
  const table = makeTable("Documents", ["ID", "Title", "Type", "Date", "Status", "Signatures"]);
  for (const d of docs) {
    const sigs = d.signatures;
    const sigStr = Array.isArray(sigs) ? `${sigs.length} signed` : s(sigs);
    table.push([
      s(d.document_id ?? d.id, 12),
      s(d.title ?? d.name),
      s(d.document_type ?? d.type),
      s(d.date ?? d.created_at),
      s(d.status),
      sigStr,
    ]);
  }
  console.log(table.toString());
}

export function printWorkItemsTable(items: ApiRecord[]): void {
  const table = makeTable("Work Items", ["ID", "Title", "Category", "Status", "Deadline", "Claimed By"]);
  for (const w of items) {
    const status = s(w.effective_status ?? w.status);
    const colored =
      status === "completed" ? chalk.green(status) :
      status === "claimed" ? chalk.yellow(status) :
      status === "cancelled" ? chalk.dim(status) :
      status;
    table.push([
      s(w.work_item_id ?? w.id, 12),
      s(w.title),
      s(w.category),
      colored,
      w.asap ? chalk.red.bold("ASAP") : s(w.deadline ?? ""),
      s(w.claimed_by ?? ""),
    ]);
  }
  console.log(table.toString());
}

export function printAgentsTable(agents: ApiRecord[]): void {
  const table = makeTable("Agents", ["ID", "Name", "Status", "Model"]);
  for (const a of agents) {
    const status = s(a.status);
    const colored =
      status === "active" ? chalk.green(status) : status === "paused" ? chalk.yellow(status) : status;
    table.push([s(a.agent_id ?? a.id, 12), s(a.name), colored, s(a.model)]);
  }
  console.log(table.toString());
}

export function printApprovalsTable(approvals: ApiRecord[]): void {
  const table = makeTable("Pending Approvals", ["ID", "Type", "Requested By", "Description", "Created"]);
  for (const a of approvals) {
    let desc = s(a.description ?? a.summary);
    if (desc.length > 60) desc = desc.slice(0, 57) + "...";
    table.push([
      s(a.approval_id ?? a.id, 12),
      s(a.approval_type ?? a.type),
      s(a.requested_by ?? a.requester),
      desc,
      s(a.created_at),
    ]);
  }
  console.log(table.toString());
}

export function printBillingPanel(status: ApiRecord, plans: ApiRecord[]): void {
  const plan = s(status.plan ?? status.tier) || "free";
  const subStatus = s(status.status) || "active";
  const periodEnd = s(status.current_period_end);

  console.log(chalk.green("─".repeat(50)));
  console.log(chalk.green.bold("  Billing Status"));
  console.log(chalk.green("─".repeat(50)));
  console.log(`  ${chalk.bold("Plan:")} ${plan}`);
  console.log(`  ${chalk.bold("Status:")} ${subStatus}`);
  if (periodEnd) console.log(`  ${chalk.bold("Current Period End:")} ${periodEnd}`);
  console.log(chalk.dim("  Manage:  corp billing portal"));
  console.log(chalk.dim("  Upgrade: corp billing upgrade --plan <plan>"));
  console.log(chalk.green("─".repeat(50)));

  if (plans.length > 0) {
    const table = makeTable("Available Plans", ["Plan", "Price", "Features"]);
    for (const p of plans) {
      const amount = (p.price_cents ?? p.amount ?? 0) as number;
      const interval = s(p.interval);
      let priceStr = "Free";
      if (amount > 0) {
        priceStr = interval ? `$${Math.round(amount / 100)}/${interval}` : `$${Math.round(amount / 100)}`;
      }
      const name = s(p.name ?? p.plan_id ?? p.tier);
      const features = Array.isArray(p.features) ? (p.features as string[]).join(", ") : s(p.description);
      table.push([name, priceStr, features]);
    }
    console.log(table.toString());
  }
}
