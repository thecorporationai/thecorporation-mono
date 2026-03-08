import { requireConfig } from "../config.js";
import { CorpAPIClient } from "../api-client.js";
import { printAgentsTable, printError, printSuccess, printJson } from "../output.js";
import chalk from "chalk";
import type { ApiRecord } from "../types.js";

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

export async function agentsCreateCommand(opts: { name: string; prompt: string; model?: string }): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    const data: ApiRecord = { name: opts.name, system_prompt: opts.prompt };
    if (opts.model) data.model = opts.model;
    const result = await client.createAgent(data);
    printSuccess(`Agent created: ${result.agent_id ?? result.id ?? "OK"}`);
    printJson(result);
  } catch (err) { printError(`Failed to create agent: ${err}`); process.exit(1); }
}

export async function agentsPauseCommand(agentId: string): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    await client.updateAgent(agentId, { status: "paused" });
    printSuccess(`Agent ${agentId} paused.`);
  } catch (err) { printError(`Failed to pause agent: ${err}`); process.exit(1); }
}

export async function agentsResumeCommand(agentId: string): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    await client.updateAgent(agentId, { status: "active" });
    printSuccess(`Agent ${agentId} resumed.`);
  } catch (err) { printError(`Failed to resume agent: ${err}`); process.exit(1); }
}

export async function agentsDeleteCommand(agentId: string): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    await client.deleteAgent(agentId);
    printSuccess(`Agent ${agentId} deleted.`);
  } catch (err) { printError(`Failed to delete agent: ${err}`); process.exit(1); }
}

export async function agentsMessageCommand(agentId: string, opts: { body: string }): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    const result = await client.sendAgentMessage(agentId, opts.body);
    printSuccess(`Message sent. Execution: ${result.execution_id ?? "OK"}`);
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
  name: string; description: string; instructions?: string;
}): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    const result = await client.addAgentSkill(agentId, {
      name: opts.name, description: opts.description, parameters: opts.instructions ? { instructions: opts.instructions } : {},
    });
    printSuccess(`Skill '${opts.name}' added to agent ${agentId}.`);
    printJson(result);
  } catch (err) { printError(`Failed to add skill: ${err}`); process.exit(1); }
}
