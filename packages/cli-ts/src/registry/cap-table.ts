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

  // ── Auto-generated from OpenAPI ──────────────────────────────
  {
    name: "equity control-links",
    description: "/v1/equity/control-links",
    route: { method: "POST", path: "/v1/equity/control-links" },
    options: [
      { flags: "--child-legal-entity-id <child-legal-entity-id>", description: "Child Legal Entity Id", required: true },
      { flags: "--control-type <control-type>", description: "Type of control relationship.", required: true, choices: ["voting", "board", "economic", "contractual"] },
      { flags: "--notes <notes>", description: "Notes" },
      { flags: "--parent-legal-entity-id <parent-legal-entity-id>", description: "Parent Legal Entity Id", required: true },
      { flags: "--voting-power-bps <voting-power-bps>", description: "Voting Power Bps" },
    ],
  },
  {
    name: "equity control-map",
    description: "/v1/equity/control-map",
    route: { method: "GET", path: "/v1/equity/control-map" },
    entity: true,
    display: { title: "Equity Control Map", cols: ["edges>Edges", "#root_entity_id>ID", "traversed_entities>Traversed Entities"] },
  },
  {
    name: "equity conversions-execute",
    description: "/v1/equity/conversions/execute",
    route: { method: "POST", path: "/v1/equity/conversions/execute" },
    options: [
      { flags: "--intent-id <intent-id>", description: "Intent Id", required: true },
      { flags: "--round-id <round-id>", description: "Round Id", required: true },
      { flags: "--source-reference <source-reference>", description: "Source Reference" },
    ],
  },
  {
    name: "equity conversions-preview",
    description: "/v1/equity/conversions/preview",
    route: { method: "POST", path: "/v1/equity/conversions/preview" },
    options: [
      { flags: "--round-id <round-id>", description: "Round Id", required: true },
      { flags: "--source-reference <source-reference>", description: "Source Reference" },
    ],
  },
  {
    name: "equity dilution-preview",
    description: "/v1/equity/dilution/preview",
    route: { method: "GET", path: "/v1/equity/dilution/preview" },
    entity: true,
    display: { title: "Equity Dilution Preview", cols: ["#issuer_legal_entity_id>ID", "pre_round_outstanding_units>Pre Round Outstanding Units", "projected_dilution_bps>Projected Dilution Bps", "projected_new_units>Projected New Units", "projected_post_outstanding_units>Projected Post Outstanding Units", "#round_id>ID"] },
  },
  {
    name: "equity entities",
    description: "/v1/equity/entities",
    route: { method: "POST", path: "/v1/equity/entities" },
    options: [
      { flags: "--linked-entity-id <linked-entity-id>", description: "Linked Entity Id" },
      { flags: "--name <name>", description: "Name", required: true },
      { flags: "--role <role>", description: "Role this legal entity plays in the ownership/control graph.", required: true, choices: ["operating", "control", "investment", "nonprofit", "spv", "other"] },
    ],
  },
  {
    name: "equity fundraising-workflows",
    description: "/v1/equity/fundraising-workflows",
    route: { method: "POST", path: "/v1/equity/fundraising-workflows" },
    options: [
      { flags: "--conversion-target-instrument-id <conversion-target-instrument-id>", description: "Conversion Target Instrument Id" },
      { flags: "--issuer-legal-entity-id <issuer-legal-entity-id>", description: "Issuer Legal Entity Id", required: true },
      { flags: "--metadata <metadata>", description: "Metadata" },
      { flags: "--name <name>", description: "Name", required: true },
      { flags: "--pre-money-cents <pre-money-cents>", description: "Pre Money Cents" },
      { flags: "--prepare-intent-id <prepare-intent-id>", description: "Prepare Intent Id", required: true },
      { flags: "--round-price-cents <round-price-cents>", description: "Round Price Cents" },
      { flags: "--target-raise-cents <target-raise-cents>", description: "Target Raise Cents" },
    ],
  },
  {
    name: "equity fundraising-workflows",
    description: "/v1/equity/fundraising-workflows/{workflow_id}",
    route: { method: "GET", path: "/v1/equity/fundraising-workflows/{pos}" },
    entity: true,
    args: [{ name: "workflow-id", required: true, description: "Workflow Id" }],
    display: { title: "Equity Fundraising Workflows", cols: ["#accept_intent_id>ID", "#active_packet_id>ID", "#board_approval_meeting_id>ID", "#board_approval_resolution_id>ID", "board_packet_documents>Board Packet Documents", "#close_intent_id>ID", "closing_packet_documents>Closing Packet Documents", "@created_at>Created At"] },
  },
  {
    name: "equity fundraising-workflows-apply-terms",
    description: "/v1/equity/fundraising-workflows/{workflow_id}/apply-terms",
    route: { method: "POST", path: "/v1/equity/fundraising-workflows/{pos}/apply-terms" },
    args: [{ name: "workflow-id", required: true, description: "Workflow Id" }],
    options: [
      { flags: "--anti-dilution-method <anti-dilution-method>", description: "Anti Dilution Method", required: true, choices: ["none", "broad_based_weighted_average", "narrow_based_weighted_average", "full_ratchet"] },
      { flags: "--conversion-precedence <conversion-precedence>", description: "Conversion Precedence", type: "array" },
      { flags: "--protective-provisions <protective-provisions>", description: "Protective Provisions" },
    ],
  },
  {
    name: "equity fundraising-workflows-compile-packet",
    description: "/v1/equity/fundraising-workflows/{workflow_id}/compile-packet",
    route: { method: "POST", path: "/v1/equity/fundraising-workflows/{pos}/compile-packet" },
    args: [{ name: "workflow-id", required: true, description: "Workflow Id" }],
    options: [
      { flags: "--phase <phase>", description: "Phase" },
      { flags: "--required-signers <required-signers>", description: "Required Signers", type: "array" },
    ],
  },
  {
    name: "equity fundraising-workflows-finalize",
    description: "/v1/equity/fundraising-workflows/{workflow_id}/finalize",
    route: { method: "POST", path: "/v1/equity/fundraising-workflows/{pos}/finalize" },
    args: [{ name: "workflow-id", required: true, description: "Workflow Id" }],
    options: [
      { flags: "--phase <phase>", description: "Phase" },
    ],
  },
  {
    name: "equity fundraising-workflows-generate-board-packet",
    description: "/v1/equity/fundraising-workflows/{workflow_id}/generate-board-packet",
    route: { method: "POST", path: "/v1/equity/fundraising-workflows/{pos}/generate-board-packet" },
    args: [{ name: "workflow-id", required: true, description: "Workflow Id" }],
    options: [
      { flags: "--documents <documents>", description: "Documents", type: "array" },
    ],
  },
  {
    name: "equity fundraising-workflows-generate-closing-packet",
    description: "/v1/equity/fundraising-workflows/{workflow_id}/generate-closing-packet",
    route: { method: "POST", path: "/v1/equity/fundraising-workflows/{pos}/generate-closing-packet" },
    args: [{ name: "workflow-id", required: true, description: "Workflow Id" }],
    options: [
      { flags: "--documents <documents>", description: "Documents", type: "array" },
    ],
  },
  {
    name: "equity fundraising-workflows-prepare-execution",
    description: "/v1/equity/fundraising-workflows/{workflow_id}/prepare-execution",
    route: { method: "POST", path: "/v1/equity/fundraising-workflows/{pos}/prepare-execution" },
    args: [{ name: "workflow-id", required: true, description: "Workflow Id" }],
    options: [
      { flags: "--approval-artifact-id <approval-artifact-id>", description: "Approval Artifact Id", required: true },
      { flags: "--document-request-ids <document-request-ids>", description: "Document Request Ids", type: "array" },
      { flags: "--intent-id <intent-id>", description: "Intent Id", required: true },
      { flags: "--phase <phase>", description: "Phase" },
    ],
  },
  {
    name: "equity fundraising-workflows-record-board-approval",
    description: "/v1/equity/fundraising-workflows/{workflow_id}/record-board-approval",
    route: { method: "POST", path: "/v1/equity/fundraising-workflows/{pos}/record-board-approval" },
    args: [{ name: "workflow-id", required: true, description: "Workflow Id" }],
    options: [
      { flags: "--meeting-id <meeting-id>", description: "Meeting Id", required: true },
      { flags: "--resolution-id <resolution-id>", description: "Resolution Id", required: true },
    ],
  },
  {
    name: "equity fundraising-workflows-record-close",
    description: "/v1/equity/fundraising-workflows/{workflow_id}/record-close",
    route: { method: "POST", path: "/v1/equity/fundraising-workflows/{pos}/record-close" },
    args: [{ name: "workflow-id", required: true, description: "Workflow Id" }],
    options: [
      { flags: "--intent-id <intent-id>", description: "Intent Id", required: true },
    ],
  },
  {
    name: "equity fundraising-workflows-record-investor-acceptance",
    description: "/v1/equity/fundraising-workflows/{workflow_id}/record-investor-acceptance",
    route: { method: "POST", path: "/v1/equity/fundraising-workflows/{pos}/record-investor-acceptance" },
    args: [{ name: "workflow-id", required: true, description: "Workflow Id" }],
    options: [
      { flags: "--accepted-by-contact-id <accepted-by-contact-id>", description: "Accepted By Contact Id" },
      { flags: "--intent-id <intent-id>", description: "Intent Id", required: true },
    ],
  },
  {
    name: "equity fundraising-workflows-record-signature",
    description: "/v1/equity/fundraising-workflows/{workflow_id}/record-signature",
    route: { method: "POST", path: "/v1/equity/fundraising-workflows/{pos}/record-signature" },
    args: [{ name: "workflow-id", required: true, description: "Workflow Id" }],
    options: [
      { flags: "--channel <channel>", description: "Channel" },
      { flags: "--signer-identity <signer-identity>", description: "Signer Identity", required: true },
    ],
  },
  {
    name: "equity fundraising-workflows-start-signatures",
    description: "/v1/equity/fundraising-workflows/{workflow_id}/start-signatures",
    route: { method: "POST", path: "/v1/equity/fundraising-workflows/{pos}/start-signatures" },
    args: [{ name: "workflow-id", required: true, description: "Workflow Id" }],
  },
  {
    name: "equity grants",
    description: "/v1/equity/grants",
    route: { method: "POST", path: "/v1/equity/grants" },
    options: [
      { flags: "--grant-type <grant-type>", description: "The type of equity grant.", required: true, choices: ["common_stock", "preferred_stock", "membership_unit", "stock_option", "iso", "nso", "rsa", "svu"] },
      { flags: "--recipient-name <recipient-name>", description: "Recipient Name", required: true },
      { flags: "--shares <shares>", description: "Shares", required: true, type: "int" },
    ],
  },
  {
    name: "equity holders",
    description: "/v1/equity/holders",
    route: { method: "POST", path: "/v1/equity/holders" },
    options: [
      { flags: "--contact-id <contact-id>", description: "Contact Id", required: true },
      { flags: "--external-reference <external-reference>", description: "External Reference" },
      { flags: "--holder-type <holder-type>", description: "Type of holder represented in the cap table.", required: true, choices: ["individual", "organization", "fund", "nonprofit", "trust", "other"] },
      { flags: "--linked-entity-id <linked-entity-id>", description: "Linked Entity Id" },
      { flags: "--name <name>", description: "Name", required: true },
    ],
  },
  {
    name: "equity instruments",
    description: "/v1/equity/instruments",
    route: { method: "POST", path: "/v1/equity/instruments" },
    options: [
      { flags: "--authorized-units <authorized-units>", description: "Authorized Units" },
      { flags: "--issue-price-cents <issue-price-cents>", description: "Issue Price Cents" },
      { flags: "--issuer-legal-entity-id <issuer-legal-entity-id>", description: "Issuer Legal Entity Id", required: true },
      { flags: "--kind <kind>", description: "Instrument kind in the ownership model.", required: true, choices: ["common_equity", "preferred_equity", "membership_unit", "option_grant", "safe", "convertible_note", "warrant"] },
      { flags: "--symbol <symbol>", description: "Symbol", required: true },
      { flags: "--terms <terms>", description: "Terms" },
    ],
  },
  {
    name: "equity positions-adjust",
    description: "/v1/equity/positions/adjust",
    route: { method: "POST", path: "/v1/equity/positions/adjust" },
    options: [
      { flags: "--holder-id <holder-id>", description: "Holder Id", required: true },
      { flags: "--instrument-id <instrument-id>", description: "Instrument Id", required: true },
      { flags: "--issuer-legal-entity-id <issuer-legal-entity-id>", description: "Issuer Legal Entity Id", required: true },
      { flags: "--principal-delta-cents <principal-delta-cents>", description: "Principal Delta Cents", type: "int" },
      { flags: "--quantity-delta <quantity-delta>", description: "Quantity Delta", required: true, type: "int" },
      { flags: "--source-reference <source-reference>", description: "Source Reference" },
    ],
  },
  {
    name: "equity rounds",
    description: "/v1/equity/rounds",
    route: { method: "POST", path: "/v1/equity/rounds" },
    options: [
      { flags: "--conversion-target-instrument-id <conversion-target-instrument-id>", description: "Conversion Target Instrument Id" },
      { flags: "--issuer-legal-entity-id <issuer-legal-entity-id>", description: "Issuer Legal Entity Id", required: true },
      { flags: "--metadata <metadata>", description: "Metadata" },
      { flags: "--name <name>", description: "Name", required: true },
      { flags: "--pre-money-cents <pre-money-cents>", description: "Pre Money Cents" },
      { flags: "--round-price-cents <round-price-cents>", description: "Round Price Cents" },
      { flags: "--target-raise-cents <target-raise-cents>", description: "Target Raise Cents" },
    ],
  },
  {
    name: "equity rounds-staged",
    description: "/v1/equity/rounds/staged",
    route: { method: "POST", path: "/v1/equity/rounds/staged" },
    options: [
      { flags: "--issuer-legal-entity-id <issuer-legal-entity-id>", description: "Issuer Legal Entity Id", required: true },
      { flags: "--metadata <metadata>", description: "Metadata" },
      { flags: "--name <name>", description: "Name", required: true },
      { flags: "--pre-money-cents <pre-money-cents>", description: "Pre Money Cents" },
      { flags: "--round-price-cents <round-price-cents>", description: "Round Price Cents" },
      { flags: "--target-raise-cents <target-raise-cents>", description: "Target Raise Cents" },
    ],
  },
  {
    name: "equity rounds-accept",
    description: "/v1/equity/rounds/{round_id}/accept",
    route: { method: "POST", path: "/v1/equity/rounds/{pos}/accept" },
    args: [{ name: "round-id", required: true, description: "Round Id" }],
    options: [
      { flags: "--accepted-by-contact-id <accepted-by-contact-id>", description: "Accepted By Contact Id" },
      { flags: "--intent-id <intent-id>", description: "Intent Id", required: true },
    ],
  },
  {
    name: "equity rounds-apply-terms",
    description: "/v1/equity/rounds/{round_id}/apply-terms",
    route: { method: "POST", path: "/v1/equity/rounds/{pos}/apply-terms" },
    args: [{ name: "round-id", required: true, description: "Round Id" }],
    options: [
      { flags: "--anti-dilution-method <anti-dilution-method>", description: "Anti Dilution Method", required: true, choices: ["none", "broad_based_weighted_average", "narrow_based_weighted_average", "full_ratchet"] },
      { flags: "--conversion-precedence <conversion-precedence>", description: "Conversion Precedence", type: "array" },
      { flags: "--protective-provisions <protective-provisions>", description: "Protective Provisions" },
    ],
  },
  {
    name: "equity rounds-board-approve",
    description: "/v1/equity/rounds/{round_id}/board-approve",
    route: { method: "POST", path: "/v1/equity/rounds/{pos}/board-approve" },
    args: [{ name: "round-id", required: true, description: "Round Id" }],
    options: [
      { flags: "--meeting-id <meeting-id>", description: "Meeting Id", required: true },
      { flags: "--resolution-id <resolution-id>", description: "Resolution Id", required: true },
    ],
  },
  {
    name: "equity rounds-issue",
    description: "/v1/equity/rounds/{round_id}/issue",
    route: { method: "POST", path: "/v1/equity/rounds/{pos}/issue" },
    args: [{ name: "round-id", required: true, description: "Round Id" }],
    options: [
      { flags: "--meeting-id <meeting-id>", description: "Meeting Id" },
      { flags: "--resolution-id <resolution-id>", description: "Resolution Id" },
    ],
  },
  {
    name: "equity rounds-securities",
    description: "/v1/equity/rounds/{round_id}/securities",
    route: { method: "POST", path: "/v1/equity/rounds/{pos}/securities" },
    args: [{ name: "round-id", required: true, description: "Round Id" }],
    options: [
      { flags: "--email <email>", description: "Email" },
      { flags: "--grant-type <grant-type>", description: "Grant Type" },
      { flags: "--holder-id <holder-id>", description: "Holder Id" },
      { flags: "--instrument-id <instrument-id>", description: "Instrument Id", required: true },
      { flags: "--principal-cents <principal-cents>", description: "Principal Cents", type: "int" },
      { flags: "--quantity <quantity>", description: "Quantity", required: true, type: "int" },
      { flags: "--recipient-name <recipient-name>", description: "Recipient Name", required: true },
    ],
  },
  {
    name: "equity transfer-workflows",
    description: "/v1/equity/transfer-workflows",
    route: { method: "POST", path: "/v1/equity/transfer-workflows" },
    options: [
      { flags: "--from-contact-id <from-contact-id>", description: "From Contact Id", required: true },
      { flags: "--governing-doc-type <governing-doc-type>", description: "The type of governing document for a share transfer.", required: true, choices: ["bylaws", "operating_agreement", "shareholder_agreement", "other"] },
      { flags: "--prepare-intent-id <prepare-intent-id>", description: "Prepare Intent Id", required: true },
      { flags: "--price-per-share-cents <price-per-share-cents>", description: "Price Per Share Cents" },
      { flags: "--relationship-to-holder <relationship-to-holder>", description: "Relationship To Holder" },
      { flags: "--share-class-id <share-class-id>", description: "Share Class Id", required: true },
      { flags: "--share-count <share-count>", description: "Share Count", required: true, type: "int" },
      { flags: "--to-contact-id <to-contact-id>", description: "To Contact Id", required: true },
      { flags: "--transfer-type <transfer-type>", description: "Type of share transfer.", required: true, choices: ["gift", "trust_transfer", "secondary_sale", "estate", "other"] },
      { flags: "--transferee-rights <transferee-rights>", description: "Rights granted to the transferee.", required: true, choices: ["full_member", "economic_only", "limited"] },
    ],
  },
  {
    name: "equity transfer-workflows",
    description: "/v1/equity/transfer-workflows/{workflow_id}",
    route: { method: "GET", path: "/v1/equity/transfer-workflows/{pos}" },
    entity: true,
    args: [{ name: "workflow-id", required: true, description: "Workflow Id" }],
    display: { title: "Equity Transfer Workflows", cols: ["#active_packet_id>ID", "#board_approval_meeting_id>ID", "#board_approval_resolution_id>ID", "@created_at>Created At", "#execute_intent_id>ID", "execution_status>Execution Status", "generated_documents>Generated Documents", "last_packet_hash>Last Packet Hash"] },
  },
  {
    name: "equity transfer-workflows-compile-packet",
    description: "/v1/equity/transfer-workflows/{workflow_id}/compile-packet",
    route: { method: "POST", path: "/v1/equity/transfer-workflows/{pos}/compile-packet" },
    args: [{ name: "workflow-id", required: true, description: "Workflow Id" }],
    options: [
      { flags: "--phase <phase>", description: "Phase" },
      { flags: "--required-signers <required-signers>", description: "Required Signers", type: "array" },
    ],
  },
  {
    name: "equity transfer-workflows-finalize",
    description: "/v1/equity/transfer-workflows/{workflow_id}/finalize",
    route: { method: "POST", path: "/v1/equity/transfer-workflows/{pos}/finalize" },
    args: [{ name: "workflow-id", required: true, description: "Workflow Id" }],
    options: [
      { flags: "--phase <phase>", description: "Phase" },
    ],
  },
  {
    name: "equity transfer-workflows-generate-docs",
    description: "/v1/equity/transfer-workflows/{workflow_id}/generate-docs",
    route: { method: "POST", path: "/v1/equity/transfer-workflows/{pos}/generate-docs" },
    args: [{ name: "workflow-id", required: true, description: "Workflow Id" }],
    options: [
      { flags: "--documents <documents>", description: "Documents", type: "array" },
    ],
  },
  {
    name: "equity transfer-workflows-prepare-execution",
    description: "/v1/equity/transfer-workflows/{workflow_id}/prepare-execution",
    route: { method: "POST", path: "/v1/equity/transfer-workflows/{pos}/prepare-execution" },
    args: [{ name: "workflow-id", required: true, description: "Workflow Id" }],
    options: [
      { flags: "--approval-artifact-id <approval-artifact-id>", description: "Approval Artifact Id", required: true },
      { flags: "--document-request-ids <document-request-ids>", description: "Document Request Ids", type: "array" },
      { flags: "--intent-id <intent-id>", description: "Intent Id", required: true },
      { flags: "--phase <phase>", description: "Phase" },
    ],
  },
  {
    name: "equity transfer-workflows-record-board-approval",
    description: "/v1/equity/transfer-workflows/{workflow_id}/record-board-approval",
    route: { method: "POST", path: "/v1/equity/transfer-workflows/{pos}/record-board-approval" },
    args: [{ name: "workflow-id", required: true, description: "Workflow Id" }],
    options: [
      { flags: "--meeting-id <meeting-id>", description: "Meeting Id", required: true },
      { flags: "--resolution-id <resolution-id>", description: "Resolution Id", required: true },
    ],
  },
  {
    name: "equity transfer-workflows-record-execution",
    description: "/v1/equity/transfer-workflows/{workflow_id}/record-execution",
    route: { method: "POST", path: "/v1/equity/transfer-workflows/{pos}/record-execution" },
    args: [{ name: "workflow-id", required: true, description: "Workflow Id" }],
    options: [
      { flags: "--intent-id <intent-id>", description: "Intent Id", required: true },
    ],
  },
  {
    name: "equity transfer-workflows-record-review",
    description: "/v1/equity/transfer-workflows/{workflow_id}/record-review",
    route: { method: "POST", path: "/v1/equity/transfer-workflows/{pos}/record-review" },
    args: [{ name: "workflow-id", required: true, description: "Workflow Id" }],
    options: [
      { flags: "--approved", description: "Approved", required: true },
      { flags: "--notes <notes>", description: "Notes", required: true },
      { flags: "--reviewer <reviewer>", description: "Reviewer", required: true },
    ],
  },
  {
    name: "equity transfer-workflows-record-rofr",
    description: "/v1/equity/transfer-workflows/{workflow_id}/record-rofr",
    route: { method: "POST", path: "/v1/equity/transfer-workflows/{pos}/record-rofr" },
    args: [{ name: "workflow-id", required: true, description: "Workflow Id" }],
    options: [
      { flags: "--offered", description: "Offered", required: true },
      { flags: "--waived", description: "Waived", required: true },
    ],
  },
  {
    name: "equity transfer-workflows-record-signature",
    description: "/v1/equity/transfer-workflows/{workflow_id}/record-signature",
    route: { method: "POST", path: "/v1/equity/transfer-workflows/{pos}/record-signature" },
    args: [{ name: "workflow-id", required: true, description: "Workflow Id" }],
    options: [
      { flags: "--channel <channel>", description: "Channel" },
      { flags: "--signer-identity <signer-identity>", description: "Signer Identity", required: true },
    ],
  },
  {
    name: "equity transfer-workflows-start-signatures",
    description: "/v1/equity/transfer-workflows/{workflow_id}/start-signatures",
    route: { method: "POST", path: "/v1/equity/transfer-workflows/{pos}/start-signatures" },
    args: [{ name: "workflow-id", required: true, description: "Workflow Id" }],
  },
  {
    name: "equity transfer-workflows-submit-review",
    description: "/v1/equity/transfer-workflows/{workflow_id}/submit-review",
    route: { method: "POST", path: "/v1/equity/transfer-workflows/{pos}/submit-review" },
    args: [{ name: "workflow-id", required: true, description: "Workflow Id" }],
  },
  {
    name: "equity workflows-status",
    description: "/v1/equity/workflows/{workflow_type}/{workflow_id}/status",
    route: { method: "GET", path: "/v1/equity/workflows/{pos}/{pos2}/status" },
    entity: true,
    args: [{ name: "workflow-type", required: true, description: "Workflow Type" }, { name: "workflow-id", required: true, description: "Workflow Id" }],
    display: { title: "Equity Workflows Status", cols: ["#active_packet_id>ID", "execution_status>Execution Status", "fundraising_workflow>Fundraising Workflow", "packet>Packet", "transfer_workflow>Transfer Workflow", "#workflow_id>ID", "workflow_type>Workflow Type"] },
  },
  {
    name: "safe-notes",
    description: "/v1/safe-notes",
    route: { method: "POST", path: "/v1/safe-notes" },
    options: [
      { flags: "--conversion-unit-type <conversion-unit-type>", description: "Conversion Unit Type" },
      { flags: "--discount-rate <discount-rate>", description: "Discount Rate" },
      { flags: "--document-id <document-id>", description: "Document Id" },
      { flags: "--email <email>", description: "Email" },
      { flags: "--investor-contact-id <investor-contact-id>", description: "Investor Contact Id" },
      { flags: "--investor-name <investor-name>", description: "Investor Name", required: true },
      { flags: "--meeting-id <meeting-id>", description: "Meeting Id" },
      { flags: "--principal-amount-cents <principal-amount-cents>", description: "Principal Amount Cents", required: true, type: "int" },
      { flags: "--pro-rata-rights", description: "Pro Rata Rights" },
      { flags: "--resolution-id <resolution-id>", description: "Resolution Id" },
      { flags: "--safe-type <safe-type>", description: "Safe Type", choices: ["post_money", "pre_money", "mfn"] },
      { flags: "--valuation-cap-cents <valuation-cap-cents>", description: "Valuation Cap Cents" },
    ],
  },
  {
    name: "share-transfers",
    description: "/v1/share-transfers",
    route: { method: "POST", path: "/v1/share-transfers" },
    options: [
      { flags: "--from-holder <from-holder>", description: "From Holder", required: true },
      { flags: "--governing-doc-type <governing-doc-type>", description: "Governing Doc Type", choices: ["bylaws", "operating_agreement", "shareholder_agreement", "other"] },
      { flags: "--share-class-id <share-class-id>", description: "Share Class Id", required: true },
      { flags: "--shares <shares>", description: "Shares", required: true, type: "int" },
      { flags: "--to-holder <to-holder>", description: "To Holder", required: true },
      { flags: "--transfer-type <transfer-type>", description: "Type of share transfer.", required: true, choices: ["gift", "trust_transfer", "secondary_sale", "estate", "other"] },
      { flags: "--transferee-rights <transferee-rights>", description: "Transferee Rights", choices: ["full_member", "economic_only", "limited"] },
    ],
  },
  {
    name: "valuations",
    description: "/v1/valuations",
    route: { method: "POST", path: "/v1/valuations" },
    options: [
      { flags: "--dlom <dlom>", description: "Dlom" },
      { flags: "--effective-date <effective-date>", description: "Effective Date", required: true },
      { flags: "--enterprise-value-cents <enterprise-value-cents>", description: "Enterprise Value Cents" },
      { flags: "--fmv-per-share-cents <fmv-per-share-cents>", description: "Fmv Per Share Cents" },
      { flags: "--hurdle-amount-cents <hurdle-amount-cents>", description: "Hurdle Amount Cents" },
      { flags: "--methodology <methodology>", description: "Methodology used for a valuation.", required: true, choices: ["income", "market", "asset", "backsolve", "hybrid", "other"] },
      { flags: "--provider-contact-id <provider-contact-id>", description: "Provider Contact Id" },
      { flags: "--report-date <report-date>", description: "Report Date" },
      { flags: "--report-document-id <report-document-id>", description: "Report Document Id" },
      { flags: "--valuation-type <valuation-type>", description: "Type of 409A or equivalent valuation.", required: true, choices: ["four_oh_nine_a", "llc_profits_interest", "fair_market_value", "gift", "estate", "other"] },
    ],
  },
  {
    name: "valuations submit-for-approval",
    description: "/v1/valuations/{valuation_id}/submit-for-approval",
    route: { method: "POST", path: "/v1/valuations/{pos}/submit-for-approval" },
    args: [{ name: "valuation-id", required: true, description: "Valuation Id" }],
  },

];