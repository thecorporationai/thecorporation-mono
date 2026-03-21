import type { CommandDef, WebRouteEntry } from "./types.js";

// Domain registries — uncomment as they're created
import { workspaceCommands } from "./workspace.js";
import { entityCommands } from "./entities.js";
import { formationCommands } from "./formation.js";
import { capTableCommands } from "./cap-table.js";
import { financeCommands } from "./finance.js";
import { governanceCommands } from "./governance.js";
import { documentCommands } from "./documents.js";
import { complianceCommands } from "./compliance.js";
import { agentCommands } from "./agents.js";
import { workItemCommands } from "./work-items.js";
import { serviceCommands } from "./services.js";
import { adminCommands } from "./admin.js";
import { executionCommands } from "./execution.js";
import { secretProxyCommands } from "./secret-proxies.js";
import { treasuryCommands } from "./treasury.js";
import { branchCommands } from "./branches.js";

export const registry: CommandDef[] = [
  ...workspaceCommands,
  ...entityCommands,
  ...formationCommands,
  ...capTableCommands,
  ...financeCommands,
  ...governanceCommands,
  ...documentCommands,
  ...complianceCommands,
  ...agentCommands,
  ...workItemCommands,
  ...serviceCommands,
  ...adminCommands,
  ...executionCommands,
  ...secretProxyCommands,
  ...treasuryCommands,
  ...branchCommands,
];

/** Attach produces/successTemplate to a web-route entry if present on the CommandDef */
function attachProducesFields(entry: WebRouteEntry, cmd: CommandDef): WebRouteEntry {
  if (cmd.produces) entry.produces = cmd.produces;
  if (cmd.successTemplate) entry.successTemplate = cmd.successTemplate;
  return entry;
}

/** Generate web-routes.json manifest from registry */
export function generateWebRoutes(commands: CommandDef[]): { commands: Record<string, WebRouteEntry> } {
  const entries: Record<string, WebRouteEntry> = {};
  for (const cmd of commands) {
    if (cmd.hidden) continue;
    if (cmd.local) {
      entries[cmd.name] = { local: true };
      continue;
    }
    const entry: WebRouteEntry = {};
    // Route info — always emit path when available so the web generic executor can use it
    if (cmd.route) {
      entry.method = cmd.route.method;
      entry.path = cmd.route.path;
      if (cmd.route.method !== "GET") entry.write = true;
    }
    // Entity scoping
    if (cmd.entity !== undefined) entry.entity = cmd.entity;
    // Display metadata
    if (cmd.display) {
      entry.title = cmd.display.title;
      if (cmd.display.cols) entry.cols = cmd.display.cols;
      if (cmd.display.listKey) entry.listKey = cmd.display.listKey;
    }
    if (cmd.optQP) entry.optQP = cmd.optQP;
    // Custom handler flag — tells web CLI a CUSTOM handler should override generic
    if (cmd.handler) entry.custom = true;
    // Skip commands with no route and no handler (nothing the web CLI can do)
    if (!cmd.route && !cmd.handler) continue;
    entries[cmd.name] = attachProducesFields(entry, cmd);
  }
  return { commands: entries };
}

/** Generate cli-schema.json from registry (for tab completion) */
export function generateSchema(commands: CommandDef[], programName: string, version: string): unknown {
  // Build hierarchical structure from flat command list
  // Group by parent: "governance seats" -> parent "governance", child "seats"

  interface SchemaCmd {
    path: string;
    name: string;
    description: string;
    aliases: string[];
    arguments: { name: string; required: boolean; variadic: boolean }[];
    options: {
      flags: string;
      name: string;
      description: string;
      required: boolean;
      mandatory: boolean;
      variadic: boolean;
      choices?: string[];
    }[];
    examples?: string[];
    subcommands: SchemaCmd[];
  }

  const parentMap = new Map<string, SchemaCmd>();
  const topLevel: SchemaCmd[] = [];

  for (const cmd of commands) {
    if (cmd.hidden) continue;
    const parts = cmd.name.split(" ");
    const entry: SchemaCmd = {
      path: `${programName} ${cmd.name}`,
      name: parts[parts.length - 1],
      description: cmd.description,
      aliases: cmd.aliases || [],
      arguments: (cmd.args || []).map((a) => ({
        name: a.name,
        required: a.required ?? false,
        variadic: a.variadic ?? false,
      })),
      options: [
        // Always include --json
        {
          flags: "--json",
          name: "json",
          description: "Output as JSON",
          required: false,
          mandatory: false,
          variadic: false,
        },
        // Entity option if applicable
        ...(cmd.entity
          ? [
              {
                flags: "--entity-id <ref>",
                name: "entityId",
                description: "Entity reference",
                required: false,
                mandatory: false,
                variadic: false,
              },
            ]
          : []),
        // Dry run if applicable
        ...(cmd.dryRun
          ? [
              {
                flags: "--dry-run",
                name: "dryRun",
                description: "Preview without executing",
                required: false,
                mandatory: false,
                variadic: false,
              },
            ]
          : []),
        // Command-specific options
        ...(cmd.options || []).map((o) => ({
          flags: o.flags,
          name: o.flags.replace(/^--/, "").split(/[\s,<]/)[0],
          description: o.description,
          required: o.required ?? false,
          mandatory: o.required ?? false,
          variadic: false,
          ...(o.choices && { choices: o.choices }),
        })),
      ],
      ...(cmd.examples?.length ? { examples: cmd.examples } : {}),
      subcommands: [],
    };

    if (parts.length === 1) {
      topLevel.push(entry);
      parentMap.set(parts[0], entry);
    } else {
      const parentName = parts[0];
      let parent = parentMap.get(parentName);
      if (!parent) {
        // Auto-create parent stub
        parent = {
          path: `${programName} ${parentName}`,
          name: parentName,
          description: "",
          aliases: [],
          arguments: [],
          options: [],
          subcommands: [],
        };
        topLevel.push(parent);
        parentMap.set(parentName, parent);
      }
      parent.subcommands.push(entry);
    }
  }

  return {
    name: programName,
    version,
    description: "corp — Corporate governance from the terminal",
    generated_at: new Date().toISOString(),
    commands: topLevel,
  };
}
