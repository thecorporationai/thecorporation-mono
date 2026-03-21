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
    route: { method: "GET", path: "/v1/api-keys" },
    display: {
      title: "API Keys",
      cols: ["name>Name", "key_prefix|prefix>Prefix", "@created_at>Created", "#api_key_id>ID"],
    },
    handler: async (ctx) => {
      const { apiKeysListCommand } = await import("../commands/api-keys.js");
      await apiKeysListCommand({ json: ctx.opts.json as boolean | undefined });
    },
  },
  {
    name: "api-keys create",
    description: "Create a new API key",
    route: { method: "POST", path: "/v1/api-keys" },
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
    route: { method: "DELETE", path: "/v1/api-keys/{pos}" },
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
    route: { method: "POST", path: "/v1/api-keys/{pos}/rotate" },
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
    route: { method: "POST", path: "/v1/workspaces/link" },
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
    route: { method: "POST", path: "/v1/entities/claim" },
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
    route: { method: "POST", path: "/v1/feedback" },
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

  // ── Auto-generated from OpenAPI ──────────────────────────────
  {
    name: "admin audit-events",
    description: "/v1/admin/audit-events",
    route: { method: "GET", path: "/v1/admin/audit-events" },
    display: { title: "Admin Audit Events", cols: ["details>Details", "#event_id>ID", "event_type>Event Type", "timestamp>Timestamp"] },
  },
  {
    name: "admin system-health",
    description: "/v1/admin/system-health",
    route: { method: "GET", path: "/v1/admin/system-health" },
    display: { title: "Admin System Health", cols: ["git_storage>Git Storage", "status>Status", "uptime_seconds>Uptime Seconds", "version>Version", "workspace_count>Workspace Count"] },
  },
  {
    name: "admin workspaces",
    description: "/v1/admin/workspaces",
    route: { method: "GET", path: "/v1/admin/workspaces" },
    display: { title: "Admin Workspaces", cols: ["entity_count>Entity Count", "name>Name", "#workspace_id>ID"] },
  },
  {
    name: "billing plans",
    description: "/v1/billing/plans",
    route: { method: "GET", path: "/v1/billing/plans" },
    display: { title: "Billing Plans", cols: ["plans>Plans"] },
  },
  {
    name: "billing status",
    description: "/v1/billing/status",
    route: { method: "GET", path: "/v1/billing/status" },
    display: { title: "Billing Status", cols: ["current_period_end>Current Period End", "plan>Plan", "status>Status", "#workspace_id>ID"] },
  },
  {
    name: "config",
    description: "/v1/config",
    route: { method: "GET", path: "/v1/config" },
    display: { title: "Config", cols: ["environment>Environment", "features>Features", "version>Version"] },
  },
  {
    name: "demo seed",
    description: "/v1/demo/seed",
    route: { method: "POST", path: "/v1/demo/seed" },
    options: [
      { flags: "--name <name>", description: "Name" },
      { flags: "--scenario <scenario>", description: "Scenario" },
    ],
  },
  {
    name: "digests trigger",
    description: "/v1/digests/trigger",
    route: { method: "POST", path: "/v1/digests/trigger" },
  },
  {
    name: "digests",
    description: "/v1/digests/{digest_key}",
    route: { method: "GET", path: "/v1/digests/{pos}" },
    args: [{ name: "digest-key", required: true, description: "Digest Key" }],
  },
  {
    name: "service-token",
    description: "/v1/service-token",
    route: { method: "GET", path: "/v1/service-token" },
    display: { title: "Service Token", cols: ["#api_key_id>ID", "expires_in>Expires In", "token>Token", "token_type>Token Type"] },
  },
  {
    name: "workspace entities",
    description: "/v1/workspace/entities",
    route: { method: "GET", path: "/v1/workspace/entities" },
    display: { title: "Workspace Entities", cols: ["#entity_id>ID"] },
  },
  {
    name: "workspace status",
    description: "/v1/workspace/status",
    route: { method: "GET", path: "/v1/workspace/status" },
    display: { title: "Workspace Status", cols: ["entity_count>Entity Count", "name>Name", "status>Status", "#workspace_id>ID"] },
  },
  {
    name: "workspaces claim",
    description: "/v1/workspaces/claim",
    route: { method: "POST", path: "/v1/workspaces/claim" },
    options: [
      { flags: "--claim-token <claim-token>", description: "Claim Token", required: true },
    ],
  },
  {
    name: "workspaces contacts",
    description: "/v1/workspaces/{workspace_id}/contacts",
    route: { method: "GET", path: "/v1/workspaces/{workspace_id}/contacts" },
    display: { title: "Workspaces Contacts", cols: ["#contact_id>ID", "#entity_id>ID"] },
  },
  {
    name: "workspaces entities",
    description: "/v1/workspaces/{workspace_id}/entities",
    route: { method: "GET", path: "/v1/workspaces/{workspace_id}/entities" },
    display: { title: "Workspaces Entities", cols: ["#entity_id>ID"] },
  },


  // ── Auto-generated from OpenAPI ──────────────────────────────
  {
    name: "auth token-exchange",
    description: "/v1/auth/token-exchange",
    route: { method: "POST", path: "/v1/auth/token-exchange" },
    options: [
      { flags: "--api-key <api-key>", description: "Api Key", required: true },
      { flags: "--ttl-seconds <ttl-seconds>", description: "Ttl Seconds", type: "int" },
    ],
  },
  {
    name: "ssh-keys",
    description: "/v1/ssh-keys",
    route: { method: "GET", path: "/v1/ssh-keys" },
    display: { title: "Ssh Keys", cols: ["algorithm>Algorithm", "#contact_id>ID", "@created_at>Created At", "entity_ids>Entity Ids", "fingerprint>Fingerprint", "#key_id>ID", "name>Name", "scopes>Scopes"] },
  },
  {
    name: "ssh-keys",
    description: "/v1/ssh-keys",
    route: { method: "POST", path: "/v1/ssh-keys" },
    options: [
      { flags: "--contact-id <contact-id>", description: "Contact Id" },
      { flags: "--entity-ids <entity-ids>", description: "Entity Ids" },
      { flags: "--name <name>", description: "Name", required: true },
      { flags: "--public-key <public-key>", description: "Public Key", required: true },
      { flags: "--scopes <scopes>", description: "Scopes", type: "array" },
    ],
  },
  {
    name: "ssh-keys",
    description: "/v1/ssh-keys/{key_id}",
    route: { method: "DELETE", path: "/v1/ssh-keys/{pos}" },
    args: [{ name: "key-id", required: true, description: "Key Id" }],
  },
  {
    name: "workspaces provision",
    description: "/v1/workspaces/provision",
    route: { method: "POST", path: "/v1/workspaces/provision" },
    options: [
      { flags: "--name <name>", description: "Name", required: true },
      { flags: "--owner-email <owner-email>", description: "Owner Email" },
    ],
  },


  // ── Auto-generated from OpenAPI ──────────────────────────────
  {
    name: "references sync",
    description: "/v1/references/sync",
    route: { method: "POST", path: "/v1/references/sync" },
    options: [
      { flags: "--items <items>", description: "Items", required: true, type: "array" },
      { flags: "--kind <kind>", description: "Kind", required: true, choices: ["entity", "contact", "share_transfer", "invoice", "bank_account", "payment", "payroll_run", "distribution", "reconciliation", "tax_filing", "deadline", "classification", "body", "meeting", "seat", "agenda_item", "resolution", "document", "work_item", "agent", "valuation", "safe_note", "instrument", "share_class", "round"] },
    ],
  },


  // ── Auto-generated from OpenAPI ──────────────────────────────
  {
    name: "secrets interpolate",
    description: "/v1/secrets/interpolate",
    route: { method: "POST", path: "/v1/secrets/interpolate" },
    options: [
      { flags: "--execution-id <execution-id>", description: "Execution Id", required: true },
      { flags: "--template <template>", description: "Template", required: true },
    ],
  },
  {
    name: "secrets resolve",
    description: "/v1/secrets/resolve",
    route: { method: "POST", path: "/v1/secrets/resolve" },
    options: [
      { flags: "--token <token>", description: "Token", required: true },
    ],
  },

  // ── workspace-scoped endpoints ──────────────────────────────────────
  {
    name: "workspaces contacts",
    description: "List contacts across a workspace",
    route: { method: "GET", path: "/v1/workspaces/{wid}/contacts" },
    display: { title: "Workspace Contacts", cols: ["name>Name", "email>Email", "category>Category", "#contact_id>ID"] },
  },
  {
    name: "workspaces entities",
    description: "List entities in a workspace",
    route: { method: "GET", path: "/v1/workspaces/{wid}/entities" },
    display: { title: "Workspace Entities", cols: ["legal_name>Name", "entity_type>Type", "#entity_id>ID"] },
  },
  {
    name: "documents validate-preview",
    description: "Validate a PDF preview without generating",
    route: { method: "GET", path: "/v1/documents/preview/pdf/validate" },
    entity: true,
  },
];