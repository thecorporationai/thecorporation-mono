/**
 * Browser-compatible entry point for corp-tools.
 *
 * Exports the subset of corp-tools that runs in the browser:
 * - API client (CorpAPIClient)
 * - Reference tracker (matching, @last tracking, types)
 * - Workflow functions (issue-equity, issue-safe, written-consent)
 * - Types (ResourceKind, WebRouteEntry, etc.)
 *
 * Does NOT export Node-specific code (process transport, server bindings).
 */

// API client
export { CorpAPIClient } from "./api-client.js";
export type { ApiRecord } from "./types.js";

// Reference tracker
export {
  ReferenceTracker,
  shortId,
  slugify,
  describeReferenceRecord,
  getReferenceId,
  getReferenceLabel,
  getReferenceAlias,
  matchRank,
  RESOURCE_KINDS,
  isValidResourceKind,
} from "./reference-tracker.js";
export type {
  ReferenceStorage,
  ResourceKind,
  MatchRecord,
  ReferenceMatch,
} from "./reference-tracker.js";

// Workflows
export { issueEquity } from "./workflows/issue-equity.js";
export { issueSafe } from "./workflows/issue-safe.js";
export { writtenConsent } from "./workflows/written-consent.js";
export {
  normalizedGrantType,
  expectedInstrumentKinds,
  grantRequiresCurrent409a,
  entityHasActiveBoard,
  ensureIssuancePreflight,
} from "./workflows/equity-helpers.js";
export type { WorkflowResult, WorkflowStep } from "./workflows/types.js";

// API schema types (useful for the web terminal)
export type {
  NextStepsResponse,
  NextStepItem,
  NextStepsSummary,
} from "./api-schemas.js";
