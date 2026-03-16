/**
 * CLI integration tests — spawns `corp` as a child process against
 * a real api-rs binary via the process:// transport.
 *
 * Requires: `cargo build` in services/api-rs (debug or release).
 * Skipped automatically when the binary is not present.
 */
import { describe, it, expect, beforeAll, afterAll } from "vitest";
import { execFileSync } from "node:child_process";
import { existsSync, mkdirSync, rmSync, writeFileSync, readFileSync } from "node:fs";
import { randomBytes } from "node:crypto";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { tmpdir } from "node:os";
import { join } from "node:path";

const __dirname = dirname(fileURLToPath(import.meta.url));
const MONOREPO_ROOT = resolve(__dirname, "../../../../");
const CLI_BIN = resolve(MONOREPO_ROOT, "packages/cli-ts/dist/index.js");
const API_BIN_RELEASE = resolve(MONOREPO_ROOT, "services/api-rs/target/release/api-rs");
const API_BIN_DEBUG = resolve(MONOREPO_ROOT, "services/api-rs/target/debug/api-rs");
const API_BIN = existsSync(API_BIN_RELEASE) ? API_BIN_RELEASE : API_BIN_DEBUG;
const CAN_RUN = existsSync(API_BIN) && existsSync(CLI_BIN);

// --- Helpers ----------------------------------------------------------------

let configDir: string;
let dataDir: string;
let env: Record<string, string>;

function corp(...args: string[]): { stdout: string; exitCode: number } {
  try {
    const stdout = execFileSync("node", [CLI_BIN, ...args], {
      env: { ...process.env, ...env },
      encoding: "utf8",
      timeout: 30_000,
      stdio: ["pipe", "pipe", "pipe"],
    });
    return { stdout, exitCode: 0 };
  } catch (err: unknown) {
    const e = err as { stdout?: string; stderr?: string; status?: number };
    return {
      stdout: (e.stdout ?? "") + (e.stderr ?? ""),
      exitCode: e.status ?? 1,
    };
  }
}

function corpJson(...args: string[]): Record<string, unknown> {
  const { stdout, exitCode } = corp(...args, "--json");
  if (exitCode !== 0) {
    throw new Error(`corp ${args.join(" ")} failed (exit ${exitCode}):\n${stdout}`);
  }
  // Extract JSON from output — skip any non-JSON preamble lines
  const lines = stdout.trim().split("\n");
  let jsonStart = -1;
  for (let i = 0; i < lines.length; i++) {
    if (lines[i].trim().startsWith("{") || lines[i].trim().startsWith("[")) {
      jsonStart = i;
      break;
    }
  }
  if (jsonStart === -1) {
    throw new Error(`No JSON found in output:\n${stdout}`);
  }
  return JSON.parse(lines.slice(jsonStart).join("\n")) as Record<string, unknown>;
}

// --- Test Setup -------------------------------------------------------------

describe.skipIf(!CAN_RUN)("CLI integration flows", () => {
  beforeAll(async () => {
    configDir = join(tmpdir(), `corp-cli-test-${Date.now()}`);
    dataDir = join(configDir, "data");
    mkdirSync(dataDir, { recursive: true });

    const jwtSecret = randomBytes(32).toString("base64url");
    const secretsMasterKey = randomBytes(32).toString("base64url") + "=";
    const internalWorkerToken = randomBytes(16).toString("hex");

    // Provision workspace by calling the api-rs binary directly
    const provisionEnv: Record<string, string> = {
      ...process.env as Record<string, string>,
      JWT_SECRET: jwtSecret,
      SECRETS_MASTER_KEY: secretsMasterKey,
      INTERNAL_WORKER_TOKEN: internalWorkerToken,
    };
    const provisionOutput = execFileSync(API_BIN, [
      "--skip-validation",
      "call", "POST", "/v1/workspaces/provision",
      "--data-dir", dataDir,
      "-H", "Content-Type: application/json",
      "--stdin",
    ], {
      encoding: "utf8",
      env: provisionEnv,
      input: JSON.stringify({ name: "CLI Test Workspace" }),
      timeout: 15_000,
    });
    const ws = JSON.parse(provisionOutput.trim()) as Record<string, unknown>;
    const apiKey = ws.api_key as string;
    const workspaceId = ws.workspace_id as string;
    expect(apiKey).toBeTruthy();
    expect(workspaceId).toBeTruthy();

    env = {
      CORP_CONFIG_DIR: configDir,
      CORP_SERVER_BIN: API_BIN,
      JWT_SECRET: jwtSecret,
      SECRETS_MASTER_KEY: secretsMasterKey,
      INTERNAL_WORKER_TOKEN: internalWorkerToken,
      CI: "true",
      NO_COLOR: "1",
    };

    // Write config files with real provisioned credentials
    const authJson = {
      api_url: "process://",
      api_key: apiKey,
      workspace_id: workspaceId,
      server_secrets: {
        jwt_secret: jwtSecret,
        secrets_master_key: secretsMasterKey,
        internal_worker_token: internalWorkerToken,
      },
    };
    const configJson = {
      hosting_mode: "local",
      data_dir: dataDir,
      user: { name: "Test User", email: "test@example.com" },
      llm: { provider: "anthropic", model: "claude-sonnet-4-6" },
    };

    writeFileSync(join(configDir, "auth.json"), JSON.stringify(authJson, null, 2));
    writeFileSync(join(configDir, "config.json"), JSON.stringify(configJson, null, 2));
  });

  afterAll(() => {
    if (configDir && existsSync(configDir)) {
      rmSync(configDir, { recursive: true, force: true });
    }
  });

  // --- Top-level commands ---------------------------------------------------

  describe("status and context", () => {
    it("corp status --json returns workspace info", () => {
      const result = corpJson("status");
      expect(result.workspace_id).toBeTruthy();
    });

    it("corp context --json returns workspace info", () => {
      const result = corpJson("context");
      // Context returns nested structure — check for any identifying field
      expect(result.workspace_id ?? result.workspace ?? result.api_url).toBeTruthy();
    });
  });

  // --- Entity formation (staged flow) ---------------------------------------

  describe("staged formation flow", () => {
    let entityId: string;

    it("corp form create --type llc --name creates pending entity", () => {
      const result = corpJson("form", "create", "--type", "llc", "--name", "Staged LLC");
      expect(result.entity_id).toBeTruthy();
      expect(result.formation_status).toBe("pending");
      entityId = result.entity_id as string;
    });

    it("corp form add-founder adds a member", () => {
      const result = corpJson(
        "form", "add-founder", entityId,
        "--name", "Jane Doe",
        "--email", "jane@staged.com",
        "--role", "member",
        "--pct", "100",
      );
      expect(result.member_count).toBeGreaterThanOrEqual(1);
    });

    it("corp form finalize generates documents and cap table", () => {
      const result = corpJson("form", "finalize", entityId);
      expect(result.entity_id).toBe(entityId);
      const docIds = result.document_ids as string[] | undefined;
      expect(docIds).toBeTruthy();
      expect(docIds!.length).toBeGreaterThan(0);
    });

    it("corp form activate advances to active", () => {
      const result = corpJson("form", "activate", entityId);
      expect(result.final_status).toBe("active");
      expect(result.signatures_added).toBeGreaterThan(0);
    });
  });

  // --- One-shot formation ---------------------------------------------------

  describe("one-shot formation", () => {
    it("corp form --type llc --name ... --member creates and finalizes in one step", () => {
      const result = corpJson(
        "form",
        "--type", "llc",
        "--name", "Oneshot LLC",
        "--member", "Bob,bob@oneshot.com,member,100",
      );
      expect(result.entity_id).toBeTruthy();
    });
  });

  // --- Entity management ----------------------------------------------------

  describe("entities", () => {
    it("corp entities --json lists entities", () => {
      const result = corpJson("entities");
      expect(Array.isArray(result)).toBe(true);
      expect((result as unknown as unknown[]).length).toBeGreaterThan(0);
    });

    it("corp use sets active entity", () => {
      const entities = corpJson("entities") as unknown as Array<Record<string, unknown>>;
      const first = entities[0];
      const id = (first.entity_id ?? first.id) as string;
      const { exitCode } = corp("use", id);
      expect(exitCode).toBe(0);
    });
  });

  // --- Cap table ------------------------------------------------------------

  describe("cap table", () => {
    it("corp cap-table --json returns cap table data", () => {
      const result = corpJson("cap-table");
      expect(result).toBeTruthy();
    });

    it("corp cap-table safes --json returns array", () => {
      const result = corpJson("cap-table", "safes");
      expect(Array.isArray(result)).toBe(true);
    });

    it("corp cap-table instruments --json returns instruments", () => {
      const result = corpJson("cap-table", "instruments");
      expect(Array.isArray(result)).toBe(true);
    });

    it("corp cap-table valuations --json returns array", () => {
      const result = corpJson("cap-table", "valuations");
      expect(Array.isArray(result)).toBe(true);
    });
  });

  // --- Governance -----------------------------------------------------------

  describe("governance", () => {
    let bodyId: string;
    let meetingId: string;

    it("set active entity to the activated staged LLC", () => {
      // Governance requires an active entity — switch to the one we activated
      const entities = corpJson("entities") as unknown as Array<Record<string, unknown>>;
      const active = entities.find(e => e.formation_status === "active" || e.status === "active");
      expect(active).toBeTruthy();
      const eid = (active!.entity_id ?? active!.id) as string;
      const { exitCode } = corp("use", eid);
      expect(exitCode).toBe(0);
    });

    it("corp governance --json lists bodies", () => {
      const result = corpJson("governance");
      expect(Array.isArray(result)).toBe(true);
    });

    it("corp governance create-body creates a member vote body for LLC", () => {
      const result = corpJson(
        "governance", "create-body",
        "--name", "Member Vote",
        "--body-type", "llc_member_vote",
      );
      expect(result.body_id).toBeTruthy();
      bodyId = result.body_id as string;
    });

    it("corp governance seats lists seats for body", () => {
      // seats subcommand doesn't have --json, use corp directly
      const { stdout, exitCode } = corp("governance", "seats", bodyId);
      expect(exitCode).toBe(0);
    });

    it("corp governance convene schedules a meeting", () => {
      const result = corpJson(
        "governance", "convene",
        "--body", bodyId,
        "--type", "member_meeting",
        "--title", "Test Meeting",
        "--date", "2027-06-01",
        "--agenda", "Item 1",
      );
      expect(result.meeting_id).toBeTruthy();
      meetingId = result.meeting_id as string;
    });

    it("corp governance meetings lists meetings for body", () => {
      const { stdout, exitCode } = corp("governance", "meetings", bodyId);
      expect(exitCode).toBe(0);
    });

    it("corp governance agenda-items lists agenda items", () => {
      const { stdout, exitCode } = corp("governance", "agenda-items", meetingId);
      expect(exitCode).toBe(0);
    });

    it("corp governance cancel cancels the meeting", () => {
      const result = corpJson("governance", "cancel", meetingId, "--yes");
      expect(result).toBeTruthy();
    });
  });

  // --- Finance --------------------------------------------------------------

  describe("finance", () => {
    it("corp finance --json returns summary", () => {
      const result = corpJson("finance");
      expect(result).toBeTruthy();
    });

    it("corp finance invoices --json returns array", () => {
      const result = corpJson("finance", "invoices");
      expect(Array.isArray(result)).toBe(true);
    });

    it("corp finance invoice creates an invoice", () => {
      const result = corpJson(
        "finance", "invoice",
        "--customer", "Acme Corp",
        "--amount-cents", "500000",
        "--due-date", "2027-06-01",
      );
      expect(result.invoice_id).toBeTruthy();
    });

    it("corp finance bank-accounts --json returns array", () => {
      const result = corpJson("finance", "bank-accounts");
      expect(Array.isArray(result)).toBe(true);
    });
  });

  // --- Tax ------------------------------------------------------------------

  describe("tax", () => {
    it("corp tax --json returns filings and deadlines", () => {
      const result = corpJson("tax");
      expect(result.filings).toBeDefined();
      expect(result.deadlines).toBeDefined();
    });

    it("corp tax deadline tracks a deadline", () => {
      const result = corpJson(
        "tax", "deadline",
        "--type", "franchise_tax",
        "--due-date", "2027-03-15",
        "--description", "Annual franchise tax",
        "--recurrence", "annual",
      );
      expect(result.deadline_id).toBeTruthy();
    });
  });

  // --- Contacts -------------------------------------------------------------

  describe("contacts", () => {
    it("corp contacts --json lists contacts", () => {
      const result = corpJson("contacts");
      expect(Array.isArray(result)).toBe(true);
    });

    it("corp contacts add creates a contact", () => {
      const result = corpJson(
        "contacts", "add",
        "--name", "Test Contact",
        "--email", "contact@test.com",
      );
      expect(result.contact_id).toBeTruthy();
    });
  });

  // --- Work items -----------------------------------------------------------

  describe("work items", () => {
    let workItemId: string;

    it("corp work-items --json lists items", () => {
      const result = corpJson("work-items");
      expect(Array.isArray(result)).toBe(true);
    });

    it("corp work-items create creates an item", () => {
      const result = corpJson(
        "work-items", "create",
        "--title", "File taxes",
        "--category", "compliance",
        "--deadline", "2027-04-15",
      );
      expect(result.work_item_id).toBeTruthy();
      workItemId = result.work_item_id as string;
    });

    it("corp work-items show displays item detail", () => {
      const result = corpJson("work-items", "show", workItemId);
      expect(result.title).toBe("File taxes");
    });

    it("corp work-items cancel cancels the item", () => {
      const result = corpJson("work-items", "cancel", workItemId, "--yes");
      expect(result).toBeTruthy();
    });
  });

  // --- Agents ---------------------------------------------------------------

  describe("agents", () => {
    let agentId: string;

    it("corp agents --json lists agents", () => {
      const result = corpJson("agents");
      expect(Array.isArray(result)).toBe(true);
    });

    it("corp agents create creates an agent", () => {
      const result = corpJson(
        "agents", "create",
        "--name", "test-agent",
        "--prompt", "You are a test agent",
      );
      expect(result.agent_id ?? result.id).toBeTruthy();
      agentId = (result.agent_id ?? result.id) as string;
    });

    it("corp agents show displays agent detail", () => {
      const result = corpJson("agents", "show", agentId);
      expect(result.name).toBe("test-agent");
    });

    it("corp agents pause pauses the agent", () => {
      const result = corpJson("agents", "pause", agentId);
      expect(result).toBeTruthy();
    });

    it("corp agents resume resumes the agent", () => {
      const result = corpJson("agents", "resume", agentId);
      expect(result).toBeTruthy();
    });

    it("corp agents delete removes the agent", () => {
      const result = corpJson("agents", "delete", agentId, "--yes");
      expect(result).toBeTruthy();
    });
  });

  // --- Services & Billing ---------------------------------------------------

  describe("services", () => {
    it("corp services --json shows catalog", () => {
      const result = corpJson("services", "catalog");
      expect(Array.isArray(result)).toBe(true);
    });
  });

  // --- Documents ------------------------------------------------------------

  describe("documents", () => {
    it("corp documents --json lists documents", () => {
      const result = corpJson("documents");
      expect(Array.isArray(result)).toBe(true);
    });
  });

  // --- Obligations ----------------------------------------------------------

  describe("obligations", () => {
    it("corp obligations --json returns obligations data", () => {
      const result = corpJson("obligations");
      expect(result).toBeTruthy();
    });
  });

  // --- Reference resolution -------------------------------------------------

  describe("reference resolution", () => {
    it("corp find entity * lists all entities", () => {
      const result = corpJson("find", "entity", "*");
      // find returns either an array or an object with matches
      const matches = Array.isArray(result) ? result : (result as Record<string, unknown>).matches ?? result;
      expect(matches).toBeTruthy();
    });
  });

  // --- Flag parsing edge cases ----------------------------------------------

  describe("flag parsing", () => {
    it("corp form create --type and --name are not swallowed by parent", () => {
      const result = corpJson("form", "create", "--type", "llc", "--name", "Flag Test LLC");
      expect(result.entity_id).toBeTruthy();
      expect(result.formation_status).toBe("pending");
    });

    it("corp cap-table safes --entity-id works after subcommand", () => {
      const entities = corpJson("entities") as unknown as Array<Record<string, unknown>>;
      const id = (entities[0]?.entity_id ?? entities[0]?.id) as string;
      // Should not error — --entity-id should be parsed by the subcommand
      const result = corpJson("cap-table", "safes", "--entity-id", id);
      expect(Array.isArray(result)).toBe(true);
    });

    it("--amount-cents is accepted on finance invoice", () => {
      // Set active entity to a formed one first, then create invoice
      const entities = corpJson("entities") as unknown as Array<Record<string, unknown>>;
      const activeEntity = entities.find(e => e.formation_status === "active" || e.status === "active");
      if (!activeEntity) {
        // If no active entity, just verify flag parsing works in help
        const { stdout } = corp("finance", "invoice", "--help");
        expect(stdout).toContain("--amount-cents");
        return;
      }
      const eid = (activeEntity.entity_id ?? activeEntity.id) as string;
      corp("use", eid);
      const result = corpJson(
        "finance", "invoice",
        "--customer", "Cents Test",
        "--amount-cents", "100000",
        "--due-date", "2027-12-01",
      );
      expect(result.invoice_id).toBeTruthy();
    });
  });
});
