import type { CommandDef, CommandContext } from "./types.js";
import {
  printAgentsTable,
  printError,
  printJson,
  printReferenceSummary,
  printSuccess,
  printWriteResult,
} from "../output.js";
import { confirm } from "@inquirer/prompts";
import chalk from "chalk";
import type { ApiRecord } from "../types.js";
import { readFileSync, realpathSync } from "node:fs";
import { relative, resolve } from "node:path";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

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
  if (inlineText) return inlineText;
  if (required) throw new Error(`Provide --${label} or --${label}-file.`);
  return undefined;
}

// ---------------------------------------------------------------------------
// Agent registry entries
// ---------------------------------------------------------------------------

export const agentCommands: CommandDef[] = [
  // --- agents (list) ---
  {
    name: "agents",
    description: "Agent management",
    route: { method: "GET", path: "/v1/agents" },
    display: {
      title: "Agents",
      cols: ["name>Name", "status>Status", "model>Model", "#agent_id|id>ID"],
    },
    handler: async (ctx) => {
      const agents = await ctx.client.listAgents();
      await ctx.resolver.stabilizeRecords("agent", agents);
      if (ctx.opts.json) { ctx.writer.json(agents); return; }
      if (agents.length === 0) { ctx.writer.writeln("No agents found."); return; }
      printAgentsTable(agents);
    },
    examples: [
      "corp agents",
      'corp agents create --name "bookkeeper" --prompt "You manage accounts payable"',
      'corp agents message @last:agent --body "Process this month\'s invoices"',
      'corp agents skill @last:agent --name invoice-processing --description "Process AP invoices"',
      "corp agents execution @last:agent <execution-id>",
      "corp agents kill @last:agent <execution-id>",
    ],
  },

  // --- agents show <agent-ref> ---
  {
    name: "agents show",
    description: "Show agent detail",
    route: { method: "GET", path: "/v1/agents/{pos}/resolved" },
    args: [{ name: "agent-ref", required: true, description: "Agent reference" }],
    display: { title: "Agent Detail" },
    handler: async (ctx) => {
      const agentRef = ctx.positional[0];
      const resolvedAgentId = await ctx.resolver.resolveAgent(agentRef);
      const agent = await ctx.client.getAgent(resolvedAgentId);
      await ctx.resolver.stabilizeRecord("agent", agent);
      if (ctx.opts.json) { ctx.writer.json(agent); return; }
      console.log(chalk.magenta("\u2500".repeat(40)));
      console.log(chalk.magenta.bold("  Agent Detail"));
      console.log(chalk.magenta("\u2500".repeat(40)));
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
        console.log(`  ${chalk.bold("Skills:")} ${(agent.skills as Array<{ name?: string }>).map((s) => s.name ?? "?").join(", ")}`);
      }
      console.log(chalk.magenta("\u2500".repeat(40)));
    },
  },

  // --- agents create ---
  {
    name: "agents create",
    description: "Create a new agent",
    route: { method: "POST", path: "/v1/agents" },
    options: [
      { flags: "--name <name>", description: "Agent name", required: true },
      { flags: "--prompt <prompt>", description: "System prompt", required: true },
      { flags: "--model <model>", description: "Model" },
    ],
    handler: async (ctx) => {
      const data: ApiRecord = {
        name: ctx.opts.name as string,
        system_prompt: ctx.opts.prompt as string,
      };
      if (ctx.opts.model) data.model = ctx.opts.model as string;
      try {
        const result = await ctx.client.createAgent(data);
        await ctx.resolver.stabilizeRecord("agent", result);
        ctx.resolver.rememberFromRecord("agent", result);
        ctx.writer.writeResult(result, `Agent created: ${result.agent_id ?? result.id ?? "OK"}`, {
          jsonOnly: ctx.opts.json,
          referenceKind: "agent",
          showReuseHint: true,
        });
      } catch (err) {
        const msg = String(err);
        if (msg.includes("409") || msg.includes("conflict") || msg.includes("already exists")) {
          printError(
            `Agent name '${ctx.opts.name}' is already in use (deleted agents still reserve their name).\n` +
            "  Choose a different name, e.g.: corp agents create --name '...-v2' --prompt '...'",
          );
        } else {
          printError(`Failed to create agent: ${err}`);
        }
        process.exit(1);
      }
    },
    produces: { kind: "agent" },
    successTemplate: "Agent created: {name}",
  },

  // --- agents pause <agent-ref> ---
  {
    name: "agents pause",
    description: "Pause an agent",
    route: { method: "POST", path: "/v1/agents/{pos}/pause" },
    args: [{ name: "agent-ref", required: true, description: "Agent reference" }],
    handler: async (ctx) => {
      const agentRef = ctx.positional[0];
      const resolvedAgentId = await ctx.resolver.resolveAgent(agentRef);
      const result = await ctx.client.updateAgent(resolvedAgentId, { status: "paused" });
      ctx.writer.writeResult(result, `Agent ${resolvedAgentId} paused.`, { jsonOnly: ctx.opts.json });
    },
  },

  // --- agents resume <agent-ref> ---
  {
    name: "agents resume",
    description: "Resume a paused agent",
    route: { method: "POST", path: "/v1/agents/{pos}/resume" },
    args: [{ name: "agent-ref", required: true, description: "Agent reference" }],
    handler: async (ctx) => {
      const agentRef = ctx.positional[0];
      const resolvedAgentId = await ctx.resolver.resolveAgent(agentRef);
      try {
        const result = await ctx.client.updateAgent(resolvedAgentId, { status: "active" });
        ctx.writer.writeResult(result, `Agent ${resolvedAgentId} resumed.`, { jsonOnly: ctx.opts.json });
      } catch (err) {
        const msg = String(err);
        if (msg.includes("409") || msg.includes("disabled") || msg.includes("deleted")) {
          printError(
            `Cannot resume agent ${agentRef}: the agent may be disabled or deleted.\n` +
            "  Disabled/deleted agents cannot be resumed. Create a new agent instead:\n" +
            "    corp agents create --name '...' --prompt '...'",
          );
        } else {
          printError(`Failed to resume agent: ${err}`);
        }
        process.exit(1);
      }
    },
  },

  // --- agents delete <agent-ref> ---
  {
    name: "agents delete",
    description: "Delete an agent",
    route: { method: "DELETE", path: "/v1/agents/{pos}" },
    args: [{ name: "agent-ref", required: true, description: "Agent reference" }],
    options: [
      { flags: "--yes, -y", description: "Skip confirmation prompt" },
    ],
    handler: async (ctx) => {
      const agentRef = ctx.positional[0];
      const resolvedAgentId = await ctx.resolver.resolveAgent(agentRef);
      if (!ctx.opts.yes) {
        const ok = await confirm({
          message: `Delete agent ${resolvedAgentId}? This cannot be undone.`,
          default: false,
        });
        if (!ok) { console.log("Cancelled."); return; }
      }
      const result = await ctx.client.deleteAgent(resolvedAgentId);
      ctx.writer.writeResult(result, `Agent ${resolvedAgentId} deleted.`, { jsonOnly: ctx.opts.json });
    },
  },

  // --- agents message <agent-ref> ---
  {
    name: "agents message",
    description: "Send a message to an agent",
    route: { method: "POST", path: "/v1/agents/{pos}/messages" },
    args: [{ name: "agent-ref", required: true, description: "Agent reference" }],
    options: [
      { flags: "--body <text>", description: "Message text" },
      { flags: "--body-file <path>", description: "Read the message body from a file" },
    ],
    handler: async (ctx) => {
      const agentRef = ctx.positional[0];
      const body = resolveTextInput(ctx.opts.body as string | undefined, ctx.opts.bodyFile as string | undefined, "body", true);
      const resolvedAgentId = await ctx.resolver.resolveAgent(agentRef);
      try {
        const result = await ctx.client.sendAgentMessage(resolvedAgentId, body!);
        ctx.writer.writeResult(result, `Message sent. Execution: ${result.execution_id ?? "OK"}`, { jsonOnly: ctx.opts.json });
      } catch (err) {
        const msg = String(err);
        if (msg.includes("409")) {
          printError(
            `Cannot message agent: the agent must be active or paused (not disabled/deleted).\n` +
            "  Check agent status: corp agents show " + agentRef + "\n" +
            "  Resume a paused agent: corp agents resume " + agentRef,
          );
        } else {
          printError(`Failed to send message: ${err}`);
        }
        process.exit(1);
      }
    },
  },

  // --- agents skill <agent-ref> ---
  {
    name: "agents skill",
    description: "Add a skill to an agent",
    route: { method: "POST", path: "/v1/agents/{pos}/skills" },
    args: [{ name: "agent-ref", required: true, description: "Agent reference" }],
    options: [
      { flags: "--name <name>", description: "Skill name", required: true },
      { flags: "--description <desc>", description: "Skill description", required: true },
      { flags: "--instructions <text>", description: "Instructions" },
      { flags: "--instructions-file <path>", description: "Read skill instructions from a file" },
    ],
    handler: async (ctx) => {
      const agentRef = ctx.positional[0];
      const instructions = resolveTextInput(
        ctx.opts.instructions as string | undefined,
        ctx.opts.instructionsFile as string | undefined,
        "instructions",
      );
      const resolvedAgentId = await ctx.resolver.resolveAgent(agentRef);
      const result = await ctx.client.addAgentSkill(resolvedAgentId, {
        name: ctx.opts.name as string,
        description: ctx.opts.description as string,
        parameters: instructions ? { instructions } : {},
      });
      ctx.writer.writeResult(result, `Skill '${ctx.opts.name}' added to agent ${resolvedAgentId}.`, { jsonOnly: ctx.opts.json });
    },
  },

  // --- agents execution <agent-ref> <execution-id> ---
  {
    name: "agents execution",
    description: "Check execution status",
    route: { method: "GET", path: "/v1/agents/{pos}/executions/{pos2}" },
    args: [
      { name: "agent-ref", required: true, description: "Agent reference" },
      { name: "execution-id", required: true, description: "Execution ID" },
    ],
    display: { title: "Execution Status" },
    handler: async (ctx) => {
      const agentRef = ctx.positional[0];
      const executionId = ctx.positional[1];
      const resolvedAgentId = await ctx.resolver.resolveAgent(agentRef);
      const result = await ctx.client.getAgentExecution(resolvedAgentId, executionId);
      if (ctx.opts.json) { ctx.writer.json(result); return; }
      console.log(chalk.magenta("\u2500".repeat(40)));
      console.log(chalk.magenta.bold("  Execution Status"));
      console.log(chalk.magenta("\u2500".repeat(40)));
      console.log(`  ${chalk.bold("Execution:")} ${executionId}`);
      console.log(`  ${chalk.bold("Agent:")} ${resolvedAgentId}`);
      console.log(`  ${chalk.bold("Status:")} ${result.status ?? "N/A"}`);
      if (result.started_at) console.log(`  ${chalk.bold("Started:")} ${result.started_at}`);
      if (result.completed_at) console.log(`  ${chalk.bold("Completed:")} ${result.completed_at}`);
      console.log(chalk.magenta("\u2500".repeat(40)));
    },
  },

  // --- agents execution-result <agent-ref> <execution-id> ---
  {
    name: "agents execution-result",
    description: "Get execution result",
    route: { method: "GET", path: "/v1/agents/{pos}/executions/{pos2}/result" },
    args: [
      { name: "agent-ref", required: true, description: "Agent reference" },
      { name: "execution-id", required: true, description: "Execution ID" },
    ],
    display: { title: "Execution Result" },
    handler: async (ctx) => {
      const agentRef = ctx.positional[0];
      const executionId = ctx.positional[1];
      const resolvedAgentId = await ctx.resolver.resolveAgent(agentRef);
      const result = await ctx.client.getAgentExecutionResult(resolvedAgentId, executionId);
      if (ctx.opts.json) { ctx.writer.json(result); return; }
      ctx.writer.success(`Result for execution ${executionId}:`);
      printJson(result);
    },
  },

  // --- agents kill <agent-ref> <execution-id> ---
  {
    name: "agents kill",
    description: "Kill a running execution",
    route: { method: "POST", path: "/v1/agents/{pos}/executions/{pos2}/kill" },
    args: [
      { name: "agent-ref", required: true, description: "Agent reference" },
      { name: "execution-id", required: true, description: "Execution ID" },
    ],
    options: [
      { flags: "--yes", description: "Skip confirmation" },
    ],
    handler: async (ctx) => {
      const agentRef = ctx.positional[0];
      const executionId = ctx.positional[1];
      const resolvedAgentId = await ctx.resolver.resolveAgent(agentRef);
      if (!ctx.opts.yes) {
        const ok = await confirm({ message: `Kill execution ${executionId}?`, default: false });
        if (!ok) { console.log("Cancelled."); return; }
      }
      const result = await ctx.client.killAgentExecution(resolvedAgentId, executionId);
      ctx.writer.writeResult(result, `Execution ${executionId} killed.`, { jsonOnly: ctx.opts.json });
    },
  },

  // ── Auto-generated from OpenAPI ──────────────────────────────
  {
    name: "agents executions-logs",
    description: "/v1/agents/{agent_id}/executions/{execution_id}/logs",
    route: { method: "GET", path: "/v1/agents/{pos}/executions/{pos2}/logs" },
    args: [{ name: "agent-id", required: true, description: "Agent Id" }, { name: "execution-id", required: true, description: "Execution Id" }],
  },


  // ── Auto-generated from OpenAPI ──────────────────────────────
  {
    name: "agents messages",
    description: "/v1/agents/{agent_id}/messages/{message_id}",
    route: { method: "GET", path: "/v1/agents/{pos}/messages/{pos2}" },
    args: [{ name: "agent-id", required: true, description: "Agent Id" }, { name: "message-id", required: true, description: "Message Id" }],
  },

];