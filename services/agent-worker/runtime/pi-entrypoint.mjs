#!/usr/bin/env node

import { spawn } from "child_process";
import {
  copyFileSync,
  existsSync,
  mkdirSync,
  readFileSync,
  writeFileSync,
} from "fs";
import { join } from "path";

const WORKSPACE = process.env.WORKSPACE || "/workspace";

let agentConfig = {};
let message = {};

try {
  agentConfig = JSON.parse(process.env.AGENT_CONFIG || "{}");
} catch (error) {
  console.error("Failed to parse AGENT_CONFIG:", error.message);
}

try {
  message = JSON.parse(process.env.MESSAGE || "{}");
} catch (error) {
  console.error("Failed to parse MESSAGE:", error.message);
}

const prompt = message.content || message.text || "Hello";
const model = agentConfig.model || "anthropic/claude-sonnet-4-6";
const systemPrompt = agentConfig.system_prompt || "";

const corpConfigDir = join(WORKSPACE, ".corp");
mkdirSync(corpConfigDir, { recursive: true });

const corpConfig = {
  api_url: agentConfig.api_url || process.env.CORP_API_URL || "http://host.docker.internal:8000",
  api_key: agentConfig.api_key || "",
  workspace_id: agentConfig.workspace_id || "",
};
writeFileSync(join(corpConfigDir, "config.json"), JSON.stringify(corpConfig, null, 2));

const piDir = join(WORKSPACE, ".pi");
const piAgentDir = join(piDir, "agent");
const piExtDir = join(piDir, "extensions");
mkdirSync(piAgentDir, { recursive: true });
mkdirSync(piExtDir, { recursive: true });

let systemPromptFull = systemPrompt;
const skills = agentConfig.skills || [];
if (skills.length > 0) {
  const skillText = skills
    .filter((skill) => skill.enabled !== false)
    .map((skill) => `### ${skill.name}\n${skill.description || ""}\n${skill.instructions || ""}`)
    .join("\n\n");
  if (skillText) {
    systemPromptFull += "\n\n## Skills\n\n" + skillText;
  }
}

if (systemPromptFull) {
  writeFileSync(join(piDir, "AGENT.md"), systemPromptFull);
}

const mcpServers = agentConfig.mcp_servers || [];
if (mcpServers.length > 0) {
  const mcpConfig = {};
  for (const server of mcpServers) {
    mcpConfig[server.name] = {
      command: server.command,
      args: server.args || [],
      env: {
        ...(server.env || {}),
        SECRETS_PROXY_URL: process.env.SECRETS_PROXY_URL || "",
      },
    };
  }
  writeFileSync(
    join(piAgentDir, "mcp.json"),
    JSON.stringify({ mcpServers: mcpConfig }, null, 2),
  );
}

const httpTools = agentConfig.tools || [];
if (httpTools.length > 0) {
  const toolRegistrations = httpTools
    .map((tool) => {
      const headers = JSON.stringify(tool.headers || {});
      const params = JSON.stringify(tool.parameters || {});
      return `
pi.registerTool({
  name: ${JSON.stringify(tool.name)},
  description: ${JSON.stringify(tool.description || tool.name)},
  parameters: ${params},
  async execute(args) {
    const url = ${JSON.stringify(tool.url)};
    const method = ${JSON.stringify(tool.method || "GET")};
    const requestHeaders = { "Content-Type": "application/json", ...${headers} };
    const requestOptions = { method, headers: requestHeaders };
    if (method !== "GET" && method !== "HEAD") {
      requestOptions.body = JSON.stringify(args);
    }
    const response = await fetch(url, requestOptions);
    const text = await response.text();
    try { return JSON.parse(text); } catch { return text; }
  },
});`;
    })
    .join("\n");

  writeFileSync(
    join(piExtDir, "agent-tools.mjs"),
    `// Auto-generated HTTP tools from agent config\n${toolRegistrations}\n`,
  );
}

const globalCorpExt = "/opt/corp-extension/extension/corp.ts";
if (existsSync(globalCorpExt)) {
  copyFileSync(globalCorpExt, join(piExtDir, "corp.ts"));
}

const llmProxyUrl = process.env.CORP_LLM_PROXY_URL || "";
const openrouterKey = process.env.OPENROUTER_API_KEY || "";
if (llmProxyUrl || openrouterKey) {
  const modelsConfig = {
    providers: {
      openrouter: {
        baseUrl: llmProxyUrl || "https://openrouter.ai/api/v1",
        apiKey: openrouterKey,
      },
    },
  };
  writeFileSync(join(piAgentDir, "models.json"), JSON.stringify(modelsConfig, null, 2));
}

const env = {
  ...process.env,
  CORP_AUTO_APPROVE: "1",
  CORP_CONFIG_DIR: corpConfigDir,
  PI_CODING_AGENT_DIR: piAgentDir,
  HOME: WORKSPACE,
};

let provider = "openrouter";
if (model.startsWith("anthropic/")) provider = "openrouter";
else if (model.startsWith("openai/")) provider = "openrouter";

const piProcess = spawn(
  "pi",
  ["-p", prompt, "--provider", provider, "--model", model],
  {
    cwd: WORKSPACE,
    env,
    stdio: ["ignore", "pipe", "pipe"],
  },
);

let stdout = "";
let stderr = "";

piProcess.stdout.on("data", (data) => {
  stdout += data.toString();
});
piProcess.stderr.on("data", (data) => {
  stderr += data.toString();
});

piProcess.on("close", (code) => {
  if (stderr) {
    process.stderr.write(stderr);
  }

  const resultPath = join(WORKSPACE, ".result.json");
  let result = null;

  if (existsSync(resultPath)) {
    try {
      result = JSON.parse(readFileSync(resultPath, "utf-8"));
    } catch {
      result = null;
    }
  }

  if (!result) {
    result = {
      success: code === 0,
      reason: code !== 0 ? `pi exited with code ${code}` : null,
      final_response: stdout.trim().split("\n").pop() || "",
      tool_calls_count: 0,
      turns: 0,
      input_tokens: 0,
      output_tokens: 0,
      transcript: [],
      tasks: [],
    };
  }

  process.stdout.write(JSON.stringify(result) + "\n");
  process.exit(code || 0);
});

piProcess.on("error", (error) => {
  const result = {
    success: false,
    reason: `Failed to spawn pi: ${error.message}`,
    final_response: "",
    tool_calls_count: 0,
    turns: 0,
    input_tokens: 0,
    output_tokens: 0,
    transcript: [],
    tasks: [],
  };
  process.stdout.write(JSON.stringify(result) + "\n");
  process.exit(1);
});
