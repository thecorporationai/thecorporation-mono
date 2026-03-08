export interface CorpConfig {
  api_url: string;
  api_key: string;
  workspace_id: string;
  hosting_mode: string;
  llm: {
    provider: string;
    api_key: string;
    model: string;
    base_url?: string;
  };
  user: {
    name: string;
    email: string;
  };
  active_entity_id: string;
  [key: string]: unknown;
}

export interface ToolCall {
  id: string;
  name: string;
  arguments: Record<string, unknown>;
}

export interface LLMResponse {
  content: string | null;
  tool_calls: ToolCall[];
  usage: { prompt_tokens: number; completion_tokens: number; total_tokens: number };
  finish_reason: string | null;
}

/** @deprecated Use specific generated types from api-schemas.ts instead */
export type ApiRecord = Record<string, unknown>;

export interface CreateEquityRoundRequest {
  entity_id: string;
  issuer_legal_entity_id: string;
  name: string;
  pre_money_cents?: number;
  round_price_cents?: number;
  target_raise_cents?: number;
  conversion_target_instrument_id?: string;
  metadata?: Record<string, unknown>;
}

export interface ApplyEquityRoundTermsRequest {
  entity_id: string;
  anti_dilution_method: string;
  conversion_precedence?: string[];
  protective_provisions?: Record<string, unknown>;
}

export interface BoardApproveEquityRoundRequest {
  entity_id: string;
  meeting_id: string;
  resolution_id: string;
}

export interface AcceptEquityRoundRequest {
  entity_id: string;
  intent_id: string;
  accepted_by_contact_id?: string;
}

export interface PreviewRoundConversionRequest {
  entity_id: string;
  round_id: string;
  source_reference?: string;
}

export interface ExecuteRoundConversionRequest {
  entity_id: string;
  round_id: string;
  intent_id: string;
  source_reference?: string;
}

export interface CreateExecutionIntentRequest {
  entity_id: string;
  intent_type: string;
  authority_tier?: string;
  description: string;
  metadata?: Record<string, unknown>;
}

export interface EquityRoundResponse extends ApiRecord {
  round_id: string;
  status: string;
}

export interface IntentResponse extends ApiRecord {
  intent_id: string;
  status: string;
}
