import { requireConfig } from "../config.js";
import { CorpAPIClient } from "../api-client.js";
import {
  printBankAccountsTable,
  printClassificationsTable,
  printDistributionsTable,
  printError,
  printInvoicesTable,
  printJson,
  printPaymentsTable,
  printPayrollRunsTable,
  printReconciliationsTable,
  printWriteResult,
} from "../output.js";
import { ReferenceResolver } from "../references.js";

export async function financeInvoicesCommand(opts: { entityId?: string; json?: boolean }): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const eid = await resolver.resolveEntity(opts.entityId);
    const invoices = await client.listInvoices(eid);
    await resolver.stabilizeRecords("invoice", invoices, eid);
    if (opts.json) printJson(invoices);
    else if (invoices.length === 0) console.log("No invoices found.");
    else printInvoicesTable(invoices);
  } catch (err) { printError(`Failed to fetch invoices: ${err}`); process.exit(1); }
}

export async function financeBankAccountsCommand(opts: { entityId?: string; json?: boolean }): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const eid = await resolver.resolveEntity(opts.entityId);
    const accounts = await client.listBankAccounts(eid);
    await resolver.stabilizeRecords("bank_account", accounts, eid);
    if (opts.json) printJson(accounts);
    else if (accounts.length === 0) console.log("No bank accounts found.");
    else printBankAccountsTable(accounts);
  } catch (err) { printError(`Failed to fetch bank accounts: ${err}`); process.exit(1); }
}

export async function financePaymentsCommand(opts: { entityId?: string; json?: boolean }): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const eid = await resolver.resolveEntity(opts.entityId);
    const payments = await client.listPayments(eid);
    await resolver.stabilizeRecords("payment", payments, eid);
    if (opts.json) printJson(payments);
    else if (payments.length === 0) console.log("No payments found.");
    else printPaymentsTable(payments);
  } catch (err) { printError(`Failed to fetch payments: ${err}`); process.exit(1); }
}

export async function financePayrollRunsCommand(opts: { entityId?: string; json?: boolean }): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const eid = await resolver.resolveEntity(opts.entityId);
    const runs = await client.listPayrollRuns(eid);
    await resolver.stabilizeRecords("payroll_run", runs, eid);
    if (opts.json) printJson(runs);
    else if (runs.length === 0) console.log("No payroll runs found.");
    else printPayrollRunsTable(runs);
  } catch (err) { printError(`Failed to fetch payroll runs: ${err}`); process.exit(1); }
}

export async function financeDistributionsCommand(opts: { entityId?: string; json?: boolean }): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const eid = await resolver.resolveEntity(opts.entityId);
    const distributions = await client.listDistributions(eid);
    await resolver.stabilizeRecords("distribution", distributions, eid);
    if (opts.json) printJson(distributions);
    else if (distributions.length === 0) console.log("No distributions found.");
    else printDistributionsTable(distributions);
  } catch (err) { printError(`Failed to fetch distributions: ${err}`); process.exit(1); }
}

export async function financeReconciliationsCommand(opts: { entityId?: string; json?: boolean }): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const eid = await resolver.resolveEntity(opts.entityId);
    const reconciliations = await client.listReconciliations(eid);
    await resolver.stabilizeRecords("reconciliation", reconciliations, eid);
    if (opts.json) printJson(reconciliations);
    else if (reconciliations.length === 0) console.log("No reconciliations found.");
    else printReconciliationsTable(reconciliations);
  } catch (err) { printError(`Failed to fetch reconciliations: ${err}`); process.exit(1); }
}

export async function financeClassificationsCommand(opts: { entityId?: string; json?: boolean }): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const eid = await resolver.resolveEntity(opts.entityId);
    const classifications = await client.listContractorClassifications(eid);
    await resolver.stabilizeRecords("classification", classifications, eid);
    if (opts.json) printJson(classifications);
    else if (classifications.length === 0) console.log("No contractor classifications found.");
    else printClassificationsTable(classifications);
  } catch (err) { printError(`Failed to fetch contractor classifications: ${err}`); process.exit(1); }
}

export async function financeInvoiceCommand(opts: {
  entityId?: string; customer: string; amount: number; dueDate: string; description: string; json?: boolean;
}): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const eid = await resolver.resolveEntity(opts.entityId);
    const result = await client.createInvoice({
      entity_id: eid, customer_name: opts.customer, amount_cents: opts.amount,
      due_date: opts.dueDate, description: opts.description,
    });
    await resolver.stabilizeRecord("invoice", result, eid);
    resolver.rememberFromRecord("invoice", result, eid);
    printWriteResult(result, `Invoice created: ${result.invoice_id ?? "OK"}`, {
      jsonOnly: opts.json,
      referenceKind: "invoice",
      showReuseHint: true,
    });
  } catch (err) { printError(`Failed to create invoice: ${err}`); process.exit(1); }
}

export async function financePayrollCommand(opts: {
  entityId?: string; periodStart: string; periodEnd: string; json?: boolean;
}): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const eid = await resolver.resolveEntity(opts.entityId);
    const result = await client.runPayroll({
      entity_id: eid, pay_period_start: opts.periodStart, pay_period_end: opts.periodEnd,
    });
    await resolver.stabilizeRecord("payroll_run", result, eid);
    resolver.rememberFromRecord("payroll_run", result, eid);
    printWriteResult(result, `Payroll run created: ${result.payroll_run_id ?? "OK"}`, {
      jsonOnly: opts.json,
      referenceKind: "payroll_run",
      showReuseHint: true,
    });
  } catch (err) { printError(`Failed to run payroll: ${err}`); process.exit(1); }
}

export async function financePayCommand(opts: {
  entityId?: string; amount: number; recipient: string; method: string; json?: boolean;
}): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const eid = await resolver.resolveEntity(opts.entityId);
    const result = await client.submitPayment({
      entity_id: eid, amount_cents: opts.amount, recipient: opts.recipient,
      payment_method: opts.method,
      description: `Payment via ${opts.method}`,
    });
    await resolver.stabilizeRecord("payment", result, eid);
    resolver.rememberFromRecord("payment", result, eid);
    printWriteResult(result, `Payment submitted: ${result.payment_id ?? "OK"}`, {
      jsonOnly: opts.json,
      referenceKind: "payment",
      showReuseHint: true,
    });
  } catch (err) { printError(`Failed to submit payment: ${err}`); process.exit(1); }
}

export async function financeOpenAccountCommand(opts: {
  entityId?: string; institution: string; json?: boolean;
}): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const eid = await resolver.resolveEntity(opts.entityId);
    const result = await client.openBankAccount({ entity_id: eid, bank_name: opts.institution });
    await resolver.stabilizeRecord("bank_account", result, eid);
    resolver.rememberFromRecord("bank_account", result, eid);
    printWriteResult(result, `Bank account opened: ${result.bank_account_id ?? result.account_id ?? "OK"}`, {
      jsonOnly: opts.json,
      referenceKind: "bank_account",
      showReuseHint: true,
    });
  } catch (err) { printError(`Failed to open bank account: ${err}`); process.exit(1); }
}

export async function financeClassifyContractorCommand(opts: {
  entityId?: string; name: string; state: string; hours: number;
  exclusive: boolean; duration: number; providesTools: boolean; json?: boolean;
}): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const eid = await resolver.resolveEntity(opts.entityId);
    const result = await client.classifyContractor({
      entity_id: eid, contractor_name: opts.name, state: opts.state, hours_per_week: opts.hours,
      exclusive_client: opts.exclusive, duration_months: opts.duration, provides_tools: opts.providesTools,
    });
    await resolver.stabilizeRecord("classification", result, eid);
    resolver.rememberFromRecord("classification", result, eid);
    printWriteResult(result, `Classification: ${result.risk_level ?? "OK"}`, {
      jsonOnly: opts.json,
      referenceKind: "classification",
      showReuseHint: true,
    });
  } catch (err) { printError(`Failed to classify contractor: ${err}`); process.exit(1); }
}

export async function financeReconcileCommand(opts: {
  entityId?: string; startDate: string; endDate: string; json?: boolean;
}): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const eid = await resolver.resolveEntity(opts.entityId);
    const result = await client.reconcileLedger({
      entity_id: eid, start_date: opts.startDate, end_date: opts.endDate,
    });
    await resolver.stabilizeRecord("reconciliation", result, eid);
    resolver.rememberFromRecord("reconciliation", result, eid);
    printWriteResult(result, `Ledger reconciled: ${result.reconciliation_id ?? "OK"}`, {
      jsonOnly: opts.json,
      referenceKind: "reconciliation",
      showReuseHint: true,
    });
  } catch (err) { printError(`Failed to reconcile ledger: ${err}`); process.exit(1); }
}
