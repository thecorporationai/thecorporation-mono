import type { ToolCall, LLMResponse } from "./types.js";

const PROVIDER_BASE_URLS: Record<string, string> = {
  openrouter: "https://openrouter.ai/api/v1",
};

export async function chat(
  messages: Record<string, unknown>[],
  tools?: Record<string, unknown>[],
  provider = "anthropic",
  apiKey = "",
  model = "",
  baseUrl?: string,
): Promise<LLMResponse> {
  if (provider === "anthropic") {
    return chatAnthropic(messages, tools, apiKey, model);
  } else if (provider === "openai" || provider === "openrouter") {
    const effectiveUrl = baseUrl ?? PROVIDER_BASE_URLS[provider];
    return chatOpenAI(messages, tools, apiKey, model, effectiveUrl);
  }
  throw new Error(`Unknown LLM provider: ${provider}`);
}

async function chatAnthropic(
  messages: Record<string, unknown>[],
  tools?: Record<string, unknown>[],
  apiKey = "",
  model = "",
): Promise<LLMResponse> {
  const { default: Anthropic } = await import("@anthropic-ai/sdk");
  const client = new Anthropic({ apiKey });

  let systemText = "";
  const convMessages: Record<string, unknown>[] = [];

  for (const msg of messages) {
    if (msg.role === "system") {
      systemText = msg.content as string;
    } else if (msg.role === "tool") {
      convMessages.push({
        role: "user",
        content: [{
          type: "tool_result",
          tool_use_id: msg.tool_call_id,
          content: msg.content,
        }],
      });
    } else if (msg.role === "assistant" && msg.tool_calls) {
      const contentBlocks: Record<string, unknown>[] = [];
      if (msg.content) contentBlocks.push({ type: "text", text: msg.content });
      for (const tc of msg.tool_calls as Record<string, unknown>[]) {
        const fn = tc.function as Record<string, unknown>;
        let args = fn.arguments;
        if (typeof args === "string") args = JSON.parse(args);
        contentBlocks.push({ type: "tool_use", id: tc.id, name: fn.name, input: args });
      }
      convMessages.push({ role: "assistant", content: contentBlocks });
    } else {
      convMessages.push({ role: msg.role, content: msg.content ?? "" });
    }
  }

  let anthropicTools: Record<string, unknown>[] | undefined;
  if (tools?.length) {
    anthropicTools = tools.map((t) => {
      const fn = (t as Record<string, unknown>).function as Record<string, unknown>;
      return {
        name: fn.name,
        description: fn.description ?? "",
        input_schema: fn.parameters ?? { type: "object", properties: {} },
      };
    });
  }

  const kwargs: Record<string, unknown> = {
    model: model || "claude-sonnet-4-20250514",
    max_tokens: 4096,
    messages: convMessages,
  };
  if (systemText) kwargs.system = systemText;
  if (anthropicTools) kwargs.tools = anthropicTools;

  const response = await client.messages.create(kwargs as Parameters<typeof client.messages.create>[0]);

  let content: string | null = null;
  const toolCallsOut: ToolCall[] = [];
  for (const block of response.content) {
    if (block.type === "text") {
      content = block.text;
    } else if (block.type === "tool_use") {
      toolCallsOut.push({
        id: block.id,
        name: block.name,
        arguments: typeof block.input === "object" ? (block.input as Record<string, unknown>) : {},
      });
    }
  }

  return {
    content,
    tool_calls: toolCallsOut,
    usage: {
      prompt_tokens: response.usage.input_tokens,
      completion_tokens: response.usage.output_tokens,
      total_tokens: response.usage.input_tokens + response.usage.output_tokens,
    },
    finish_reason: response.stop_reason ?? null,
  };
}

async function chatOpenAI(
  messages: Record<string, unknown>[],
  tools?: Record<string, unknown>[],
  apiKey = "",
  model = "",
  baseUrl?: string,
): Promise<LLMResponse> {
  const { default: OpenAI } = await import("openai");
  const clientOpts: Record<string, unknown> = { apiKey };
  if (baseUrl) clientOpts.baseURL = baseUrl;
  const client = new OpenAI(clientOpts as ConstructorParameters<typeof OpenAI>[0]);

  const kwargs: Record<string, unknown> = {
    model: model || "gpt-4o",
    messages,
    max_tokens: 4096,
  };
  if (tools?.length) {
    kwargs.tools = tools;
    kwargs.tool_choice = "auto";
  }

  const response = await client.chat.completions.create(kwargs as Parameters<typeof client.chat.completions.create>[0]);
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
