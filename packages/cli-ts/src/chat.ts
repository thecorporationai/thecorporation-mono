import { createInterface } from "node:readline";
import chalk from "chalk";
import { requireConfig, configForDisplay, getValue, setValue, saveConfig } from "./config.js";
import { CorpAPIClient } from "./api-client.js";
import { chat as llmChat } from "./llm.js";
import { TOOL_DEFINITIONS, executeTool, isWriteTool } from "./tools.js";
import { printError, printStatusPanel, printObligationsTable, printJson } from "./output.js";
import type { CorpConfig, LLMResponse, ToolCall } from "./types.js";

const SYSTEM_PROMPT = `You are **corp**, an AI assistant for corporate governance.
You help users manage their companies — entities, cap tables, compliance, governance, finances, and more.
You have tools to read and write corporate data. Use them to fulfill user requests.
For write operations, confirm with the user before proceeding.
Monetary values are in cents (e.g. 100000 = $1,000.00).
Documents must be signed by the human — you cannot sign on their behalf.
After completing actions, suggest logical next steps.`;

export async function chatCommand(): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id", "llm.api_key");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);

  const messages: Record<string, unknown>[] = [{ role: "system", content: SYSTEM_PROMPT }];
  let totalTokens = 0;

  const rl = createInterface({ input: process.stdin, output: process.stdout });
  const prompt = () => new Promise<string>((resolve) => rl.question(chalk.green.bold("> "), resolve));

  console.log(chalk.blue.bold("corp chat") + " — type /help for commands, /quit to exit\n");

  const slashHandlers: Record<string, (args: string) => void | Promise<void>> = {
    "/status": async () => {
      try { printStatusPanel(await client.getStatus()); } catch (e) { printError(`Status error: ${e}`); }
    },
    "/obligations": async () => {
      try {
        const data = await client.getObligations();
        const obls = (data.obligations ?? []) as Record<string, unknown>[];
        if (obls.length) printObligationsTable(obls);
        else console.log(chalk.dim("No obligations found."));
      } catch (e) { printError(`Obligations error: ${e}`); }
    },
    "/digest": async () => {
      try {
        const digests = await client.listDigests();
        if (digests.length) printJson(digests);
        else console.log(chalk.dim("No digest history."));
      } catch (e) { printError(`Digest error: ${e}`); }
    },
    "/config": () => printJson(configForDisplay(cfg)),
    "/model": (args: string) => {
      const model = args.trim();
      if (!model) { console.log(`Current model: ${getValue(cfg as unknown as Record<string, unknown>, "llm.model")}`); return; }
      setValue(cfg as unknown as Record<string, unknown>, "llm.model", model);
      saveConfig(cfg);
      console.log(`Model switched to: ${model}`);
    },
    "/cost": () => console.log(`Session tokens used: ${totalTokens.toLocaleString()}`),
    "/clear": () => {
      messages.length = 0;
      messages.push({ role: "system", content: SYSTEM_PROMPT });
      totalTokens = 0;
      console.log(chalk.dim("Conversation cleared."));
    },
    "/help": () => {
      console.log(`
${chalk.bold("Chat Slash Commands")}
  /status        Show workspace status
  /obligations   List obligations
  /digest        Show digest history
  /config        Show current config (masked keys)
  /model <name>  Switch LLM model
  /cost          Show token usage
  /clear         Clear conversation
  /help          Show this help
  /quit          Exit chat`);
    },
  };

  try {
    while (true) {
      let userInput: string;
      try {
        userInput = (await prompt()).trim();
      } catch {
        console.log("\n" + chalk.dim("Goodbye."));
        break;
      }

      if (!userInput) continue;

      if (userInput.startsWith("/")) {
        const [cmd, ...rest] = userInput.split(/\s+/);
        const args = rest.join(" ");
        if (cmd === "/quit" || cmd === "/exit") {
          console.log(chalk.dim("Goodbye."));
          break;
        }
        const handler = slashHandlers[cmd.toLowerCase()];
        if (handler) { await handler(args); continue; }
        printError(`Unknown command: ${cmd}. Type /help for available commands.`);
        continue;
      }

      messages.push({ role: "user", content: userInput });

      const llmCfg = cfg.llm;
      while (true) {
        let response: LLMResponse;
        try {
          response = await llmChat(
            messages, TOOL_DEFINITIONS, llmCfg.provider, llmCfg.api_key, llmCfg.model, llmCfg.base_url,
          );
        } catch (err) {
          printError(`LLM error: ${err}`);
          break;
        }

        totalTokens += response.usage.total_tokens;

        const assistantMsg: Record<string, unknown> = { role: "assistant", content: response.content };
        if (response.tool_calls.length > 0) {
          assistantMsg.tool_calls = response.tool_calls.map((tc) => ({
            id: tc.id, type: "function",
            function: { name: tc.name, arguments: JSON.stringify(tc.arguments) },
          }));
          if (!response.content) assistantMsg.content = null;
        }
        messages.push(assistantMsg);

        if (response.tool_calls.length === 0) {
          if (response.content) console.log("\n" + response.content + "\n");
          break;
        }

        for (const tc of response.tool_calls) {
          console.log(chalk.dim(`  ${isWriteTool(tc.name, tc.arguments) ? "\u2699" : "\u2139"} ${tc.name}(${JSON.stringify(tc.arguments).slice(0, 80)})`));
          const result = await executeTool(tc.name, tc.arguments, client);
          const short = result.length > 200 ? result.slice(0, 197) + "..." : result;
          console.log(chalk.dim(`    => ${short}`));
          messages.push({ role: "tool", tool_call_id: tc.id, content: result });
        }
      }
    }
  } finally {
    rl.close();
  }
}
