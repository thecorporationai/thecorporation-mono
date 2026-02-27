import { requireConfig, resolveEntityId } from "../config.js";
import { CorpAPIClient } from "../api-client.js";
import {
  printCapTable, printSafesTable, printTransfersTable,
  printValuationsTable, printError, printSuccess, printJson,
} from "../output.js";
import chalk from "chalk";

export async function capTableCommand(opts: { entityId?: string; json?: boolean }): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const eid = resolveEntityId(cfg, opts.entityId);
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    const data = await client.getCapTable(eid);
    if (opts.json) { printJson(data); return; }
    if ((data.access_level as string) === "none") {
      printError("You do not have access to this entity's cap table.");
      process.exit(1);
    }
    printCapTable(data);
    try {
      const val = await client.getCurrent409a(eid);
      if (val) print409a(val);
    } catch { /* ignore */ }
  } catch (err) {
    printError(`Failed to fetch cap table: ${err}`);
    process.exit(1);
  }
}

export async function safesCommand(opts: { entityId?: string; json?: boolean }): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const eid = resolveEntityId(cfg, opts.entityId);
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    const safes = await client.getSafeNotes(eid);
    if (opts.json) printJson(safes);
    else if (safes.length === 0) console.log("No SAFE notes found.");
    else printSafesTable(safes);
  } catch (err) {
    printError(`Failed to fetch SAFE notes: ${err}`);
    process.exit(1);
  }
}

export async function transfersCommand(opts: { entityId?: string; json?: boolean }): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const eid = resolveEntityId(cfg, opts.entityId);
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    const transfers = await client.getShareTransfers(eid);
    if (opts.json) printJson(transfers);
    else if (transfers.length === 0) console.log("No share transfers found.");
    else printTransfersTable(transfers);
  } catch (err) {
    printError(`Failed to fetch transfers: ${err}`);
    process.exit(1);
  }
}

export async function valuationsCommand(opts: { entityId?: string; json?: boolean }): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const eid = resolveEntityId(cfg, opts.entityId);
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    const valuations = await client.getValuations(eid);
    if (opts.json) printJson(valuations);
    else if (valuations.length === 0) console.log("No valuations found.");
    else printValuationsTable(valuations);
  } catch (err) {
    printError(`Failed to fetch valuations: ${err}`);
    process.exit(1);
  }
}

export async function fourOhNineACommand(opts: { entityId?: string; json?: boolean }): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const eid = resolveEntityId(cfg, opts.entityId);
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    const data = await client.getCurrent409a(eid);
    if (opts.json) printJson(data);
    else if (!data || Object.keys(data).length === 0) console.log("No 409A valuation found.");
    else print409a(data);
  } catch (err) {
    printError(`Failed to fetch 409A valuation: ${err}`);
    process.exit(1);
  }
}

export async function issueEquityCommand(opts: {
  entityId?: string;
  grantType: string;
  shares: number;
  recipient: string;
}): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const eid = resolveEntityId(cfg, opts.entityId);
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    const result = await client.issueEquity({
      entity_id: eid, grant_type: opts.grantType, shares: opts.shares, recipient_name: opts.recipient,
    });
    printSuccess(`Equity issued: ${result.grant_id ?? "OK"}`);
    printJson(result);
  } catch (err) {
    printError(`Failed to issue equity: ${err}`);
    process.exit(1);
  }
}

export async function issueSafeCommand(opts: {
  entityId?: string;
  investor: string;
  amount: number;
  safeType: string;
  valuationCap: number;
}): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const eid = resolveEntityId(cfg, opts.entityId);
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    const result = await client.issueSafe({
      entity_id: eid, investor_name: opts.investor, principal_amount_cents: opts.amount,
      safe_type: opts.safeType, valuation_cap_cents: opts.valuationCap,
    });
    printSuccess(`SAFE issued: ${result.safe_note_id ?? result.safe_id ?? "OK"}`);
    printJson(result);
  } catch (err) {
    printError(`Failed to issue SAFE: ${err}`);
    process.exit(1);
  }
}

export async function transferSharesCommand(opts: {
  entityId?: string;
  fromGrant: string;
  to: string;
  shares: number;
  type: string;
}): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const eid = resolveEntityId(cfg, opts.entityId);
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    const result = await client.transferShares({
      entity_id: eid, from_holder: opts.fromGrant, to_holder: opts.to,
      shares: opts.shares, transfer_type: opts.type,
    });
    printSuccess(`Transfer complete: ${result.transfer_id ?? "OK"}`);
    printJson(result);
  } catch (err) {
    printError(`Failed to transfer shares: ${err}`);
    process.exit(1);
  }
}

export async function distributeCommand(opts: {
  entityId?: string;
  amount: number;
  type: string;
}): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const eid = resolveEntityId(cfg, opts.entityId);
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    const result = await client.calculateDistribution({
      entity_id: eid, total_amount_cents: opts.amount, distribution_type: opts.type,
    });
    printSuccess(`Distribution calculated: ${result.distribution_id ?? "OK"}`);
    printJson(result);
  } catch (err) {
    printError(`Failed to calculate distribution: ${err}`);
    process.exit(1);
  }
}

function print409a(data: Record<string, unknown>): void {
  console.log(chalk.green("─".repeat(40)));
  console.log(chalk.green.bold("  409A Valuation"));
  console.log(chalk.green("─".repeat(40)));
  console.log(`  ${chalk.bold("FMV/Share:")} $${data.fmv_per_share ?? "N/A"}`);
  console.log(`  ${chalk.bold("Enterprise Value:")} $${data.enterprise_value ?? "N/A"}`);
  console.log(`  ${chalk.bold("Valuation Date:")} ${data.valuation_date ?? "N/A"}`);
  if (data.provider) console.log(`  ${chalk.bold("Provider:")} ${data.provider}`);
  console.log(chalk.green("─".repeat(40)));
}
