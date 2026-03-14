import { describe, it, expect, beforeAll } from "vitest";
import { existsSync } from "node:fs";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { randomBytes } from "node:crypto";
import { processRequest, resetCache } from "../process-transport.js";

// Resolve relative to monorepo root, not cwd
const __dirname = dirname(fileURLToPath(import.meta.url));
const MONOREPO_ROOT = resolve(__dirname, "../../../../");
const BIN_PATH = resolve(MONOREPO_ROOT, "services/api-rs/target/release/api-rs");

describe.skipIf(!existsSync(BIN_PATH))("process transport integration", () => {
  beforeAll(() => {
    resetCache();
    process.env.CORP_SERVER_BIN = BIN_PATH;
    process.env.JWT_SECRET = "dev-secret-32-bytes-minimum-length";
    process.env.INTERNAL_WORKER_TOKEN = "test";
    if (!process.env.SECRETS_MASTER_KEY) {
      process.env.SECRETS_MASTER_KEY = randomBytes(32).toString("base64url") + "=";
    }
  });

  it("GET /health returns 200 ok", async () => {
    const resp = processRequest(
      "process://",
      "GET",
      "/health",
      { Accept: "application/json" },
    );
    expect(resp.status).toBe(200);
    expect(resp.ok).toBe(true);
    const body = await resp.json();
    expect(body).toEqual({ status: "ok" });
  });

  it("GET /v1/openapi.json returns large JSON", async () => {
    const resp = processRequest(
      "process://",
      "GET",
      "/v1/openapi.json",
      { Accept: "application/json" },
    );
    expect(resp.status).toBe(200);
    const body = await resp.json();
    expect(body).toHaveProperty("openapi");
  });

  it("returns error status for nonexistent routes", () => {
    const resp = processRequest(
      "process://",
      "GET",
      "/v1/nonexistent",
      { Accept: "application/json" },
    );
    expect(resp.ok).toBe(false);
  });
});
