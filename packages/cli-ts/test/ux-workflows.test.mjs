import assert from "node:assert/strict";
import { mkdtempSync, mkdirSync, openSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import test from "node:test";

const CLI_DIR = fileURLToPath(new URL("..", import.meta.url));

function makeConfigDir(apiUrl = "https://api.thecorporation.ai") {
  const dir = mkdtempSync(join(tmpdir(), "corp-ux-test-"));
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
  const dir = mkdtempSync(join(tmpdir(), "corp-ux-fetch-"));
  const path = join(dir, "fetch-mock.mjs");
  writeFileSync(
    path,
    `
let documentSigned = false;
let formationStatus = "documents_generated";
let attested = false;
let evidenceCount = 0;

function json(body, status = 200) {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json" },
  });
}

function parseBody(init = {}) {
  return init.body ? JSON.parse(String(init.body)) : {};
}

function formationResponse() {
  return {
    entity_id: "ent_test",
    formation_id: "ent_test",
    legal_name: "Acme Test Inc.",
    entity_type: "c_corp",
    jurisdiction: "US-DE",
    formation_status: formationStatus,
    next_action: formationStatus === "active" ? null : "continue",
  };
}

function documentSummary() {
  return {
    document_id: "doc_123",
    title: "Bylaws",
    document_type: "bylaws",
    status: documentSigned ? "signed" : "draft",
    signatures: documentSigned ? [{ signer_role: "Director" }] : [],
  };
}

function fullDocument() {
  return {
    ...documentSummary(),
    content: {
      signature_requirements: [
        {
          role: "Director",
          signer_name: "Alice Director",
          signer_email: "alice@example.com",
        },
      ],
    },
  };
}

function blockers() {
  const list = [];
  if (!attested) list.push("filing attestation required");
  if (evidenceCount === 0) list.push("registered agent consent evidence required");
  return list;
}

globalThis.fetch = async function mockedFetch(input, init = {}) {
  const method = typeof input === "string"
    ? String(init.method ?? "GET").toUpperCase()
    : String(input.method ?? "GET").toUpperCase();
  const url = typeof input === "string" ? input : input.url;
  const parsed = new URL(url);
  const key = method + " " + parsed.pathname + parsed.search;
  const body = parseBody(init);

  if (key === "GET /v1/entities") {
    return json([{ entity_id: "ent_test", legal_name: "Acme Test Inc." }]);
  }
  if (key === "GET /v1/agents") {
    return json([{ agent_id: "agt_123", name: "Demo Operator" }]);
  }
  if (key === "GET /v1/entities/ent_test/work-items") {
    return json([{ work_item_id: "wi_123", title: "Review demo", status: "open" }]);
  }
  if (key === "POST /v1/entities/ent_test/work-items/wi_123/claim") {
    if (body.claimed_by_actor?.actor_type !== "agent" || body.claimed_by_actor?.actor_id !== "agt_123") {
      return json({ error: { detail: "bad claim payload" } }, 400);
    }
    return json({
      work_item_id: "wi_123",
      title: "Review demo",
      status: "claimed",
      effective_status: "claimed",
      claimed_by: "Demo Operator",
      claimed_by_actor: {
        actor_type: "agent",
        actor_id: "agt_123",
        label: "Demo Operator",
      },
    });
  }
  if (key === "GET /v1/entities/ent_test/invoices") {
    return json([
      {
        invoice_id: "inv_123",
        customer_name: "Acme Customer",
        amount_cents: 125000,
        due_date: "2026-04-01",
        status: "open",
      },
    ]);
  }
  if (key === "GET /v1/entities/ent_test/bank-accounts") {
    return json([
      {
        bank_account_id: "bank_123",
        bank_name: "Mercury",
        status: "active",
      },
    ]);
  }
  if (key === "GET /v1/entities/ent_test/payments") {
    return json([
      {
        payment_id: "pay_123",
        amount_cents: 50000,
        recipient: "Vendor",
        status: "pending",
        submitted_at: "2026-03-12T00:00:00Z",
      },
    ]);
  }
  if (key === "GET /v1/entities/ent_test/payroll-runs") {
    return json([
      {
        payroll_run_id: "run_123",
        pay_period_start: "2026-03-01",
        pay_period_end: "2026-03-15",
        status: "completed",
      },
    ]);
  }
  if (key === "GET /v1/entities/ent_test/distributions") {
    return json([
      {
        distribution_id: "dist_123",
        amount_cents: 75000,
        status: "declared",
        declared_at: "2026-03-10T00:00:00Z",
      },
    ]);
  }
  if (key === "GET /v1/entities/ent_test/reconciliations") {
    return json([
      {
        reconciliation_id: "rec_123",
        as_of_date: "2026-03-11",
        is_balanced: true,
        status: "completed",
      },
    ]);
  }
  if (key === "GET /v1/entities/ent_test/contractor-classifications") {
    return json([
      {
        classification_id: "cls_123",
        contractor_name: "Pat Consultant",
        risk_level: "high",
      },
    ]);
  }
  if (key === "GET /v1/formations/ent_test/documents") {
    return json([documentSummary()]);
  }
  if (key === "GET /v1/documents/doc_123?entity_id=ent_test") {
    return json(fullDocument());
  }
  if (key === "POST /v1/documents/doc_123/sign?entity_id=ent_test") {
    documentSigned = true;
    return json({
      signature_id: "sig_123",
      document_id: "doc_123",
      document_status: "signed",
      signed_at: "2026-03-12T00:00:00Z",
    });
  }
  if (key === "GET /v1/formations/ent_test") {
    return json(formationResponse());
  }
  if (key === "POST /v1/formations/ent_test/mark-documents-signed") {
    formationStatus = "documents_signed";
    return json(formationResponse());
  }
  if (key === "GET /v1/formations/ent_test/gates") {
    return json({
      entity_id: "ent_test",
      filing_submission_blockers: blockers(),
      requires_natural_person_attestation: true,
      designated_attestor_name: "Alice Director",
      designated_attestor_email: "alice@example.com",
      designated_attestor_role: "Director",
      attestation_recorded: attested,
      requires_registered_agent_consent_evidence: true,
      registered_agent_consent_evidence_count: evidenceCount,
      service_agreement_required_for_tier1_autonomy: false,
      service_agreement_executed: false,
      service_agreement_executed_at: null,
      service_agreement_contract_id: null,
      service_agreement_document_id: null,
      service_agreement_notes: null,
    });
  }
  if (key === "POST /v1/formations/ent_test/filing-attestation") {
    attested = true;
    return json({ ok: true });
  }
  if (key === "POST /v1/formations/ent_test/registered-agent-consent-evidence") {
    evidenceCount = 1;
    return json({ ok: true });
  }
  if (key === "POST /v1/formations/ent_test/submit-filing") {
    formationStatus = "filing_submitted";
    return json(formationResponse());
  }
  if (key === "POST /v1/formations/ent_test/filing-confirmation") {
    formationStatus = "filed";
    return json(formationResponse());
  }
  if (key === "POST /v1/formations/ent_test/apply-ein") {
    formationStatus = "ein_applied";
    return json(formationResponse());
  }
  if (key === "POST /v1/formations/ent_test/ein-confirmation") {
    formationStatus = "active";
    return json(formationResponse());
  }
  if (method === "POST" && parsed.pathname === "/v1/references/sync") {
    const syncBody = parseBody(init);
    return json({
      references: (syncBody.items ?? []).map((item) => ({
        kind: syncBody.kind,
        resource_id: item.resource_id,
        handle: String(item.label ?? item.resource_id).toLowerCase().replace(/[^a-z0-9]+/g, "-"),
        label: item.label,
        entity_id: syncBody.entity_id,
        created_at: "2026-03-12T00:00:00Z",
        updated_at: "2026-03-12T00:00:00Z",
      })),
    });
  }

  return json({ error: { detail: key } }, 404);
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

test("work-items claim accepts agent refs and sends typed actor payloads", () => {
  const configDir = makeConfigDir();
  const mockPath = makeFetchMockModule();
  const result = runCli(
    ["work-items", "claim", "review-demo", "--by", "demo-operator", "--json"],
    configDir,
    ["--import", mockPath],
  );
  assert.equal(result.status, 0, result.stderr);
  const parsed = JSON.parse(result.stdout);
  assert.equal(parsed.claimed_by_actor.actor_type, "agent");
  assert.equal(parsed.claimed_by_actor.actor_id, "agt_123");
});

test("finance top-level returns a JSON dashboard summary", () => {
  const configDir = makeConfigDir();
  const mockPath = makeFetchMockModule();
  const result = runCli(["finance", "--json"], configDir, ["--import", mockPath]);
  assert.equal(result.status, 0, result.stderr);
  const parsed = JSON.parse(result.stdout);
  assert.equal(parsed.invoices.count, 1);
  assert.equal(parsed.invoices.total_amount_cents, 125000);
  assert.equal(parsed.bank_accounts.active_count, 1);
  assert.equal(parsed.contractor_classifications.high_risk_count, 1);
});

test("documents sign auto-signs missing required formation signatures", () => {
  const configDir = makeConfigDir();
  const mockPath = makeFetchMockModule();
  const result = runCli(
    ["documents", "sign", "bylaws", "--json"],
    configDir,
    ["--import", mockPath],
  );
  assert.equal(result.status, 0, result.stderr);
  const parsed = JSON.parse(result.stdout);
  assert.equal(parsed.signatures_added, 1);
  assert.equal(parsed.document.status, "signed");
});

test("form activate advances a documents_generated entity to active", () => {
  const configDir = makeConfigDir();
  const mockPath = makeFetchMockModule();
  const result = runCli(
    ["form", "activate", "ent_test", "--json"],
    configDir,
    ["--import", mockPath],
  );
  assert.equal(result.status, 0, result.stderr);
  const parsed = JSON.parse(result.stdout);
  assert.equal(parsed.final_status, "active");
  assert.ok(parsed.steps.some((step) => /submitted filing/i.test(step)));
  assert.equal(parsed.formation.formation_status, "active");
});
