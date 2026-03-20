import chalk from "chalk";
import type { CommandDef, CommandContext } from "./types.js";
import type { ApiRecord } from "../types.js";
import {
  printCapTable,
  printInstrumentsTable,
  printReferenceSummary,
  printRoundsTable,
  printSafesTable,
  printShareClassesTable,
  printTransfersTable,
  printValuationsTable,
} from "../output.js";
import { shortId } from "../references.js";
import {
  entityHasActiveBoard,
  issueEquity,
  issueSafe,
} from "@thecorporation/corp-tools";

// Helpers (normalizedGrantType, expectedInstrumentKinds, grantRequiresCurrent409a,
// buildInstrumentCreationHint, resolveInstrumentForGrant, entityHasActiveBoard,
// ensureIssuancePreflight) are now imported from @thecorporation/corp-tools.

// ---------------------------------------------------------------------------
// Local output helper — 409A panel
// ---------------------------------------------------------------------------

function print409a(data: Record<string, unknown>): void {
  console.log(chalk.green("\u2500".repeat(40)));
  console.log(chalk.green.bold("  409A Valuation"));
  console.log(chalk.green("\u2500".repeat(40)));
  const fmv = typeof data.fmv_per_share_cents === "number" ? (data.fmv_per_share_cents as number) / 100 : data.fmv_per_share;
  const enterpriseValue = typeof data.enterprise_value_cents === "number"
    ? (data.enterprise_value_cents as number) / 100
    : data.enterprise_value;
  console.log(`  ${chalk.bold("FMV/Share:")} $${fmv ?? "N/A"}`);
  console.log(`  ${chalk.bold("Enterprise Value:")} $${enterpriseValue ?? "N/A"}`);
  console.log(`  ${chalk.bold("Valuation Date:")} ${data.effective_date ?? data.valuation_date ?? "N/A"}`);
  if (data.provider) console.log(`  ${chalk.bold("Provider:")} ${data.provider}`);
  console.log(chalk.green("\u2500".repeat(40)));
}

// ---------------------------------------------------------------------------
// Cap-table registry entries
// ---------------------------------------------------------------------------

export const capTableCommands: CommandDef[] = [
  // --- cap-table (overview) ---
  {
    name: "cap-table",
    description: "Cap table, equity grants, SAFEs, transfers, and valuations",
    route: { method: "GET", path: "/v1/entities/{eid}/cap-table" },
    entity: true,
    display: {
      title: "Cap Table",
    },
    handler: async (ctx) => {
      const eid = await ctx.resolver.resolveEntity(ctx.opts.entityId as string | undefined);
      const data = await ctx.client.getCapTable(eid);
      const instruments = Array.isArray(data.instruments) ? data.instruments as ApiRecord[] : [];
      const shareClasses = Array.isArray(data.share_classes) ? data.share_classes as ApiRecord[] : [];
      await ctx.resolver.stabilizeRecords("instrument", instruments, eid);
      await ctx.resolver.stabilizeRecords("share_class", shareClasses, eid);
      if (ctx.opts.json) { ctx.writer.json(data); return; }
      if ((data.access_level as string) === "none") {
        ctx.writer.error("You do not have access to this entity's cap table.");
        process.exit(1);
      }
      printCapTable(data);
      try {
        const val = await ctx.client.getCurrent409a(eid);
        if (val) print409a(val);
      } catch { /* ignore */ }
    },
    examples: [
      "corp cap-table",
      'corp cap-table issue-equity --grant-type common --shares 1000000 --recipient "Alice Smith"',
      'corp cap-table issue-safe --investor "Seed Fund" --amount-cents 50000000 --valuation-cap-cents 1000000000',
      "corp cap-table create-valuation --type four_oh_nine_a --date 2026-01-01 --methodology market",
      "corp cap-table transfer --from alice --to bob --shares 1000 --share-class-id COMMON --governing-doc-type bylaws --transferee-rights full_member",
    ],
  },

  // --- cap-table safes ---
  {
    name: "cap-table safes",
    description: "SAFE notes",
    route: { method: "GET", path: "/v1/entities/{eid}/safe-notes" },
    entity: true,
    display: {
      title: "SAFE Notes",
      cols: ["investor_name|investor>Investor", "principal_amount_cents|investment_amount|amount>Amount", "valuation_cap_cents|valuation_cap|cap>Cap", "discount_rate|discount>Discount", "issued_at|date|created_at>Date"],
    },
    handler: async (ctx) => {
      const eid = await ctx.resolver.resolveEntity(ctx.opts.entityId as string | undefined);
      const safes = await ctx.client.getSafeNotes(eid);
      await ctx.resolver.stabilizeRecords("safe_note", safes, eid);
      if (ctx.opts.json) { ctx.writer.json(safes); return; }
      if (safes.length === 0) { ctx.writer.writeln("No SAFE notes found."); return; }
      printSafesTable(safes);
    },
  },

  // --- cap-table transfers ---
  {
    name: "cap-table transfers",
    description: "Share transfers",
    route: { method: "GET", path: "/v1/entities/{eid}/share-transfers" },
    entity: true,
    display: {
      title: "Share Transfers",
      cols: ["from_holder|from>From", "to_holder|to>To", "shares|share_count>Shares", "transfer_type>Type", "status>Status"],
    },
    handler: async (ctx) => {
      const eid = await ctx.resolver.resolveEntity(ctx.opts.entityId as string | undefined);
      const transfers = await ctx.client.getShareTransfers(eid);
      await ctx.resolver.stabilizeRecords("share_transfer", transfers, eid);
      if (ctx.opts.json) { ctx.writer.json(transfers); return; }
      if (transfers.length === 0) { ctx.writer.writeln("No share transfers found."); return; }
      printTransfersTable(transfers);
    },
  },

  // --- cap-table instruments ---
  {
    name: "cap-table instruments",
    description: "Cap table instruments",
    route: { method: "GET", path: "/v1/entities/{eid}/cap-table" },
    entity: true,
    display: {
      title: "Instruments",
      listKey: "instruments",
      cols: ["symbol>Symbol", "kind>Kind", "authorized_units>Authorized", "issued_units>Issued", "status>Status"],
    },
    handler: async (ctx) => {
      const eid = await ctx.resolver.resolveEntity(ctx.opts.entityId as string | undefined);
      const capTable = await ctx.client.getCapTable(eid);
      const instruments = Array.isArray(capTable.instruments) ? capTable.instruments as ApiRecord[] : [];
      await ctx.resolver.stabilizeRecords("instrument", instruments, eid);
      if (ctx.opts.json) { ctx.writer.json(instruments); return; }
      if (instruments.length === 0) { ctx.writer.writeln("No instruments found."); return; }
      printInstrumentsTable(instruments);
    },
  },

  // --- cap-table share-classes ---
  {
    name: "cap-table share-classes",
    description: "Share classes",
    route: { method: "GET", path: "/v1/entities/{eid}/cap-table" },
    entity: true,
    display: {
      title: "Share Classes",
      listKey: "share_classes",
      cols: ["class_code|name|share_class>Class", "authorized>Authorized", "outstanding>Outstanding"],
    },
    handler: async (ctx) => {
      const eid = await ctx.resolver.resolveEntity(ctx.opts.entityId as string | undefined);
      const capTable = await ctx.client.getCapTable(eid);
      const shareClasses = Array.isArray(capTable.share_classes) ? capTable.share_classes as ApiRecord[] : [];
      await ctx.resolver.stabilizeRecords("share_class", shareClasses, eid);
      if (ctx.opts.json) { ctx.writer.json(shareClasses); return; }
      if (shareClasses.length === 0) { ctx.writer.writeln("No share classes found."); return; }
      printShareClassesTable(shareClasses);
    },
  },

  // --- cap-table rounds ---
  {
    name: "cap-table rounds",
    description: "Staged equity rounds",
    route: { method: "GET", path: "/v1/entities/{eid}/equity-rounds" },
    entity: true,
    display: {
      title: "Equity Rounds",
      cols: ["name>Name", "status>Status", "issuer_legal_entity_id>Issuer", "@created_at>Created"],
    },
    handler: async (ctx) => {
      const eid = await ctx.resolver.resolveEntity(ctx.opts.entityId as string | undefined);
      const rounds = await ctx.client.listEquityRounds(eid);
      await ctx.resolver.stabilizeRecords("round", rounds, eid);
      if (ctx.opts.json) { ctx.writer.json(rounds); return; }
      if (rounds.length === 0) { ctx.writer.writeln("No rounds found."); return; }
      printRoundsTable(rounds);
    },
  },

  // --- cap-table valuations ---
  {
    name: "cap-table valuations",
    description: "Valuations history",
    route: { method: "GET", path: "/v1/entities/{eid}/valuations" },
    entity: true,
    display: {
      title: "Valuations",
      cols: ["@effective_date|valuation_date|date>Date", "valuation_type|type>Type", "enterprise_value_cents|enterprise_value|valuation>Valuation", "fmv_per_share_cents|price_per_share|pps|fmv_per_share>PPS"],
    },
    handler: async (ctx) => {
      const eid = await ctx.resolver.resolveEntity(ctx.opts.entityId as string | undefined);
      const valuations = await ctx.client.getValuations(eid);
      await ctx.resolver.stabilizeRecords("valuation", valuations, eid);
      if (ctx.opts.json) { ctx.writer.json(valuations); return; }
      if (valuations.length === 0) { ctx.writer.writeln("No valuations found."); return; }
      printValuationsTable(valuations);
    },
  },

  // --- cap-table 409a ---
  {
    name: "cap-table 409a",
    description: "Current 409A valuation",
    route: { method: "GET", path: "/v1/entities/{eid}/current-409a" },
    entity: true,
    display: { title: "409A Valuation" },
    handler: async (ctx) => {
      const eid = await ctx.resolver.resolveEntity(ctx.opts.entityId as string | undefined);
      try {
        const data = await ctx.client.getCurrent409a(eid);
        await ctx.resolver.stabilizeRecord("valuation", data, eid);
        if (ctx.opts.json) { ctx.writer.json(data); return; }
        if (!data || Object.keys(data).length === 0) { ctx.writer.writeln("No 409A valuation found."); return; }
        print409a(data);
      } catch (err) {
        const msg = String(err);
        if (msg.includes("404") || msg.includes("Not found") || msg.includes("not found")) {
          try {
            const valuations = await ctx.client.getValuations(eid);
            const pending409a = valuations
              .filter((valuation) => valuation.valuation_type === "four_oh_nine_a")
              .find((valuation) => valuation.status === "pending_approval");
            if (pending409a) {
              const effectiveDate = pending409a.effective_date ?? "unknown date";
              ctx.writer.writeln(
                `No current approved 409A valuation found. A 409A valuation is pending approval (${effectiveDate}).\n` +
                "  Complete board approval, then re-run: corp cap-table 409a",
              );
            } else {
              ctx.writer.writeln(
                "No 409A valuation found for this entity. Create one with:\n" +
                "  corp cap-table create-valuation --type four_oh_nine_a --date YYYY-MM-DD --methodology <method>",
              );
            }
          } catch {
            ctx.writer.writeln(
              "No 409A valuation found for this entity. Create one with:\n" +
              "  corp cap-table create-valuation --type four_oh_nine_a --date YYYY-MM-DD --methodology <method>",
            );
          }
        } else {
          ctx.writer.error(`Failed to fetch 409A valuation: ${err}`);
        }
        process.exit(1);
      }
    },
  },

  // --- cap-table control-map ---
  {
    name: "cap-table control-map",
    description: "View entity control/ownership map",
    route: { method: "GET", path: "/v1/entities/{eid}/control-map" },
    entity: "query",
    display: { title: "Control Map" },
    options: [
      { flags: "--root-entity-id <ref>", description: "Root entity for ownership tree (defaults to active entity)" },
    ],
    handler: async (ctx) => {
      const eid = await ctx.resolver.resolveEntity(ctx.opts.entityId as string | undefined);
      const rootEntityId = (ctx.opts.rootEntityId as string | undefined)
        ? await ctx.resolver.resolveEntity(ctx.opts.rootEntityId as string)
        : eid;

      let result: ApiRecord;
      try {
        result = await ctx.client.getControlMap(eid, rootEntityId);
      } catch (firstErr) {
        const msg = String(firstErr);
        if (msg.includes("404") && !ctx.opts.rootEntityId) {
          try {
            const capTable = await ctx.client.getCapTable(eid);
            const issuerLegalEntityId = capTable.issuer_legal_entity_id as string | undefined;
            if (issuerLegalEntityId && issuerLegalEntityId !== eid) {
              result = await ctx.client.getControlMap(eid, issuerLegalEntityId);
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
      if (ctx.opts.json) { ctx.writer.json(result); return; }
      ctx.writer.json(result);
    },
  },

  // --- cap-table dilution ---
  {
    name: "cap-table dilution",
    description: "Preview dilution impact of a round",
    route: { method: "GET", path: "/v1/entities/{eid}/dilution-preview" },
    entity: "query",
    display: { title: "Dilution Preview" },
    options: [
      { flags: "--round-id <ref>", description: "Round reference to model dilution for", required: true },
    ],
    handler: async (ctx) => {
      const eid = await ctx.resolver.resolveEntity(ctx.opts.entityId as string | undefined);
      const roundId = await ctx.resolver.resolveRound(eid, ctx.opts.roundId as string);
      const result = await ctx.client.getDilutionPreview(eid, roundId);
      if (ctx.opts.json) { ctx.writer.json(result); return; }
      if (result.round_status === "closed" || result.round_status === "issued") {
        console.log(chalk.yellow("Note: This round is already closed. Dilution preview reflects the finalized state, not a scenario model."));
        console.log(chalk.dim("  For scenario modeling, create a new round with: corp cap-table start-round --name '...' --issuer-legal-entity-id '...'"));
      }
      ctx.writer.json(result);
    },
  },

  // --- cap-table create-instrument ---
  {
    name: "cap-table create-instrument",
    description: "Create a cap table instrument",
    route: { method: "POST", path: "/v1/entities/{eid}/instruments" },
    entity: true,
    dryRun: true,
    options: [
      { flags: "--kind <kind>", description: "Instrument kind (common_equity, preferred_equity, membership_unit, option_grant, safe)", required: true },
      { flags: "--symbol <symbol>", description: "Instrument symbol", required: true },
      { flags: "--issuer-legal-entity-id <ref>", description: "Issuer legal entity reference (ID, short ID, @last, or unique name)" },
      { flags: "--authorized-units <n>", description: "Authorized units", type: "int" },
      { flags: "--issue-price-cents <n>", description: "Issue price in cents", type: "int" },
      { flags: "--terms-json <json>", description: "JSON object of instrument terms" },
    ],
    handler: async (ctx) => {
      const eid = await ctx.resolver.resolveEntity(ctx.opts.entityId as string | undefined);
      let issuerLegalEntityId = ctx.opts.issuerLegalEntityId as string | undefined;
      if (!issuerLegalEntityId) {
        const capTable = await ctx.client.getCapTable(eid);
        issuerLegalEntityId = capTable.issuer_legal_entity_id as string | undefined;
      }
      if (!issuerLegalEntityId) {
        throw new Error("No issuer legal entity found. Has this entity been formed with a cap table?");
      }
      issuerLegalEntityId = await ctx.resolver.resolveEntity(issuerLegalEntityId);

      const terms = (ctx.opts.termsJson as string | undefined)
        ? JSON.parse(ctx.opts.termsJson as string) as Record<string, unknown>
        : {};
      const payload: Record<string, unknown> = {
        entity_id: eid,
        issuer_legal_entity_id: issuerLegalEntityId,
        kind: ctx.opts.kind as string,
        symbol: ctx.opts.symbol as string,
        terms,
      };
      if (ctx.opts.authorizedUnits != null) payload.authorized_units = ctx.opts.authorizedUnits;
      if (ctx.opts.issuePriceCents != null) payload.issue_price_cents = ctx.opts.issuePriceCents;
      if (ctx.dryRun) {
        ctx.writer.dryRun("cap_table.create_instrument", payload);
        return;
      }
      const result = await ctx.client.createInstrument(payload);
      await ctx.resolver.stabilizeRecord("instrument", result, eid);
      ctx.resolver.rememberFromRecord("instrument", result, eid);
      if (ctx.opts.json) { ctx.writer.json(result); return; }
      ctx.writer.success(`Instrument created: ${result.instrument_id ?? "OK"}`);
      printReferenceSummary("instrument", result, { showReuseHint: true });
    },
    produces: { kind: "instrument" },
    successTemplate: "Instrument created: {symbol}",
  },

  // --- cap-table issue-equity ---
  {
    name: "cap-table issue-equity",
    description: "Issue an equity grant (creates a round, adds security, and issues it)",
    route: { method: "POST", path: "/v1/entities/{eid}/equity-rounds" },
    entity: true,
    dryRun: true,
    options: [
      { flags: "--grant-type <type>", description: "Grant type (common, preferred, membership_unit, stock_option, iso, nso, rsa)", required: true },
      { flags: "--shares <n>", description: "Number of shares", required: true, type: "int" },
      { flags: "--recipient <name>", description: "Recipient name", required: true },
      { flags: "--email <email>", description: "Recipient email (auto-creates contact if needed)" },
      { flags: "--instrument-id <ref>", description: "Instrument reference (ID, short ID, symbol, or @last)" },
      { flags: "--meeting-id <ref>", description: "Board meeting reference required when a board approval already exists or is being recorded" },
      { flags: "--resolution-id <ref>", description: "Board resolution reference required when issuing under a board-governed entity" },
    ],
    handler: async (ctx) => {
      const eid = await ctx.resolver.resolveEntity(ctx.opts.entityId as string | undefined);
      const grantType = ctx.opts.grantType as string;
      const shares = ctx.opts.shares as number;
      const recipient = ctx.opts.recipient as string;
      const email = ctx.opts.email as string | undefined;
      const optInstrumentId = ctx.opts.instrumentId as string | undefined;
      const optMeetingId = ctx.opts.meetingId as string | undefined;
      const optResolutionId = ctx.opts.resolutionId as string | undefined;

      if (ctx.dryRun) {
        ctx.writer.dryRun("cap_table.issue_equity", {
          entity_id: eid,
          grant_type: grantType,
          shares,
          recipient,
          email,
          instrument_id: optInstrumentId,
          meeting_id: optMeetingId,
          resolution_id: optResolutionId,
        });
        return;
      }

      // Resolve references before passing to workflow
      const instrumentId = optInstrumentId
        ? await ctx.resolver.resolveInstrument(eid, optInstrumentId)
        : undefined;
      const meetingId = optMeetingId ? await ctx.resolver.resolveMeeting(eid, optMeetingId) : undefined;
      const resolutionId = optResolutionId
        ? await ctx.resolver.resolveResolution(eid, optResolutionId, meetingId)
        : undefined;

      const result = await issueEquity(ctx.client, {
        entityId: eid,
        grantType,
        shares,
        recipientName: recipient,
        recipientEmail: email,
        instrumentId,
        meetingId,
        resolutionId,
      });

      if (!result.success) {
        ctx.writer.error(result.error!);
        return;
      }

      // Track references for the created round
      const round = result.data?.round as Record<string, unknown> | undefined;
      if (round) {
        await ctx.resolver.stabilizeRecord("round", round, eid);
        ctx.resolver.rememberFromRecord("round", round, eid);
      }

      // Show instrument selection detail
      const instrStep = result.steps.find((s) => s.name === "resolve_instrument");
      if (instrStep && !optInstrumentId) {
        console.log(instrStep.detail);
      }

      if (ctx.opts.json) { ctx.writer.json(result.data); return; }
      ctx.writer.success(`Equity issued: ${shares} shares (${grantType}) to ${recipient}`);
      if (round) {
        printReferenceSummary("round", round, { label: "Round Ref:", showReuseHint: true });
      }
    },
    produces: { kind: "round" },
    successTemplate: "Equity issued: {round_name}",
  },

  // --- cap-table issue-safe ---
  {
    name: "cap-table issue-safe",
    description: "Issue a SAFE note",
    route: { method: "POST", path: "/v1/entities/{eid}/safe-notes" },
    entity: true,
    dryRun: true,
    options: [
      { flags: "--investor <name>", description: "Investor name", required: true },
      { flags: "--amount-cents <n>", description: "Principal amount in cents (e.g. 5000000000 = $50M)", required: true, type: "int" },
      { flags: "--amount <n>", description: "", type: "int" },
      { flags: "--safe-type <type>", description: "SAFE type", default: "post_money" },
      { flags: "--valuation-cap-cents <n>", description: "Valuation cap in cents (e.g. 1000000000 = $10M)", required: true, type: "int" },
      { flags: "--valuation-cap <n>", description: "", type: "int" },
      { flags: "--meeting-id <ref>", description: "Board meeting reference required when issuing under a board-governed entity" },
      { flags: "--resolution-id <ref>", description: "Board resolution reference required when issuing under a board-governed entity" },
    ],
    handler: async (ctx) => {
      const eid = await ctx.resolver.resolveEntity(ctx.opts.entityId as string | undefined);
      const investor = ctx.opts.investor as string;
      const amountCents = (ctx.opts.amountCents ?? ctx.opts.amount) as number;
      const safeType = (ctx.opts.safeType ?? "post_money") as string;
      const valuationCapCents = (ctx.opts.valuationCapCents ?? ctx.opts.valuationCap) as number;
      const email = ctx.opts.email as string | undefined;
      const optMeetingId = ctx.opts.meetingId as string | undefined;
      const optResolutionId = ctx.opts.resolutionId as string | undefined;

      if (ctx.dryRun) {
        ctx.writer.dryRun("cap_table.issue_safe", {
          entity_id: eid,
          investor,
          amount_cents: amountCents,
          safe_type: safeType,
          valuation_cap_cents: valuationCapCents,
          email,
          meeting_id: optMeetingId,
          resolution_id: optResolutionId,
        });
        return;
      }

      // Resolve references before passing to workflow
      const meetingId = optMeetingId ? await ctx.resolver.resolveMeeting(eid, optMeetingId) : undefined;
      const resolutionId = optResolutionId
        ? await ctx.resolver.resolveResolution(eid, optResolutionId, meetingId)
        : undefined;

      const result = await issueSafe(ctx.client, {
        entityId: eid,
        investorName: investor,
        amountCents,
        valuationCapCents,
        safeType,
        email,
        meetingId,
        resolutionId,
      });

      if (!result.success) {
        ctx.writer.error(result.error!);
        return;
      }

      await ctx.resolver.stabilizeRecord("safe_note", result.data!, eid);
      ctx.resolver.rememberFromRecord("safe_note", result.data!, eid);
      if (ctx.opts.json) { ctx.writer.json(result.data); return; }
      ctx.writer.success(`SAFE issued: $${(amountCents / 100).toLocaleString()} to ${investor}`);
      printReferenceSummary("safe_note", result.data!, { showReuseHint: true });
    },
    produces: { kind: "safe_note" },
    successTemplate: "SAFE created: {investor_name}",
  },

  // --- cap-table transfer ---
  {
    name: "cap-table transfer",
    description: "Create a share transfer workflow",
    route: { method: "POST", path: "/v1/entities/{eid}/share-transfers" },
    entity: true,
    dryRun: true,
    options: [
      { flags: "--from <ref>", description: "Source contact reference (from_contact_id)", required: true },
      { flags: "--to <ref>", description: "Destination contact reference (to_contact_id)", required: true },
      { flags: "--shares <n>", description: "Number of shares to transfer", required: true, type: "int" },
      { flags: "--share-class-id <ref>", description: "Share class reference", required: true },
      { flags: "--governing-doc-type <type>", description: "Governing doc type (bylaws, operating_agreement, shareholder_agreement, other)", required: true },
      { flags: "--transferee-rights <rights>", description: "Transferee rights (full_member, economic_only, limited)", required: true },
      { flags: "--prepare-intent-id <id>", description: "Prepare intent ID (auto-created if omitted)" },
      { flags: "--type <type>", description: "Transfer type (gift, trust_transfer, secondary_sale, estate, other)", default: "secondary_sale" },
      { flags: "--price-per-share-cents <n>", description: "Price per share in cents", type: "int" },
      { flags: "--relationship <rel>", description: "Relationship to holder" },
    ],
    handler: async (ctx) => {
      const eid = await ctx.resolver.resolveEntity(ctx.opts.entityId as string | undefined);
      const fromContactId = await ctx.resolver.resolveContact(eid, ctx.opts.from as string);
      const toContactId = await ctx.resolver.resolveContact(eid, ctx.opts.to as string);
      const shareClassId = await ctx.resolver.resolveShareClass(eid, ctx.opts.shareClassId as string);
      const shares = ctx.opts.shares as number;
      const pricePerShareCents = ctx.opts.pricePerShareCents as number | undefined;
      const relationship = ctx.opts.relationship as string | undefined;
      const transferType = (ctx.opts.type ?? "secondary_sale") as string;
      const prepareIntentId = ctx.opts.prepareIntentId as string | undefined;

      if (pricePerShareCents != null && pricePerShareCents < 0) {
        throw new Error("price-per-share-cents cannot be negative");
      }
      if (fromContactId === toContactId) {
        throw new Error("--from and --to must be different contacts");
      }
      if (ctx.dryRun) {
        ctx.writer.dryRun("cap_table.transfer_shares", {
          entity_id: eid,
          from_contact_id: fromContactId,
          to_contact_id: toContactId,
          share_count: shares,
          transfer_type: transferType,
          share_class_id: shareClassId,
          governing_doc_type: ctx.opts.governingDocType as string,
          transferee_rights: ctx.opts.transfereeRights as string,
          prepare_intent_id: prepareIntentId,
          price_per_share_cents: pricePerShareCents,
          relationship_to_holder: relationship,
        });
        return;
      }

      let intentId = prepareIntentId;
      if (!intentId) {
        const intent = await ctx.client.createExecutionIntent({
          entity_id: eid,
          intent_type: "equity.transfer.prepare",
          description: `Transfer ${shares} shares from ${fromContactId} to ${toContactId}`,
        });
        intentId = (intent.intent_id ?? intent.id) as string;
        await ctx.client.evaluateIntent(intentId, eid);
        await ctx.client.authorizeIntent(intentId, eid);
      }
      const body: Record<string, unknown> = {
        entity_id: eid,
        share_class_id: shareClassId,
        from_contact_id: fromContactId,
        to_contact_id: toContactId,
        transfer_type: transferType,
        share_count: shares,
        governing_doc_type: ctx.opts.governingDocType as string,
        transferee_rights: ctx.opts.transfereeRights as string,
        prepare_intent_id: intentId,
      };
      if (pricePerShareCents != null) body.price_per_share_cents = pricePerShareCents;
      if (relationship) body.relationship_to_holder = relationship;
      const result = await ctx.client.transferShares(body);
      await ctx.resolver.stabilizeRecord("share_transfer", result, eid);
      ctx.resolver.rememberFromRecord("share_transfer", result, eid);
      if (ctx.opts.json) { ctx.writer.json(result); return; }
      ctx.writer.success(`Transfer workflow created: ${result.transfer_workflow_id ?? "OK"}`);
      printReferenceSummary("share_transfer", result, { label: "Transfer Ref:", showReuseHint: true });
    },
    produces: { kind: "share_transfer" },
    successTemplate: "Transfer created",
  },

  // --- cap-table distribute ---
  {
    name: "cap-table distribute",
    description: "Calculate a distribution",
    route: { method: "POST", path: "/v1/entities/{eid}/distributions" },
    entity: true,
    dryRun: true,
    options: [
      { flags: "--amount-cents <n>", description: "Total distribution amount in cents (e.g. 100000 = $1,000.00)", required: true, type: "int" },
      { flags: "--amount <n>", description: "", type: "int" },
      { flags: "--type <type>", description: "Distribution type (dividend, return, liquidation)", default: "dividend" },
      { flags: "--description <desc>", description: "Distribution description", required: true },
    ],
    handler: async (ctx) => {
      const eid = await ctx.resolver.resolveEntity(ctx.opts.entityId as string | undefined);
      const amountCents = (ctx.opts.amountCents ?? ctx.opts.amount) as number;
      const distributionType = (ctx.opts.type ?? "dividend") as string;
      const description = ctx.opts.description as string;
      const payload = {
        entity_id: eid,
        total_amount_cents: amountCents,
        distribution_type: distributionType,
        description,
      };
      if (ctx.dryRun) {
        ctx.writer.dryRun("cap_table.distribute", payload);
        return;
      }
      const result = await ctx.client.calculateDistribution(payload);
      await ctx.resolver.stabilizeRecord("distribution", result, eid);
      ctx.resolver.rememberFromRecord("distribution", result, eid);
      if (ctx.opts.json) { ctx.writer.json(result); return; }
      ctx.writer.success(`Distribution calculated: ${result.distribution_id ?? "OK"}`);
      printReferenceSummary("distribution", result, { showReuseHint: true });
    },
  },

  // --- cap-table start-round ---
  {
    name: "cap-table start-round",
    description: "Start a staged equity round",
    route: { method: "POST", path: "/v1/entities/{eid}/equity-rounds" },
    entity: true,
    dryRun: true,
    options: [
      { flags: "--name <name>", description: "Round name", required: true },
      { flags: "--issuer-legal-entity-id <ref>", description: "Issuer legal entity reference", required: true },
    ],
    handler: async (ctx) => {
      const eid = await ctx.resolver.resolveEntity(ctx.opts.entityId as string | undefined);
      const issuerLegalEntityId = await ctx.resolver.resolveEntity(ctx.opts.issuerLegalEntityId as string);
      const payload = {
        entity_id: eid,
        name: ctx.opts.name as string,
        issuer_legal_entity_id: issuerLegalEntityId,
      };
      if (ctx.dryRun) {
        ctx.writer.dryRun("cap_table.start_round", payload);
        return;
      }
      const result = await ctx.client.startEquityRound(payload);
      await ctx.resolver.stabilizeRecord("round", result, eid);
      ctx.resolver.rememberFromRecord("round", result, eid);
      if (ctx.opts.json) { ctx.writer.json(result); return; }
      ctx.writer.success(`Round started: ${result.round_id ?? "OK"}`);
      printReferenceSummary("round", result, { showReuseHint: true });
    },
    produces: { kind: "round" },
    successTemplate: "Round started: {round_name}",
  },

  // --- cap-table add-security ---
  {
    name: "cap-table add-security",
    description: "Add a security to a staged equity round",
    route: { method: "POST", path: "/v1/equity-rounds/{pos}/securities" },
    entity: true,
    dryRun: true,
    options: [
      { flags: "--round-id <ref>", description: "Round reference", required: true },
      { flags: "--instrument-id <ref>", description: "Instrument reference", required: true },
      { flags: "--quantity <n>", description: "Number of shares/units", required: true, type: "int" },
      { flags: "--recipient-name <name>", description: "Recipient display name", required: true },
      { flags: "--holder-id <ref>", description: "Existing holder reference" },
      { flags: "--email <email>", description: "Recipient email (to find or create holder)" },
      { flags: "--principal-cents <n>", description: "Principal amount in cents", type: "int" },
      { flags: "--grant-type <type>", description: "Grant type" },
    ],
    handler: async (ctx) => {
      const eid = await ctx.resolver.resolveEntity(ctx.opts.entityId as string | undefined);
      const roundId = await ctx.resolver.resolveRound(eid, ctx.opts.roundId as string);
      const instrumentId = await ctx.resolver.resolveInstrument(eid, ctx.opts.instrumentId as string);
      const body: Record<string, unknown> = {
        entity_id: eid,
        instrument_id: instrumentId,
        quantity: ctx.opts.quantity as number,
        recipient_name: ctx.opts.recipientName as string,
      };
      if (ctx.opts.holderId) body.holder_id = await ctx.resolver.resolveContact(eid, ctx.opts.holderId as string);
      if (ctx.opts.email) body.email = ctx.opts.email as string;
      if (ctx.opts.principalCents) body.principal_cents = ctx.opts.principalCents as number;
      if (ctx.opts.grantType) body.grant_type = ctx.opts.grantType as string;
      if (ctx.dryRun) {
        ctx.writer.dryRun("cap_table.add_security", { round_id: roundId, ...body });
        return;
      }
      const result = await ctx.client.addRoundSecurity(roundId, body);
      if (ctx.opts.json) { ctx.writer.json(result); return; }
      ctx.writer.success(`Security added for ${ctx.opts.recipientName}`);
    },
  },

  // --- cap-table issue-round ---
  {
    name: "cap-table issue-round",
    description: "Issue all securities and close a staged round",
    route: { method: "POST", path: "/v1/equity-rounds/{pos}/issue" },
    entity: true,
    dryRun: true,
    options: [
      { flags: "--round-id <ref>", description: "Round reference", required: true },
      { flags: "--meeting-id <ref>", description: "Board meeting reference required when issuing under a board-governed entity" },
      { flags: "--resolution-id <ref>", description: "Board resolution reference required when issuing under a board-governed entity" },
    ],
    handler: async (ctx) => {
      const eid = await ctx.resolver.resolveEntity(ctx.opts.entityId as string | undefined);
      const roundId = await ctx.resolver.resolveRound(eid, ctx.opts.roundId as string);
      const meetingId = (ctx.opts.meetingId as string | undefined)
        ? await ctx.resolver.resolveMeeting(eid, ctx.opts.meetingId as string)
        : undefined;
      const resolutionId = (ctx.opts.resolutionId as string | undefined)
        ? await ctx.resolver.resolveResolution(eid, ctx.opts.resolutionId as string, meetingId)
        : undefined;
      if (ctx.dryRun) {
        ctx.writer.dryRun("cap_table.issue_round", {
          entity_id: eid,
          round_id: roundId,
          meeting_id: meetingId,
          resolution_id: resolutionId,
        });
        return;
      }
      if ((!meetingId || !resolutionId) && await entityHasActiveBoard(ctx.client, eid)) {
        throw new Error(
          "Board approval is required before issuing this round. Pass --meeting-id and --resolution-id from a passed board vote.",
        );
      }
      const body: Record<string, unknown> = { entity_id: eid };
      if (meetingId) body.meeting_id = meetingId;
      if (resolutionId) body.resolution_id = resolutionId;
      const result = await ctx.client.issueRound(roundId, body);
      ctx.resolver.remember("round", roundId, eid);
      const roundMatch = (await ctx.resolver.find("round", shortId(roundId), { entityId: eid }))
        .find((match) => match.id === roundId);
      if (ctx.opts.json) { ctx.writer.json(result); return; }
      ctx.writer.success("Round issued and closed");
      if (roundMatch) {
        printReferenceSummary("round", roundMatch.raw, { showReuseHint: true });
      }
    },
  },

  // --- cap-table create-valuation ---
  {
    name: "cap-table create-valuation",
    description: "Create a valuation",
    route: { method: "POST", path: "/v1/entities/{eid}/valuations" },
    entity: true,
    dryRun: true,
    options: [
      { flags: "--type <type>", description: "Valuation type (four_oh_nine_a, fair_market_value, etc.)", required: true },
      { flags: "--date <date>", description: "Effective date (ISO 8601)", required: true },
      { flags: "--methodology <method>", description: "Methodology (income, market, asset, backsolve, hybrid)", required: true },
      { flags: "--fmv <cents>", description: "FMV per share in cents", type: "int" },
      { flags: "--enterprise-value <cents>", description: "Enterprise value in cents", type: "int" },
    ],
    handler: async (ctx) => {
      const eid = await ctx.resolver.resolveEntity(ctx.opts.entityId as string | undefined);
      const body: Record<string, unknown> = {
        entity_id: eid,
        valuation_type: ctx.opts.type as string,
        effective_date: ctx.opts.date as string,
        methodology: ctx.opts.methodology as string,
      };
      if (ctx.opts.fmv != null) body.fmv_per_share_cents = ctx.opts.fmv;
      if (ctx.opts.enterpriseValue != null) body.enterprise_value_cents = ctx.opts.enterpriseValue;
      if (ctx.dryRun) {
        ctx.writer.dryRun("cap_table.create_valuation", body);
        return;
      }
      const result = await ctx.client.createValuation(body);
      await ctx.resolver.stabilizeRecord("valuation", result, eid);
      ctx.resolver.rememberFromRecord("valuation", result, eid);
      if (ctx.opts.json) { ctx.writer.json(result); return; }
      ctx.writer.success(`Valuation created: ${result.valuation_id ?? "OK"}`);
      printReferenceSummary("valuation", result, { showReuseHint: true });
    },
    produces: { kind: "valuation" },
    successTemplate: "Valuation created",
  },

  // --- cap-table submit-valuation <valuation-ref> ---
  {
    name: "cap-table submit-valuation",
    description: "Submit a valuation for board approval",
    route: { method: "POST", path: "/v1/valuations/{pos}/submit" },
    entity: true,
    dryRun: true,
    args: [{ name: "valuation-ref", required: true, description: "Valuation reference" }],
    handler: async (ctx) => {
      const valuationRef = ctx.positional[0];
      const eid = await ctx.resolver.resolveEntity(ctx.opts.entityId as string | undefined);
      const valuationId = await ctx.resolver.resolveValuation(eid, valuationRef);
      if (ctx.dryRun) {
        ctx.writer.dryRun("cap_table.submit_valuation", { entity_id: eid, valuation_id: valuationId });
        return;
      }
      try {
        const result = await ctx.client.submitValuationForApproval(valuationId, eid);
        await ctx.resolver.stabilizeRecord("valuation", result, eid);
        ctx.resolver.remember("valuation", valuationId, eid);
        if (result.meeting_id) ctx.resolver.remember("meeting", String(result.meeting_id), eid);
        if (result.agenda_item_id) ctx.resolver.remember("agenda_item", String(result.agenda_item_id), eid);
        if (ctx.opts.json) { ctx.writer.json(result); return; }
        ctx.writer.success(`Valuation submitted for approval: ${result.valuation_id ?? valuationId ?? "OK"}`);
        printReferenceSummary("valuation", result, { showReuseHint: true });
        if (result.meeting_id) {
          const meetingMatch = (await ctx.resolver.find("meeting", shortId(String(result.meeting_id)), { entityId: eid }))
            .find((match) => match.id === String(result.meeting_id));
          if (meetingMatch) {
            printReferenceSummary("meeting", meetingMatch.raw, { label: "Meeting Ref:" });
          } else {
            printReferenceSummary("meeting", { meeting_id: result.meeting_id }, { label: "Meeting Ref:" });
          }
        }
        if (result.agenda_item_id) {
          const agendaMatch = (await ctx.resolver.find("agenda_item", shortId(String(result.agenda_item_id)), {
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
        if (msg.includes("404") || msg.includes("Not found") || msg.includes("not found")) {
          ctx.writer.error(`Valuation not found. List valuations with: corp cap-table valuations`);
        } else {
          ctx.writer.error(`Failed to submit valuation: ${err}`);
        }
        process.exit(1);
      }
    },
  },

  // --- cap-table approve-valuation <valuation-ref> ---
  {
    name: "cap-table approve-valuation",
    description: "Approve a valuation",
    route: { method: "POST", path: "/v1/valuations/{pos}/approve" },
    entity: true,
    dryRun: true,
    args: [{ name: "valuation-ref", required: true, description: "Valuation reference" }],
    options: [
      { flags: "--resolution-id <ref>", description: "Resolution reference from the board vote" },
    ],
    handler: async (ctx) => {
      const valuationRef = ctx.positional[0];
      const eid = await ctx.resolver.resolveEntity(ctx.opts.entityId as string | undefined);
      const valuationId = await ctx.resolver.resolveValuation(eid, valuationRef);
      const resolutionId = (ctx.opts.resolutionId as string | undefined)
        ? await ctx.resolver.resolveResolution(eid, ctx.opts.resolutionId as string)
        : undefined;
      if (ctx.dryRun) {
        ctx.writer.dryRun("cap_table.approve_valuation", {
          entity_id: eid,
          valuation_id: valuationId,
          resolution_id: resolutionId,
        });
        return;
      }
      try {
        const result = await ctx.client.approveValuation(valuationId, eid, resolutionId);
        await ctx.resolver.stabilizeRecord("valuation", result, eid);
        if (ctx.opts.json) { ctx.writer.json(result); return; }
        ctx.writer.success(`Valuation approved: ${result.valuation_id ?? valuationId ?? "OK"}`);
        printReferenceSummary("valuation", result);
      } catch (err) {
        const msg = String(err);
        if (msg.includes("400")) {
          ctx.writer.error(`Bad request \u2014 a --resolution-id from a board vote may be required. Submit for approval first: corp cap-table submit-valuation <valuation-ref>`);
        } else {
          ctx.writer.error(`Failed to approve valuation: ${err}`);
        }
        process.exit(1);
      }
    },
  },

  // --- cap-table preview-conversion ---
  {
    name: "cap-table preview-conversion",
    description: "Preview SAFE-to-equity conversion",
    route: { method: "GET", path: "/v1/entities/{eid}/safe-conversion-preview" },
    entity: true,
    options: [
      { flags: "--safe-id <ref>", description: "SAFE note reference to convert", required: true },
      { flags: "--price-per-share-cents <n>", description: "Conversion price per share in cents", required: true, type: "int" },
    ],
    display: { title: "Conversion Preview" },
    handler: async (ctx) => {
      const eid = await ctx.resolver.resolveEntity(ctx.opts.entityId as string | undefined);
      const safeId = await ctx.resolver.resolveSafeNote(eid, ctx.opts.safeId as string);
      const result = await ctx.client.previewRoundConversion({
        entity_id: eid,
        safe_note_id: safeId,
        price_per_share_cents: ctx.opts.pricePerShareCents as number,
      } as unknown as Parameters<typeof ctx.client.previewRoundConversion>[0]);
      if (ctx.opts.json) { ctx.writer.json(result); return; }
      ctx.writer.success("Conversion Preview:");
      if (result.shares_issued) console.log(`  Shares to issue: ${result.shares_issued}`);
      if (result.ownership_pct) console.log(`  Post-conversion ownership: ${result.ownership_pct}%`);
      ctx.writer.json(result);
    },
  },

  // --- cap-table convert ---
  {
    name: "cap-table convert",
    description: "Execute SAFE-to-equity conversion",
    route: { method: "POST", path: "/v1/entities/{eid}/safe-conversions" },
    entity: true,
    dryRun: true,
    options: [
      { flags: "--safe-id <ref>", description: "SAFE note reference to convert", required: true },
      { flags: "--price-per-share-cents <n>", description: "Conversion price per share in cents", required: true, type: "int" },
    ],
    handler: async (ctx) => {
      const eid = await ctx.resolver.resolveEntity(ctx.opts.entityId as string | undefined);
      const safeId = await ctx.resolver.resolveSafeNote(eid, ctx.opts.safeId as string);
      const payload = {
        entity_id: eid,
        safe_note_id: safeId,
        price_per_share_cents: ctx.opts.pricePerShareCents as number,
      };
      if (ctx.dryRun) {
        ctx.writer.dryRun("equity.conversion.execute", payload);
        return;
      }
      const result = await ctx.client.executeRoundConversion(
        payload as unknown as Parameters<typeof ctx.client.executeRoundConversion>[0],
      );
      if (ctx.opts.json) { ctx.writer.json(result); return; }
      ctx.writer.success(`Conversion executed for SAFE ${safeId}`);
    },
  },
];
