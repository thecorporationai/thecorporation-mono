import { requireConfig } from "../config.js";
import { CorpAPIClient } from "../api-client.js";
import {
  printCapTable,
  printDryRun,
  printError,
  printInstrumentsTable,
  printJson,
  printReferenceSummary,
  printRoundsTable,
  printSafesTable,
  printShareClassesTable,
  printSuccess,
  printTransfersTable,
  printValuationsTable,
  printWriteResult,
} from "../output.js";
import { ReferenceResolver, shortId } from "../references.js";
import type { ApiRecord } from "../types.js";
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
        "Board approval is required before issuing this round. Pass --meeting-id and --resolution-id from a passed board vote.\n  Tip: Use 'corp governance quick-approve --text \"RESOLVED: ...\"' for one-step approval.",
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
        "Stock option issuances require a current approved 409A valuation. Create and approve one first with: corp cap-table create-valuation --type four_oh_nine_a --date YYYY-MM-DD --methodology <method>; corp cap-table submit-valuation <valuation-ref>; corp cap-table approve-valuation <valuation-ref> --resolution-id <resolution-ref>",
      );
    }
    throw err;
  }
}

export async function capTableCommand(opts: { entityId?: string; json?: boolean }): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const eid = await resolver.resolveEntity(opts.entityId);
    const data = await client.getCapTable(eid);
    const instruments = Array.isArray(data.instruments) ? data.instruments as ApiRecord[] : [];
    const shareClasses = Array.isArray(data.share_classes) ? data.share_classes as ApiRecord[] : [];
    await resolver.stabilizeRecords("instrument", instruments, eid);
    await resolver.stabilizeRecords("share_class", shareClasses, eid);
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
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const eid = await resolver.resolveEntity(opts.entityId);
    const safes = await client.getSafeNotes(eid);
    await resolver.stabilizeRecords("safe_note", safes, eid);
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
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const eid = await resolver.resolveEntity(opts.entityId);
    const transfers = await client.getShareTransfers(eid);
    await resolver.stabilizeRecords("share_transfer", transfers, eid);
    if (opts.json) printJson(transfers);
    else if (transfers.length === 0) console.log("No share transfers found.");
    else printTransfersTable(transfers);
  } catch (err) {
    printError(`Failed to fetch transfers: ${err}`);
    process.exit(1);
  }
}

export async function instrumentsCommand(opts: { entityId?: string; json?: boolean }): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const eid = await resolver.resolveEntity(opts.entityId);
    const capTable = await client.getCapTable(eid);
    const instruments = Array.isArray(capTable.instruments) ? capTable.instruments as Record<string, unknown>[] : [];
    await resolver.stabilizeRecords("instrument", instruments as ApiRecord[], eid);
    if (opts.json) printJson(instruments);
    else if (instruments.length === 0) console.log("No instruments found.");
    else printInstrumentsTable(instruments);
  } catch (err) {
    printError(`Failed to fetch instruments: ${err}`);
    process.exit(1);
  }
}

export async function shareClassesCommand(opts: { entityId?: string; json?: boolean }): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const eid = await resolver.resolveEntity(opts.entityId);
    const capTable = await client.getCapTable(eid);
    const shareClasses = Array.isArray(capTable.share_classes)
      ? capTable.share_classes as Record<string, unknown>[]
      : [];
    await resolver.stabilizeRecords("share_class", shareClasses as ApiRecord[], eid);
    if (opts.json) printJson(shareClasses);
    else if (shareClasses.length === 0) console.log("No share classes found.");
    else printShareClassesTable(shareClasses);
  } catch (err) {
    printError(`Failed to fetch share classes: ${err}`);
    process.exit(1);
  }
}

export async function roundsCommand(opts: { entityId?: string; json?: boolean }): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const eid = await resolver.resolveEntity(opts.entityId);
    const rounds = await client.listEquityRounds(eid);
    await resolver.stabilizeRecords("round", rounds, eid);
    if (opts.json) printJson(rounds);
    else if (rounds.length === 0) console.log("No rounds found.");
    else printRoundsTable(rounds);
  } catch (err) {
    printError(`Failed to fetch rounds: ${err}`);
    process.exit(1);
  }
}

export async function valuationsCommand(opts: { entityId?: string; json?: boolean }): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const eid = await resolver.resolveEntity(opts.entityId);
    const valuations = await client.getValuations(eid);
    await resolver.stabilizeRecords("valuation", valuations, eid);
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
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const eid = await resolver.resolveEntity(opts.entityId);
    const data = await client.getCurrent409a(eid);
    await resolver.stabilizeRecord("valuation", data, eid);
    if (opts.json) printJson(data);
    else if (!data || Object.keys(data).length === 0) console.log("No 409A valuation found.");
    else print409a(data);
  } catch (err) {
    const msg = String(err);
    if (msg.includes("404")) {
      try {
        const eid = await resolver.resolveEntity(opts.entityId);
        const valuations = await client.getValuations(eid);
        const pending409a = valuations
          .filter((valuation) => valuation.valuation_type === "four_oh_nine_a")
          .find((valuation) => valuation.status === "pending_approval");
        if (pending409a) {
          const effectiveDate = pending409a.effective_date ?? "unknown date";
          console.log(
            `No current approved 409A valuation found. A 409A valuation is pending approval (${effectiveDate}).\n` +
            "  Complete board approval, then re-run: corp cap-table 409a",
          );
        } else {
          console.log(
            "No 409A valuation found for this entity. Create one with:\n" +
            "  corp cap-table create-valuation --type four_oh_nine_a --date YYYY-MM-DD --methodology <method>",
          );
        }
      } catch {
        console.log(
          "No 409A valuation found for this entity. Create one with:\n" +
          "  corp cap-table create-valuation --type four_oh_nine_a --date YYYY-MM-DD --methodology <method>",
        );
      }
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
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const eid = await resolver.resolveEntity(opts.entityId);
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
    const explicitInstrumentId = opts.instrumentId
      ? await resolver.resolveInstrument(eid, opts.instrumentId)
      : undefined;
    const instrument = resolveInstrumentForGrant(instruments, opts.grantType, explicitInstrumentId);
    const instrumentId = instrument.instrument_id;
    if (!opts.instrumentId) {
      console.log(`Using instrument: ${instrument.symbol} (${instrument.kind})`);
    }
    const meetingId = opts.meetingId ? await resolver.resolveMeeting(eid, opts.meetingId) : undefined;
    const resolutionId = opts.resolutionId
      ? await resolver.resolveResolution(eid, opts.resolutionId, meetingId)
      : undefined;
    await ensureIssuancePreflight(
      client,
      eid,
      opts.grantType,
      instrument,
      meetingId,
      resolutionId,
    );

    // 1. Start a staged round
    const round = await client.startEquityRound({
      entity_id: eid,
      name: `${opts.grantType} grant — ${opts.recipient}`,
      issuer_legal_entity_id: issuerLegalEntityId,
    });
    await resolver.stabilizeRecord("round", round, eid);
    const roundId = (round.round_id ?? round.equity_round_id) as string;

    // 2. Add the security — deduplicate holder by matching name/email against existing holders
    const securityData: Record<string, unknown> = {
      entity_id: eid,
      instrument_id: instrumentId,
      quantity: opts.shares,
      recipient_name: opts.recipient,
      grant_type: opts.grantType,
    };
    if (opts.email) securityData.email = opts.email;

    // Attempt to find existing holder to avoid creating duplicates
    const existingHolders = (capTable.holders ?? []) as ApiRecord[];
    const matchingHolder = existingHolders.find((h) => {
      const nameMatch = String(h.name ?? "").toLowerCase() === opts.recipient.toLowerCase();
      const emailMatch = opts.email && String(h.email ?? "").toLowerCase() === opts.email.toLowerCase();
      return nameMatch || emailMatch;
    });
    if (matchingHolder) {
      const holderId = matchingHolder.holder_id ?? matchingHolder.contact_id ?? matchingHolder.id;
      if (holderId) securityData.holder_id = holderId;
    }

    await client.addRoundSecurity(roundId, securityData);

    // 3. Issue the round
    const issuePayload: Record<string, unknown> = { entity_id: eid };
    if (meetingId) issuePayload.meeting_id = meetingId;
    if (resolutionId) issuePayload.resolution_id = resolutionId;
    const result = await client.issueRound(roundId, issuePayload);
    resolver.rememberFromRecord("round", round, eid);

    if (opts.json) {
      printJson(result);
      return;
    }
    printSuccess(`Equity issued: ${opts.shares} shares (${opts.grantType}) to ${opts.recipient}`);
    printReferenceSummary("round", round, { label: "Round Ref:", showReuseHint: true });
  } catch (err) {
    printError(`Failed to issue equity: ${err}`);
    process.exit(1);
  }
}

export async function issueSafeCommand(opts: {
  entityId?: string;
  investor: string;
  amountCents: number;
  safeType: string;
  valuationCapCents: number;
  email?: string;
  meetingId?: string;
  resolutionId?: string;
  json?: boolean;
  dryRun?: boolean;
}): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const eid = await resolver.resolveEntity(opts.entityId);
    if (opts.dryRun) {
      printDryRun("cap_table.issue_safe", {
        entity_id: eid,
        investor: opts.investor,
        amount_cents: opts.amountCents,
        safe_type: opts.safeType,
        valuation_cap_cents: opts.valuationCapCents,
        email: opts.email,
        meeting_id: opts.meetingId,
        resolution_id: opts.resolutionId,
      });
      return;
    }

    const meetingId = opts.meetingId ? await resolver.resolveMeeting(eid, opts.meetingId) : undefined;
    const resolutionId = opts.resolutionId
      ? await resolver.resolveResolution(eid, opts.resolutionId, meetingId)
      : undefined;
    await ensureIssuancePreflight(
      client,
      eid,
      opts.safeType,
      undefined,
      meetingId,
      resolutionId,
    );

    const body: Record<string, unknown> = {
      entity_id: eid,
      investor_name: opts.investor,
      principal_amount_cents: opts.amountCents,
      valuation_cap_cents: opts.valuationCapCents,
      safe_type: opts.safeType,
    };
    if (opts.email) body.email = opts.email;
    if (meetingId) body.meeting_id = meetingId;
    if (resolutionId) body.resolution_id = resolutionId;
    const result = await client.createSafeNote(body);
    await resolver.stabilizeRecord("safe_note", result, eid);
    resolver.rememberFromRecord("safe_note", result, eid);
    if (opts.json) {
      printJson(result);
      return;
    }
    printSuccess(`SAFE issued: $${(opts.amountCents / 100).toLocaleString()} to ${opts.investor}`);
    printReferenceSummary("safe_note", result, { showReuseHint: true });
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
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const eid = await resolver.resolveEntity(opts.entityId);
    const fromContactId = await resolver.resolveContact(eid, opts.from);
    const toContactId = await resolver.resolveContact(eid, opts.to);
    const shareClassId = await resolver.resolveShareClass(eid, opts.shareClassId);
    if (opts.pricePerShareCents != null && opts.pricePerShareCents < 0) {
      throw new Error("price-per-share-cents cannot be negative");
    }
    if (fromContactId === toContactId) {
      throw new Error("--from and --to must be different contacts");
    }
    if (opts.dryRun) {
      printDryRun("cap_table.transfer_shares", {
        entity_id: eid,
        from_contact_id: fromContactId,
        to_contact_id: toContactId,
        share_count: opts.shares,
        transfer_type: opts.type,
        share_class_id: shareClassId,
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
        description: `Transfer ${opts.shares} shares from ${fromContactId} to ${toContactId}`,
      });
      intentId = (intent.intent_id ?? intent.id) as string;
      await client.evaluateIntent(intentId, eid);
      await client.authorizeIntent(intentId, eid);
    }
    const body: Record<string, unknown> = {
      entity_id: eid,
      share_class_id: shareClassId,
      from_contact_id: fromContactId,
      to_contact_id: toContactId,
      transfer_type: opts.type,
      share_count: opts.shares,
      governing_doc_type: opts.governingDocType,
      transferee_rights: opts.transfereeRights,
      prepare_intent_id: intentId,
    };
    if (opts.pricePerShareCents != null) body.price_per_share_cents = opts.pricePerShareCents;
    if (opts.relationship) body.relationship_to_holder = opts.relationship;
    const result = await client.transferShares(body);
    await resolver.stabilizeRecord("share_transfer", result, eid);
    resolver.rememberFromRecord("share_transfer", result, eid);
    printWriteResult(result, `Transfer workflow created: ${result.transfer_workflow_id ?? "OK"}`, {
      jsonOnly: opts.json,
      referenceKind: "share_transfer",
      referenceLabel: "Transfer Ref:",
      showReuseHint: true,
    });
  } catch (err) {
    printError(`Failed to create transfer workflow: ${err}`);
    process.exit(1);
  }
}

export async function distributeCommand(opts: {
  entityId?: string;
  amountCents: number;
  type: string;
  description: string;
  json?: boolean;
  dryRun?: boolean;
}): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const eid = await resolver.resolveEntity(opts.entityId);
    const payload = {
      entity_id: eid, total_amount_cents: opts.amountCents, distribution_type: opts.type,
      description: opts.description,
    };
    if (opts.dryRun) {
      printDryRun("cap_table.distribute", payload);
      return;
    }
    const result = await client.calculateDistribution(payload);
    await resolver.stabilizeRecord("distribution", result, eid);
    resolver.rememberFromRecord("distribution", result, eid);
    printWriteResult(result, `Distribution calculated: ${result.distribution_id ?? "OK"}`, {
      jsonOnly: opts.json,
      referenceKind: "distribution",
      showReuseHint: true,
    });
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
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const eid = await resolver.resolveEntity(opts.entityId);
    const issuerLegalEntityId = await resolver.resolveEntity(opts.issuerLegalEntityId);
    const payload = {
      entity_id: eid,
      name: opts.name,
      issuer_legal_entity_id: issuerLegalEntityId,
    };
    if (opts.dryRun) {
      printDryRun("cap_table.start_round", payload);
      return;
    }
    const result = await client.startEquityRound(payload);
    await resolver.stabilizeRecord("round", result, eid);
    resolver.rememberFromRecord("round", result, eid);
    printWriteResult(result, `Round started: ${result.round_id ?? "OK"}`, {
      jsonOnly: opts.json,
      referenceKind: "round",
      showReuseHint: true,
    });
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
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const eid = await resolver.resolveEntity(opts.entityId);
    let issuerLegalEntityId = opts.issuerLegalEntityId;
    if (!issuerLegalEntityId) {
      const capTable = await client.getCapTable(eid);
      issuerLegalEntityId = capTable.issuer_legal_entity_id as string | undefined;
    }
    if (!issuerLegalEntityId) {
      throw new Error("No issuer legal entity found. Has this entity been formed with a cap table?");
    }
    issuerLegalEntityId = await resolver.resolveEntity(issuerLegalEntityId);

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
    await resolver.stabilizeRecord("instrument", result, eid);
    resolver.rememberFromRecord("instrument", result, eid);
    printWriteResult(result, `Instrument created: ${result.instrument_id ?? "OK"}`, {
      jsonOnly: opts.json,
      referenceKind: "instrument",
      showReuseHint: true,
    });
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
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const eid = await resolver.resolveEntity(opts.entityId);
    const roundId = await resolver.resolveRound(eid, opts.roundId);
    const instrumentId = await resolver.resolveInstrument(eid, opts.instrumentId);
    const body: Record<string, unknown> = {
      entity_id: eid,
      instrument_id: instrumentId,
      quantity: opts.quantity,
      recipient_name: opts.recipientName,
    };
    if (opts.holderId) body.holder_id = await resolver.resolveContact(eid, opts.holderId);
    if (opts.email) body.email = opts.email;
    if (opts.principalCents) body.principal_cents = opts.principalCents;
    if (opts.grantType) body.grant_type = opts.grantType;
    if (opts.dryRun) {
      printDryRun("cap_table.add_security", { round_id: roundId, ...body });
      return;
    }
    const result = await client.addRoundSecurity(roundId, body);
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
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const eid = await resolver.resolveEntity(opts.entityId);
    const roundId = await resolver.resolveRound(eid, opts.roundId);
    const meetingId = opts.meetingId ? await resolver.resolveMeeting(eid, opts.meetingId) : undefined;
    const resolutionId = opts.resolutionId
      ? await resolver.resolveResolution(eid, opts.resolutionId, meetingId)
      : undefined;
    if (opts.dryRun) {
      printDryRun("cap_table.issue_round", {
        entity_id: eid,
        round_id: roundId,
        meeting_id: meetingId,
        resolution_id: resolutionId,
      });
      return;
    }
    if ((!meetingId || !resolutionId) && await entityHasActiveBoard(client, eid)) {
      throw new Error(
        "Board approval is required before issuing this round. Pass --meeting-id and --resolution-id from a passed board vote.\n  Tip: Use 'corp governance quick-approve --text \"RESOLVED: ...\"' for one-step approval.",
      );
    }
    const body: Record<string, unknown> = { entity_id: eid };
    if (meetingId) body.meeting_id = meetingId;
    if (resolutionId) body.resolution_id = resolutionId;
    const result = await client.issueRound(roundId, body);
    resolver.remember("round", roundId, eid);
    const roundMatch = (await resolver.find("round", shortId(roundId), { entityId: eid }))
      .find((match) => match.id === roundId);
    if (opts.json) {
      printJson(result);
      return;
    }
    printSuccess("Round issued and closed");
    if (roundMatch) {
      printReferenceSummary("round", roundMatch.raw, { showReuseHint: true });
    }
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
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const eid = await resolver.resolveEntity(opts.entityId);
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
    await resolver.stabilizeRecord("valuation", result, eid);
    resolver.rememberFromRecord("valuation", result, eid);
    printWriteResult(result, `Valuation created: ${result.valuation_id ?? "OK"}`, {
      jsonOnly: opts.json,
      referenceKind: "valuation",
      showReuseHint: true,
    });
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
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const eid = await resolver.resolveEntity(opts.entityId);
    const valuationId = await resolver.resolveValuation(eid, opts.valuationId);
    if (opts.dryRun) {
      printDryRun("cap_table.submit_valuation", { entity_id: eid, valuation_id: valuationId });
      return;
    }
    const result = await client.submitValuationForApproval(valuationId, eid);
    await resolver.stabilizeRecord("valuation", result, eid);
    resolver.remember("valuation", valuationId, eid);
    if (result.meeting_id) resolver.remember("meeting", String(result.meeting_id), eid);
    if (result.agenda_item_id) resolver.remember("agenda_item", String(result.agenda_item_id), eid);
    if (opts.json) {
      printJson(result);
      return;
    }
    printSuccess(`Valuation submitted for approval: ${result.valuation_id ?? valuationId ?? "OK"}`);
    printReferenceSummary("valuation", result, { showReuseHint: true });
    if (result.meeting_id) {
      const meetingMatch = (await resolver.find("meeting", shortId(String(result.meeting_id)), { entityId: eid }))
        .find((match) => match.id === String(result.meeting_id));
      if (meetingMatch) {
        printReferenceSummary("meeting", meetingMatch.raw, { label: "Meeting Ref:" });
      } else {
        printReferenceSummary("meeting", { meeting_id: result.meeting_id }, { label: "Meeting Ref:" });
      }
    }
    if (result.agenda_item_id) {
      const agendaMatch = (await resolver.find("agenda_item", shortId(String(result.agenda_item_id)), {
        entityId: eid,
        meetingId: result.meeting_id ? String(result.meeting_id) : undefined,
      }))
        .find((match) => match.id === String(result.agenda_item_id));
      if (agendaMatch) {
        printReferenceSummary("agenda_item", agendaMatch.raw, { label: "Agenda Ref:" });
      } else {
        printReferenceSummary("agenda_item", { agenda_item_id: result.agenda_item_id }, { label: "Agenda Ref:" });
      }
    }
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
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const eid = await resolver.resolveEntity(opts.entityId);
    const valuationId = await resolver.resolveValuation(eid, opts.valuationId);
    const resolutionId = opts.resolutionId
      ? await resolver.resolveResolution(eid, opts.resolutionId)
      : undefined;
    if (opts.dryRun) {
      printDryRun("cap_table.approve_valuation", {
        entity_id: eid,
        valuation_id: valuationId,
        resolution_id: resolutionId,
      });
      return;
    }
    const result = await client.approveValuation(valuationId, eid, resolutionId);
    await resolver.stabilizeRecord("valuation", result, eid);
    printWriteResult(result, `Valuation approved: ${result.valuation_id ?? valuationId ?? "OK"}`, {
      jsonOnly: opts.json,
      referenceKind: "valuation",
    });
  } catch (err) {
    const msg = String(err);
    if (msg.includes("400")) {
      printError(`Bad request — a --resolution-id from a board vote may be required. Submit for approval first: corp cap-table submit-valuation <valuation-ref>`);
    } else {
      printError(`Failed to approve valuation: ${err}`);
    }
    process.exit(1);
  }
}

export async function previewConversionCommand(opts: {
  entityId?: string; safeId: string; pricePerShareCents: number; json?: boolean;
}): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const eid = await resolver.resolveEntity(opts.entityId);
    const safeId = await resolver.resolveSafeNote(eid, opts.safeId);
    const result = await client.previewRoundConversion({
      entity_id: eid,
      safe_note_id: safeId,
      price_per_share_cents: opts.pricePerShareCents,
    } as unknown as Parameters<typeof client.previewRoundConversion>[0]);
    if (opts.json) { printJson(result); return; }
    printSuccess("Conversion Preview:");
    if (result.shares_issued) console.log(`  Shares to issue: ${result.shares_issued}`);
    if (result.ownership_pct) console.log(`  Post-conversion ownership: ${result.ownership_pct}%`);
    printJson(result);
  } catch (err) { printError(`Failed to preview conversion: ${err}`); process.exit(1); }
}

export async function executeConversionCommand(opts: {
  entityId?: string; safeId: string; pricePerShareCents: number;
  json?: boolean; dryRun?: boolean;
}): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const eid = await resolver.resolveEntity(opts.entityId);
    const safeId = await resolver.resolveSafeNote(eid, opts.safeId);
    const payload = {
      entity_id: eid,
      safe_note_id: safeId,
      price_per_share_cents: opts.pricePerShareCents,
    };
    if (opts.dryRun) { printDryRun("equity.conversion.execute", payload); return; }
    const result = await client.executeRoundConversion(
      payload as unknown as Parameters<typeof client.executeRoundConversion>[0],
    );
    printWriteResult(result, `Conversion executed for SAFE ${safeId}`, {
      jsonOnly: opts.json,
    });
  } catch (err) { printError(`Failed to execute conversion: ${err}`); process.exit(1); }
}

export async function dilutionPreviewCommand(opts: {
  entityId?: string; roundId: string; json?: boolean;
}): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const eid = await resolver.resolveEntity(opts.entityId);
    const roundId = await resolver.resolveRound(eid, opts.roundId);
    const result = await client.getDilutionPreview(eid, roundId);
    if (opts.json) { printJson(result); return; }
    // Warn if the round is closed — dilution preview on closed rounds returns 0
    if (result.round_status === "closed" || result.round_status === "issued") {
      console.log(chalk.yellow("Note: This round is already closed. Dilution preview reflects the finalized state, not a scenario model."));
      console.log(chalk.dim("  For scenario modeling, create a new round with: corp cap-table start-round --name '...' --issuer-legal-entity-id '...'"));
    }
    printJson(result);
  } catch (err) { printError(`Failed to preview dilution: ${err}`); process.exit(1); }
}

export async function controlMapCommand(opts: {
  entityId?: string; rootEntityId?: string; json?: boolean;
}): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const eid = await resolver.resolveEntity(opts.entityId);
    const rootEntityId = opts.rootEntityId
      ? await resolver.resolveEntity(opts.rootEntityId)
      : eid;

    let result: ApiRecord;
    try {
      result = await client.getControlMap(eid, rootEntityId);
    } catch (firstErr) {
      // If the entity_id itself works but root_entity_id fails, try using
      // the cap table's issuer_legal_entity_id as the root instead.
      const msg = String(firstErr);
      if (msg.includes("404") && !opts.rootEntityId) {
        try {
          const capTable = await client.getCapTable(eid);
          const issuerLegalEntityId = capTable.issuer_legal_entity_id as string | undefined;
          if (issuerLegalEntityId && issuerLegalEntityId !== eid) {
            result = await client.getControlMap(eid, issuerLegalEntityId);
          } else {
            throw firstErr;
          }
        } catch {
          throw firstErr;
        }
      } else {
        throw firstErr;
      }
    }
    if (opts.json) { printJson(result); return; }
    printJson(result);
  } catch (err) {
    const msg = String(err);
    if (msg.includes("404") && (msg.includes("root_entity_id") || msg.includes("not found"))) {
      printError(
        `Control map: entity not found. Ensure the entity is active and has a cap table.\n` +
        "  Try: corp cap-table control-map --root-entity-id <legal-entity-id>",
      );
    } else {
      printError(`Failed to fetch control map: ${err}`);
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
