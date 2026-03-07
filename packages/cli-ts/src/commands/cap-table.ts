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
  email?: string;
  instrumentId?: string;
}): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const eid = resolveEntityId(cfg, opts.entityId);
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    // Fetch cap table to get issuer and instrument info
    const capTable = await client.getCapTable(eid);
    const issuerLegalEntityId = capTable.issuer_legal_entity_id as string;
    if (!issuerLegalEntityId) {
      printError("No issuer legal entity found. Has this entity been formed with a cap table?");
      process.exit(1);
    }

    // Resolve instrument ID — use provided, or match by grant type, or use first common stock
    let instrumentId = opts.instrumentId;
    if (!instrumentId) {
      const instruments = capTable.instruments as Array<{ instrument_id: string; kind: string; symbol: string }>;
      if (!instruments?.length) {
        printError("No instruments found on cap table. Create an instrument first.");
        process.exit(1);
      }
      const grantLower = opts.grantType.toLowerCase();
      const match = instruments.find(
        (i) => i.kind.toLowerCase().includes(grantLower) || i.symbol.toLowerCase().includes(grantLower),
      ) ?? instruments.find((i) => i.kind.toLowerCase().includes("common"));
      if (match) {
        instrumentId = match.instrument_id;
        console.log(`Using instrument: ${match.symbol} (${match.kind})`);
      } else {
        instrumentId = instruments[0].instrument_id;
        console.log(`Using first instrument: ${instruments[0].symbol} (${instruments[0].kind})`);
      }
    }

    // 1. Start a staged round
    const round = await client.startEquityRound({
      entity_id: eid,
      name: `${opts.grantType} grant — ${opts.recipient}`,
      issuer_legal_entity_id: issuerLegalEntityId,
    });
    const roundId = (round.round_id ?? round.equity_round_id) as string;

    // 2. Add the security
    const securityData: Record<string, unknown> = {
      entity_id: eid,
      instrument_id: instrumentId,
      quantity: opts.shares,
      recipient_name: opts.recipient,
      grant_type: opts.grantType,
    };
    if (opts.email) securityData.email = opts.email;
    await client.addRoundSecurity(roundId, securityData);

    // 3. Issue the round
    const result = await client.issueRound(roundId, { entity_id: eid });

    printSuccess(`Equity issued: ${opts.shares} shares (${opts.grantType}) to ${opts.recipient}`);
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
  email?: string;
}): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const eid = resolveEntityId(cfg, opts.entityId);
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    // Fetch cap table for issuer and SAFE instrument
    const capTable = await client.getCapTable(eid);
    const issuerLegalEntityId = capTable.issuer_legal_entity_id as string;
    if (!issuerLegalEntityId) {
      printError("No issuer legal entity found. Has this entity been formed with a cap table?");
      process.exit(1);
    }

    const instruments = capTable.instruments as Array<{ instrument_id: string; kind: string; symbol: string }>;
    const safeInstrument = instruments?.find((i) => i.kind.toLowerCase() === "safe");
    if (!safeInstrument) {
      printError("No SAFE instrument found on cap table. Create a SAFE instrument first.");
      process.exit(1);
    }

    // Start a staged round for the SAFE
    const round = await client.startEquityRound({
      entity_id: eid,
      name: `SAFE — ${opts.investor}`,
      issuer_legal_entity_id: issuerLegalEntityId,
    });
    const roundId = (round.round_id ?? round.equity_round_id) as string;

    // Add the SAFE security — use principal_cents as quantity since quantity must be > 0
    const securityData: Record<string, unknown> = {
      entity_id: eid,
      instrument_id: safeInstrument.instrument_id,
      quantity: opts.amount,
      recipient_name: opts.investor,
      principal_cents: opts.amount,
      grant_type: opts.safeType,
    };
    if (opts.email) securityData.email = opts.email;
    await client.addRoundSecurity(roundId, securityData);

    // Issue the round
    const result = await client.issueRound(roundId, { entity_id: eid });
    printSuccess(`SAFE issued: $${(opts.amount / 100).toLocaleString()} to ${opts.investor}`);
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
      entity_id: eid, from_holder_id: opts.fromGrant, to_holder_id: opts.to,
      quantity: opts.shares, transfer_type: opts.type,
    });
    printSuccess(`Transfer workflow created: ${result.workflow_id ?? "OK"}`);
    printJson(result);
  } catch (err) {
    printError(`Failed to create transfer workflow: ${err}`);
    process.exit(1);
  }
}

export async function distributeCommand(opts: {
  entityId?: string;
  amount: number;
  type: string;
  description: string;
}): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const eid = resolveEntityId(cfg, opts.entityId);
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    const result = await client.calculateDistribution({
      entity_id: eid, total_amount_cents: opts.amount, distribution_type: opts.type,
      description: opts.description,
    });
    printSuccess(`Distribution calculated: ${result.distribution_id ?? "OK"}`);
    printJson(result);
  } catch (err) {
    printError(`Failed to calculate distribution: ${err}`);
    process.exit(1);
  }
}

export async function startRoundCommand(opts: {
  entityId?: string;
  name: string;
  issuerLegalEntityId: string;
}): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const eid = resolveEntityId(cfg, opts.entityId);
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    const result = await client.startEquityRound({
      entity_id: eid,
      name: opts.name,
      issuer_legal_entity_id: opts.issuerLegalEntityId,
    });
    printSuccess(`Round started: ${result.round_id ?? "OK"}`);
    printJson(result);
  } catch (err) {
    printError(`Failed to start round: ${err}`);
    process.exit(1);
  }
}

export async function addSecurityCommand(opts: {
  entityId?: string;
  roundId: string;
  holderId?: string;
  email?: string;
  instrumentId: string;
  quantity: number;
  recipientName: string;
  principalCents?: number;
  grantType?: string;
}): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const eid = resolveEntityId(cfg, opts.entityId);
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    const body: Record<string, unknown> = {
      entity_id: eid,
      instrument_id: opts.instrumentId,
      quantity: opts.quantity,
      recipient_name: opts.recipientName,
    };
    if (opts.holderId) body.holder_id = opts.holderId;
    if (opts.email) body.email = opts.email;
    if (opts.principalCents) body.principal_cents = opts.principalCents;
    if (opts.grantType) body.grant_type = opts.grantType;
    const result = await client.addRoundSecurity(opts.roundId, body);
    printSuccess(`Security added for ${opts.recipientName}`);
    printJson(result);
  } catch (err) {
    printError(`Failed to add security: ${err}`);
    process.exit(1);
  }
}

export async function issueRoundCommand(opts: {
  entityId?: string;
  roundId: string;
}): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const eid = resolveEntityId(cfg, opts.entityId);
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    const result = await client.issueRound(opts.roundId, { entity_id: eid });
    printSuccess("Round issued and closed");
    printJson(result);
  } catch (err) {
    printError(`Failed to issue round: ${err}`);
    process.exit(1);
  }
}

export async function createValuationCommand(opts: {
  entityId?: string;
  type: string;
  date: string;
  methodology: string;
  fmv?: number;
  enterpriseValue?: number;
}): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const eid = resolveEntityId(cfg, opts.entityId);
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    const body: Record<string, unknown> = {
      entity_id: eid,
      valuation_type: opts.type,
      effective_date: opts.date,
      methodology: opts.methodology,
    };
    if (opts.fmv != null) body.fmv_per_share_cents = opts.fmv;
    if (opts.enterpriseValue != null) body.enterprise_value_cents = opts.enterpriseValue;
    const result = await client.createValuation(body);
    printSuccess(`Valuation created: ${result.valuation_id ?? "OK"}`);
    printJson(result);
  } catch (err) {
    printError(`Failed to create valuation: ${err}`);
    process.exit(1);
  }
}

export async function submitValuationCommand(opts: {
  entityId?: string;
  valuationId: string;
}): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const eid = resolveEntityId(cfg, opts.entityId);
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    const result = await client.submitValuationForApproval(opts.valuationId, eid);
    printSuccess(`Valuation submitted for approval: ${result.valuation_id ?? "OK"}`);
    if (result.meeting_id) console.log(`  Meeting: ${result.meeting_id}`);
    if (result.agenda_item_id) console.log(`  Agenda Item: ${result.agenda_item_id}`);
    printJson(result);
  } catch (err) {
    printError(`Failed to submit valuation: ${err}`);
    process.exit(1);
  }
}

export async function approveValuationCommand(opts: {
  entityId?: string;
  valuationId: string;
  resolutionId?: string;
}): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const eid = resolveEntityId(cfg, opts.entityId);
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    const result = await client.approveValuation(opts.valuationId, eid, opts.resolutionId);
    printSuccess(`Valuation approved: ${result.valuation_id ?? "OK"}`);
    printJson(result);
  } catch (err) {
    printError(`Failed to approve valuation: ${err}`);
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
