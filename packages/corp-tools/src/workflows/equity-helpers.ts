// ---------------------------------------------------------------------------
// Pure business-logic helpers for equity issuance
// No Node/CLI/browser dependencies — just data transforms and validation.
// ---------------------------------------------------------------------------

import type { CorpAPIClient } from "../api-client.js";
import type { CapTableInstrument } from "../tools.js";

export type { CapTableInstrument } from "../tools.js";

// ---------------------------------------------------------------------------
// Grant-type normalization
// ---------------------------------------------------------------------------

export function normalizedGrantType(grantType: string): string {
  return grantType.trim().toLowerCase().replaceAll("-", "_").replaceAll(" ", "_");
}

// ---------------------------------------------------------------------------
// Instrument-kind mapping
// ---------------------------------------------------------------------------

export function expectedInstrumentKinds(grantType: string): string[] {
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

// ---------------------------------------------------------------------------
// 409A requirement check
// ---------------------------------------------------------------------------

export function grantRequiresCurrent409a(
  grantType: string,
  instrumentKind?: string,
): boolean {
  return (
    instrumentKind?.toLowerCase() === "option_grant" ||
    expectedInstrumentKinds(grantType).includes("option_grant")
  );
}

// ---------------------------------------------------------------------------
// Instrument creation hint (CLI-agnostic text)
// ---------------------------------------------------------------------------

export function buildInstrumentCreationHint(grantType: string): string {
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

// ---------------------------------------------------------------------------
// Resolve the instrument for a given grant type
// ---------------------------------------------------------------------------

export function resolveInstrumentForGrant(
  instruments: CapTableInstrument[],
  grantType: string,
  explicitInstrumentId?: string,
): CapTableInstrument {
  if (explicitInstrumentId) {
    const explicit = instruments.find(
      (instrument) => instrument.instrument_id === explicitInstrumentId,
    );
    if (!explicit) {
      throw new Error(
        `Instrument ${explicitInstrumentId} was not found on the cap table.`,
      );
    }
    return explicit;
  }

  const kinds = expectedInstrumentKinds(grantType);
  if (kinds.length === 0) {
    throw new Error(
      `No default instrument mapping exists for grant type "${grantType}". ${buildInstrumentCreationHint(grantType)}`,
    );
  }
  const match = instruments.find((instrument) =>
    kinds.includes(String(instrument.kind).toLowerCase()),
  );
  if (!match) {
    throw new Error(
      `No instrument found for grant type "${grantType}". Expected one of: ${kinds.join(", ")}. ${buildInstrumentCreationHint(grantType)}`,
    );
  }
  return match;
}

// ---------------------------------------------------------------------------
// Board check
// ---------------------------------------------------------------------------

export async function entityHasActiveBoard(
  client: CorpAPIClient,
  entityId: string,
): Promise<boolean> {
  const bodies = await client.listGovernanceBodies(entityId);
  return bodies.some(
    (body) =>
      String(body.body_type ?? "").toLowerCase() === "board_of_directors" &&
      String(body.status ?? "active").toLowerCase() === "active",
  );
}

// ---------------------------------------------------------------------------
// Issuance preflight: board approval + 409A
// ---------------------------------------------------------------------------

export async function ensureIssuancePreflight(
  client: CorpAPIClient,
  entityId: string,
  grantType: string,
  instrument?: CapTableInstrument,
  meetingId?: string,
  resolutionId?: string,
  operationLabel?: string,
): Promise<void> {
  if (!meetingId || !resolutionId) {
    if (await entityHasActiveBoard(client, entityId)) {
      const label = operationLabel ?? "this issuance";
      throw new Error(
        `Board approval is required for ${label}. Pass --meeting-id and --resolution-id from a passed board vote.\n` +
        `  Tip: Use 'corp governance quick-approve --text "RESOLVED: authorize ${label}"' for one-step approval.`,
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
    if (
      msg.includes("404") ||
      msg.includes("Not found") ||
      msg.includes("not found")
    ) {
      // Auto-create and auto-approve a 409A valuation for early-stage companies
      try {
        const today = new Date().toISOString().slice(0, 10);
        const valuation = await client.createValuation({
          entity_id: entityId,
          valuation_type: "four_oh_nine_a",
          effective_date: today,
          methodology: "backsolve",
          fmv_per_share_cents: 1, // $0.01 par value — typical early stage
        });
        const valuationId = String((valuation as Record<string, unknown>).valuation_id ?? "");
        if (valuationId) {
          await client.submitValuationForApproval(valuationId, entityId);
          await client.approveValuation(valuationId, entityId);
        }
        return; // 409A now exists, proceed with grant
      } catch {
        // Auto-create failed (board may be required) — fall through to original error
      }
      throw new Error(
        "Stock option issuances require a current approved 409A valuation.\n" +
        "  Auto-creation failed. Create manually:\n" +
        "    corp cap-table create-valuation --type four_oh_nine_a --date YYYY-MM-DD --methodology backsolve --auto-approve",
      );
    }
    throw err;
  }
}
