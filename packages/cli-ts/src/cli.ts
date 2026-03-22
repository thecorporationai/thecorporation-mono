import { Command, Option } from "commander";
import type { CommandDef, CommandContext } from "./registry/types.js";
import { executeGenericRead, executeGenericWrite } from "./generic-executor.js";
import { createWriter } from "./writer.js";
import { requireConfig, resolveEntityId } from "./config.js";
import { CorpAPIClient } from "./api-client.js";
import { ReferenceResolver } from "./references.js";

/**
 * Build a Commander program from a flat array of CommandDef objects.
 *
 * Top-level commands (name has no space) are added directly to the program.
 * Sub-commands (e.g. "governance seats") are grouped under a parent command
 * whose name is the first word.
 */
export function buildCLI(commands: CommandDef[], version: string): Command {
  const program = new Command();
  program
    .name("corp")
    .description("corp — Corporate governance from the terminal")
    .version(version)
    .enablePositionalOptions();
  program.option("-q, --quiet", "Only output the resource ID (for scripting)");
  program.action(() => {
    program.outputHelp();
  });
  program.addHelpText(
    "after",
    '\nTip: Run "corp next" to see your recommended next actions.\n',
  );

  // ── Group commands: top-level vs subcommands ──────────────────────────────

  const topLevel: CommandDef[] = [];
  const children = new Map<string, CommandDef[]>();

  for (const def of commands) {
    const parts = def.name.split(" ");
    if (parts.length === 1) {
      topLevel.push(def);
    } else {
      const parent = parts[0];
      if (!children.has(parent)) children.set(parent, []);
      children.get(parent)!.push(def);
    }
  }

  // ── Create top-level commands ─────────────────────────────────────────────

  const parentCmds = new Map<string, Command>();
  for (const def of topLevel) {
    const cmd = wireCommand(program, def);
    parentCmds.set(def.name, cmd);
  }

  // ── Create subcommands ────────────────────────────────────────────────────

  for (const [parentName, childDefs] of children) {
    let parentCmd = parentCmds.get(parentName);
    if (!parentCmd) {
      // Parent not explicitly defined — create a stub so children have a home.
      const parentDescriptions: Record<string, string> = {
        workspaces: "Workspace management",
        workspace: "Workspace settings",
        equity: "Equity system (low-level grants, holders, instruments)",
        execution: "Execution intents, obligations, and approval artifacts",
        meetings: "Meeting management (convene, adjourn, notice)",
        compliance: "Compliance escalations and monitoring",
        contractors: "Contractor classification",
        admin: "Admin tools (audit, billing, SSH keys, secrets)",
        auth: "Authentication and API key management",
        references: "Reference tracking and sync",
        secrets: "Secret management and proxy configuration",
        "document-requests": "Document request fulfillment",
        intents: "Execution intent management",
        "bank-accounts": "Bank account management",
        "journal-entries": "Ledger journal entries",
        ledger: "Ledger operations and reconciliation",
        payroll: "Payroll runs",
        payments: "Payment submission and tracking",
        invoices: "Invoice management",
        treasury: "Treasury, invoices, payments, and payouts",
        "governance-seats": "Governance seat management",
        "governance-bodies": "Governance body management",
        "human-obligations": "Human obligation fulfillment",
        "ssh-keys": "SSH key management",
        "secret-proxies": "Secret proxy configuration",
        formations: "Formation workflows (low-level API)",
        valuations: "Valuation management",
        branches: "Git branch management",
        digests: "Digest generation and viewing",
        obligations: "Obligation tracking and fulfillment",
        "spending-limits": "Spending limit management",
        receipts: "Execution receipts",
        "share-transfers": "Share transfer workflows",
        "safe-notes": "SAFE note management",
        distributions: "Distribution management",
        deadlines: "Compliance deadline management",
      };
      parentCmd = program.command(parentName).description(parentDescriptions[parentName] ?? "");
      parentCmds.set(parentName, parentCmd);
    }
    for (const def of childDefs) {
      const childName = def.name.split(" ").slice(1).join(" ");
      wireCommand(parentCmd, { ...def, name: childName });
    }
  }

  return program;
}

// ── Internal: attach a single CommandDef to a parent Command ────────────────

function wireCommand(parent: Command, def: CommandDef): Command {
  // Build command string with positional args
  let cmdStr = def.name;
  for (const arg of def.args || []) {
    if (arg.variadic) {
      cmdStr += arg.required ? ` <${arg.name}...>` : ` [${arg.name}...]`;
    } else {
      cmdStr += arg.required ? ` <${arg.name}>` : ` [${arg.name}]`;
    }
  }

  const cmd = def.hidden
    ? parent.command(cmdStr, { hidden: true }).description(def.description)
    : parent.command(cmdStr).description(def.description);

  // Aliases
  for (const alias of def.aliases || []) {
    cmd.alias(alias);
  }

  // Standard options — every command gets --json (unless already defined in options)
  const definedFlags = new Set((def.options || []).map((o) => o.flags));
  if (!definedFlags.has("--json")) {
    cmd.option("--json", "Output as JSON");
  }

  // Entity-scoped commands get --entity-id
  if (def.entity && !definedFlags.has("--entity-id <ref>")) {
    cmd.option(
      "--entity-id <ref>",
      "Entity reference (overrides active entity and parent command)",
    );
  }

  // Dry-run support
  if (def.dryRun && !definedFlags.has("--dry-run")) {
    cmd.option("--dry-run", "Preview the request without executing");
  }

  // Command-specific options
  for (const opt of def.options || []) {
    let coerce: ((val: string, prev?: unknown) => unknown) | undefined;
    if (opt.type === "int") coerce = (v) => parseInt(v, 10);
    else if (opt.type === "float") coerce = (v) => parseFloat(v);
    else if (opt.type === "array")
      coerce = (v: string, prev: unknown) => [
        ...((prev as string[]) || []),
        v,
      ];

    // Hidden options: accepted by parser but not shown in --help
    if (opt.hidden) {
      const o = new Option(opt.flags, opt.description);
      o.hideHelp();
      if (coerce) o.argParser(coerce as (value: string, previous: unknown) => unknown);
      cmd.addOption(o);
      continue;
    }

    // When choices are specified, use Commander's Option class with .choices()
    // so Commander validates the value at parse time.
    if (opt.choices && opt.choices.length > 0) {
      const o = new Option(opt.flags, opt.description);
      o.choices(opt.choices);
      if (opt.required) o.makeOptionMandatory(true);
      if (opt.default !== undefined) o.default(opt.default);
      if (coerce) o.argParser(coerce as (value: string, previous: unknown) => unknown);
      cmd.addOption(o);
    } else {
      const defaultVal = opt.default as string | boolean | string[] | undefined;
      if (opt.required) {
        if (coerce) cmd.requiredOption(opt.flags, opt.description, coerce, opt.default);
        else cmd.requiredOption(opt.flags, opt.description, defaultVal);
      } else {
        if (coerce) cmd.option(opt.flags, opt.description, coerce, opt.default);
        else cmd.option(opt.flags, opt.description, defaultVal);
      }
    }
  }

  // Help text — examples
  if (def.examples?.length) {
    cmd.addHelpText(
      "after",
      "\nExamples:\n" + def.examples.map((e) => `  $ ${e}`).join("\n") + "\n",
    );
  }

  // Pass-through options (e.g. for commands that forward unknown flags)
  if (def.passThroughOptions) {
    cmd.enablePositionalOptions().passThroughOptions();
  }

  // ── Action handler ──────────────────────────────────────────────────────

  cmd.action(async (...actionArgs: unknown[]) => {
    // Commander passes: (positionalArg1, ..., positionalArgN, opts, command)
    const cmdInstance = actionArgs[actionArgs.length - 1] as Command;
    const opts = actionArgs[actionArgs.length - 2] as Record<string, unknown>;
    const positional = actionArgs.slice(0, -2).map(String);

    // Merge parent opts (child values take precedence over parent)
    const parentOpts = cmdInstance.parent?.opts() ?? {};
    const mergedOpts: Record<string, unknown> = { ...parentOpts, ...opts };
    // Inherit specific options from parent when child doesn't set them
    for (const key of ["json", "entityId", "dryRun", "quiet"]) {
      if (mergedOpts[key] === undefined && parentOpts[key] !== undefined) {
        mergedOpts[key] = parentOpts[key];
      }
    }

    const quiet = !!mergedOpts.quiet;
    const dryRun = !!mergedOpts.dryRun;
    const writer = createWriter();

    // ── Local commands: no API client needed ───────────────────────────

    if (def.local) {
      if (def.handler) {
        try {
          await def.handler({
            client: null as unknown as CommandContext["client"],
            positional,
            opts: mergedOpts,
            resolver: null as unknown as CommandContext["resolver"],
            writer,
            quiet,
            dryRun,
          });
        } catch (err: unknown) {
          writer.error(err instanceof Error ? err.message : String(err));
          process.exit(1);
        }
      }
      return;
    }

    // ── API commands: set up client + resolver ─────────────────────────

    const cfg = requireConfig("api_url", "api_key", "workspace_id");
    const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
    const resolver = new ReferenceResolver(client, cfg);

    // Resolve entity ID for entity-scoped commands.
    // For generic reads (display without handler), let the generic executor
    // handle missing entity gracefully rather than hard-exiting here.
    let entityId: string | undefined;
    if (def.entity) {
      const explicitEid = mergedOpts.entityId as string | undefined;
      if (def.handler) {
        // Custom handler — use resolveEntityId which exits on missing
        entityId = resolveEntityId(cfg, explicitEid);
      } else {
        // Generic read — soft resolve; executor handles missing entity
        entityId = explicitEid || (cfg.active_entity_id || undefined);
        if (!entityId && cfg.workspace_id && cfg.active_entity_ids?.[cfg.workspace_id]) {
          entityId = cfg.active_entity_ids[cfg.workspace_id];
        }
      }
    }

    const ctx: CommandContext = {
      client,
      positional,
      opts: mergedOpts,
      entityId,
      resolver,
      writer,
      quiet,
      dryRun,
    };

    try {
      if (def.handler) {
        await def.handler(ctx);
      } else if (def.display) {
        await executeGenericRead(def, ctx);
      } else if (def.route && def.route.method !== "GET") {
        await executeGenericWrite(def, ctx);
      } else {
        writer.error(`Command "${def.name}" has no handler or display config`);
        process.exit(1);
      }

      // After generic write commands with produces metadata, print a @last hint.
      // Skip for commands with custom handlers — they manage their own output.
      if (def.produces?.kind && !def.handler && !quiet && !mergedOpts.json) {
        const kind = def.produces.kind as string;
        const lastId = resolver.getLastId(kind as Parameters<typeof resolver.getLastId>[0]);
        if (lastId) {
          const shortRef = lastId.length > 12 ? lastId.slice(0, 8) + "…" : lastId;
          console.log(`  Ref: @last:${kind} → ${shortRef}`);
        }
      }
    } catch (err: unknown) {
      writer.error(`Failed: ${err instanceof Error ? err.message : String(err)}`);
      process.exit(1);
    }
  });

  return cmd;
}
