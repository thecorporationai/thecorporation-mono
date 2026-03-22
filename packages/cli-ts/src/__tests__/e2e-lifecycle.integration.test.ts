/**
 * E2E lifecycle test — formation → equity → governance pipeline.
 *
 * Runs the full C-Corp lifecycle against a real HTTP API server:
 *   1. form (one-shot c_corp with a director/incorporator founder)
 *   2. form activate @last:entity
 *   3. cap-table (verify cap table exists after activation)
 *   4. contacts add (create a contact to use as a governance seat holder)
 *   5. governance create-body --body-type board_of_directors
 *   6. governance add-seat @last:body --holder <contact-ref>
 *   7. cap-table issue-equity --grant-type common --shares 1000
 *   8. cap-table (verify shares appear in the cap table)
 *
 * Required environment variables:
 *   CORP_API_URL        — base URL of a running api-rs instance, e.g. http://localhost:8080
 *   CORP_API_KEY        — API key for the workspace
 *   CORP_WORKSPACE_ID   — workspace ID
 *
 * Optional:
 *   CORP_E2E_DATA_DIR   — path to a persistent config dir (default: a fresh temp dir)
 *
 * Run with:
 *   CORP_API_URL=http://localhost:8080 \
 *   CORP_API_KEY=your-key \
 *   CORP_WORKSPACE_ID=your-workspace-id \
 *   npx vitest run --config vitest.integration.config.ts --reporter verbose \
 *     src/__tests__/e2e-lifecycle.integration.test.ts
 *
 * The test is skipped automatically when CORP_API_URL is not set.
 *
 * Prerequisites:
 *   - api-rs server running and reachable at CORP_API_URL
 *   - CLI built:  cd packages/cli-ts && npm run build
 */

import { describe, it, expect, beforeAll, afterAll } from "vitest";
import { execFileSync } from "node:child_process";
import { existsSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { tmpdir } from "node:os";
import { join } from "node:path";

// ---------------------------------------------------------------------------
// Paths
// ---------------------------------------------------------------------------

const __dirname = dirname(fileURLToPath(import.meta.url));
const MONOREPO_ROOT = resolve(__dirname, "../../../../");
const CLI_BIN = resolve(MONOREPO_ROOT, "packages/cli-ts/dist/index.js");

// ---------------------------------------------------------------------------
// Environment guards — skip when server or CLI binary is not available
// ---------------------------------------------------------------------------

const CORP_API_URL = process.env.CORP_API_URL ?? "";
const CORP_API_KEY = process.env.CORP_API_KEY ?? "";
const CORP_WORKSPACE_ID = process.env.CORP_WORKSPACE_ID ?? "";

const SERVER_AVAILABLE =
  Boolean(CORP_API_URL) && Boolean(CORP_API_KEY) && Boolean(CORP_WORKSPACE_ID);

const CLI_BUILT = existsSync(CLI_BIN);

const CAN_RUN = SERVER_AVAILABLE && CLI_BUILT;

if (!CAN_RUN) {
  if (!SERVER_AVAILABLE) {
    console.warn(
      "[e2e-lifecycle] Skipping: CORP_API_URL / CORP_API_KEY / CORP_WORKSPACE_ID not set.\n" +
        "  Set these env vars pointing to a running api-rs instance to enable this test.",
    );
  }
  if (!CLI_BUILT) {
    console.warn(
      `[e2e-lifecycle] Skipping: CLI binary not found at ${CLI_BIN}.\n` +
        "  Run: npm run build  (inside packages/cli-ts/) to enable this test.",
    );
  }
}

// ---------------------------------------------------------------------------
// Config directory — isolated per test run
// ---------------------------------------------------------------------------

let configDir: string;
let env: Record<string, string>;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/**
 * Run `corp <...args>` and return stdout, stderr, and exit code.
 * Non-zero exits do NOT throw; callers inspect exitCode.
 */
function corp(...args: string[]): { stdout: string; stderr: string; exitCode: number } {
  try {
    const stdout = execFileSync("node", [CLI_BIN, ...args], {
      env: { ...process.env, ...env },
      encoding: "utf8",
      timeout: 30_000,
      stdio: ["pipe", "pipe", "pipe"],
    });
    return { stdout, stderr: "", exitCode: 0 };
  } catch (err: unknown) {
    const e = err as { stdout?: string; stderr?: string; status?: number };
    return {
      stdout: e.stdout ?? "",
      stderr: e.stderr ?? "",
      exitCode: e.status ?? 1,
    };
  }
}

/**
 * Run `corp <...args> --json`, parse and return the JSON payload.
 * Throws a descriptive error if the command exits non-zero or produces no JSON.
 */
function corpJson(...args: string[]): Record<string, unknown> {
  const { stdout, stderr, exitCode } = corp(...args, "--json");
  if (exitCode !== 0) {
    throw new Error(
      `corp ${args.join(" ")} --json  failed (exit ${exitCode}):\n` +
        `  stdout: ${stdout.trim()}\n` +
        `  stderr: ${stderr.trim()}`,
    );
  }
  const combined = stdout.trim();
  if (!combined) {
    throw new Error(`corp ${args.join(" ")} --json  produced no output`);
  }
  // Scan forward to the first JSON token — skip any spinner / chalk preamble lines.
  const lines = combined.split("\n");
  for (let i = 0; i < lines.length; i++) {
    const trimmed = lines[i].trim();
    if (trimmed.startsWith("{") || trimmed.startsWith("[")) {
      return JSON.parse(lines.slice(i).join("\n")) as Record<string, unknown>;
    }
  }
  throw new Error(
    `corp ${args.join(" ")} --json  produced no parseable JSON.\n  output: ${combined}`,
  );
}

// ---------------------------------------------------------------------------
// Suite
// ---------------------------------------------------------------------------

describe.skipIf(!CAN_RUN)(
  "E2E lifecycle: C-Corp formation → equity → governance",
  () => {
    // IDs threaded through the lifecycle steps (set during execution)
    let entityId: string;
    let contactId: string;
    // bodyId is captured but not needed after step 6; kept for clarity
    // eslint-disable-next-line @typescript-eslint/no-unused-vars
    let bodyId: string;

    // ----------------------------------------------------------------
    // Setup: write an isolated config pointing at the external server
    // ----------------------------------------------------------------

    beforeAll(() => {
      const base =
        process.env.CORP_E2E_DATA_DIR ??
        join(tmpdir(), `corp-e2e-lifecycle-${Date.now()}`);
      configDir = base;
      mkdirSync(configDir, { recursive: true });

      const authJson = {
        api_url: CORP_API_URL,
        api_key: CORP_API_KEY,
        workspace_id: CORP_WORKSPACE_ID,
      };
      const configJson = {
        hosting_mode: "remote",
        user: { name: "E2E Test User", email: "e2e@example.com" },
      };

      writeFileSync(join(configDir, "auth.json"), JSON.stringify(authJson, null, 2));
      writeFileSync(join(configDir, "config.json"), JSON.stringify(configJson, null, 2));

      env = {
        CORP_CONFIG_DIR: configDir,
        CI: "true",
        NO_COLOR: "1",
        // Disable TTY-dependent spinners and interactive prompts.
        TERM: "dumb",
      };
    });

    afterAll(() => {
      // Only clean up a temp dir we created ourselves.
      if (!process.env.CORP_E2E_DATA_DIR && configDir && existsSync(configDir)) {
        rmSync(configDir, { recursive: true, force: true });
      }
    });

    // ----------------------------------------------------------------
    // Step 1 — one-shot C-Corp formation
    //
    // Member spec format (comma-separated positional fields):
    //   name, email, role, ownership_pct,
    //   address (pipe-separated street|city|state|zip),
    //   officer_title, is_incorporator
    //
    // C-Corp requires: address, incorporator, director role, officer.
    // ----------------------------------------------------------------

    it("step 1: form --type c_corp creates and finalizes entity", () => {
      const result = corpJson(
        "form",
        "--type", "c_corp",
        "--name", "E2E Test Corp",
        "--jurisdiction", "US-DE",
        "--address", "123 Test St,Wilmington,DE,19801",
        "--member",
        "E2E Founder,e2e-founder@example.com,director,100,123 Test St|Wilmington|DE|19801,ceo,true",
      );

      expect(result.entity_id, "entity_id missing from form response").toBeTruthy();
      entityId = result.entity_id as string;

      // After one-shot formation the entity is finalized but not yet signed.
      // Status will be "pending_activation" or "finalized" (never "pending").
      const status = (result.formation_status ?? result.status) as string | undefined;
      expect(
        status,
        `unexpected formation_status after one-shot form: "${status}"`,
      ).toMatch(/pending_activation|finalized|active/);
    });

    // ----------------------------------------------------------------
    // Step 2 — activate (sign documents, advance to "active")
    // ----------------------------------------------------------------

    it("step 2: form activate advances entity to active", () => {
      // @last:entity is resolved from the reference cache written by step 1.
      const result = corpJson("form", "activate", "@last:entity");

      // activate returns { final_status, ... } or embeds formation_status
      const status = result.final_status ?? result.formation_status;
      expect(status, "entity not active after activate").toBe("active");
    });

    // ----------------------------------------------------------------
    // Step 3 — cap-table (verify it exists for the active entity)
    // ----------------------------------------------------------------

    it("step 3: cap-table returns data for the active entity", () => {
      const result = corpJson("cap-table");

      expect(result, "cap-table returned empty response").toBeTruthy();

      // Must carry an entity identifier at some nesting level.
      const hasEntityRef =
        typeof result.entity_id === "string" ||
        typeof result.legal_entity_id === "string" ||
        typeof (result.entity as Record<string, unknown> | undefined)?.entity_id === "string";
      expect(hasEntityRef, "cap-table response missing any entity reference").toBe(true);
    });

    // ----------------------------------------------------------------
    // Step 4 — create a contact to use as the governance seat holder
    // ----------------------------------------------------------------

    it("step 4: contacts add creates a seat-holder contact", () => {
      const result = corpJson(
        "contacts", "add",
        "--name", "E2E Director",
        "--email", "e2e-director@example.com",
      );

      expect(result.contact_id, "contact_id missing from contacts add response").toBeTruthy();
      contactId = result.contact_id as string;
    });

    // ----------------------------------------------------------------
    // Step 5 — governance create-body (board of directors)
    // ----------------------------------------------------------------

    it("step 5: governance create-body creates a board_of_directors body", () => {
      const result = corpJson(
        "governance", "create-body",
        "--name", "Board of Directors",
        "--body-type", "board_of_directors",
      );

      expect(result.body_id, "body_id missing from governance create-body response").toBeTruthy();
      bodyId = result.body_id as string;
    });

    // ----------------------------------------------------------------
    // Step 6 — governance add-seat
    //
    // We pass contactId directly (not @last:contact) so the test is
    // resilient to reference-cache state across separate process calls.
    // ----------------------------------------------------------------

    it("step 6: governance add-seat adds director to the board", () => {
      const result = corpJson(
        "governance", "add-seat", "@last:body",
        "--holder", contactId,
      );

      // Server returns the new seat record; seat_id confirms creation.
      const created = result.seat_id ?? result.body_id;
      expect(created, "seat_id / body_id missing from add-seat response").toBeTruthy();
    });

    // ----------------------------------------------------------------
    // Step 7 — cap-table issue-equity (1 000 common shares)
    //
    // For a board-governed C-Corp the server may require board approval
    // before issuance.  If that check fires the test fails with the
    // server's message, which will include "quick-approve" guidance.
    // ----------------------------------------------------------------

    it("step 7: cap-table issue-equity issues 1000 common shares", () => {
      const result = corpJson(
        "cap-table", "issue-equity",
        "--grant-type", "common",
        "--shares", "1000",
        "--recipient", "E2E Test Founder",
        "--email", "e2e-founder@example.com",
      );

      // Success response must contain a round or security identifier.
      const issued =
        result.round_id ??
        result.security_id ??
        (result.round as Record<string, unknown> | undefined)?.round_id;
      expect(issued, "No round_id / security_id in issue-equity response").toBeTruthy();
    });

    // ----------------------------------------------------------------
    // Verification — re-fetch cap table and confirm equity appears
    // ----------------------------------------------------------------

    it("verification: cap-table reflects the issued shares", () => {
      const result = corpJson("cap-table");

      const instruments = Array.isArray(result.instruments)
        ? (result.instruments as unknown[])
        : [];
      const shareClasses = Array.isArray(result.share_classes)
        ? (result.share_classes as unknown[])
        : [];

      expect(
        instruments.length > 0 || shareClasses.length > 0,
        "cap-table has no instruments or share classes after issuance",
      ).toBe(true);
    });
  },
);
