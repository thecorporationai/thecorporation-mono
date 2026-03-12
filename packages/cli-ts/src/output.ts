import chalk from "chalk";
import Table from "cli-table3";
import type { ApiRecord } from "./types.js";
import {
  getReferenceAlias,
  getReferenceId,
  shortId,
  type ResourceKind,
} from "./references.js";

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

export function printDryRun(operation: string, payload: unknown): void {
  printJson({
    dry_run: true,
    operation,
    payload,
  });
}

type WriteResultOptions =
  | boolean
  | {
      jsonOnly?: boolean;
      referenceKind?: ResourceKind;
      referenceLabel?: string;
      showReuseHint?: boolean;
    };

function normalizeWriteResultOptions(options?: WriteResultOptions): {
  jsonOnly?: boolean;
  referenceKind?: ResourceKind;
  referenceLabel?: string;
  showReuseHint?: boolean;
} {
  if (typeof options === "boolean") {
    return { jsonOnly: options };
  }
  return options ?? {};
}

function formatReferenceCell(kind: ResourceKind, record: ApiRecord): string {
  const id = getReferenceId(kind, record);
  if (!id) return "";
  const alias = getReferenceAlias(kind, record);
  return alias ? `${alias} [${shortId(id)}]` : shortId(id);
}

export function printReferenceSummary(
  kind: ResourceKind,
  record: ApiRecord,
  opts: { label?: string; showReuseHint?: boolean } = {},
): void {
  const id = getReferenceId(kind, record);
  if (!id) return;
  const alias = getReferenceAlias(kind, record);
  const token = alias ? `${alias} [${shortId(id)}]` : shortId(id);
  console.log(`  ${chalk.bold(opts.label ?? "Ref:")} ${token}`);
  console.log(`  ${chalk.bold("ID:")} ${id}`);
  if (opts.showReuseHint) {
    console.log(`  ${chalk.bold("Reuse:")} @last:${kind}`);
  }
}

export function printWriteResult(
  result: unknown,
  successMessage: string,
  options?: WriteResultOptions,
): void {
  const normalized = normalizeWriteResultOptions(options);
  if (normalized.jsonOnly) {
    printJson(result);
    return;
  }
  printSuccess(successMessage);
  if (
    normalized.referenceKind
    && typeof result === "object"
    && result !== null
    && !Array.isArray(result)
  ) {
    printReferenceSummary(normalized.referenceKind, result as ApiRecord, {
      label: normalized.referenceLabel,
      showReuseHint: normalized.showReuseHint,
    });
  }
  printJson(result);
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

export function printFinanceSummaryPanel(data: ApiRecord): void {
  const invoices = (data.invoices ?? {}) as ApiRecord;
  const bankAccounts = (data.bank_accounts ?? {}) as ApiRecord;
  const payments = (data.payments ?? {}) as ApiRecord;
  const payrollRuns = (data.payroll_runs ?? {}) as ApiRecord;
  const distributions = (data.distributions ?? {}) as ApiRecord;
  const reconciliations = (data.reconciliations ?? {}) as ApiRecord;
  const classifications = (data.contractor_classifications ?? {}) as ApiRecord;

  console.log(chalk.green("─".repeat(54)));
  console.log(chalk.green.bold("  Finance Summary"));
  console.log(chalk.green("─".repeat(54)));
  console.log(`  ${chalk.bold("Entity:")} ${s(data.entity_id) || "N/A"}`);
  console.log(`  ${chalk.bold("Invoices:")} ${s(invoices.count)} total, ${s(invoices.open_count)} open, ${money(invoices.total_amount_cents)}`);
  if (invoices.latest_due_date) {
    console.log(`  ${chalk.bold("Invoice Horizon:")} next due ${date(invoices.latest_due_date)}`);
  }
  console.log(`  ${chalk.bold("Bank Accounts:")} ${s(bankAccounts.active_count)}/${s(bankAccounts.count)} active`);
  console.log(`  ${chalk.bold("Payments:")} ${s(payments.count)} total, ${s(payments.pending_count)} pending, ${money(payments.total_amount_cents)}`);
  console.log(`  ${chalk.bold("Payroll Runs:")} ${s(payrollRuns.count)} total${payrollRuns.latest_period_end ? `, latest ${date(payrollRuns.latest_period_end)}` : ""}`);
  console.log(`  ${chalk.bold("Distributions:")} ${s(distributions.count)} total, ${money(distributions.total_amount_cents)}`);
  console.log(`  ${chalk.bold("Reconciliations:")} ${s(reconciliations.balanced_count)}/${s(reconciliations.count)} balanced`);
  console.log(`  ${chalk.bold("Contractors:")} ${s(classifications.count)} classifications, ${s(classifications.high_risk_count)} high risk`);
  console.log(chalk.green("─".repeat(54)));
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

function actorLabel(record: ApiRecord, field: "claimed_by" | "completed_by" | "created_by"): string {
  const actor = record[`${field}_actor`];
  if (actor && typeof actor === "object" && !Array.isArray(actor)) {
    const label = s((actor as ApiRecord).label);
    const actorType = s((actor as ApiRecord).actor_type);
    if (label) {
      return actorType ? `${label} (${actorType})` : label;
    }
  }
  return s(record[field]);
}

// --- Domain tables ---

export function printEntitiesTable(entities: ApiRecord[]): void {
  const table = makeTable("Entities", ["Ref", "Name", "Type", "Jurisdiction", "Status"]);
  for (const e of entities) {
    table.push([
      formatReferenceCell("entity", e),
      s(e.legal_name ?? e.name),
      s(e.entity_type),
      s(e.jurisdiction),
      s(e.formation_status ?? e.status),
    ]);
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
  const table = makeTable("Contacts", ["Ref", "Name", "Email", "Category", "Entity"]);
  for (const c of contacts) {
    table.push([
      formatReferenceCell("contact", c),
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
  const instruments = (data.instruments ?? []) as ApiRecord[];
  if (instruments.length > 0) {
    const table = makeTable("Cap Table — Instruments", ["Ref", "Symbol", "Kind", "Authorized", "Issued", "Diluted"]);
    for (const instrument of instruments) {
      table.push([
        formatReferenceCell("instrument", instrument),
        s(instrument.symbol),
        s(instrument.kind),
        s(instrument.authorized_units ?? "unlimited"),
        s(instrument.issued_units),
        s(instrument.diluted_units),
      ]);
    }
    console.log(table.toString());
  }

  const holders = (data.holders ?? []) as ApiRecord[];
  if (holders.length > 0 && accessLevel !== "summary") {
    const table = makeTable(
      "Ownership Breakdown",
      ["Holder", "Outstanding", "As Converted", "Fully Diluted", "Fully Diluted %"],
    );
    for (const holder of holders) {
      const dilutedBps = typeof holder.fully_diluted_bps === "number"
        ? `${(holder.fully_diluted_bps / 100).toFixed(2)}%`
        : "";
      table.push([
        s(holder.name),
        s(holder.outstanding_units),
        s(holder.as_converted_units),
        s(holder.fully_diluted_units),
        dilutedBps,
      ]);
    }
    console.log(table.toString());
  }

  const shareClasses = (data.share_classes ?? []) as ApiRecord[];
  if (shareClasses.length > 0) {
    const cols = ["Ref", "Class", "Authorized", "Outstanding"];
    if (accessLevel !== "summary") cols.push("Holders");
    const table = makeTable("Cap Table — Share Classes", cols);
    for (const sc of shareClasses) {
      const row = [
        formatReferenceCell("share_class", sc),
        s(sc.class_code ?? sc.name),
        s(sc.authorized),
        s(sc.outstanding),
      ];
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
  if (data.total_units != null) {
    console.log(`\n${chalk.bold("Cap Table Basis:")} ${s(data.basis) || "outstanding"}`);
    console.log(`${chalk.bold("Total Units:")} ${typeof data.total_units === "number" ? data.total_units.toLocaleString() : data.total_units}`);
  }
}

export function printSafesTable(safes: ApiRecord[]): void {
  const table = makeTable("SAFE Notes", ["Ref", "Investor", "Amount", "Cap", "Discount", "Date"]);
  for (const s_ of safes) {
    table.push([
      formatReferenceCell("safe_note", s_),
      s(s_.investor_name ?? s_.investor),
      money(s_.principal_amount_cents ?? s_.investment_amount ?? s_.amount, false),
      money(s_.valuation_cap_cents ?? s_.valuation_cap ?? s_.cap, false),
      s(s_.discount_rate ?? s_.discount),
      s(s_.issued_at ?? s_.date ?? s_.created_at),
    ]);
  }
  console.log(table.toString());
}

export function printTransfersTable(transfers: ApiRecord[]): void {
  const table = makeTable("Share Transfers", ["Ref", "From", "To", "Shares", "Type", "Status"]);
  for (const t of transfers) {
    table.push([
      formatReferenceCell("share_transfer", t),
      s(t.from_holder ?? t.from),
      s(t.to_holder ?? t.to),
      s(t.shares ?? t.share_count),
      s(t.transfer_type),
      s(t.status),
    ]);
  }
  console.log(table.toString());
}

export function printInstrumentsTable(instruments: ApiRecord[]): void {
  const table = makeTable("Instruments", ["Ref", "Symbol", "Kind", "Authorized", "Issued", "Status"]);
  for (const instrument of instruments) {
    table.push([
      formatReferenceCell("instrument", instrument),
      s(instrument.symbol),
      s(instrument.kind),
      s(instrument.authorized_units ?? "unlimited"),
      s(instrument.issued_units),
      s(instrument.status),
    ]);
  }
  console.log(table.toString());
}

export function printShareClassesTable(shareClasses: ApiRecord[]): void {
  const table = makeTable("Share Classes", ["Ref", "Class", "Authorized", "Outstanding"]);
  for (const shareClass of shareClasses) {
    table.push([
      formatReferenceCell("share_class", shareClass),
      s(shareClass.class_code ?? shareClass.name ?? shareClass.share_class),
      s(shareClass.authorized),
      s(shareClass.outstanding),
    ]);
  }
  console.log(table.toString());
}

export function printRoundsTable(rounds: ApiRecord[]): void {
  const table = makeTable("Equity Rounds", ["Ref", "Name", "Status", "Issuer", "Created"]);
  for (const round of rounds) {
    table.push([
      formatReferenceCell("round", round),
      s(round.name),
      s(round.status),
      s(round.issuer_legal_entity_id),
      date(round.created_at),
    ]);
  }
  console.log(table.toString());
}

export function printInvoicesTable(invoices: ApiRecord[]): void {
  const table = makeTable("Invoices", ["Ref", "Customer", "Amount", "Due", "Status"]);
  for (const invoice of invoices) {
    table.push([
      formatReferenceCell("invoice", invoice),
      s(invoice.customer_name),
      money(invoice.amount_cents),
      date(invoice.due_date),
      s(invoice.status),
    ]);
  }
  console.log(table.toString());
}

export function printBankAccountsTable(accounts: ApiRecord[]): void {
  const table = makeTable("Bank Accounts", ["Ref", "Bank", "Type", "Status", "Created"]);
  for (const account of accounts) {
    table.push([
      formatReferenceCell("bank_account", account),
      s(account.bank_name),
      s(account.account_type),
      s(account.status),
      date(account.created_at),
    ]);
  }
  console.log(table.toString());
}

export function printPaymentsTable(payments: ApiRecord[]): void {
  const table = makeTable("Payments", ["Ref", "Recipient", "Amount", "Method", "Status"]);
  for (const payment of payments) {
    table.push([
      formatReferenceCell("payment", payment),
      s(payment.recipient),
      money(payment.amount_cents),
      s(payment.payment_method),
      s(payment.status),
    ]);
  }
  console.log(table.toString());
}

export function printPayrollRunsTable(runs: ApiRecord[]): void {
  const table = makeTable("Payroll Runs", ["Ref", "Start", "End", "Status", "Created"]);
  for (const run of runs) {
    table.push([
      formatReferenceCell("payroll_run", run),
      date(run.pay_period_start),
      date(run.pay_period_end),
      s(run.status),
      date(run.created_at),
    ]);
  }
  console.log(table.toString());
}

export function printDistributionsTable(distributions: ApiRecord[]): void {
  const table = makeTable("Distributions", ["Ref", "Type", "Amount", "Status", "Description"]);
  for (const distribution of distributions) {
    table.push([
      formatReferenceCell("distribution", distribution),
      s(distribution.distribution_type),
      money(distribution.total_amount_cents),
      s(distribution.status),
      s(distribution.description),
    ]);
  }
  console.log(table.toString());
}

export function printReconciliationsTable(reconciliations: ApiRecord[]): void {
  const table = makeTable("Reconciliations", ["Ref", "As Of", "Debits", "Credits", "Status"]);
  for (const reconciliation of reconciliations) {
    table.push([
      formatReferenceCell("reconciliation", reconciliation),
      date(reconciliation.as_of_date),
      money(reconciliation.total_debits_cents),
      money(reconciliation.total_credits_cents),
      s(reconciliation.status),
    ]);
  }
  console.log(table.toString());
}

export function printValuationsTable(valuations: ApiRecord[]): void {
  const table = makeTable("Valuations", ["Ref", "Date", "Type", "Valuation", "PPS"]);
  for (const v of valuations) {
    table.push([
      formatReferenceCell("valuation", v),
      date(v.effective_date ?? v.valuation_date ?? v.date),
      s(v.valuation_type ?? v.type),
      money(v.enterprise_value_cents ?? v.enterprise_value ?? v.valuation),
      money(v.fmv_per_share_cents ?? v.price_per_share ?? v.pps ?? v.fmv_per_share),
    ]);
  }
  console.log(table.toString());
}

export function printTaxFilingsTable(filings: ApiRecord[]): void {
  const table = makeTable("Tax Filings", ["Ref", "Type", "Year", "Status", "Document"]);
  for (const filing of filings) {
    table.push([
      formatReferenceCell("tax_filing", filing),
      s(filing.document_type),
      s(filing.tax_year),
      s(filing.status),
      s(filing.document_id, 12),
    ]);
  }
  console.log(table.toString());
}

export function printDeadlinesTable(deadlines: ApiRecord[]): void {
  const table = makeTable("Deadlines", ["Ref", "Type", "Due", "Status", "Description"]);
  for (const deadline of deadlines) {
    table.push([
      formatReferenceCell("deadline", deadline),
      s(deadline.deadline_type),
      date(deadline.due_date),
      s(deadline.status),
      s(deadline.description),
    ]);
  }
  console.log(table.toString());
}

export function printClassificationsTable(classifications: ApiRecord[]): void {
  const table = makeTable("Contractor Classifications", ["Ref", "Contractor", "State", "Risk", "Result"]);
  for (const classification of classifications) {
    table.push([
      formatReferenceCell("classification", classification),
      s(classification.contractor_name),
      s(classification.state),
      s(classification.risk_level),
      s(classification.classification),
    ]);
  }
  console.log(table.toString());
}

export function printGovernanceTable(bodies: ApiRecord[]): void {
  const table = makeTable("Governance Bodies", ["Ref", "Body", "Type", "Seats", "Meetings"]);
  for (const b of bodies) {
    table.push([
      formatReferenceCell("body", b),
      s(b.name),
      s(b.body_type ?? b.type),
      s(b.seat_count ?? b.seats),
      s(b.meeting_count ?? b.meetings),
    ]);
  }
  console.log(table.toString());
}

export function printSeatsTable(seats: ApiRecord[]): void {
  const table = makeTable("Seats", ["Ref", "Holder", "Role", "Status"]);
  for (const st of seats) {
    table.push([
      formatReferenceCell("seat", st),
      s(st.holder_name ?? st.holder),
      s(st.role),
      s(st.status),
    ]);
  }
  console.log(table.toString());
}

export function printMeetingsTable(meetings: ApiRecord[]): void {
  const table = makeTable("Meetings", ["Ref", "Title", "Date", "Status", "Resolutions"]);
  for (const m of meetings) {
    table.push([
      formatReferenceCell("meeting", m),
      s(m.title ?? m.name),
      s(m.scheduled_date ?? m.meeting_date ?? m.date),
      s(m.status),
      s(m.resolution_count ?? m.resolutions),
    ]);
  }
  console.log(table.toString());
}

export function printResolutionsTable(resolutions: ApiRecord[]): void {
  const table = makeTable("Resolutions", ["Ref", "Title", "Type", "Status", "For", "Against"]);
  for (const r of resolutions) {
    table.push([
      formatReferenceCell("resolution", r),
      s(r.title),
      s(r.resolution_type ?? r.type),
      s(r.status),
      s(r.votes_for),
      s(r.votes_against),
    ]);
  }
  console.log(table.toString());
}

export function printAgendaItemsTable(items: ApiRecord[]): void {
  const table = makeTable("Agenda Items", ["Ref", "Title", "Status", "Type"]);
  for (const item of items) {
    table.push([
      formatReferenceCell("agenda_item", item),
      s(item.title),
      s(item.status),
      s(item.item_type ?? item.type),
    ]);
  }
  console.log(table.toString());
}

export function printDocumentsTable(docs: ApiRecord[]): void {
  const table = makeTable("Documents", ["Ref", "Title", "Type", "Date", "Status", "Signatures"]);
  for (const d of docs) {
    const sigs = d.signatures;
    const sigStr = Array.isArray(sigs) ? `${sigs.length} signed` : s(sigs);
    table.push([
      formatReferenceCell("document", d),
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
  const table = makeTable("Work Items", ["Ref", "Title", "Category", "Status", "Deadline", "Claimed By"]);
  for (const w of items) {
    const status = s(w.effective_status ?? w.status);
    const colored =
      status === "completed" ? chalk.green(status) :
      status === "claimed" ? chalk.yellow(status) :
      status === "cancelled" ? chalk.dim(status) :
      status;
    table.push([
      formatReferenceCell("work_item", w),
      s(w.title),
      s(w.category),
      colored,
      w.asap ? chalk.red.bold("ASAP") : s(w.deadline ?? ""),
      actorLabel(w, "claimed_by"),
    ]);
  }
  console.log(table.toString());
}

export function printAgentsTable(agents: ApiRecord[]): void {
  const table = makeTable("Agents", ["Ref", "Name", "Status", "Model"]);
  for (const a of agents) {
    const status = s(a.status);
    const colored =
      status === "active" ? chalk.green(status) : status === "paused" ? chalk.yellow(status) : status;
    table.push([formatReferenceCell("agent", a), s(a.name), colored, s(a.model)]);
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

export function printServiceCatalogTable(items: ApiRecord[]): void {
  const table = makeTable("Service Catalog", ["Slug", "Name", "Price", "Type"]);
  for (const item of items) {
    table.push([
      s(item.slug),
      s(item.name),
      money(item.amount_cents),
      s(item.price_type),
    ]);
  }
  console.log(table.toString());
}

export function printServiceRequestsTable(requests: ApiRecord[]): void {
  const table = makeTable("Service Requests", ["Ref", "Service", "Amount", "Status", "Created"]);
  for (const r of requests) {
    const status = s(r.status);
    const colored =
      status === "fulfilled" ? chalk.green(status) :
      status === "paid" ? chalk.cyan(status) :
      status === "checkout" ? chalk.yellow(status) :
      status === "failed" ? chalk.dim(status) :
      status;
    table.push([
      formatReferenceCell("service_request", r),
      s(r.service_slug),
      money(r.amount_cents),
      colored,
      date(r.created_at),
    ]);
  }
  console.log(table.toString());
}

export function printBillingPanel(status: ApiRecord, plans: ApiRecord[]): void {
  const plan = s(status.plan ?? status.tier) || "free";
  const subStatus = s(status.status) || "active";
  const periodEnd = s(status.current_period_end);
  const explanation = s(status.status_explanation);

  console.log(chalk.green("─".repeat(50)));
  console.log(chalk.green.bold("  Billing Status"));
  console.log(chalk.green("─".repeat(50)));
  console.log(`  ${chalk.bold("Plan:")} ${plan}`);
  console.log(`  ${chalk.bold("Status:")} ${subStatus}`);
  if (periodEnd) console.log(`  ${chalk.bold("Current Period End:")} ${periodEnd}`);
  if (explanation) console.log(`  ${chalk.bold("Explanation:")} ${explanation}`);
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
