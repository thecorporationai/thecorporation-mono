/**
 * Convenience re-exports of key generated OpenAPI schema types.
 * Regenerate source with: npm run generate:types
 */
import type { components } from "./api-types.generated.js";

// ── Workspace ────────────────────────────────────────────────────────
export type WorkspaceStatusResponse = components["schemas"]["WorkspaceStatusResponse"];
export type WorkspaceEntitySummary = components["schemas"]["WorkspaceEntitySummary"];
export type WorkspaceContactSummary = components["schemas"]["WorkspaceContactSummary"];

// ── Entities / Formation ─────────────────────────────────────────────
export type FormationResponse = components["schemas"]["FormationResponse"];
export type FormationWithCapTableResponse = components["schemas"]["FormationWithCapTableResponse"];
export type FormationStatusResponse = components["schemas"]["FormationStatusResponse"];
export type FormationGatesResponse = components["schemas"]["FormationGatesResponse"];
export type PendingFormationResponse = components["schemas"]["PendingFormationResponse"];
export type CreateFormationRequest = components["schemas"]["CreateFormationRequest"];
export type CreatePendingFormationRequest = components["schemas"]["CreatePendingFormationRequest"];
export type AddFounderRequest = components["schemas"]["AddFounderRequest"];
export type AddFounderResponse = components["schemas"]["AddFounderResponse"];
export type ConvertEntityRequest = components["schemas"]["ConvertEntityRequest"];
export type DissolveEntityRequest = components["schemas"]["DissolveEntityRequest"];

// ── Contacts ─────────────────────────────────────────────────────────
export type ContactResponse = components["schemas"]["ContactResponse"];
export type ContactProfileResponse = components["schemas"]["ContactProfileResponse"];
export type CreateContactRequest = components["schemas"]["CreateContactRequest"];
export type UpdateContactRequest = components["schemas"]["UpdateContactRequest"];

// ── Cap Table & Equity ───────────────────────────────────────────────
export type CapTableResponse = components["schemas"]["CapTableResponse"];
export type CapTableHolderSummary = components["schemas"]["CapTableHolderSummary"];
export type CapTableInstrumentSummary = components["schemas"]["CapTableInstrumentSummary"];
export type CreateRoundRequest = components["schemas"]["CreateRoundRequest"];
export type RoundResponse = components["schemas"]["RoundResponse"];
export type ApplyRoundTermsRequest = components["schemas"]["ApplyRoundTermsRequest"];
export type BoardApproveRoundRequest = components["schemas"]["BoardApproveRoundRequest"];
export type AcceptRoundRequest = components["schemas"]["AcceptRoundRequest"];
export type StartStagedRoundRequest = components["schemas"]["StartStagedRoundRequest"];
export type AddSecurityRequest = components["schemas"]["AddSecurityRequest"];
export type IssueStagedRoundResponse = components["schemas"]["IssueStagedRoundResponse"];

// ── Governance ───────────────────────────────────────────────────────
export type GovernanceBodyResponse = components["schemas"]["GovernanceBodyResponse"];
export type GovernanceSeatResponse = components["schemas"]["GovernanceSeatResponse"];
export type MeetingResponse = components["schemas"]["MeetingResponse"];
export type ResolutionResponse = components["schemas"]["ResolutionResponse"];
export type AgendaItemResponse = components["schemas"]["AgendaItemResponse"];
export type VoteResponse = components["schemas"]["VoteResponse"];
export type ScheduleMeetingRequest = components["schemas"]["ScheduleMeetingRequest"];
export type ConveneMeetingRequest = components["schemas"]["ConveneMeetingRequest"];
export type CastVoteRequest = components["schemas"]["CastVoteRequest"];
export type FinalizeAgendaItemRequest = components["schemas"]["FinalizeAgendaItemRequest"];
export type ComputeResolutionRequest = components["schemas"]["ComputeResolutionRequest"];
export type AttachResolutionDocumentRequest = components["schemas"]["AttachResolutionDocumentRequest"];
export type WrittenConsentRequest = components["schemas"]["WrittenConsentRequest"];
export type WrittenConsentResponse = components["schemas"]["WrittenConsentResponse"];

// ── Agents ───────────────────────────────────────────────────────────
export type AgentResponse = components["schemas"]["AgentResponse"];
export type CreateAgentRequest = components["schemas"]["CreateAgentRequest"];
export type UpdateAgentRequest = components["schemas"]["UpdateAgentRequest"];

// ── Obligations ──────────────────────────────────────────────────────
export type ObligationResponse = components["schemas"]["ObligationResponse"];
export type ObligationsSummaryResponse = components["schemas"]["ObligationsSummaryResponse"];

// ── Documents ────────────────────────────────────────────────────────
export type DocumentResponse = components["schemas"]["DocumentResponse"];
export type DocumentSummary = components["schemas"]["DocumentSummary"];

// ── Digests ──────────────────────────────────────────────────────────
export type DigestSummary = components["schemas"]["DigestSummary"];
export type DigestTriggerResponse = components["schemas"]["DigestTriggerResponse"];

// ── Billing ──────────────────────────────────────────────────────────
export type InvoiceResponse = components["schemas"]["InvoiceResponse"];

// ── Next Steps ──────────────────────────────────────────────────────
export interface NextStepItem {
  category: string;
  title: string;
  description?: string;
  command: string;
  urgency: string;
}
export interface NextStepsSummary {
  critical: number;
  high: number;
  medium: number;
  low: number;
}
export interface NextStepsResponse {
  top: NextStepItem | null;
  backlog: NextStepItem[];
  summary: NextStepsSummary;
}
