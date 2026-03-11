import assert from "node:assert/strict";
import { mkdtempSync, mkdirSync, openSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import test from "node:test";

const CLI_DIR = fileURLToPath(new URL("..", import.meta.url));

function makeConfigDir(apiUrl) {
  const dir = mkdtempSync(join(tmpdir(), "corp-ref-test-"));
  mkdirSync(dir, { recursive: true });
  writeFileSync(
    join(dir, "config.json"),
    JSON.stringify({
      api_url: apiUrl,
      api_key: "sk_test_existing",
      workspace_id: "ws_test",
      active_entity_id: "ent_11111111-1111-4111-8111-111111111111",
    }),
  );
  return dir;
}

function makeFetchMockModule() {
  const dir = mkdtempSync(join(tmpdir(), "corp-fetch-mock-"));
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
  ["/v1/entities/ent_11111111-1111-4111-8111-111111111111/contacts", [
    {
      contact_id: "89facaea-abac-4001-9bac-e7143f58d4df",
      name: "Alice Johnson",
      email: "alice@example.com",
    },
  ]],
  ["/v1/entities/ent_11111111-1111-4111-8111-111111111111/governance-bodies", [
    {
      body_id: "fa533094-7a9e-47c1-86c5-bb7587d454dd",
      name: "Board of Directors",
    },
  ]],
  ["/v1/governance-bodies/fa533094-7a9e-47c1-86c5-bb7587d454dd/meetings?entity_id=ent_11111111-1111-4111-8111-111111111111", [
    {
      meeting_id: "0b1d6184-5981-4c79-a82d-7b58c5e64a30",
      title: "Regular Board Meeting",
    },
  ]],
  ["/v1/meetings/0b1d6184-5981-4c79-a82d-7b58c5e64a30/agenda-items?entity_id=ent_11111111-1111-4111-8111-111111111111", [
    {
      agenda_item_id: "2c2d6184-5981-4c79-a82d-7b58c5e64a30",
      title: "Approve Budget",
    },
  ]],
  ["/v1/entities/ent_11111111-1111-4111-8111-111111111111/safe-notes", [
    {
      safe_note_id: "safe_11111111-1111-4111-8111-111111111111",
      investor_name: "Alice Investor",
      safe_type: "post_money",
    },
  ]],
  ["/v1/entities/ent_11111111-1111-4111-8111-111111111111/share-transfers", [
    {
      transfer_id: "trn_11111111-1111-4111-8111-111111111111",
      from_holder: "Alice Johnson",
      to_holder: "Bob Buyer",
      transfer_type: "secondary_sale",
      status: "pending_review",
    },
  ]],
  ["/v1/entities/ent_11111111-1111-4111-8111-111111111111/invoices", [
    {
      invoice_id: "inv_11111111-1111-4111-8111-111111111111",
      customer_name: "Acme Customer",
      description: "Services rendered",
      due_date: "2026-03-31",
    },
  ]],
  ["/v1/entities/ent_11111111-1111-4111-8111-111111111111/cap-table", {
    issuer_legal_entity_id: "ent_11111111-1111-4111-8111-111111111111",
    instruments: [
      {
        instrument_id: "ins_11111111-1111-4111-8111-111111111111",
        symbol: "COMMON",
        kind: "common_equity",
      },
    ],
    share_classes: [
      {
        share_class_id: "cls_11111111-1111-4111-8111-111111111111",
        class_code: "COMMON",
        name: "Common Stock",
      },
    ],
  }],
  ["/v1/entities/ent_11111111-1111-4111-8111-111111111111/equity-rounds", [
    {
      round_id: "rnd_11111111-1111-4111-8111-111111111111",
      name: "Seed Round",
      status: "draft",
    },
  ]],
  ["/v1/entities/ent_11111111-1111-4111-8111-111111111111/tax-filings", [
    {
      filing_id: "fil_11111111-1111-4111-8111-111111111111",
      document_type: "form_1120",
      tax_year: 2026,
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
    const entityId = typeof body.entity_id === "string" ? body.entity_id : undefined;
    const kind = String(body.kind ?? "");
    const items = Array.isArray(body.items) ? body.items : [];
    return new Response(JSON.stringify({
      references: items.map((item) => ({
        kind,
        resource_id: item.resource_id,
        handle: slugify(item.label || item.resource_id),
        label: item.label,
        entity_id: entityId,
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

function runCli(configDir, mockPath, args) {
  const { PATH, HOME, TMPDIR, TMP, TEMP } = process.env;
  const stdoutPath = join(configDir, "stdout.txt");
  const stderrPath = join(configDir, "stderr.txt");
  const stdoutFd = openSync(stdoutPath, "w");
  const stderrFd = openSync(stderrPath, "w");
  const result = spawnSync(process.execPath, ["--import", mockPath, "dist/index.js", ...args], {
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

function parseJson(stdout) {
  return JSON.parse(stdout.trim());
}

test("resolve command handles slug, short ID, title, and @last", () => {
  const apiUrl = "https://api.thecorporation.ai";
  const configDir = makeConfigDir(apiUrl);
  const mockPath = makeFetchMockModule();

  let result = runCli(configDir, mockPath, ["resolve", "entity", "acme-holdings-llc"]);
  assert.equal(result.status, 0, result.stderr);
  assert.equal(
    parseJson(result.stdout).resolved_id,
    "ent_11111111-1111-4111-8111-111111111111",
  );

  result = runCli(configDir, mockPath, ["resolve", "entity", "ent_1111"]);
  assert.equal(result.status, 0, result.stderr);
  assert.equal(
    parseJson(result.stdout).resolved_id,
    "ent_11111111-1111-4111-8111-111111111111",
  );

  result = runCli(configDir, mockPath, ["resolve", "contact", "alice@example.com"]);
  assert.equal(result.status, 0, result.stderr);
  assert.equal(
    parseJson(result.stdout).resolved_id,
    "89facaea-abac-4001-9bac-e7143f58d4df",
  );

  result = runCli(configDir, mockPath, ["resolve", "contact", "89facaea"]);
  assert.equal(result.status, 0, result.stderr);
  assert.equal(
    parseJson(result.stdout).resolved_id,
    "89facaea-abac-4001-9bac-e7143f58d4df",
  );

  result = runCli(configDir, mockPath, ["resolve", "meeting", "regular-board-meeting"]);
  assert.equal(result.status, 0, result.stderr);
  assert.equal(
    parseJson(result.stdout).resolved_id,
    "0b1d6184-5981-4c79-a82d-7b58c5e64a30",
  );

  result = runCli(configDir, mockPath, [
    "resolve",
    "agenda_item",
    "approve-budget",
    "--meeting-id",
    "@last:meeting",
  ]);
  assert.equal(result.status, 0, result.stderr);
  assert.equal(
    parseJson(result.stdout).resolved_id,
    "2c2d6184-5981-4c79-a82d-7b58c5e64a30",
  );

  result = runCli(configDir, mockPath, ["find", "contact", "alice", "--json"]);
  assert.equal(result.status, 0, result.stderr);
  assert.deepEqual(parseJson(result.stdout).matches, [
    {
      kind: "contact",
      id: "89facaea-abac-4001-9bac-e7143f58d4df",
      short_id: "89facaea",
      alias: "alice-johnson",
      label: "Alice Johnson",
    },
  ]);

  result = runCli(configDir, mockPath, ["resolve", "safe_note", "alice-investor"]);
  assert.equal(result.status, 0, result.stderr);
  assert.equal(
    parseJson(result.stdout).resolved_id,
    "safe_11111111-1111-4111-8111-111111111111",
  );

  result = runCli(configDir, mockPath, ["resolve", "share_transfer", "alice-johnson"]);
  assert.equal(result.status, 0, result.stderr);
  assert.equal(
    parseJson(result.stdout).resolved_id,
    "trn_11111111-1111-4111-8111-111111111111",
  );

  result = runCli(configDir, mockPath, ["resolve", "invoice", "acme-customer"]);
  assert.equal(result.status, 0, result.stderr);
  assert.equal(
    parseJson(result.stdout).resolved_id,
    "inv_11111111-1111-4111-8111-111111111111",
  );

  result = runCli(configDir, mockPath, ["resolve", "instrument", "common"]);
  assert.equal(result.status, 0, result.stderr);
  assert.equal(
    parseJson(result.stdout).resolved_id,
    "ins_11111111-1111-4111-8111-111111111111",
  );

  result = runCli(configDir, mockPath, ["resolve", "share_class", "common-stock"]);
  assert.equal(result.status, 0, result.stderr);
  assert.equal(
    parseJson(result.stdout).resolved_id,
    "cls_11111111-1111-4111-8111-111111111111",
  );

  result = runCli(configDir, mockPath, ["resolve", "round", "seed-round"]);
  assert.equal(result.status, 0, result.stderr);
  assert.equal(
    parseJson(result.stdout).resolved_id,
    "rnd_11111111-1111-4111-8111-111111111111",
  );

  result = runCli(configDir, mockPath, ["find", "tax_filing", "1120", "--json"]);
  assert.equal(result.status, 0, result.stderr);
  assert.deepEqual(parseJson(result.stdout).matches, [
    {
      kind: "tax_filing",
      id: "fil_11111111-1111-4111-8111-111111111111",
      short_id: "fil_1111",
      alias: "form-1120",
      label: "form_1120",
    },
  ]);
});

test("resolve rejects unknown @last kinds", () => {
  const apiUrl = "https://api.thecorporation.ai";
  const configDir = makeConfigDir(apiUrl);
  const mockPath = makeFetchMockModule();

  const result = runCli(configDir, mockPath, ["resolve", "entity", "@last:enitty"]);
  assert.notEqual(result.status, 0, result.stdout);
  assert.match(result.stderr, /Unknown reference kind: enitty/);
});

test("@last does not fall back from entity-scoped refs to workspace-scoped refs", () => {
  const apiUrl = "https://api.thecorporation.ai";
  const configDir = makeConfigDir(apiUrl);
  const configPath = join(configDir, "config.json");
  const saved = JSON.parse(readFileSync(configPath, "utf8"));
  saved.last_references = {
    "workspace:ws_test:contact": "89facaea-abac-4001-9bac-e7143f58d4df",
  };
  writeFileSync(configPath, JSON.stringify(saved));

  const mockPath = makeFetchMockModule();
  const result = runCli(configDir, mockPath, ["resolve", "contact", "@last"]);
  assert.notEqual(result.status, 0, result.stdout);
  assert.match(result.stderr, /No contact is recorded for @last/);
});
