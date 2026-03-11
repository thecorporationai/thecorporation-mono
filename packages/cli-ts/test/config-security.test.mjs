import assert from "node:assert/strict";
import { mkdtempSync, mkdirSync, openSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import test from "node:test";

const CLI_DIR = fileURLToPath(new URL("..", import.meta.url));

function makeConfigDir() {
  const dir = mkdtempSync(join(tmpdir(), "corp-config-test-"));
  mkdirSync(dir, { recursive: true });
  writeFileSync(
    join(dir, "config.json"),
    JSON.stringify({
      api_url: "https://api.thecorporation.ai",
      api_key: "sk_test_existing",
      workspace_id: "ws_test",
      active_entity_id: "ent_alpha",
    }),
  );
  return dir;
}

function makeFetchMockModule() {
  const dir = mkdtempSync(join(tmpdir(), "corp-config-fetch-"));
  const path = join(dir, "fetch-mock.mjs");
  writeFileSync(
    path,
    `
function slugify(value) {
  return String(value ?? "")
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");
}

const routes = new Map([
  ["/v1/entities", [
    {
      entity_id: "ent_11111111-1111-4111-8111-111111111111",
      legal_name: "Acme Holdings LLC",
    },
  ]],
]);

globalThis.fetch = async function mockedFetch(input, init = {}) {
  const method = typeof input === "string"
    ? String(init.method ?? "GET").toUpperCase()
    : String(input.method ?? "GET").toUpperCase();
  const url = typeof input === "string" ? input : input.url;
  const parsed = new URL(url);
  const key = parsed.pathname + parsed.search;
  if (method === "POST" && parsed.pathname === "/v1/references/sync") {
    const body = typeof input === "string"
      ? JSON.parse(String(init.body ?? "{}"))
      : await input.json();
    const kind = String(body.kind ?? "");
    const items = Array.isArray(body.items) ? body.items : [];
    return new Response(JSON.stringify({
      references: items.map((item) => ({
        kind,
        resource_id: item.resource_id,
        handle: slugify(item.label || item.resource_id),
        label: item.label,
        created_at: "2026-03-11T00:00:00Z",
        updated_at: "2026-03-11T00:00:00Z",
      })),
    }), {
      status: 200,
      headers: { "content-type": "application/json" },
    });
  }
  if (!routes.has(key)) {
    return new Response(JSON.stringify({ error: "not found", key }), {
      status: 404,
      headers: { "content-type": "application/json" },
    });
  }
  return new Response(JSON.stringify(routes.get(key)), {
    status: 200,
    headers: { "content-type": "application/json" },
  });
};
`,
  );
  return path;
}

function runCli(args, envOverrides = {}, nodeArgs = []) {
  const { PATH, HOME, TMPDIR, TMP, TEMP } = process.env;
  const configDir = makeConfigDir();
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
      ...envOverrides,
    },
    stdio: ["ignore", stdoutFd, stderrFd],
  });
  return {
    configDir,
    status: result.status,
    stdout: readFileSync(stdoutPath, "utf8"),
    stderr: readFileSync(stderrPath, "utf8"),
  };
}

test("config set rejects security-sensitive keys without --force", () => {
  const result = runCli(["config", "set", "api_key", "sk_test_new"]);
  assert.notEqual(result.status, 0, result.stdout);
  assert.match(result.stderr, /security-sensitive key/i);
});

test("config set persists credentials in auth.json instead of config.json", () => {
  const result = runCli(["config", "set", "api_key", "sk_test_new", "--force"]);
  assert.equal(result.status, 0, result.stderr);
  const config = JSON.parse(readFileSync(join(result.configDir, "config.json"), "utf8"));
  const auth = JSON.parse(readFileSync(join(result.configDir, "auth.json"), "utf8"));
  assert.equal(config.api_key, undefined);
  assert.equal(config.workspace_id, undefined);
  assert.equal(config.api_url, undefined);
  assert.equal(auth.api_key, "sk_test_new");
  assert.equal(auth.workspace_id, "ws_test");
  assert.equal(auth.api_url, "https://api.thecorporation.ai");
});

test("config list migrates legacy credentials out of config.json", () => {
  const result = runCli(["config", "list"]);
  assert.equal(result.status, 0, result.stderr);
  const config = JSON.parse(readFileSync(join(result.configDir, "config.json"), "utf8"));
  const auth = JSON.parse(readFileSync(join(result.configDir, "auth.json"), "utf8"));
  assert.equal(config.api_key, undefined);
  assert.equal(config.workspace_id, undefined);
  assert.equal(config.api_url, undefined);
  assert.equal(auth.api_key, "sk_test_existing");
  assert.equal(auth.workspace_id, "ws_test");
  assert.equal(auth.api_url, "https://api.thecorporation.ai");
});

test("config set rejects unsupported config keys", () => {
  const result = runCli(["config", "set", "__proto__.polluted", "true", "--force"]);
  assert.notEqual(result.status, 0, result.stdout);
  assert.match(result.stderr, /unsupported config key/i);
});

test("config set rejects untrusted remote api_url values", () => {
  const result = runCli(["config", "set", "api_url", "https://evil.example", "--force"]);
  assert.notEqual(result.status, 0, result.stdout);
  assert.match(result.stderr, /trusted TheCorporation host/i);
});

test("config set allows untrusted remote api_url values only with explicit env override", () => {
  const result = runCli(
    ["config", "set", "api_url", "https://corp.internal.example", "--force"],
    { CORP_UNSAFE_API_URL: "1" },
  );
  assert.equal(result.status, 0, result.stderr);
  const auth = JSON.parse(readFileSync(join(result.configDir, "auth.json"), "utf8"));
  assert.equal(auth.api_url, "https://corp.internal.example");
});

test("config set rejects non-https llm.base_url values outside loopback", () => {
  const result = runCli(["config", "set", "llm.base_url", "http://evil.example/v1"]);
  assert.notEqual(result.status, 0, result.stdout);
  assert.match(result.stderr, /llm\.base_url must use https/i);
});

test("config set allows https llm.base_url values", () => {
  const result = runCli(["config", "set", "llm.base_url", "https://openrouter.ai/api/v1"]);
  assert.equal(result.status, 0, result.stderr);
  const config = JSON.parse(readFileSync(join(result.configDir, "config.json"), "utf8"));
  assert.equal(config.llm.base_url, "https://openrouter.ai/api/v1");
});

test("config set active_entity_id stores a workspace-scoped active entity", () => {
  const result = runCli(["config", "set", "active_entity_id", "ent_beta"]);
  assert.equal(result.status, 0, result.stderr);
  const saved = JSON.parse(readFileSync(join(result.configDir, "config.json"), "utf8"));
  assert.equal(saved.active_entity_id, "ent_beta");
  assert.equal(saved.active_entity_ids.ws_test, "ent_beta");
  assert.equal(saved.api_key, undefined);
});

test("config set active_entity_id resolves an entity reference before saving", () => {
  const mockPath = makeFetchMockModule();
  const result = runCli(
    ["config", "set", "active_entity_id", "acme-holdings-llc"],
    {},
    ["--import", mockPath],
  );
  assert.equal(result.status, 0, result.stderr);
  const saved = JSON.parse(readFileSync(join(result.configDir, "config.json"), "utf8"));
  assert.equal(saved.active_entity_id, "ent_11111111-1111-4111-8111-111111111111");
  assert.equal(
    saved.active_entity_ids.ws_test,
    "ent_11111111-1111-4111-8111-111111111111",
  );
});

test("agents message rejects file inputs outside the working directory", () => {
  const bodyPath = join(tmpdir(), `corp-secret-${Date.now()}.txt`);
  writeFileSync(bodyPath, "top secret\n");
  const result = runCli([
    "agents",
    "message",
    "agent_test",
    "--body-file",
    bodyPath,
  ]);
  assert.notEqual(result.status, 0, result.stdout);
  assert.match(result.stderr, /must stay inside the current working directory/i);
});

test("form finalize rejects members-file outside the working directory", () => {
  const membersPath = join(tmpdir(), `corp-members-${Date.now()}.json`);
  writeFileSync(
    membersPath,
    JSON.stringify([{ name: "Alice", email: "alice@example.com", role: "member" }]),
  );
  const result = runCli([
    "form",
    "--legal-name",
    "Acme LLC",
    "--entity-type",
    "llc",
    "--members-file",
    membersPath,
  ]);
  assert.notEqual(result.status, 0, result.stdout);
  assert.match(result.stderr, /must stay inside the current working directory/i);
});
