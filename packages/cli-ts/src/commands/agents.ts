import { requireConfig } from "../config.js";
import { CorpAPIClient } from "../api-client.js";
import { printAgentsTable, printError, printJson, printReferenceSummary, printSuccess, printWriteResult } from "../output.js";
import { ReferenceResolver } from "../references.js";
import { confirm } from "@inquirer/prompts";
import chalk from "chalk";
import type { ApiRecord } from "../types.js";
import { readFileSync, realpathSync } from "node:fs";
import { relative, resolve } from "node:path";

export async function agentsListCommand(opts: { json?: boolean }): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const agents = await client.listAgents();
    await resolver.stabilizeRecords("agent", agents);
    if (opts.json) printJson(agents);
    else if (agents.length === 0) console.log("No agents found.");
    else printAgentsTable(agents);
  } catch (err) { printError(`Failed to fetch agents: ${err}`); process.exit(1); }
}

export async function agentsShowCommand(agentId: string, opts: { json?: boolean }): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const resolvedAgentId = await resolver.resolveAgent(agentId);
    const agent = await client.getAgent(resolvedAgentId);
    await resolver.stabilizeRecord("agent", agent);
    if (opts.json) { printJson(agent); return; }
    console.log(chalk.magenta("─".repeat(40)));
    console.log(chalk.magenta.bold("  Agent Detail"));
    console.log(chalk.magenta("─".repeat(40)));
    console.log(`  ${chalk.bold("Name:")} ${agent.name ?? "N/A"}`);
    console.log(`  ${chalk.bold("Status:")} ${agent.status ?? "N/A"}`);
    console.log(`  ${chalk.bold("Model:")} ${agent.model ?? "N/A"}`);
    printReferenceSummary("agent", agent, { showReuseHint: true });
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
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const data: ApiRecord = { name: opts.name, system_prompt: opts.prompt };
    if (opts.model) data.model = opts.model;
    const result = await client.createAgent(data);
    await resolver.stabilizeRecord("agent", result);
    resolver.rememberFromRecord("agent", result);
    printWriteResult(result, `Agent created: ${result.agent_id ?? result.id ?? "OK"}`, {
      jsonOnly: opts.json,
      referenceKind: "agent",
      showReuseHint: true,
    });
  } catch (err) {
    const msg = String(err);
    if (msg.includes("409") || msg.includes("conflict") || msg.includes("already exists")) {
      printError(
        `Agent name '${opts.name}' is already in use (deleted agents still reserve their name).\n` +
        "  Choose a different name, e.g.: corp agents create --name '...-v2' --prompt '...'",
      );
    } else {
      printError(`Failed to create agent: ${err}`);
    }
    process.exit(1);
  }
}

export async function agentsPauseCommand(agentId: string, opts: { json?: boolean }): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const resolvedAgentId = await resolver.resolveAgent(agentId);
    const result = await client.updateAgent(resolvedAgentId, { status: "paused" });
    printWriteResult(result, `Agent ${resolvedAgentId} paused.`, opts.json);
  } catch (err) { printError(`Failed to pause agent: ${err}`); process.exit(1); }
}

export async function agentsResumeCommand(agentId: string, opts: { json?: boolean }): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const resolvedAgentId = await resolver.resolveAgent(agentId);
    const result = await client.updateAgent(resolvedAgentId, { status: "active" });
    printWriteResult(result, `Agent ${resolvedAgentId} resumed.`, opts.json);
  } catch (err) {
    const msg = String(err);
    if (msg.includes("409") || msg.includes("disabled") || msg.includes("deleted")) {
      printError(
        `Cannot resume agent ${agentId}: the agent may be disabled or deleted.\n` +
        "  Disabled/deleted agents cannot be resumed. Create a new agent instead:\n" +
        "    corp agents create --name '...' --prompt '...'",
      );
    } else {
      printError(`Failed to resume agent: ${err}`);
    }
    process.exit(1);
  }
}

export async function agentsDeleteCommand(agentId: string, opts: { json?: boolean; yes?: boolean }): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const resolvedAgentId = await resolver.resolveAgent(agentId);
    if (!opts.yes) {
      const ok = await confirm({
        message: `Delete agent ${resolvedAgentId}? This cannot be undone.`,
        default: false,
      });
      if (!ok) {
        console.log("Cancelled.");
        return;
      }
    }
    const result = await client.deleteAgent(resolvedAgentId);
    printWriteResult(result, `Agent ${resolvedAgentId} deleted.`, opts.json);
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
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const body = resolveTextInput(opts.body, opts.bodyFile, "body", true);
    const resolvedAgentId = await resolver.resolveAgent(agentId);
    const result = await client.sendAgentMessage(resolvedAgentId, body!);
    printWriteResult(result, `Message sent. Execution: ${result.execution_id ?? "OK"}`, opts.json);
  } catch (err) {
    const msg = String(err);
    if (msg.includes("409")) {
      printError(
        `Cannot message agent: the agent must be active or paused (not disabled/deleted).\n` +
        "  Check agent status: corp agents show " + agentId + "\n" +
        "  Resume a paused agent: corp agents resume " + agentId,
      );
    } else {
      printError(`Failed to send message: ${err}`);
    }
    process.exit(1);
  }
}

export async function agentsExecutionCommand(
  agentId: string,
  executionId: string,
  opts: { json?: boolean },
): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const resolvedAgentId = await resolver.resolveAgent(agentId);
    const result = await client.getAgentExecution(resolvedAgentId, executionId);
    if (opts.json) { printJson(result); return; }
    console.log(chalk.magenta("─".repeat(40)));
    console.log(chalk.magenta.bold("  Execution Status"));
    console.log(chalk.magenta("─".repeat(40)));
    console.log(`  ${chalk.bold("Execution:")} ${executionId}`);
    console.log(`  ${chalk.bold("Agent:")} ${resolvedAgentId}`);
    console.log(`  ${chalk.bold("Status:")} ${result.status ?? "N/A"}`);
    if (result.started_at) console.log(`  ${chalk.bold("Started:")} ${result.started_at}`);
    if (result.completed_at) console.log(`  ${chalk.bold("Completed:")} ${result.completed_at}`);
    console.log(chalk.magenta("─".repeat(40)));
  } catch (err) { printError(`Failed to get execution: ${err}`); process.exit(1); }
}

export async function agentsExecutionResultCommand(
  agentId: string,
  executionId: string,
  opts: { json?: boolean },
): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const resolvedAgentId = await resolver.resolveAgent(agentId);
    const result = await client.getAgentExecutionResult(resolvedAgentId, executionId);
    if (opts.json) { printJson(result); return; }
    printSuccess(`Result for execution ${executionId}:`);
    printJson(result);
  } catch (err) { printError(`Failed to get execution result: ${err}`); process.exit(1); }
}

export async function agentsKillCommand(
  agentId: string,
  executionId: string,
  opts: { yes?: boolean; json?: boolean },
): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const resolvedAgentId = await resolver.resolveAgent(agentId);
    if (!opts.yes) {
      const ok = await confirm({ message: `Kill execution ${executionId}?`, default: false });
      if (!ok) { console.log("Cancelled."); return; }
    }
    const result = await client.killAgentExecution(resolvedAgentId, executionId);
    printWriteResult(result, `Execution ${executionId} killed.`, opts.json);
  } catch (err) { printError(`Failed to kill execution: ${err}`); process.exit(1); }
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
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const instructions = resolveTextInput(
      opts.instructions,
      opts.instructionsFile,
      "instructions",
    );
    const resolvedAgentId = await resolver.resolveAgent(agentId);
    const result = await client.addAgentSkill(resolvedAgentId, {
      name: opts.name,
      description: opts.description,
      parameters: instructions ? { instructions } : {},
    });
    printWriteResult(result, `Skill '${opts.name}' added to agent ${resolvedAgentId}.`, opts.json);
  } catch (err) { printError(`Failed to add skill: ${err}`); process.exit(1); }
}
