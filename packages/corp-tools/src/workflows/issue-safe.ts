// ---------------------------------------------------------------------------
// Workflow: issue-safe
//
// Pure function: preflight checks → create SAFE note.
// No console output, no process.exit — returns a structured WorkflowResult.
// ---------------------------------------------------------------------------

import type { CorpAPIClient } from "../api-client.js";
import type { WorkflowResult } from "./types.js";
import { ensureIssuancePreflight } from "./equity-helpers.js";

// ---------------------------------------------------------------------------
// Args
// ---------------------------------------------------------------------------

export interface IssueSafeArgs {
  entityId: string;
  investorName: string;
  amountCents: number;
  valuationCapCents: number;
  safeType?: string;
  email?: string;
  /** Pre-resolved meeting ID. */
  meetingId?: string;
  /** Pre-resolved resolution ID. */
  resolutionId?: string;
}

// ---------------------------------------------------------------------------
// Workflow
// ---------------------------------------------------------------------------

export async function issueSafe(
  client: CorpAPIClient,
  args: IssueSafeArgs,
): Promise<WorkflowResult> {
  const steps: WorkflowResult["steps"] = [];
  const safeType = args.safeType ?? "post_money";

  try {
    // ── Preflight checks ─────────────────────────────────────────
    await ensureIssuancePreflight(
      client,
      args.entityId,
      safeType,
      undefined,
      args.meetingId,
      args.resolutionId,
      "SAFE issuance",
    );
    steps.push({ name: "preflight", status: "ok" });

    // ── Create SAFE note ─────────────────────────────────────────
    const body: Record<string, unknown> = {
      entity_id: args.entityId,
      investor_name: args.investorName,
      principal_amount_cents: args.amountCents,
      valuation_cap_cents: args.valuationCapCents,
      safe_type: safeType,
    };
    if (args.email) body.email = args.email;
    if (args.meetingId) body.meeting_id = args.meetingId;
    if (args.resolutionId) body.resolution_id = args.resolutionId;

    const result = await client.createSafeNote(body);
    steps.push({ name: "create_safe", status: "ok" });

    return {
      success: true,
      data: {
        ...result,
        investor_name: args.investorName,
        amount_cents: args.amountCents,
        valuation_cap_cents: args.valuationCapCents,
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
