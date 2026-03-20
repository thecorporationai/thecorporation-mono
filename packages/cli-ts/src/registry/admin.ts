import type { CommandDef } from "./types.js";

// ---------------------------------------------------------------------------
// Admin / utility commands
// ---------------------------------------------------------------------------

export const adminCommands: CommandDef[] = [
  // ── setup (local, interactive) ──────────────────────────────────────
  {
    name: "setup",
    description: "Interactive setup wizard",
    local: true,
    handler: async () => {
      const { setupCommand } = await import("../commands/setup.js");
      await setupCommand();
    },
  },

  // ── config (local) ──────────────────────────────────────────────────
  {
    name: "config",
    description: "Manage configuration",
    local: true,
  },
  {
    name: "config set",
    description: "Set a config value (dot-path)",
    local: true,
    args: [
      { name: "key", required: true, description: "Config key (dot-path)" },
      { name: "value", required: true, description: "Value to set" },
    ],
    options: [
      { flags: "--force", description: "Allow updating a security-sensitive config key" },
    ],
    handler: async (ctx) => {
      const { configSetCommand } = await import("../commands/config.js");
      await configSetCommand(
        ctx.positional[0],
        ctx.positional[1],
        { force: ctx.opts.force as boolean | undefined },
      );
    },
  },
  {
    name: "config get",
    description: "Get a config value (dot-path)",
    local: true,
    args: [
      { name: "key", required: true, description: "Config key (dot-path)" },
    ],
    handler: async (ctx) => {
      const { configGetCommand } = await import("../commands/config.js");
      configGetCommand(ctx.positional[0]);
    },
  },
  {
    name: "config list",
    description: "List all config (API keys masked)",
    local: true,
    handler: async () => {
      const { configListCommand } = await import("../commands/config.js");
      configListCommand();
    },
  },

  // ── schema (local, special) ─────────────────────────────────────────
  {
    name: "schema",
    description: "Dump the CLI command catalog as JSON",
    local: true,
    options: [
      { flags: "--compact", description: "Emit compact JSON" },
      { flags: "--web-routes", description: "Emit web-routes manifest instead of command schema" },
    ],
    handler: async (ctx) => {
      const { registry, generateWebRoutes, generateSchema } = await import("../registry/index.js");
      if (ctx.opts.webRoutes) {
        const manifest = generateWebRoutes(registry);
        console.log(JSON.stringify(manifest));
        return;
      }
      const { createRequire } = await import("node:module");
      const require = createRequire(import.meta.url);
      let pkg: { version: string };
      try { pkg = require("../../package.json"); } catch { pkg = require("../package.json"); }
      const schema = generateSchema(registry, "corp", pkg.version);
      if (ctx.opts.compact) {
        console.log(JSON.stringify(schema));
      } else {
        ctx.writer.json(schema);
      }
    },
  },

  // ── serve (local, complex) ──────────────────────────────────────────
  {
    name: "serve",
    description: "Start the API server locally",
    local: true,
    options: [
      { flags: "--port <port>", description: "Port to listen on", default: "8000" },
      { flags: "--data-dir <path>", description: "Data directory", default: "./data/repos" },
    ],
    handler: async (ctx) => {
      const { serveCommand } = await import("../commands/serve.js");
      await serveCommand({
        port: (ctx.opts.port as string) ?? "8000",
        dataDir: (ctx.opts.dataDir as string) ?? "./data/repos",
      });
    },
  },

  // ── demo (complex, uses API) ────────────────────────────────────────
  {
    name: "demo",
    description: "Create a usable demo workspace environment",
    local: true,
    options: [
      { flags: "--name <name>", description: "Corporation name", required: true },
      { flags: "--scenario <scenario>", description: "Scenario to create (startup, llc, restaurant)", default: "startup" },
      { flags: "--minimal", description: "Use the minimal server-side demo seed instead of the full CLI workflow" },
      { flags: "--json", description: "Output as JSON" },
    ],
    handler: async (ctx) => {
      const { demoCommand } = await import("../commands/demo.js");
      await demoCommand({
        name: ctx.opts.name as string,
        scenario: ctx.opts.scenario as string | undefined,
        minimal: ctx.opts.minimal as boolean | undefined,
        json: ctx.opts.json as boolean | undefined,
      });
    },
  },

  // ── chat (local, interactive) ───────────────────────────────────────
  {
    name: "chat",
    description: "Interactive LLM chat session",
    local: true,
    handler: async () => {
      const { chatCommand } = await import("../chat.js");
      await chatCommand();
    },
  },

  // ── api-keys (API, parent + subcommands) ────────────────────────────
  {
    name: "api-keys",
    description: "API key management",
    handler: async (ctx) => {
      const { apiKeysListCommand } = await import("../commands/api-keys.js");
      await apiKeysListCommand({ json: ctx.opts.json as boolean | undefined });
    },
  },
  {
    name: "api-keys create",
    description: "Create a new API key",
    options: [
      { flags: "--name <name>", description: "Key name/label", required: true },
      { flags: "--scopes <scopes>", description: "Comma-separated scopes" },
      { flags: "--json", description: "Output as JSON" },
    ],
    handler: async (ctx) => {
      const { apiKeysCreateCommand } = await import("../commands/api-keys.js");
      await apiKeysCreateCommand({
        name: ctx.opts.name as string,
        scopes: ctx.opts.scopes as string | undefined,
        json: ctx.opts.json as boolean | undefined,
      });
    },
    produces: { kind: "api_key" },
    successTemplate: "API key created",
  },
  {
    name: "api-keys revoke",
    description: "Revoke an API key",
    args: [
      { name: "key-id", required: true, description: "API key ID to revoke" },
    ],
    options: [
      { flags: "--yes", description: "Skip confirmation" },
      { flags: "--json", description: "Output as JSON" },
    ],
    handler: async (ctx) => {
      const { apiKeysRevokeCommand } = await import("../commands/api-keys.js");
      await apiKeysRevokeCommand(ctx.positional[0], {
        yes: ctx.opts.yes as boolean | undefined,
        json: ctx.opts.json as boolean | undefined,
      });
    },
  },
  {
    name: "api-keys rotate",
    description: "Rotate an API key (returns new key)",
    args: [
      { name: "key-id", required: true, description: "API key ID to rotate" },
    ],
    options: [
      { flags: "--json", description: "Output as JSON" },
    ],
    handler: async (ctx) => {
      const { apiKeysRotateCommand } = await import("../commands/api-keys.js");
      await apiKeysRotateCommand(ctx.positional[0], {
        json: ctx.opts.json as boolean | undefined,
      });
    },
    produces: { kind: "api_key" },
    successTemplate: "API key rotated",
  },

  // ── link (API, write) ───────────────────────────────────────────────
  {
    name: "link",
    description: "Link workspace to an external provider",
    options: [
      { flags: "--external-id <id>", description: "External ID to link", required: true },
      { flags: "--provider <provider>", description: "Provider name (e.g. stripe, github)", required: true },
    ],
    handler: async (ctx) => {
      const { linkCommand } = await import("../commands/link.js");
      await linkCommand({
        externalId: ctx.opts.externalId as string,
        provider: ctx.opts.provider as string,
      });
    },
  },

  // ── claim (API, write) ──────────────────────────────────────────────
  {
    name: "claim",
    description: "Redeem a claim code to join a workspace",
    args: [
      { name: "code", required: true, description: "Claim code to redeem" },
    ],
    handler: async (ctx) => {
      const { claimCommand } = await import("../commands/claim.js");
      await claimCommand(ctx.positional[0]);
    },
    produces: { kind: "entity", trackEntity: true },
  },

  // ── feedback (API, write) ───────────────────────────────────────────
  {
    name: "feedback",
    description: "Submit feedback to TheCorporation",
    args: [
      { name: "message", required: true, description: "Feedback message" },
    ],
    options: [
      { flags: "--category <category>", description: "Category (e.g. bug, feature, general)", default: "general" },
      { flags: "--email <email>", description: "Your email address (to receive a copy)" },
    ],
    handler: async (ctx) => {
      const { feedbackCommand } = await import("../commands/feedback.js");
      await feedbackCommand(ctx.positional[0], {
        category: ctx.opts.category as string | undefined,
        email: ctx.opts.email as string | undefined,
        json: ctx.opts.json as boolean | undefined,
      });
    },
  },

  // ── resolve (API, read) ─────────────────────────────────────────────
  {
    name: "resolve",
    description: "Resolve a human-friendly reference to a canonical ID",
    args: [
      { name: "kind", required: true, description: "Resource kind to resolve" },
      { name: "ref", required: true, description: "Human-friendly reference" },
    ],
    options: [
      { flags: "--entity-id <ref>", description: "Entity reference for entity-scoped resources" },
      { flags: "--body-id <ref>", description: "Governance body reference for body-scoped resources" },
      { flags: "--meeting-id <ref>", description: "Meeting reference for meeting-scoped resources" },
    ],
    handler: async (ctx) => {
      const { resolveCommand } = await import("../commands/resolve.js");
      await resolveCommand(ctx.positional[0], ctx.positional[1], {
        entityId: ctx.opts.entityId as string | undefined,
        bodyId: ctx.opts.bodyId as string | undefined,
        meetingId: ctx.opts.meetingId as string | undefined,
      });
    },
  },

  // ── find (API, read) ────────────────────────────────────────────────
  {
    name: "find",
    description: "List matching references for a resource kind",
    args: [
      { name: "kind", required: true, description: "Resource kind to search" },
      { name: "query", required: true, description: "Fuzzy search query" },
    ],
    options: [
      { flags: "--entity-id <ref>", description: "Entity reference for entity-scoped resources" },
      { flags: "--body-id <ref>", description: "Governance body reference for body-scoped resources" },
      { flags: "--meeting-id <ref>", description: "Meeting reference for meeting-scoped resources" },
      { flags: "--json", description: "Output as JSON" },
    ],
    handler: async (ctx) => {
      const { findCommand } = await import("../commands/find.js");
      await findCommand(ctx.positional[0], ctx.positional[1], {
        entityId: ctx.opts.entityId as string | undefined,
        bodyId: ctx.opts.bodyId as string | undefined,
        meetingId: ctx.opts.meetingId as string | undefined,
        json: ctx.opts.json as boolean | undefined,
      });
    },
  },

  // ── approvals (informational) ───────────────────────────────────────
  {
    name: "approvals",
    description: "Approvals are managed through governance meetings and execution intents",
    local: true,
    handler: async () => {
      process.stderr.write(
        "Approvals are managed through governance meetings and execution intents.\n" +
        "Use these commands to manage approvals:\n\n" +
        "  Board approval via meeting vote:\n" +
        '    corp governance convene --body <body> --type board_meeting --title "Approve X"\n' +
        "    corp governance vote <meeting> <item> --voter <contact> --vote for\n\n" +
        "  Written consent (no meeting needed):\n" +
        '    corp governance written-consent --body <body> --title "Approve X" --description "..."\n\n' +
        "  View pending items:\n" +
        "    corp governance meetings <body>        # see scheduled meetings\n" +
        "    corp governance agenda-items <meeting>  # see items awaiting votes\n" +
        "    corp cap-table valuations               # see pending valuations\n",
      );
      process.exit(1);
    },
  },
];
