import { requireConfig, resolveEntityId } from "../config.js";
import { CorpAPIClient } from "../api-client.js";
import { printError, printSuccess, printJson } from "../output.js";

export async function financeInvoiceCommand(opts: {
  entityId?: string; customer: string; amount: number; dueDate: string; description: string;
}): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const eid = resolveEntityId(cfg, opts.entityId);
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    const result = await client.createInvoice({
      entity_id: eid, customer_name: opts.customer, amount_cents: opts.amount,
      due_date: opts.dueDate, description: opts.description,
    });
    printSuccess(`Invoice created: ${result.invoice_id ?? "OK"}`);
    printJson(result);
  } catch (err) { printError(`Failed to create invoice: ${err}`); process.exit(1); }
}

export async function financePayrollCommand(opts: {
  entityId?: string; periodStart: string; periodEnd: string;
}): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const eid = resolveEntityId(cfg, opts.entityId);
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    const result = await client.runPayroll({
      entity_id: eid, pay_period_start: opts.periodStart, pay_period_end: opts.periodEnd,
    });
    printSuccess(`Payroll run created: ${result.payroll_run_id ?? "OK"}`);
    printJson(result);
  } catch (err) { printError(`Failed to run payroll: ${err}`); process.exit(1); }
}

export async function financePayCommand(opts: {
  entityId?: string; amount: number; recipient: string; method: string;
}): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const eid = resolveEntityId(cfg, opts.entityId);
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    const result = await client.submitPayment({
      entity_id: eid, amount_cents: opts.amount, recipient: opts.recipient,
      payment_method: opts.method,
      description: `Payment via ${opts.method}`,
    });
    printSuccess(`Payment submitted: ${result.payment_id ?? "OK"}`);
    printJson(result);
  } catch (err) { printError(`Failed to submit payment: ${err}`); process.exit(1); }
}

export async function financeOpenAccountCommand(opts: {
  entityId?: string; institution: string;
}): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const eid = resolveEntityId(cfg, opts.entityId);
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    const result = await client.openBankAccount({ entity_id: eid, bank_name: opts.institution });
    printSuccess(`Bank account opened: ${result.account_id ?? "OK"}`);
    printJson(result);
  } catch (err) { printError(`Failed to open bank account: ${err}`); process.exit(1); }
}

export async function financeClassifyContractorCommand(opts: {
  entityId?: string; name: string; state: string; hours: number;
  exclusive: boolean; duration: number; providesTools: boolean;
}): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const eid = resolveEntityId(cfg, opts.entityId);
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    const result = await client.classifyContractor({
      entity_id: eid, contractor_name: opts.name, state: opts.state, hours_per_week: opts.hours,
      exclusive_client: opts.exclusive, duration_months: opts.duration, provides_tools: opts.providesTools,
    });
    printSuccess(`Classification: ${result.risk_level ?? "OK"}`);
    printJson(result);
  } catch (err) { printError(`Failed to classify contractor: ${err}`); process.exit(1); }
}

export async function financeReconcileCommand(opts: {
  entityId?: string; startDate: string; endDate: string;
}): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const eid = resolveEntityId(cfg, opts.entityId);
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    const result = await client.reconcileLedger({
      entity_id: eid, start_date: opts.startDate, end_date: opts.endDate,
    });
    printSuccess(`Ledger reconciled: ${result.reconciliation_id ?? "OK"}`);
    printJson(result);
  } catch (err) { printError(`Failed to reconcile ledger: ${err}`); process.exit(1); }
}
