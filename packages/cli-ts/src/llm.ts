import type { ToolCall, LLMResponse } from "./types.js";
import type OpenAI from "openai";
import type { ChatCompletionCreateParamsNonStreaming } from "openai/resources/chat/completions.js";

const PROVIDER_BASE_URLS: Record<string, string> = {
  openrouter: "https://openrouter.ai/api/v1",
};

export async function chat(
  messages: Record<string, unknown>[],
  tools?: Record<string, unknown>[],
  provider = "openrouter",
  apiKey = "",
  model = "",
  baseUrl?: string,
): Promise<LLMResponse> {
  const effectiveUrl = baseUrl ?? PROVIDER_BASE_URLS[provider] ?? PROVIDER_BASE_URLS.openrouter;
  const { default: OpenAIClient } = await import("openai");
  const client = new OpenAIClient({ apiKey, baseURL: effectiveUrl });

  const params: ChatCompletionCreateParamsNonStreaming = {
    model: model || "gpt-4o",
    messages: messages as unknown as OpenAI.ChatCompletionMessageParam[],
    max_tokens: 4096,
  };
  if (tools?.length) {
    params.tools = tools as unknown as OpenAI.ChatCompletionTool[];
    params.tool_choice = "auto";
  }

  const response = await client.chat.completions.create(params);
  const choice = response.choices[0];
  const message = choice.message;

  const toolCallsOut: ToolCall[] = [];
  if (message.tool_calls) {
    for (const tc of message.tool_calls) {
      let args: Record<string, unknown>;
      try {
        args = JSON.parse(tc.function.arguments);
      } catch {
        args = { _raw: tc.function.arguments };
      }
      toolCallsOut.push({ id: tc.id, name: tc.function.name, arguments: args });
    }
  }

  return {
    content: message.content,
    tool_calls: toolCallsOut,
    usage: {
      prompt_tokens: response.usage?.prompt_tokens ?? 0,
      completion_tokens: response.usage?.completion_tokens ?? 0,
      total_tokens: response.usage?.total_tokens ?? 0,
    },
    finish_reason: choice.finish_reason ?? null,
  };
}
