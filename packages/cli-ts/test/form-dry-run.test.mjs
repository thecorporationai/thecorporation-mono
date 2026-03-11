import assert from "node:assert/strict";
import { mkdtempSync, mkdirSync, openSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import test from "node:test";

const CLI_DIR = fileURLToPath(new URL("..", import.meta.url));

function makeConfigDir() {
  const dir = mkdtempSync(join(tmpdir(), "corp-cli-test-"));
  mkdirSync(dir, { recursive: true });
  writeFileSync(join(dir, "config.json"), JSON.stringify({
    api_url: "http://127.0.0.1:9",
    api_key: "sk_test",
    workspace_id: "ws_test",
    active_entity_id: "ent_test",
  }));
  return dir;
}

function runCli(args) {
  const configDir = makeConfigDir();
  const stdoutPath = join(configDir, "stdout.txt");
  const stderrPath = join(configDir, "stderr.txt");
  const stdoutFd = openSync(stdoutPath, "w");
  const stderrFd = openSync(stderrPath, "w");
  const result = spawnSync(process.execPath, ["dist/index.js", ...args], {
    cwd: CLI_DIR,
    env: { ...process.env, CORP_CONFIG_DIR: configDir, NO_COLOR: "1" },
    stdio: ["ignore", stdoutFd, stderrFd],
  });
  return {
    status: result.status,
    stdout: readFileSync(stdoutPath, "utf8"),
    stderr: readFileSync(stderrPath, "utf8"),
  };
}

function extractDryRun(stdout) {
  const marker = '{\n  "dry_run": true,';
  const index = stdout.lastIndexOf(marker);
  assert.notEqual(index, -1, `dry-run payload not found in stdout:\n${stdout}`);
  return JSON.parse(stdout.slice(index));
}

test("form finalize --dry-run prints payload and exits without calling the API", () => {
  const result = runCli([
    "form",
    "finalize",
    "ent_final",
    "--authorized-shares",
    "1000000",
    "--formation-date",
    "2026-03-11",
    "--dry-run",
  ]);

  assert.equal(result.status, 0, result.stderr);
  const payload = extractDryRun(result.stdout);
  assert.equal(payload.operation, "formation.finalize");
  assert.equal(payload.payload.entity_id, "ent_final");
  assert.equal(payload.payload.authorized_shares, 1000000);
  assert.equal(payload.payload.formation_date, "2026-03-11");
});

test("scripted form --member CSV supports address, officer title, and incorporator fields", () => {
  const result = runCli([
    "form",
    "--entity-type",
    "c_corp",
    "--legal-name",
    "Example Corp",
    "--member",
    "Alice Founder,alice@example.com,director,60,1 Main St|San Francisco|CA|94105,ceo,true",
    "--member",
    "Bob Founder,bob@example.com,officer,40",
    "--dry-run",
  ]);

  assert.equal(result.status, 0, result.stderr);
  const payload = extractDryRun(result.stdout);
  assert.equal(payload.operation, "formation.create_with_cap_table");
  assert.equal(payload.payload.members[0].address.street, "1 Main St");
  assert.equal(payload.payload.members[0].address.city, "San Francisco");
  assert.equal(payload.payload.members[0].officer_title, "ceo");
  assert.equal(payload.payload.members[0].is_incorporator, true);
});
