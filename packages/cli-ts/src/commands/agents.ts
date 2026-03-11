import { requireConfig } from "../config.js";
import { CorpAPIClient } from "../api-client.js";
import { printAgentsTable, printError, printJson, printWriteResult } from "../output.js";
import chalk from "chalk";
import type { ApiRecord } from "../types.js";
import { readFileSync, realpathSync } from "node:fs";
import { relative, resolve } from "node:path";

export async function agentsListCommand(opts: { json?: boolean }): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    const agents = await client.listAgents();
    if (opts.json) printJson(agents);
    else if (agents.length === 0) console.log("No agents found.");
    else printAgentsTable(agents);
  } catch (err) { printError(`Failed to fetch agents: ${err}`); process.exit(1); }
}

export async function agentsShowCommand(agentId: string, opts: { json?: boolean }): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    const agent = await client.getAgent(agentId);
    if (opts.json) { printJson(agent); return; }
    console.log(chalk.magenta("─".repeat(40)));
    console.log(chalk.magenta.bold("  Agent Detail"));
    console.log(chalk.magenta("─".repeat(40)));
    console.log(`  ${chalk.bold("Name:")} ${agent.name ?? "N/A"}`);
    console.log(`  ${chalk.bold("Status:")} ${agent.status ?? "N/A"}`);
    console.log(`  ${chalk.bold("Model:")} ${agent.model ?? "N/A"}`);
    console.log(`  ${chalk.bold("ID:")} ${agent.agent_id ?? "N/A"}`);
    if (agent.system_prompt) {
      let prompt = String(agent.system_prompt);
      if (prompt.length > 100) prompt = prompt.slice(0, 97) + "...";
      console.log(`  ${chalk.bold("Prompt:")} ${prompt}`);
    }
    if (agent.skills && Array.isArray(agent.skills) && agent.skills.length > 0) {
      console.log(`  ${chalk.bold("Skills:")} ${(agent.skills as Array<{name?: string}>).map((s) => s.name ?? "?").join(", ")}`);
    }
    console.log(chalk.magenta("─".repeat(40)));
  } catch (err) { printError(`Failed to fetch agent: ${err}`); process.exit(1); }
}

export async function agentsCreateCommand(opts: {
  name: string;
  prompt: string;
  model?: string;
  json?: boolean;
}): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    const data: ApiRecord = { name: opts.name, system_prompt: opts.prompt };
    if (opts.model) data.model = opts.model;
    const result = await client.createAgent(data);
    printWriteResult(result, `Agent created: ${result.agent_id ?? result.id ?? "OK"}`, opts.json);
  } catch (err) { printError(`Failed to create agent: ${err}`); process.exit(1); }
}

export async function agentsPauseCommand(agentId: string, opts: { json?: boolean }): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    const result = await client.updateAgent(agentId, { status: "paused" });
    printWriteResult(result, `Agent ${agentId} paused.`, opts.json);
  } catch (err) { printError(`Failed to pause agent: ${err}`); process.exit(1); }
}

export async function agentsResumeCommand(agentId: string, opts: { json?: boolean }): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    const result = await client.updateAgent(agentId, { status: "active" });
    printWriteResult(result, `Agent ${agentId} resumed.`, opts.json);
  } catch (err) { printError(`Failed to resume agent: ${err}`); process.exit(1); }
}

export async function agentsDeleteCommand(agentId: string, opts: { json?: boolean }): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    const result = await client.deleteAgent(agentId);
    printWriteResult(result, `Agent ${agentId} deleted.`, opts.json);
  } catch (err) { printError(`Failed to delete agent: ${err}`); process.exit(1); }
}

function resolveTextInput(
  inlineText: string | undefined,
  filePath: string | undefined,
  label: string,
  required = false,
): string | undefined {
  if (inlineText && filePath) {
    throw new Error(`Pass either --${label} or --${label}-file, not both.`);
  }
  if (filePath) {
    if (process.env.CORP_ALLOW_UNSAFE_FILE_INPUT === "1") {
      return readFileSync(filePath, "utf8");
    }
    const resolvedFile = realpathSync(resolve(filePath));
    const workingTreeRoot = realpathSync(process.cwd());
    const rel = relative(workingTreeRoot, resolvedFile);
    if (rel === "" || (!rel.startsWith("..") && !rel.startsWith("/"))) {
      return readFileSync(resolvedFile, "utf8");
    }
    throw new Error(
      `--${label}-file must stay inside the current working directory unless CORP_ALLOW_UNSAFE_FILE_INPUT=1 is set.`,
    );
  }
  if (inlineText) {
    return inlineText;
  }
  if (required) {
    throw new Error(`Provide --${label} or --${label}-file.`);
  }
  return undefined;
}

export async function agentsMessageCommand(
  agentId: string,
  opts: { body?: string; bodyFile?: string; json?: boolean },
): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    const body = resolveTextInput(opts.body, opts.bodyFile, "body", true);
    const result = await client.sendAgentMessage(agentId, body!);
    printWriteResult(result, `Message sent. Execution: ${result.execution_id ?? "OK"}`, opts.json);
  } catch (err) { printError(`Failed to send message: ${err}`); process.exit(1); }
}

export async function agentsExecutionsCommand(agentId: string, _opts: { json?: boolean }): Promise<void> {
  // No list-executions endpoint exists yet; individual executions can be
  // queried via GET /v1/agents/{agent_id}/executions/{execution_id}.
  printError(
    `Listing executions is not yet supported.\n` +
    `  To inspect a specific run, use the execution ID returned by "agents message":\n` +
    `  GET /v1/agents/${agentId}/executions/<execution-id>`,
  );
  process.exit(1);
}


export async function agentsSkillCommand(agentId: string, opts: {
  name: string;
  description: string;
  instructions?: string;
  instructionsFile?: string;
  json?: boolean;
}): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    const instructions = resolveTextInput(
      opts.instructions,
      opts.instructionsFile,
      "instructions",
    );
    const result = await client.addAgentSkill(agentId, {
      name: opts.name,
      description: opts.description,
      parameters: instructions ? { instructions } : {},
    });
    printWriteResult(result, `Skill '${opts.name}' added to agent ${agentId}.`, opts.json);
  } catch (err) { printError(`Failed to add skill: ${err}`); process.exit(1); }
}
