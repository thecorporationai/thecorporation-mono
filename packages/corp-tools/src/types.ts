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

export type ApiRecord = Record<string, unknown>;
