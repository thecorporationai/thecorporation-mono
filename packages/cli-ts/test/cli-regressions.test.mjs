import assert from "node:assert/strict";
import { mkdtempSync, mkdirSync, openSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import test from "node:test";

const CLI_DIR = fileURLToPath(new URL("..", import.meta.url));

function makeConfigDir(apiUrl = "https://api.thecorporation.ai") {
  const dir = mkdtempSync(join(tmpdir(), "corp-cli-regression-"));
  mkdirSync(dir, { recursive: true });
  writeFileSync(
    join(dir, "config.json"),
    JSON.stringify({
      active_entity_id: "ent_test",
      active_entity_ids: { ws_test: "ent_test" },
    }),
  );
  writeFileSync(
    join(dir, "auth.json"),
    JSON.stringify({
      api_url: apiUrl,
      api_key: "sk_test_existing",
      workspace_id: "ws_test",
    }),
  );
  return dir;
}

function makeFetchMockModule() {
  const dir = mkdtempSync(join(tmpdir(), "corp-cli-regression-fetch-"));
  const path = join(dir, "fetch-mock.mjs");
  writeFileSync(
    path,
    `
globalThis.fetch = async function mockedFetch(input, init = {}) {
  const method = typeof input === "string"
    ? String(init.method ?? "GET").toUpperCase()
    : String(input.method ?? "GET").toUpperCase();
  const url = typeof input === "string" ? input : input.url;
  const parsed = new URL(url);
  const key = method + " " + parsed.pathname + parsed.search;

  if (key === "GET /v1/documents/preview/pdf/validate?entity_id=ent_test&document_id=bylaws") {
    return new Response(JSON.stringify({ ok: true }), {
      status: 200,
      headers: { "content-type": "application/json" },
    });
  }

  if (key === "GET /v1/entities/ent_test/current-409a") {
    return new Response(JSON.stringify({ error: { detail: "not found" } }), {
      status: 404,
      headers: { "content-type": "application/json" },
    });
  }

  if (key === "GET /v1/entities/ent_test/valuations") {
    return new Response(JSON.stringify([
      {
        valuation_id: "val_123",
        valuation_type: "four_oh_nine_a",
        status: "pending_approval",
        effective_date: "2026-03-10",
      },
    ]), {
      status: 200,
      headers: { "content-type": "application/json" },
    });
  }

  if (key === "POST /v1/feedback") {
    return new Response(JSON.stringify({ error: { detail: "not found" } }), {
      status: 404,
      headers: { "content-type": "application/json" },
    });
  }

  return new Response(JSON.stringify({ error: { detail: key } }), {
    status: 404,
    headers: { "content-type": "application/json" },
  });
};
`,
  );
  return path;
}

function runCli(args, configDir, nodeArgs = []) {
  const { PATH, HOME, TMPDIR, TMP, TEMP } = process.env;
  const stdoutPath = join(configDir, "stdout.txt");
  const stderrPath = join(configDir, "stderr.txt");
  const stdoutFd = openSync(stdoutPath, "w");
  const stderrFd = openSync(stderrPath, "w");
  const result = spawnSync(process.execPath, [...nodeArgs, "dist/index.js", ...args], {
    cwd: CLI_DIR,
    env: {
      PATH,
      HOME,
      TMPDIR,
      TMP,
      TEMP,
      CORP_CONFIG_DIR: configDir,
      NO_COLOR: "1",
    },
    stdio: ["ignore", stdoutFd, stderrFd],
  });
  return {
    status: result.status,
    stdout: readFileSync(stdoutPath, "utf8"),
    stderr: readFileSync(stderrPath, "utf8"),
  };
}

test("documents preview-pdf accepts --document-id as a deprecated alias for --definition-id", () => {
  const configDir = makeConfigDir();
  const mockPath = makeFetchMockModule();
  const result = runCli(
    ["documents", "preview-pdf", "--document-id", "bylaws"],
    configDir,
    ["--import", mockPath],
  );
  assert.equal(result.status, 0, result.stderr);
  assert.match(result.stdout, /document_id=bylaws/);
  assert.match(result.stdout, /Preview PDF URL:/);
});

test("governance finalize-item rejects invalid status values at the CLI layer", () => {
  const configDir = makeConfigDir();
  const result = runCli(
    ["governance", "finalize-item", "meeting_123", "item_123", "--status", "approved"],
    configDir,
  );
  assert.notEqual(result.status, 0, result.stdout);
  assert.match(result.stderr, /Allowed choices are discussed, voted, tabled, withdrawn/i);
});

test("approvals exits non-zero with governance guidance", () => {
  const configDir = makeConfigDir();
  const result = runCli(["approvals"], configDir);
  assert.notEqual(result.status, 0, result.stdout);
  assert.match(result.stderr, /Approvals are managed through governance meetings/i);
});

test("cap-table 409a mentions a pending approval instead of claiming nothing exists", () => {
  const configDir = makeConfigDir();
  const mockPath = makeFetchMockModule();
  const result = runCli(["cap-table", "409a"], configDir, ["--import", mockPath]);
  assert.notEqual(result.status, 0, result.stdout);
  assert.match(result.stdout, /pending approval/i);
  assert.doesNotMatch(result.stdout, /^No 409A valuation found\./m);
});

test("feedback surfaces local server endpoint support failures", () => {
  const configDir = makeConfigDir();
  const mockPath = makeFetchMockModule();
  const result = runCli(
    ["feedback", "local server missing endpoint"],
    configDir,
    ["--import", mockPath],
  );
  assert.notEqual(result.status, 0, result.stdout);
  assert.match(result.stderr, /does not expose \/v1\/feedback/i);
});

test("staged form create preserves explicit jurisdiction on dry-run", () => {
  const configDir = makeConfigDir();
  const result = runCli(
    ["form", "create", "--type", "llc", "--name", "Acme LLC", "--jurisdiction", "US-TX", "--dry-run"],
    configDir,
  );
  assert.equal(result.status, 0, result.stderr);
  const parsed = JSON.parse(result.stdout);
  assert.equal(parsed.payload.jurisdiction, "US-TX");
});

test("staged form finalize exposes required corporation and LLC metadata flags", () => {
  const configDir = makeConfigDir();
  const result = runCli(
    [
      "form",
      "finalize",
      "ent_staged",
      "--board-size",
      "3",
      "--principal-name",
      "Alice Founder",
      "--incorporator-address",
      "251 Little Falls Dr, Wilmington, DE",
      "--dry-run",
    ],
    configDir,
  );
  assert.equal(result.status, 0, result.stderr);
  const parsed = JSON.parse(result.stdout);
  assert.equal(parsed.payload.board_size, 3);
  assert.equal(parsed.payload.principal_name, "Alice Founder");
  assert.equal(
    parsed.payload.incorporator_address,
    "251 Little Falls Dr, Wilmington, DE",
  );
});
