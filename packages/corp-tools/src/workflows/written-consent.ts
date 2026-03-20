// ---------------------------------------------------------------------------
// Workflow: written-consent
//
// Pure function: create written consent → auto-open with all body seats.
// No console output, no process.exit — returns a structured WorkflowResult.
// ---------------------------------------------------------------------------

import type { CorpAPIClient } from "../api-client.js";
import type { WorkflowResult } from "./types.js";

// ---------------------------------------------------------------------------
// Args
// ---------------------------------------------------------------------------

export interface WrittenConsentArgs {
  entityId: string;
  /** Pre-resolved governance body ID. */
  bodyId: string;
  title: string;
  description: string;
}

// ---------------------------------------------------------------------------
// Workflow
// ---------------------------------------------------------------------------

export async function writtenConsent(
  client: CorpAPIClient,
  args: WrittenConsentArgs,
): Promise<WorkflowResult> {
  const steps: WorkflowResult["steps"] = [];

  try {
    // ── Create written consent ───────────────────────────────────
    const payload = {
      entity_id: args.entityId,
      body_id: args.bodyId,
      title: args.title,
      description: args.description,
    };
    const result = await client.writtenConsent(payload);
    const meetingId = String(result.meeting_id ?? "");
    steps.push({
      name: "create_written_consent",
      status: "ok",
      data: { meeting_id: meetingId || undefined },
    });

    // ── Auto-open with all body seats ────────────────────────────
    if (meetingId) {
      try {
        const seats = await client.getGovernanceSeats(
          args.bodyId,
          args.entityId,
        );
        const seatIds = seats
          .map((s) =>
            String(
              s.seat_id ?? (s as Record<string, unknown>).id ?? "",
            ),
          )
          .filter((id) => id.length > 0);
        if (seatIds.length > 0) {
          await client.conveneMeeting(meetingId, args.entityId, {
            present_seat_ids: seatIds,
          });
          steps.push({
            name: "auto_open",
            status: "ok",
            data: { seat_count: seatIds.length },
            detail: `Opened with ${seatIds.length} seat(s) present`,
          });
        } else {
          steps.push({
            name: "auto_open",
            status: "skipped",
            detail: "No seats found on body",
          });
        }
      } catch {
        // Non-fatal: written consent can still proceed without open step
        steps.push({
          name: "auto_open",
          status: "skipped",
          detail: "Failed to auto-open meeting (non-fatal)",
        });
      }
    } else {
      steps.push({
        name: "auto_open",
        status: "skipped",
        detail: "No meeting_id returned",
      });
    }

    return {
      success: true,
      data: { ...result, meeting_id: meetingId || undefined },
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
