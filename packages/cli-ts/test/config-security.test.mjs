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

function runCli(args, envOverrides = {}) {
  const configDir = makeConfigDir();
  const stdoutPath = join(configDir, "stdout.txt");
  const stderrPath = join(configDir, "stderr.txt");
  const stdoutFd = openSync(stdoutPath, "w");
  const stderrFd = openSync(stderrPath, "w");
  const result = spawnSync(process.execPath, ["dist/index.js", ...args], {
    cwd: CLI_DIR,
    env: {
      ...process.env,
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

test("config set rejects unsupported config keys", () => {
  const result = runCli(["config", "set", "__proto__.polluted", "true", "--force"]);
  assert.notEqual(result.status, 0, result.stdout);
  assert.match(result.stderr, /unsupported config key/i);
});

test("config set active_entity_id stores a workspace-scoped active entity", () => {
  const result = runCli(["config", "set", "active_entity_id", "ent_beta"]);
  assert.equal(result.status, 0, result.stderr);
  const saved = JSON.parse(readFileSync(join(result.configDir, "config.json"), "utf8"));
  assert.equal(saved.active_entity_id, "ent_beta");
  assert.equal(saved.active_entity_ids.ws_test, "ent_beta");
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
