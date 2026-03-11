import { requireConfig, resolveEntityId } from "../config.js";
import { CorpAPIClient } from "../api-client.js";
import {
  printCapTable, printSafesTable, printTransfersTable,
  printValuationsTable, printDryRun, printError, printSuccess, printJson, printWriteResult,
} from "../output.js";
import chalk from "chalk";

type CapTableInstrument = {
  instrument_id: string;
  kind: string;
  symbol: string;
  status?: string;
};

function normalizedGrantType(grantType: string): string {
  return grantType.trim().toLowerCase().replaceAll("-", "_").replaceAll(" ", "_");
}

function expectedInstrumentKinds(grantType: string): string[] {
  switch (normalizedGrantType(grantType)) {
    case "common":
    case "common_stock":
      return ["common_equity"];
    case "preferred":
    case "preferred_stock":
      return ["preferred_equity"];
    case "unit":
    case "membership_unit":
      return ["membership_unit"];
    case "option":
    case "options":
    case "stock_option":
    case "iso":
    case "nso":
      return ["option_grant"];
    case "rsa":
      return ["common_equity", "preferred_equity"];
    default:
      return [];
  }
}

function grantRequiresCurrent409a(grantType: string, instrumentKind?: string): boolean {
  return instrumentKind?.toLowerCase() === "option_grant" || expectedInstrumentKinds(grantType).includes("option_grant");
}

function buildInstrumentCreationHint(grantType: string): string {
  const normalized = normalizedGrantType(grantType);
  switch (normalized) {
    case "preferred":
    case "preferred_stock":
      return "Create one with: corp cap-table create-instrument --kind preferred_equity --symbol SERIES-A --authorized-units <shares>";
    case "option":
    case "options":
    case "stock_option":
    case "iso":
    case "nso":
      return "Create one with: corp cap-table create-instrument --kind option_grant --symbol OPTION-PLAN --authorized-units <shares>";
    case "membership_unit":
    case "unit":
      return "Create one with: corp cap-table create-instrument --kind membership_unit --symbol UNIT --authorized-units <units>";
    case "common":
    case "common_stock":
      return "Create one with: corp cap-table create-instrument --kind common_equity --symbol COMMON --authorized-units <shares>";
    default:
      return "Create a matching instrument first, then pass --instrument-id explicitly.";
  }
}

function resolveInstrumentForGrant(
  instruments: CapTableInstrument[],
  grantType: string,
  explicitInstrumentId?: string,
): CapTableInstrument {
  if (explicitInstrumentId) {
    const explicit = instruments.find((instrument) => instrument.instrument_id === explicitInstrumentId);
    if (!explicit) {
      throw new Error(`Instrument ${explicitInstrumentId} was not found on the cap table.`);
    }
    return explicit;
  }

  const expectedKinds = expectedInstrumentKinds(grantType);
  if (expectedKinds.length === 0) {
    throw new Error(
      `No default instrument mapping exists for grant type "${grantType}". ${buildInstrumentCreationHint(grantType)}`,
    );
  }
  const match = instruments.find((instrument) => expectedKinds.includes(String(instrument.kind).toLowerCase()));
  if (!match) {
    throw new Error(
      `No instrument found for grant type "${grantType}". Expected one of: ${expectedKinds.join(", ")}. ${buildInstrumentCreationHint(grantType)}`,
    );
  }
  return match;
}

async function entityHasActiveBoard(client: CorpAPIClient, entityId: string): Promise<boolean> {
  const bodies = await client.listGovernanceBodies(entityId);
  return bodies.some((body) =>
    String(body.body_type ?? "").toLowerCase() === "board_of_directors"
      && String(body.status ?? "active").toLowerCase() === "active"
  );
}

async function ensureIssuancePreflight(
  client: CorpAPIClient,
  entityId: string,
  grantType: string,
  instrument?: CapTableInstrument,
  meetingId?: string,
  resolutionId?: string,
): Promise<void> {
  if (!meetingId || !resolutionId) {
    if (await entityHasActiveBoard(client, entityId)) {
      throw new Error(
        "Board approval is required before issuing this round. Pass --meeting-id and --resolution-id from a passed board vote.",
      );
    }
  }

  if (!grantRequiresCurrent409a(grantType, instrument?.kind)) {
    return;
  }

  try {
    await client.getCurrent409a(entityId);
  } catch (err) {
    const msg = String(err);
    if (msg.includes("404")) {
      throw new Error(
        "Stock option issuances require a current approved 409A valuation. Create and approve one first with: corp cap-table create-valuation --type four_oh_nine_a --date YYYY-MM-DD --methodology <method>; corp cap-table submit-valuation <id>; corp cap-table approve-valuation <id> --resolution-id <resolution-id>",
      );
    }
    throw err;
  }
}

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
    const msg = String(err);
    if (msg.includes("404")) {
      console.log("No 409A valuation found for this entity. Create one with:\n  corp cap-table create-valuation --type four_oh_nine_a --date YYYY-MM-DD --methodology <method>");
    } else {
      printError(`Failed to fetch 409A valuation: ${err}`);
    }
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
  meetingId?: string;
  resolutionId?: string;
  json?: boolean;
  dryRun?: boolean;
}): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const eid = resolveEntityId(cfg, opts.entityId);
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    if (opts.dryRun) {
      printDryRun("cap_table.issue_equity", {
        entity_id: eid,
        grant_type: opts.grantType,
        shares: opts.shares,
        recipient: opts.recipient,
        email: opts.email,
        instrument_id: opts.instrumentId,
        meeting_id: opts.meetingId,
        resolution_id: opts.resolutionId,
      });
      return;
    }

    // Fetch cap table to get issuer and instrument info
    const capTable = await client.getCapTable(eid);
    const issuerLegalEntityId = capTable.issuer_legal_entity_id as string;
    if (!issuerLegalEntityId) {
      printError("No issuer legal entity found. Has this entity been formed with a cap table?");
      process.exit(1);
    }

    // Resolve instrument ID — use provided, or match by grant type, or use first common stock
    const instruments = (capTable.instruments ?? []) as CapTableInstrument[];
    if (!instruments.length) {
      printError("No instruments found on cap table. Create one with: corp cap-table create-instrument --kind common_equity --symbol COMMON --authorized-units <shares>");
      process.exit(1);
    }
    const instrument = resolveInstrumentForGrant(instruments, opts.grantType, opts.instrumentId);
    const instrumentId = instrument.instrument_id;
    if (!opts.instrumentId) {
      console.log(`Using instrument: ${instrument.symbol} (${instrument.kind})`);
    }
    await ensureIssuancePreflight(
      client,
      eid,
      opts.grantType,
      instrument,
      opts.meetingId,
      opts.resolutionId,
    );

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
    const issuePayload: Record<string, unknown> = { entity_id: eid };
    if (opts.meetingId) issuePayload.meeting_id = opts.meetingId;
    if (opts.resolutionId) issuePayload.resolution_id = opts.resolutionId;
    const result = await client.issueRound(roundId, issuePayload);

    if (opts.json) {
      printJson(result);
      return;
    }
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
  meetingId?: string;
  resolutionId?: string;
  json?: boolean;
  dryRun?: boolean;
}): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const eid = resolveEntityId(cfg, opts.entityId);
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    if (opts.dryRun) {
      printDryRun("cap_table.issue_safe", {
        entity_id: eid,
        investor: opts.investor,
        amount: opts.amount,
        safe_type: opts.safeType,
        valuation_cap: opts.valuationCap,
        email: opts.email,
        meeting_id: opts.meetingId,
        resolution_id: opts.resolutionId,
      });
      return;
    }

    await ensureIssuancePreflight(
      client,
      eid,
      opts.safeType,
      undefined,
      opts.meetingId,
      opts.resolutionId,
    );

    const body: Record<string, unknown> = {
      entity_id: eid,
      investor_name: opts.investor,
      principal_amount_cents: opts.amount,
      valuation_cap_cents: opts.valuationCap,
      safe_type: opts.safeType,
    };
    if (opts.email) body.email = opts.email;
    if (opts.meetingId) body.meeting_id = opts.meetingId;
    if (opts.resolutionId) body.resolution_id = opts.resolutionId;
    const result = await client.createSafeNote(body);
    if (opts.json) {
      printJson(result);
      return;
    }
    printSuccess(`SAFE issued: $${(opts.amount / 100).toLocaleString()} to ${opts.investor}`);
    printJson(result);
  } catch (err) {
    printError(`Failed to issue SAFE: ${err}`);
    process.exit(1);
  }
}

export async function transferSharesCommand(opts: {
  entityId?: string;
  from: string;
  to: string;
  shares: number;
  type: string;
  shareClassId: string;
  governingDocType: string;
  transfereeRights: string;
  prepareIntentId?: string;
  pricePerShareCents?: number;
  relationship?: string;
  json?: boolean;
  dryRun?: boolean;
}): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const eid = resolveEntityId(cfg, opts.entityId);
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    if (opts.pricePerShareCents != null && opts.pricePerShareCents < 0) {
      throw new Error("price-per-share-cents cannot be negative");
    }
    if (opts.from === opts.to) {
      throw new Error("--from and --to must be different contacts");
    }
    if (opts.dryRun) {
      printDryRun("cap_table.transfer_shares", {
        entity_id: eid,
        from_contact_id: opts.from,
        to_contact_id: opts.to,
        share_count: opts.shares,
        transfer_type: opts.type,
        share_class_id: opts.shareClassId,
        governing_doc_type: opts.governingDocType,
        transferee_rights: opts.transfereeRights,
        prepare_intent_id: opts.prepareIntentId,
        price_per_share_cents: opts.pricePerShareCents,
        relationship_to_holder: opts.relationship,
      });
      return;
    }

    let intentId = opts.prepareIntentId;
    if (!intentId) {
      const intent = await client.createExecutionIntent({
        entity_id: eid,
        intent_type: "equity.transfer.prepare",
        description: `Transfer ${opts.shares} shares from ${opts.from} to ${opts.to}`,
      });
      intentId = (intent.intent_id ?? intent.id) as string;
      await client.evaluateIntent(intentId, eid);
      await client.authorizeIntent(intentId, eid);
    }
    const body: Record<string, unknown> = {
      entity_id: eid,
      share_class_id: opts.shareClassId,
      from_contact_id: opts.from,
      to_contact_id: opts.to,
      transfer_type: opts.type,
      share_count: opts.shares,
      governing_doc_type: opts.governingDocType,
      transferee_rights: opts.transfereeRights,
      prepare_intent_id: intentId,
    };
    if (opts.pricePerShareCents != null) body.price_per_share_cents = opts.pricePerShareCents;
    if (opts.relationship) body.relationship_to_holder = opts.relationship;
    const result = await client.transferShares(body);
    printWriteResult(result, `Transfer workflow created: ${result.workflow_id ?? "OK"}`, opts.json);
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
  json?: boolean;
  dryRun?: boolean;
}): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const eid = resolveEntityId(cfg, opts.entityId);
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    const payload = {
      entity_id: eid, total_amount_cents: opts.amount, distribution_type: opts.type,
      description: opts.description,
    };
    if (opts.dryRun) {
      printDryRun("cap_table.distribute", payload);
      return;
    }
    const result = await client.calculateDistribution(payload);
    printWriteResult(result, `Distribution calculated: ${result.distribution_id ?? "OK"}`, opts.json);
  } catch (err) {
    printError(`Failed to calculate distribution: ${err}`);
    process.exit(1);
  }
}

export async function startRoundCommand(opts: {
  entityId?: string;
  name: string;
  issuerLegalEntityId: string;
  json?: boolean;
  dryRun?: boolean;
}): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const eid = resolveEntityId(cfg, opts.entityId);
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    const payload = {
      entity_id: eid,
      name: opts.name,
      issuer_legal_entity_id: opts.issuerLegalEntityId,
    };
    if (opts.dryRun) {
      printDryRun("cap_table.start_round", payload);
      return;
    }
    const result = await client.startEquityRound(payload);
    printWriteResult(result, `Round started: ${result.round_id ?? "OK"}`, opts.json);
  } catch (err) {
    printError(`Failed to start round: ${err}`);
    process.exit(1);
  }
}

export async function createInstrumentCommand(opts: {
  entityId?: string;
  issuerLegalEntityId?: string;
  kind: string;
  symbol: string;
  authorizedUnits?: number;
  issuePriceCents?: number;
  termsJson?: string;
  json?: boolean;
  dryRun?: boolean;
}): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const eid = resolveEntityId(cfg, opts.entityId);
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    let issuerLegalEntityId = opts.issuerLegalEntityId;
    if (!issuerLegalEntityId) {
      const capTable = await client.getCapTable(eid);
      issuerLegalEntityId = capTable.issuer_legal_entity_id as string | undefined;
    }
    if (!issuerLegalEntityId) {
      throw new Error("No issuer legal entity found. Has this entity been formed with a cap table?");
    }

    const terms = opts.termsJson ? JSON.parse(opts.termsJson) as Record<string, unknown> : {};
    const payload: Record<string, unknown> = {
      entity_id: eid,
      issuer_legal_entity_id: issuerLegalEntityId,
      kind: opts.kind,
      symbol: opts.symbol,
      terms,
    };
    if (opts.authorizedUnits != null) payload.authorized_units = opts.authorizedUnits;
    if (opts.issuePriceCents != null) payload.issue_price_cents = opts.issuePriceCents;
    if (opts.dryRun) {
      printDryRun("cap_table.create_instrument", payload);
      return;
    }
    const result = await client.createInstrument(payload);
    printWriteResult(result, `Instrument created: ${result.instrument_id ?? "OK"}`, opts.json);
  } catch (err) {
    printError(`Failed to create instrument: ${err}`);
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
  json?: boolean;
  dryRun?: boolean;
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
    if (opts.dryRun) {
      printDryRun("cap_table.add_security", { round_id: opts.roundId, ...body });
      return;
    }
    const result = await client.addRoundSecurity(opts.roundId, body);
    printWriteResult(result, `Security added for ${opts.recipientName}`, opts.json);
  } catch (err) {
    printError(`Failed to add security: ${err}`);
    process.exit(1);
  }
}

export async function issueRoundCommand(opts: {
  entityId?: string;
  roundId: string;
  meetingId?: string;
  resolutionId?: string;
  json?: boolean;
  dryRun?: boolean;
}): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const eid = resolveEntityId(cfg, opts.entityId);
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    if (opts.dryRun) {
      printDryRun("cap_table.issue_round", {
        entity_id: eid,
        round_id: opts.roundId,
        meeting_id: opts.meetingId,
        resolution_id: opts.resolutionId,
      });
      return;
    }
    if ((!opts.meetingId || !opts.resolutionId) && await entityHasActiveBoard(client, eid)) {
      throw new Error(
        "Board approval is required before issuing this round. Pass --meeting-id and --resolution-id from a passed board vote.",
      );
    }
    const body: Record<string, unknown> = { entity_id: eid };
    if (opts.meetingId) body.meeting_id = opts.meetingId;
    if (opts.resolutionId) body.resolution_id = opts.resolutionId;
    const result = await client.issueRound(opts.roundId, body);
    printWriteResult(result, "Round issued and closed", opts.json);
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
  json?: boolean;
  dryRun?: boolean;
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
    if (opts.dryRun) {
      printDryRun("cap_table.create_valuation", body);
      return;
    }
    const result = await client.createValuation(body);
    printWriteResult(result, `Valuation created: ${result.valuation_id ?? "OK"}`, opts.json);
  } catch (err) {
    printError(`Failed to create valuation: ${err}`);
    process.exit(1);
  }
}

export async function submitValuationCommand(opts: {
  entityId?: string;
  valuationId: string;
  json?: boolean;
  dryRun?: boolean;
}): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const eid = resolveEntityId(cfg, opts.entityId);
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    if (opts.dryRun) {
      printDryRun("cap_table.submit_valuation", { entity_id: eid, valuation_id: opts.valuationId });
      return;
    }
    const result = await client.submitValuationForApproval(opts.valuationId, eid);
    if (opts.json) {
      printJson(result);
      return;
    }
    printSuccess(`Valuation submitted for approval: ${result.valuation_id ?? "OK"}`);
    if (result.meeting_id) console.log(`  Meeting: ${result.meeting_id}`);
    if (result.agenda_item_id) console.log(`  Agenda Item: ${result.agenda_item_id}`);
    printJson(result);
  } catch (err) {
    const msg = String(err);
    if (msg.includes("404")) {
      printError(`Valuation not found. List valuations with: corp cap-table valuations`);
    } else {
      printError(`Failed to submit valuation: ${err}`);
    }
    process.exit(1);
  }
}

export async function approveValuationCommand(opts: {
  entityId?: string;
  valuationId: string;
  resolutionId?: string;
  json?: boolean;
  dryRun?: boolean;
}): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const eid = resolveEntityId(cfg, opts.entityId);
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    if (opts.dryRun) {
      printDryRun("cap_table.approve_valuation", {
        entity_id: eid,
        valuation_id: opts.valuationId,
        resolution_id: opts.resolutionId,
      });
      return;
    }
    const result = await client.approveValuation(opts.valuationId, eid, opts.resolutionId);
    printWriteResult(result, `Valuation approved: ${result.valuation_id ?? "OK"}`, opts.json);
  } catch (err) {
    const msg = String(err);
    if (msg.includes("400")) {
      printError(`Bad request — a --resolution-id from a board vote may be required. Submit for approval first: corp cap-table submit-valuation <id>`);
    } else {
      printError(`Failed to approve valuation: ${err}`);
    }
    process.exit(1);
  }
}

function print409a(data: Record<string, unknown>): void {
  console.log(chalk.green("─".repeat(40)));
  console.log(chalk.green.bold("  409A Valuation"));
  console.log(chalk.green("─".repeat(40)));
  const fmv = typeof data.fmv_per_share_cents === "number" ? (data.fmv_per_share_cents as number) / 100 : data.fmv_per_share;
  const enterpriseValue = typeof data.enterprise_value_cents === "number"
    ? (data.enterprise_value_cents as number) / 100
    : data.enterprise_value;
  console.log(`  ${chalk.bold("FMV/Share:")} $${fmv ?? "N/A"}`);
  console.log(`  ${chalk.bold("Enterprise Value:")} $${enterpriseValue ?? "N/A"}`);
  console.log(`  ${chalk.bold("Valuation Date:")} ${data.effective_date ?? data.valuation_date ?? "N/A"}`);
  if (data.provider) console.log(`  ${chalk.bold("Provider:")} ${data.provider}`);
  console.log(chalk.green("─".repeat(40)));
}
