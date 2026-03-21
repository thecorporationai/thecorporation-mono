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
    examples: ["corp setup"],
  },

  // ── config (local) ──────────────────────────────────────────────────
  {
    name: "config",
    description: "Manage configuration",
    local: true,
    examples: ["corp config"],
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
    examples: ["corp config set"],
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
    examples: ["corp config get"],
  },
  {
    name: "config list",
    description: "List all config (API keys masked)",
    local: true,
    handler: async () => {
      const { configListCommand } = await import("../commands/config.js");
      configListCommand();
    },
    examples: ["corp config list"],
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
    examples: ["corp schema"],
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
    examples: ["corp serve"],
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
    examples: ["corp demo"],
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
    examples: ["corp chat"],
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
    examples: ["corp api-keys"],
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
    examples: ["corp api-keys create --name 'name'", "corp api-keys create --json"],
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
    examples: ["corp api-keys revoke <key-id>", "corp api-keys revoke --json"],
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
    examples: ["corp api-keys rotate <key-id>", "corp api-keys rotate --json"],
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
    examples: ["corp link --external-id 'id' --provider 'provider'"],
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
    examples: ["corp claim <code>"],
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
    examples: ["corp feedback <message>", "corp feedback --json"],
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
    examples: ["corp resolve"],
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
    examples: ["corp find"],
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
    examples: ["corp approvals"],
  },

  // ── Auto-generated from OpenAPI ──────────────────────────────
  {
    name: "admin audit-events",
    description: "Admin Audit Events",
    route: { method: "GET", path: "/v1/admin/audit-events" },
    display: { title: "Admin Audit Events", cols: ["details>Details", "#event_id>ID", "event_type>Event Type", "timestamp>Timestamp"] },
    examples: ["corp admin audit-events"],
  },
  {
    name: "admin system-health",
    description: "Admin System Health",
    route: { method: "GET", path: "/v1/admin/system-health" },
    display: { title: "Admin System Health", cols: ["git_storage>Git Storage", "status>Status", "uptime_seconds>Uptime Seconds", "version>Version", "workspace_count>Workspace Count"] },
    examples: ["corp admin system-health"],
  },
  {
    name: "admin workspaces",
    description: "Admin Workspaces",
    route: { method: "GET", path: "/v1/admin/workspaces" },
    display: { title: "Admin Workspaces", cols: ["entity_count>Entity Count", "name>Name", "#workspace_id>ID"] },
    examples: ["corp admin workspaces"],
  },
  {
    name: "billing plans",
    description: "Billing Plans",
    route: { method: "GET", path: "/v1/billing/plans" },
    display: { title: "Billing Plans", cols: ["plans>Plans"] },
    examples: ["corp billing plans"],
  },
  {
    name: "billing status",
    description: "Billing Status",
    route: { method: "GET", path: "/v1/billing/status" },
    display: { title: "Billing Status", cols: ["current_period_end>Current Period End", "plan>Plan", "status>Status", "#workspace_id>ID"] },
    examples: ["corp billing status"],
  },
  {
    name: "config",
    description: "Config",
    route: { method: "GET", path: "/v1/config" },
    display: { title: "Config", cols: ["environment>Environment", "features>Features", "version>Version"] },
    examples: ["corp config"],
  },
  {
    name: "demo seed",
    description: "Demo Seed",
    route: { method: "POST", path: "/v1/demo/seed" },
    options: [
      { flags: "--name <name>", description: "Name" },
      { flags: "--scenario <scenario>", description: "Scenario" },
    ],
    examples: ["corp demo seed", "corp demo seed --json"],
  },
  {
    name: "digests trigger",
    description: "Digests Trigger",
    route: { method: "POST", path: "/v1/digests/trigger" },
    examples: ["corp digests trigger"],
  },
  {
    name: "digests",
    description: "Digests",
    route: { method: "GET", path: "/v1/digests/{pos}" },
    args: [{ name: "digest-key", required: true, description: "Digest Key" }],
    examples: ["corp digests"],
  },
  {
    name: "service-token",
    description: "Service Token",
    route: { method: "GET", path: "/v1/service-token" },
    display: { title: "Service Token", cols: ["#api_key_id>ID", "expires_in>Expires In", "token>Token", "token_type>Token Type"] },
    examples: ["corp service-token"],
  },
  {
    name: "workspace entities",
    description: "Workspace Entities",
    route: { method: "GET", path: "/v1/workspace/entities" },
    display: { title: "Workspace Entities", cols: ["#entity_id>ID"] },
    examples: ["corp workspace entities"],
  },
  {
    name: "workspace status",
    description: "Workspace Status",
    route: { method: "GET", path: "/v1/workspace/status" },
    display: { title: "Workspace Status", cols: ["entity_count>Entity Count", "name>Name", "status>Status", "#workspace_id>ID"] },
    examples: ["corp workspace status"],
  },
  {
    name: "workspaces claim",
    description: "Workspaces Claim",
    route: { method: "POST", path: "/v1/workspaces/claim" },
    options: [
      { flags: "--claim-token <claim-token>", description: "Claim Token", required: true },
    ],
    examples: ["corp workspaces claim --claim-token 'claim-token'"],
  },
  {
    name: "workspaces contacts",
    description: "Workspaces Contacts",
    route: { method: "GET", path: "/v1/workspaces/{workspace_id}/contacts" },
    display: { title: "Workspaces Contacts", cols: ["#contact_id>ID", "#entity_id>ID"] },
    examples: ["corp workspaces contacts"],
  },
  {
    name: "workspaces entities",
    description: "Workspaces Entities",
    route: { method: "GET", path: "/v1/workspaces/{workspace_id}/entities" },
    display: { title: "Workspaces Entities", cols: ["#entity_id>ID"] },
    examples: ["corp workspaces entities"],
  },


  // ── Auto-generated from OpenAPI ──────────────────────────────
  {
    name: "auth token-exchange",
    description: "Auth Token Exchange",
    route: { method: "POST", path: "/v1/auth/token-exchange" },
    options: [
      { flags: "--api-key <api-key>", description: "Api Key", required: true },
      { flags: "--ttl-seconds <ttl-seconds>", description: "Ttl Seconds", type: "int" },
    ],
    examples: ["corp auth token-exchange --api-key 'api-key'", "corp auth token-exchange --json"],
  },
  {
    name: "ssh-keys",
    description: "Ssh Keys",
    route: { method: "GET", path: "/v1/ssh-keys" },
    display: { title: "Ssh Keys", cols: ["algorithm>Algorithm", "#contact_id>ID", "@created_at>Created At", "entity_ids>Entity Ids", "fingerprint>Fingerprint", "#key_id>ID", "name>Name", "scopes>Scopes"] },
    examples: ["corp ssh-keys"],
  },
  {
    name: "ssh-keys",
    description: "Ssh Keys",
    route: { method: "POST", path: "/v1/ssh-keys" },
    options: [
      { flags: "--contact-id <contact-id>", description: "Contact Id" },
      { flags: "--entity-ids <entity-ids>", description: "Entity Ids" },
      { flags: "--name <name>", description: "Name", required: true },
      { flags: "--public-key <public-key>", description: "Public Key", required: true },
      { flags: "--scopes <scopes>", description: "Scopes", type: "array" },
    ],
    examples: ["corp ssh-keys --name 'name' --public-key 'public-key'", "corp ssh-keys --json"],
  },
  {
    name: "ssh-keys",
    description: "Ssh Keys",
    route: { method: "DELETE", path: "/v1/ssh-keys/{pos}" },
    args: [{ name: "key-id", required: true, description: "Key Id" }],
    examples: ["corp ssh-keys <key-id>"],
  },
  {
    name: "workspaces provision",
    description: "Workspaces Provision",
    route: { method: "POST", path: "/v1/workspaces/provision" },
    options: [
      { flags: "--name <name>", description: "Name", required: true },
      { flags: "--owner-email <owner-email>", description: "Owner Email" },
    ],
    examples: ["corp workspaces provision --name 'name'", "corp workspaces provision --json"],
  },


  // ── Auto-generated from OpenAPI ──────────────────────────────
  {
    name: "references sync",
    description: "References Sync",
    route: { method: "POST", path: "/v1/references/sync" },
    options: [
      { flags: "--items <items>", description: "Items", required: true, type: "array" },
      { flags: "--kind <kind>", description: "Kind", required: true, choices: ["entity", "contact", "share_transfer", "invoice", "bank_account", "payment", "payroll_run", "distribution", "reconciliation", "tax_filing", "deadline", "classification", "body", "meeting", "seat", "agenda_item", "resolution", "document", "work_item", "agent", "valuation", "safe_note", "instrument", "share_class", "round"] },
    ],
    examples: ["corp references sync --items 'items' --kind 'kind'"],
  },


  // ── Auto-generated from OpenAPI ──────────────────────────────
  {
    name: "secrets interpolate",
    description: "Secrets Interpolate",
    route: { method: "POST", path: "/v1/secrets/interpolate" },
    options: [
      { flags: "--execution-id <execution-id>", description: "Execution Id", required: true },
      { flags: "--template <template>", description: "Template", required: true },
    ],
    examples: ["corp secrets interpolate --execution-id 'execution-id' --template 'template'"],
  },
  {
    name: "secrets resolve",
    description: "Secrets Resolve",
    route: { method: "POST", path: "/v1/secrets/resolve" },
    options: [
      { flags: "--token <token>", description: "Token", required: true },
    ],
    examples: ["corp secrets resolve --token 'token'"],
  },

  // ── workspace-scoped endpoints ──────────────────────────────────────
  {
    name: "workspaces contacts",
    description: "List contacts across a workspace",
    route: { method: "GET", path: "/v1/workspaces/{wid}/contacts" },
    display: { title: "Workspace Contacts", cols: ["name>Name", "email>Email", "category>Category", "#contact_id>ID"] },
    examples: ["corp workspaces contacts"],
  },
  {
    name: "workspaces entities",
    description: "List entities in a workspace",
    route: { method: "GET", path: "/v1/workspaces/{wid}/entities" },
    display: { title: "Workspace Entities", cols: ["legal_name>Name", "entity_type>Type", "#entity_id>ID"] },
    examples: ["corp workspaces entities"],
  },
  {
    name: "documents validate-preview",
    description: "Validate a PDF preview without generating",
    route: { method: "GET", path: "/v1/documents/preview/pdf/validate" },
    entity: true,
    examples: ["corp documents validate-preview", "corp documents validate-preview --json"],
  },
];