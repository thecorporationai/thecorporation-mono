// ---------------------------------------------------------------------------
// Workflow barrel — re-exports all shared workflow functions and types
// ---------------------------------------------------------------------------

// Result types
export type { WorkflowResult, WorkflowStep } from "./types.js";

// Equity helpers (pure business logic)
export {
  normalizedGrantType,
  expectedInstrumentKinds,
  grantRequiresCurrent409a,
  buildInstrumentCreationHint,
  resolveInstrumentForGrant,
  entityHasActiveBoard,
  ensureIssuancePreflight,
} from "./equity-helpers.js";
export type { CapTableInstrument } from "./equity-helpers.js";

// Multi-step workflows
export { issueEquity } from "./issue-equity.js";
export type { IssueEquityArgs } from "./issue-equity.js";

export { issueSafe } from "./issue-safe.js";
export type { IssueSafeArgs } from "./issue-safe.js";

export { writtenConsent } from "./written-consent.js";
export type { WrittenConsentArgs } from "./written-consent.js";
