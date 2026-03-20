// ---------------------------------------------------------------------------
// Workflow: issue-equity
//
// Pure function: preflight checks → start round → add security → issue round.
// No console output, no process.exit — returns a structured WorkflowResult.
// ---------------------------------------------------------------------------

import type { CorpAPIClient } from "../api-client.js";
import type { ApiRecord } from "../types.js";
import type { WorkflowResult } from "./types.js";
import {
  type CapTableInstrument,
  ensureIssuancePreflight,
  resolveInstrumentForGrant,
} from "./equity-helpers.js";

// ---------------------------------------------------------------------------
// Args
// ---------------------------------------------------------------------------

export interface IssueEquityArgs {
  entityId: string;
  grantType: string;
  shares: number;
  recipientName: string;
  recipientEmail?: string;
  /** Pre-resolved instrument ID (already resolved by the caller). */
  instrumentId?: string;
  /** Pre-resolved meeting ID. */
  meetingId?: string;
  /** Pre-resolved resolution ID. */
  resolutionId?: string;
}

// ---------------------------------------------------------------------------
// Workflow
// ---------------------------------------------------------------------------

export async function issueEquity(
  client: CorpAPIClient,
  args: IssueEquityArgs,
): Promise<WorkflowResult> {
  const steps: WorkflowResult["steps"] = [];

  try {
    // ── Fetch cap table ──────────────────────────────────────────
    const capTable = await client.getCapTable(args.entityId);
    const issuerLegalEntityId = capTable.issuer_legal_entity_id as
      | string
      | undefined;
    if (!issuerLegalEntityId) {
      return {
        success: false,
        error:
          "No issuer legal entity found. Has this entity been formed with a cap table?",
        steps,
      };
    }

    const instruments = (capTable.instruments ?? []) as CapTableInstrument[];
    if (!instruments.length) {
      return {
        success: false,
        error:
          "No instruments found on cap table. Create one with: corp cap-table create-instrument --kind common_equity --symbol COMMON --authorized-units <shares>",
        steps,
      };
    }

    // ── Resolve instrument ───────────────────────────────────────
    const instrument = resolveInstrumentForGrant(
      instruments,
      args.grantType,
      args.instrumentId,
    );
    const instrumentId = instrument.instrument_id;
    steps.push({
      name: "resolve_instrument",
      status: "ok",
      data: {
        instrument_id: instrumentId,
        symbol: instrument.symbol,
        kind: instrument.kind,
      },
      detail: `Using instrument: ${instrument.symbol} (${instrument.kind})`,
    });

    // ── Preflight checks ─────────────────────────────────────────
    await ensureIssuancePreflight(
      client,
      args.entityId,
      args.grantType,
      instrument,
      args.meetingId,
      args.resolutionId,
    );
    steps.push({ name: "preflight", status: "ok" });

    // ── Step 1: Start a staged round ─────────────────────────────
    const round = await client.startEquityRound({
      entity_id: args.entityId,
      name: `${args.grantType} grant \u2014 ${args.recipientName}`,
      issuer_legal_entity_id: issuerLegalEntityId,
    });
    const roundId = (round.round_id ?? round.equity_round_id) as string;
    steps.push({
      name: "start_round",
      status: "ok",
      data: { round_id: roundId },
    });

    // ── Step 2: Add the security ─────────────────────────────────
    const securityData: Record<string, unknown> = {
      entity_id: args.entityId,
      instrument_id: instrumentId,
      quantity: args.shares,
      recipient_name: args.recipientName,
      grant_type: args.grantType,
    };
    if (args.recipientEmail) securityData.email = args.recipientEmail;

    // Attempt to find existing holder to avoid duplicates
    const existingHolders = (capTable.holders ?? []) as ApiRecord[];
    const matchingHolder = existingHolders.find((h) => {
      const nameMatch =
        String(h.name ?? "").toLowerCase() ===
        args.recipientName.toLowerCase();
      const emailMatch =
        args.recipientEmail &&
        String(h.email ?? "").toLowerCase() ===
          args.recipientEmail.toLowerCase();
      return nameMatch || emailMatch;
    });
    if (matchingHolder) {
      const holderId =
        matchingHolder.holder_id ??
        matchingHolder.contact_id ??
        matchingHolder.id;
      if (holderId) securityData.holder_id = holderId;
    }

    await client.addRoundSecurity(roundId, securityData);
    steps.push({ name: "add_security", status: "ok" });

    // ── Step 3: Issue the round ──────────────────────────────────
    const issuePayload: Record<string, unknown> = {
      entity_id: args.entityId,
    };
    if (args.meetingId) issuePayload.meeting_id = args.meetingId;
    if (args.resolutionId) issuePayload.resolution_id = args.resolutionId;
    const result = await client.issueRound(roundId, issuePayload);
    steps.push({ name: "issue_round", status: "ok" });

    return {
      success: true,
      data: {
        ...result,
        round_id: roundId,
        round,
        shares: args.shares,
        grant_type: args.grantType,
        recipient: args.recipientName,
      },
      steps,
    };
  } catch (err) {
    steps.push({
      name: "error",
      status: "failed",
      detail: String(err),
    });
    return {
      success: false,
      error: err instanceof Error ? err.message : String(err),
      steps,
    };
  }
}
