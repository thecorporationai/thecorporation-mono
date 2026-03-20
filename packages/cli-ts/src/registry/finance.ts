import type { CommandDef, CommandContext } from "./types.js";
import {
  printBankAccountsTable,
  printClassificationsTable,
  printDistributionsTable,
  printError,
  printFinanceSummaryPanel,
  printInvoicesTable,
  printJson,
  printPaymentsTable,
  printPayrollRunsTable,
  printReconciliationsTable,
  printWriteResult,
} from "../output.js";
import type { ApiRecord } from "../types.js";

// ---------------------------------------------------------------------------
// Helpers (relocated from commands/finance.ts)
// ---------------------------------------------------------------------------

function numeric(value: unknown): number {
  return typeof value === "number" && Number.isFinite(value) ? value : 0;
}

function sumAmounts(records: ApiRecord[], candidates: string[]): number {
  return records.reduce((sum, record) => {
    for (const key of candidates) {
      if (typeof record[key] === "number" && Number.isFinite(record[key])) {
        return sum + Number(record[key]);
      }
    }
    return sum;
  }, 0);
}

function latestDate(records: ApiRecord[], candidates: string[]): string | undefined {
  const values = records
    .flatMap((record) => candidates.map((key) => record[key]))
    .filter((value): value is string => typeof value === "string" && value.trim().length > 0)
    .map((value) => ({ raw: value, time: new Date(value).getTime() }))
    .filter((value) => Number.isFinite(value.time))
    .sort((a, b) => b.time - a.time);
  return values[0]?.raw;
}

function countByStatus(records: ApiRecord[], statuses: string[]): number {
  const expected = new Set(statuses.map((status) => status.toLowerCase()));
  return records.filter((record) => expected.has(String(record.status ?? "").toLowerCase())).length;
}

function countByField(records: ApiRecord[], field: string, values: string[]): number {
  const expected = new Set(values.map((value) => value.toLowerCase()));
  return records.filter((record) => expected.has(String(record[field] ?? "").toLowerCase())).length;
}

// ---------------------------------------------------------------------------
// Finance registry entries
// ---------------------------------------------------------------------------

export const financeCommands: CommandDef[] = [
  // --- finance (summary panel) ---
  {
    name: "finance",
    description: "Invoicing, payroll, payments, banking",
    route: { method: "GET", path: "/v1/entities/{eid}/finance/summary" },
    entity: true,
    display: { title: "Finance Summary" },
    handler: async (ctx) => {
      const eid = await ctx.resolver.resolveEntity(ctx.opts.entityId as string | undefined);
      const [
        invoices,
        accounts,
        payments,
        payrollRuns,
        distributions,
        reconciliations,
        classifications,
      ] = await Promise.all([
        ctx.client.listInvoices(eid),
        ctx.client.listBankAccounts(eid),
        ctx.client.listPayments(eid),
        ctx.client.listPayrollRuns(eid),
        ctx.client.listDistributions(eid),
        ctx.client.listReconciliations(eid),
        ctx.client.listContractorClassifications(eid),
      ]);

      await Promise.all([
        ctx.resolver.stabilizeRecords("invoice", invoices, eid),
        ctx.resolver.stabilizeRecords("bank_account", accounts, eid),
        ctx.resolver.stabilizeRecords("payment", payments, eid),
        ctx.resolver.stabilizeRecords("payroll_run", payrollRuns, eid),
        ctx.resolver.stabilizeRecords("distribution", distributions, eid),
        ctx.resolver.stabilizeRecords("reconciliation", reconciliations, eid),
        ctx.resolver.stabilizeRecords("classification", classifications, eid),
      ]);

      const summary: ApiRecord = {
        entity_id: eid,
        invoices: {
          count: invoices.length,
          open_count: invoices.length - countByStatus(invoices, ["paid", "cancelled", "void"]),
          overdue_count: countByStatus(invoices, ["overdue"]),
          total_amount_cents: sumAmounts(invoices, ["amount_cents", "total_amount_cents"]),
          latest_due_date: latestDate(invoices, ["due_date", "created_at"]),
        },
        bank_accounts: {
          count: accounts.length,
          active_count: countByStatus(accounts, ["active", "approved", "open"]),
        },
        payments: {
          count: payments.length,
          pending_count: countByStatus(payments, ["pending", "submitted", "queued"]),
          total_amount_cents: sumAmounts(payments, ["amount_cents"]),
          latest_submitted_at: latestDate(payments, ["submitted_at", "created_at"]),
        },
        payroll_runs: {
          count: payrollRuns.length,
          latest_period_end: latestDate(payrollRuns, ["pay_period_end", "created_at"]),
        },
        distributions: {
          count: distributions.length,
          total_amount_cents: sumAmounts(distributions, ["amount_cents", "distribution_amount_cents"]),
          latest_declared_at: latestDate(distributions, ["declared_at", "created_at"]),
        },
        reconciliations: {
          count: reconciliations.length,
          balanced_count: reconciliations.filter((record) => record.is_balanced === true).length,
          latest_as_of_date: latestDate(reconciliations, ["as_of_date", "created_at"]),
        },
        contractor_classifications: {
          count: classifications.length,
          high_risk_count: countByField(classifications, "risk_level", ["high"]),
          medium_risk_count: countByField(classifications, "risk_level", ["medium"]),
        },
      };

      if (ctx.opts.json) { ctx.writer.json(summary); return; }
      printFinanceSummaryPanel(summary);
    },
    examples: [
      "corp finance",
      'corp finance invoice --customer "Client Co" --amount-cents 500000 --due-date 2026-04-01',
      'corp finance pay --amount-cents 250000 --recipient "Vendor" --method ach',
      "corp finance payroll --period-start 2026-03-01 --period-end 2026-03-15",
      "corp finance open-account --institution Mercury",
      "corp finance statements --period 2026-Q1",
    ],
  },

  // --- finance invoices ---
  {
    name: "finance invoices",
    description: "List invoices",
    route: { method: "GET", path: "/v1/entities/{eid}/invoices" },
    entity: true,
    display: {
      title: "Invoices",
      cols: ["customer_name|customer>Customer", "$amount_cents>Amount", "status>Status", "@due_date>Due", "#invoice_id>ID"],
    },
    handler: async (ctx) => {
      const eid = await ctx.resolver.resolveEntity(ctx.opts.entityId as string | undefined);
      const invoices = await ctx.client.listInvoices(eid);
      await ctx.resolver.stabilizeRecords("invoice", invoices, eid);
      if (ctx.opts.json) { ctx.writer.json(invoices); return; }
      if (invoices.length === 0) { ctx.writer.writeln("No invoices found."); return; }
      printInvoicesTable(invoices);
    },
  },

  // --- finance invoice (create) ---
  {
    name: "finance invoice",
    description: "Create an invoice",
    route: { method: "POST", path: "/v1/entities/{eid}/invoices" },
    entity: true,
    options: [
      { flags: "--customer <name>", description: "Customer name", required: true },
      { flags: "--amount-cents <n>", description: "Amount in cents (e.g. 500000 = $5,000.00)", type: "int" },
      { flags: "--amount <n>", description: "Amount in dollars (converted to cents)", type: "int" },
      { flags: "--due-date <date>", description: "Due date (ISO 8601)", required: true },
      { flags: "--description <desc>", description: "Description", default: "Services rendered" },
    ],
    handler: async (ctx) => {
      const eid = await ctx.resolver.resolveEntity(ctx.opts.entityId as string | undefined);
      const amountCents = (ctx.opts.amountCents as number | undefined) ?? ((ctx.opts.amount as number | undefined) != null ? (ctx.opts.amount as number) * 100 : undefined);
      if (amountCents == null) {
        printError("required option '--amount-cents <n>' or '--amount <n>' not specified");
        process.exit(1);
      }
      const result = await ctx.client.createInvoice({
        entity_id: eid,
        customer_name: ctx.opts.customer as string,
        amount_cents: amountCents,
        due_date: ctx.opts.dueDate as string,
        description: ctx.opts.description as string,
      });
      await ctx.resolver.stabilizeRecord("invoice", result, eid);
      ctx.resolver.rememberFromRecord("invoice", result, eid);
      ctx.writer.writeResult(result, `Invoice created: ${result.invoice_id ?? "OK"}`, {
        jsonOnly: ctx.opts.json,
        referenceKind: "invoice",
        showReuseHint: true,
      });
    },
    produces: { kind: "invoice" },
    successTemplate: "Invoice created: {customer_name}",
  },

  // --- finance payments ---
  {
    name: "finance payments",
    description: "List payments",
    route: { method: "GET", path: "/v1/entities/{eid}/payments" },
    entity: true,
    display: {
      title: "Payments",
      cols: ["recipient>Recipient", "$amount_cents>Amount", "status>Status", "@submitted_at>Submitted", "#payment_id>ID"],
    },
    handler: async (ctx) => {
      const eid = await ctx.resolver.resolveEntity(ctx.opts.entityId as string | undefined);
      const payments = await ctx.client.listPayments(eid);
      await ctx.resolver.stabilizeRecords("payment", payments, eid);
      if (ctx.opts.json) { ctx.writer.json(payments); return; }
      if (payments.length === 0) { ctx.writer.writeln("No payments found."); return; }
      printPaymentsTable(payments);
    },
  },

  // --- finance pay ---
  {
    name: "finance pay",
    description: "Submit a payment",
    route: { method: "POST", path: "/v1/entities/{eid}/payments" },
    entity: true,
    options: [
      { flags: "--amount-cents <n>", description: "Amount in cents (e.g. 500000 = $5,000.00)", type: "int" },
      { flags: "--amount <n>", description: "Amount in dollars (converted to cents)", type: "int" },
      { flags: "--recipient <name>", description: "Recipient name", required: true },
      { flags: "--method <method>", description: "Payment method", default: "ach" },
    ],
    handler: async (ctx) => {
      const eid = await ctx.resolver.resolveEntity(ctx.opts.entityId as string | undefined);
      const amountCents = (ctx.opts.amountCents as number | undefined) ?? ((ctx.opts.amount as number | undefined) != null ? (ctx.opts.amount as number) * 100 : undefined);
      if (amountCents == null) {
        printError("required option '--amount-cents <n>' or '--amount <n>' not specified");
        process.exit(1);
      }
      const method = ctx.opts.method as string;
      const result = await ctx.client.submitPayment({
        entity_id: eid,
        amount_cents: amountCents,
        recipient: ctx.opts.recipient as string,
        payment_method: method,
        description: `Payment via ${method}`,
      });
      await ctx.resolver.stabilizeRecord("payment", result, eid);
      ctx.resolver.rememberFromRecord("payment", result, eid);
      ctx.writer.writeResult(result, `Payment submitted: ${result.payment_id ?? "OK"}`, {
        jsonOnly: ctx.opts.json,
        referenceKind: "payment",
        showReuseHint: true,
      });
    },
    produces: { kind: "payment" },
    successTemplate: "Payment submitted: {recipient_name}",
  },

  // --- finance bank-accounts ---
  {
    name: "finance bank-accounts",
    description: "List bank accounts",
    route: { method: "GET", path: "/v1/entities/{eid}/bank-accounts" },
    entity: true,
    display: {
      title: "Bank Accounts",
      cols: ["bank_name|institution>Bank", "status>Status", "#bank_account_id|account_id>ID"],
    },
    handler: async (ctx) => {
      const eid = await ctx.resolver.resolveEntity(ctx.opts.entityId as string | undefined);
      const accounts = await ctx.client.listBankAccounts(eid);
      await ctx.resolver.stabilizeRecords("bank_account", accounts, eid);
      if (ctx.opts.json) { ctx.writer.json(accounts); return; }
      if (accounts.length === 0) { ctx.writer.writeln("No bank accounts found."); return; }
      printBankAccountsTable(accounts);
    },
  },

  // --- finance open-account ---
  {
    name: "finance open-account",
    description: "Open a business bank account",
    route: { method: "POST", path: "/v1/entities/{eid}/bank-accounts" },
    entity: true,
    options: [
      { flags: "--institution <name>", description: "Banking institution", default: "Mercury" },
    ],
    handler: async (ctx) => {
      const eid = await ctx.resolver.resolveEntity(ctx.opts.entityId as string | undefined);
      const result = await ctx.client.openBankAccount({ entity_id: eid, bank_name: ctx.opts.institution as string });
      await ctx.resolver.stabilizeRecord("bank_account", result, eid);
      ctx.resolver.rememberFromRecord("bank_account", result, eid);
      ctx.writer.writeResult(result, `Bank account opened: ${result.bank_account_id ?? result.account_id ?? "OK"}`, {
        jsonOnly: ctx.opts.json,
        referenceKind: "bank_account",
        showReuseHint: true,
      });
    },
    produces: { kind: "bank_account" },
    successTemplate: "Bank account opened: {bank_name}",
  },

  // --- finance activate-account <account-ref> ---
  {
    name: "finance activate-account",
    description: "Activate a bank account (transitions from pending_review to active)",
    route: { method: "POST", path: "/v1/bank-accounts/{pos}/activate" },
    entity: true,
    args: [{ name: "account-ref", required: true, description: "Bank account reference" }],
    handler: async (ctx) => {
      const accountRef = ctx.positional[0];
      const eid = await ctx.resolver.resolveEntity(ctx.opts.entityId as string | undefined);
      const resolvedId = await ctx.resolver.resolveBankAccount(eid, accountRef);
      const result = await ctx.client.activateBankAccount(resolvedId, eid);
      await ctx.resolver.stabilizeRecord("bank_account", result, eid);
      ctx.resolver.rememberFromRecord("bank_account", result, eid);
      ctx.writer.writeResult(result, `Bank account activated: ${resolvedId}`, {
        jsonOnly: ctx.opts.json,
        referenceKind: "bank_account",
        showReuseHint: true,
      });
    },
  },

  // --- finance payroll-runs ---
  {
    name: "finance payroll-runs",
    description: "List payroll runs",
    route: { method: "GET", path: "/v1/entities/{eid}/payroll-runs" },
    entity: true,
    display: {
      title: "Payroll Runs",
      cols: ["@pay_period_start>Period Start", "@pay_period_end>Period End", "status>Status", "#payroll_run_id>ID"],
    },
    handler: async (ctx) => {
      const eid = await ctx.resolver.resolveEntity(ctx.opts.entityId as string | undefined);
      const runs = await ctx.client.listPayrollRuns(eid);
      await ctx.resolver.stabilizeRecords("payroll_run", runs, eid);
      if (ctx.opts.json) { ctx.writer.json(runs); return; }
      if (runs.length === 0) { ctx.writer.writeln("No payroll runs found."); return; }
      printPayrollRunsTable(runs);
    },
  },

  // --- finance payroll ---
  {
    name: "finance payroll",
    description: "Run payroll",
    route: { method: "POST", path: "/v1/entities/{eid}/payroll-runs" },
    entity: true,
    options: [
      { flags: "--period-start <date>", description: "Pay period start", required: true },
      { flags: "--period-end <date>", description: "Pay period end", required: true },
    ],
    handler: async (ctx) => {
      const eid = await ctx.resolver.resolveEntity(ctx.opts.entityId as string | undefined);
      const result = await ctx.client.runPayroll({
        entity_id: eid,
        pay_period_start: ctx.opts.periodStart as string,
        pay_period_end: ctx.opts.periodEnd as string,
      });
      await ctx.resolver.stabilizeRecord("payroll_run", result, eid);
      ctx.resolver.rememberFromRecord("payroll_run", result, eid);
      ctx.writer.writeResult(result, `Payroll run created: ${result.payroll_run_id ?? "OK"}`, {
        jsonOnly: ctx.opts.json,
        referenceKind: "payroll_run",
        showReuseHint: true,
      });
    },
    produces: { kind: "payroll_run" },
    successTemplate: "Payroll run created",
  },

  // --- finance distributions ---
  {
    name: "finance distributions",
    description: "List distributions",
    route: { method: "GET", path: "/v1/entities/{eid}/distributions" },
    entity: true,
    display: {
      title: "Distributions",
      cols: ["$amount_cents|distribution_amount_cents>Amount", "status>Status", "@declared_at>Declared", "#distribution_id>ID"],
    },
    handler: async (ctx) => {
      const eid = await ctx.resolver.resolveEntity(ctx.opts.entityId as string | undefined);
      const distributions = await ctx.client.listDistributions(eid);
      await ctx.resolver.stabilizeRecords("distribution", distributions, eid);
      if (ctx.opts.json) { ctx.writer.json(distributions); return; }
      if (distributions.length === 0) { ctx.writer.writeln("No distributions found."); return; }
      printDistributionsTable(distributions);
    },
  },

  // --- finance reconciliations ---
  {
    name: "finance reconciliations",
    description: "List reconciliations",
    route: { method: "GET", path: "/v1/entities/{eid}/reconciliations" },
    entity: true,
    display: {
      title: "Reconciliations",
      cols: ["@as_of_date>As Of", "is_balanced>Balanced", "#reconciliation_id>ID"],
    },
    handler: async (ctx) => {
      const eid = await ctx.resolver.resolveEntity(ctx.opts.entityId as string | undefined);
      const reconciliations = await ctx.client.listReconciliations(eid);
      await ctx.resolver.stabilizeRecords("reconciliation", reconciliations, eid);
      if (ctx.opts.json) { ctx.writer.json(reconciliations); return; }
      if (reconciliations.length === 0) { ctx.writer.writeln("No reconciliations found."); return; }
      printReconciliationsTable(reconciliations);
    },
  },

  // --- finance reconcile ---
  {
    name: "finance reconcile",
    description: "Reconcile ledger (requires --start-date and --end-date)",
    route: { method: "POST", path: "/v1/entities/{eid}/reconciliations" },
    entity: true,
    options: [
      { flags: "--start-date <date>", description: "Period start (required, ISO 8601)", required: true },
      { flags: "--end-date <date>", description: "Period end (required, ISO 8601)", required: true },
    ],
    handler: async (ctx) => {
      const eid = await ctx.resolver.resolveEntity(ctx.opts.entityId as string | undefined);
      const result = await ctx.client.reconcileLedger({
        entity_id: eid,
        start_date: ctx.opts.startDate as string,
        end_date: ctx.opts.endDate as string,
      });
      await ctx.resolver.stabilizeRecord("reconciliation", result, eid);
      ctx.resolver.rememberFromRecord("reconciliation", result, eid);
      ctx.writer.writeResult(result, `Ledger reconciled: ${result.reconciliation_id ?? "OK"}`, {
        jsonOnly: ctx.opts.json,
        referenceKind: "reconciliation",
        showReuseHint: true,
      });
    },
    produces: { kind: "reconciliation" },
    successTemplate: "Reconciliation created",
  },

  // --- finance classifications ---
  {
    name: "finance classifications",
    description: "List contractor classifications",
    route: { method: "GET", path: "/v1/entities/{eid}/contractor-classifications" },
    entity: true,
    display: {
      title: "Contractor Classifications",
      cols: ["contractor_name>Contractor", "risk_level>Risk", "state>State", "#classification_id>ID"],
    },
    handler: async (ctx) => {
      const eid = await ctx.resolver.resolveEntity(ctx.opts.entityId as string | undefined);
      const classifications = await ctx.client.listContractorClassifications(eid);
      await ctx.resolver.stabilizeRecords("classification", classifications, eid);
      if (ctx.opts.json) { ctx.writer.json(classifications); return; }
      if (classifications.length === 0) { ctx.writer.writeln("No contractor classifications found."); return; }
      printClassificationsTable(classifications);
    },
  },

  // --- finance classify-contractor ---
  {
    name: "finance classify-contractor",
    description: "Analyze contractor classification risk",
    route: { method: "POST", path: "/v1/entities/{eid}/contractor-classifications" },
    entity: true,
    options: [
      { flags: "--name <name>", description: "Contractor name", required: true },
      { flags: "--state <code>", description: "US state code", required: true },
      { flags: "--hours <n>", description: "Hours per week", required: true, type: "int" },
      { flags: "--exclusive", description: "Exclusive client", default: false },
      { flags: "--duration <n>", description: "Duration in months", required: true, type: "int" },
      { flags: "--provides-tools", description: "Company provides tools", default: false },
    ],
    handler: async (ctx) => {
      const eid = await ctx.resolver.resolveEntity(ctx.opts.entityId as string | undefined);
      const result = await ctx.client.classifyContractor({
        entity_id: eid,
        contractor_name: ctx.opts.name as string,
        state: ctx.opts.state as string,
        hours_per_week: ctx.opts.hours as number,
        exclusive_client: !!ctx.opts.exclusive,
        duration_months: ctx.opts.duration as number,
        provides_tools: !!ctx.opts.providesTools,
      });
      await ctx.resolver.stabilizeRecord("classification", result, eid);
      ctx.resolver.rememberFromRecord("classification", result, eid);
      ctx.writer.writeResult(result, `Classification: ${result.risk_level ?? "OK"}`, {
        jsonOnly: ctx.opts.json,
        referenceKind: "classification",
        showReuseHint: true,
      });
    },
    produces: { kind: "classification" },
    successTemplate: "Classification created: {contractor_name}",
  },

  // --- finance statements ---
  {
    name: "finance statements",
    description: "View financial statements (P&L, balance sheet)",
    route: { method: "GET", path: "/v1/entities/{eid}/financial-statements" },
    entity: "query",
    display: { title: "Financial Statements" },
    options: [
      { flags: "--period <period>", description: "Period (e.g. 2026-Q1, 2025)" },
    ],
    handler: async (ctx) => {
      const eid = await ctx.resolver.resolveEntity(ctx.opts.entityId as string | undefined);
      const params: Record<string, string> = {};
      if (ctx.opts.period) params.period = ctx.opts.period as string;
      const result = await ctx.client.getFinancialStatements(eid, params);
      if (ctx.opts.json) { ctx.writer.json(result); return; }
      printJson(result);
    },
  },
];
