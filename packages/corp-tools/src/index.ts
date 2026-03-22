export type {
  CorpConfig,
  ToolCall,
  LLMResponse,
  ApiRecord,
  CreateEquityRoundRequest,
  ApplyEquityRoundTermsRequest,
  BoardApproveEquityRoundRequest,
  AcceptEquityRoundRequest,
  PreviewRoundConversionRequest,
  ExecuteRoundConversionRequest,
  CreateExecutionIntentRequest,
  EquityRoundResponse,
  IntentResponse,
} from "./types.js";
export { CorpAPIClient, SessionExpiredError, provisionWorkspace } from "./api-client.js";
export { TOOL_DEFINITIONS, TOOL_DISPATCH_COUNT, isWriteTool, executeTool, ensureSafeInstrument } from "./tools.js";
export type { ToolContext } from "./tools.js";
export type { CapTableInstrument } from "./types.js";
export { describeToolCall } from "./tool-descriptions.js";

// System prompt
export { SYSTEM_PROMPT_BASE, formatConfigSection } from "./system-prompt.js";

// Definitions registry
export { TOOL_REGISTRY, GENERATED_TOOL_DEFINITIONS, READ_ONLY_TOOLS } from "./definitions.js";

// Generated OpenAPI types and runtime enum constants
export * from "./api-enums.generated.js";
export type * from "./api-schemas.js";
export { ensureEnvFile, loadEnvFile, generateFernetKey, generateSecret } from "./env.js";
export { processRequest, resolveBinaryPath, resetCache } from "./process-transport.js";
export type { ProcessRequestOptions } from "./process-transport.js";

// Reference matching / tracking (browser-compatible core)
export {
  ReferenceTracker,
  shortId,
  slugify,
  normalize,
  validateReferenceInput,
  describeReferenceRecord,
  getReferenceId,
  getReferenceLabel,
  getReferenceAlias,
  matchRank,
  isOpaqueUuid,
  isShortIdCandidate,
  parseLastReference,
  uniqueStrings,
  kindLabel,
  isEntityScopedKind,
  extractId,
  isValidResourceKind,
  RESOURCE_KINDS,
} from "./reference-tracker.js";
export type {
  ReferenceStorage,
  ResourceKind,
  MatchRecord,
  ReferenceMatch,
} from "./reference-tracker.js";

// Shared workflows (multi-step business logic)
export {
  // Equity helpers
  normalizedGrantType,
  expectedInstrumentKinds,
  grantRequiresCurrent409a,
  buildInstrumentCreationHint,
  resolveInstrumentForGrant,
  entityHasActiveBoard,
  ensureIssuancePreflight,
  // Multi-step workflows
  issueEquity,
  issueSafe,
  writtenConsent,
} from "./workflows/index.js";
export type {
  WorkflowResult,
  WorkflowStep,
  IssueEquityArgs,
  IssueSafeArgs,
  WrittenConsentArgs,
} from "./workflows/index.js";
