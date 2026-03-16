/**
 * Integration tests for CorpAPIClient — every method against a real api-rs instance.
 *
 * Skipped when CORP_API_URL is not set or the server is unreachable.
 * Run via: CORP_API_URL=http://localhost:8000 npx vitest run --config vitest.integration.config.ts
 */
import { describe, it, expect, beforeAll } from "vitest";
import { CorpAPIClient, provisionWorkspace } from "../api-client.js";
import type { ApiRecord } from "../types.js";

const API_URL = process.env.CORP_API_URL ?? "";

async function serverReachable(): Promise<boolean> {
  if (!API_URL) return false;
  try {
    const resp = await fetch(`${API_URL}/health`, { signal: AbortSignal.timeout(3000) });
    return resp.ok;
  } catch {
    return false;
  }
}

// Helper: provision a workspace and return an authenticated client
async function freshClient(name: string): Promise<{ client: CorpAPIClient; wsId: string; apiKey: string }> {
  const ws = await provisionWorkspace(API_URL, name);
  const wsId = ws.workspace_id as string;
  const apiKey = ws.api_key as string;
  return { client: new CorpAPIClient(API_URL, apiKey, wsId), wsId, apiKey };
}

// Expect a structured JSON response (not 422/500).  404/400 are acceptable for
// endpoints that require prerequisite data we haven't set up.
function expectStructuredResponse(val: unknown): void {
  expect(val).toBeDefined();
  expect(val).not.toBeNull();
}

// ── Gate ────────────────────────────────────────────────────────────

let canRun = false;

beforeAll(async () => {
  canRun = await serverReachable();
  if (!canRun) {
    console.warn("⚠ CORP_API_URL not set or server unreachable — skipping integration tests");
  }
});

function skipIfNoServer() {
  if (!canRun) return true;
  return false;
}

// ── provisionWorkspace ─────────────────────────────────────────────

describe("provisionWorkspace", () => {
  it.skipIf(!canRun)("provision with name succeeds", async () => {
    const ws = await provisionWorkspace(API_URL, "Integration Test WS");
    expect(ws.workspace_id).toBeDefined();
    expect(ws.api_key).toBeDefined();
    expect((ws.api_key as string).startsWith("sk_")).toBe(true);
    expect(ws.name).toBe("Integration Test WS");
  });

  it.skipIf(!canRun)("provision without name returns validation error", async () => {
    await expect(provisionWorkspace(API_URL)).rejects.toThrow(/Validation error/);
  });
});

// ── Workspace endpoints ────────────────────────────────────────────

describe("workspace endpoints", () => {
  let client: CorpAPIClient;

  beforeAll(async () => {
    if (skipIfNoServer()) return;
    ({ client } = await freshClient("WS Endpoints Test"));
  });

  it.skipIf(!canRun)("getStatus", async () => {
    const status = await client.getStatus();
    expectStructuredResponse(status);
  });

  it.skipIf(!canRun)("listEntities", async () => {
    const entities = await client.listEntities();
    expect(Array.isArray(entities)).toBe(true);
  });

  it.skipIf(!canRun)("listContacts", async () => {
    // listContacts requires an entity_id; skip if no entities exist
    const entities = await client.listEntities();
    if (entities.length > 0) {
      const eid = (entities[0] as any).entity_id;
      const contacts = await client.listContacts(eid);
      expect(Array.isArray(contacts)).toBe(true);
    }
  });

  it.skipIf(!canRun)("getObligations", async () => {
    const obs = await client.getObligations();
    expectStructuredResponse(obs);
  });

  it.skipIf(!canRun)("getObligations with tier", async () => {
    const obs = await client.getObligations("tier1");
    expectStructuredResponse(obs);
  });

  it.skipIf(!canRun)("listDigests", async () => {
    const digests = await client.listDigests();
    expect(Array.isArray(digests)).toBe(true);
  });

  it.skipIf(!canRun)("triggerDigest", async () => {
    const result = await client.triggerDigest();
    expectStructuredResponse(result);
  });

  it.skipIf(!canRun)("getBillingStatus", async () => {
    const billing = await client.getBillingStatus();
    expectStructuredResponse(billing);
  });

  it.skipIf(!canRun)("getBillingPlans", async () => {
    const plans = await client.getBillingPlans();
    expect(Array.isArray(plans)).toBe(true);
  });

  it.skipIf(!canRun)("listApiKeys", async () => {
    const keys = await client.listApiKeys();
    expect(Array.isArray(keys)).toBe(true);
    expect(keys.length).toBeGreaterThanOrEqual(1);
  });

  it.skipIf(!canRun)("getConfig", async () => {
    const config = await client.getConfig();
    expectStructuredResponse(config);
  });

  it.skipIf(!canRun)("getHumanObligations", async () => {
    const obs = await client.getHumanObligations();
    expect(Array.isArray(obs)).toBe(true);
  });

  it.skipIf(!canRun)("listSupportedModels", async () => {
    const models = await client.listSupportedModels();
    expect(Array.isArray(models)).toBe(true);
  });

  it.skipIf(!canRun)("listAgents", async () => {
    const agents = await client.listAgents();
    expect(Array.isArray(agents)).toBe(true);
  });

  // Approvals: no standalone endpoint; managed through governance meetings.

  it.skipIf(!canRun)("createLink", async () => {
    const link = await client.createLink("test-ext-id", "test-provider");
    expectStructuredResponse(link);
  });
});

// ── Entity lifecycle ───────────────────────────────────────────────

describe("entity lifecycle", () => {
  let client: CorpAPIClient;
  let entityId: string;
  let formationId: string;

  beforeAll(async () => {
    if (skipIfNoServer()) return;
    ({ client } = await freshClient("Entity Lifecycle Test"));

    // Create a formation to get an entity
    const formation = await client.createFormation({
      entity_type: "corporation",
      legal_name: "IntegTest Corp",
      jurisdiction: "Delaware",
      members: [
        {
          name: "Test Founder",
          investor_type: "natural_person",
          email: "founder@test.com",
          ownership_pct: 100.0,
        },
      ],
    });
    formationId = formation.entity_id as string ?? formation.id as string;
    entityId = formationId;
  });

  it.skipIf(!canRun)("createFormation", () => {
    expect(entityId).toBeDefined();
  });

  it.skipIf(!canRun)("getFormation", async () => {
    const f = await client.getFormation(formationId);
    expectStructuredResponse(f);
  });

  it.skipIf(!canRun)("getFormationDocuments", async () => {
    const docs = await client.getFormationDocuments(formationId);
    expect(Array.isArray(docs)).toBe(true);
  });

  it.skipIf(!canRun)("getEntityDocuments", async () => {
    const docs = await client.getEntityDocuments(entityId);
    expect(Array.isArray(docs)).toBe(true);
  });

  it.skipIf(!canRun)("getCapTable", async () => {
    try {
      const cap = await client.getCapTable(entityId);
      expectStructuredResponse(cap);
    } catch (e: any) {
      // 404 acceptable if entity not fully active
      expect(e.message).toMatch(/Not found|HTTP 40[04]/);
    }
  });

  it.skipIf(!canRun)("getSafeNotes", async () => {
    try {
      const notes = await client.getSafeNotes(entityId);
      expect(Array.isArray(notes)).toBe(true);
    } catch (e: any) {
      expect(e.message).toMatch(/Not found|HTTP 40[04]/);
    }
  });

  it.skipIf(!canRun)("getShareTransfers", async () => {
    try {
      const transfers = await client.getShareTransfers(entityId);
      expect(Array.isArray(transfers)).toBe(true);
    } catch (e: any) {
      expect(e.message).toMatch(/Not found|HTTP 40[04]/);
    }
  });

  it.skipIf(!canRun)("getValuations", async () => {
    try {
      const vals = await client.getValuations(entityId);
      expect(Array.isArray(vals)).toBe(true);
    } catch (e: any) {
      expect(e.message).toMatch(/Not found|HTTP 40[04]/);
    }
  });

  it.skipIf(!canRun)("getCurrent409a", async () => {
    try {
      const val = await client.getCurrent409a(entityId);
      expectStructuredResponse(val);
    } catch (e: any) {
      expect(e.message).toMatch(/Not found|HTTP 40[04]/);
    }
  });

  it.skipIf(!canRun)("listGovernanceBodies", async () => {
    try {
      const bodies = await client.listGovernanceBodies(entityId);
      expect(Array.isArray(bodies)).toBe(true);
    } catch (e: any) {
      expect(e.message).toMatch(/Not found|HTTP 40[04]/);
    }
  });

  // Write endpoints — may fail with 400/404 due to prerequisite state, but should not 422/500
  const writeEndpoints: Array<{ name: string; fn: (c: CorpAPIClient, eid: string) => Promise<unknown> }> = [
    { name: "transferShares", fn: (c, eid) => c.transferShares({ entity_id: eid, from_holder_id: "h_1", to_holder_id: "h_2", quantity: 100 }) },
    { name: "calculateDistribution", fn: (c, eid) => c.calculateDistribution({ entity_id: eid, total_amount: 10000 }) },
    { name: "createInvoice", fn: (c, eid) => c.createInvoice({ entity_id: eid, amount: 1000, description: "Test" }) },
    { name: "runPayroll", fn: (c, eid) => c.runPayroll({ entity_id: eid }) },
    { name: "submitPayment", fn: (c, eid) => c.submitPayment({ entity_id: eid, amount: 500 }) },
    { name: "openBankAccount", fn: (c, eid) => c.openBankAccount({ entity_id: eid, bank_name: "Test Bank" }) },
    { name: "classifyContractor", fn: (c, eid) => c.classifyContractor({ entity_id: eid, name: "Contractor" }) },
    { name: "reconcileLedger", fn: (c, eid) => c.reconcileLedger({ entity_id: eid }) },
    { name: "generateContract", fn: (c, eid) => c.generateContract({ entity_id: eid, template: "nda" }) },
    { name: "fileTaxDocument", fn: (c, eid) => c.fileTaxDocument({ entity_id: eid, type: "1099" }) },
    { name: "trackDeadline", fn: (c, eid) => c.trackDeadline({ entity_id: eid, description: "Q1 Filing" }) },
    { name: "convertEntity", fn: (c, eid) => c.convertEntity(eid, { target_type: "llc" }) },
    { name: "dissolveEntity", fn: (c, eid) => c.dissolveEntity(eid, { reason: "test" }) },
  ];

  for (const { name, fn } of writeEndpoints) {
    it.skipIf(!canRun)(`${name} sends well-formed request`, async () => {
      try {
        const result = await fn(client, entityId);
        expectStructuredResponse(result);
      } catch (e: any) {
        // 400/404 is acceptable (missing prereqs), but 422 or 500 means broken request
        expect(e.message).not.toMatch(/Validation error/);
        expect(e.message).not.toMatch(/Server error/);
      }
    });
  }
});

// ── Equity rounds ──────────────────────────────────────────────────

describe("equity rounds", () => {
  let client: CorpAPIClient;
  let entityId: string;

  beforeAll(async () => {
    if (skipIfNoServer()) return;
    ({ client } = await freshClient("Equity Rounds Test"));
    const formation = await client.createFormation({
      entity_type: "corporation",
      legal_name: "RoundTest Corp",
      jurisdiction: "Delaware",
      members: [{ name: "Founder", investor_type: "natural_person", email: "f@test.com", ownership_pct: 100 }],
    });
    entityId = (formation.entity_id ?? formation.id) as string;
  });

  it.skipIf(!canRun)("createEquityRound", async () => {
    try {
      const round = await client.createEquityRound({ entity_id: entityId, name: "Seed", issuer_legal_entity_id: "le_fake" });
      expectStructuredResponse(round);
    } catch (e: any) {
      expect(e.message).not.toMatch(/Validation error/);
      expect(e.message).not.toMatch(/Server error/);
    }
  });

  // These require a valid round ID — test they send well-formed requests
  const roundMethods = [
    { name: "applyEquityRoundTerms", fn: (c: CorpAPIClient) => c.applyEquityRoundTerms("fake-round", { entity_id: "ent_1", anti_dilution_method: "broad_weighted_average" }) },
    { name: "boardApproveEquityRound", fn: (c: CorpAPIClient) => c.boardApproveEquityRound("fake-round", { entity_id: "ent_1", meeting_id: "m_1", resolution_id: "res_1" }) },
    { name: "acceptEquityRound", fn: (c: CorpAPIClient) => c.acceptEquityRound("fake-round", { entity_id: "ent_1", intent_id: "intent_1" }) },
    { name: "previewRoundConversion", fn: (c: CorpAPIClient) => c.previewRoundConversion({ entity_id: "ent_1", round_id: "fake-round" }) },
    { name: "executeRoundConversion", fn: (c: CorpAPIClient) => c.executeRoundConversion({ entity_id: "ent_1", round_id: "fake-round", intent_id: "intent_1" }) },
  ];

  for (const { name, fn } of roundMethods) {
    it.skipIf(!canRun)(`${name} sends well-formed request`, async () => {
      try {
        await fn(client);
      } catch (e: any) {
        expect(e.message).not.toMatch(/Validation error/);
        expect(e.message).not.toMatch(/Server error/);
      }
    });
  }
});

// ── Intent lifecycle ───────────────────────────────────────────────

describe("intent lifecycle", () => {
  let client: CorpAPIClient;
  let entityId: string;

  beforeAll(async () => {
    if (skipIfNoServer()) return;
    ({ client } = await freshClient("Intent Lifecycle Test"));
    const formation = await client.createFormation({
      entity_type: "corporation",
      legal_name: "IntentTest Corp",
      jurisdiction: "Delaware",
      members: [{ name: "Founder", investor_type: "natural_person", email: "f@test.com", ownership_pct: 100 }],
    });
    entityId = (formation.entity_id ?? formation.id) as string;
  });

  it.skipIf(!canRun)("createExecutionIntent", async () => {
    try {
      const intent = await client.createExecutionIntent({
        entity_id: entityId,
        intent_type: "issue_equity",
        authority_tier: "board_majority",
        description: "Issue 1000 shares",
      });
      expectStructuredResponse(intent);
    } catch (e: any) {
      expect(e.message).not.toMatch(/Validation error/);
      expect(e.message).not.toMatch(/Server error/);
    }
  });

  it.skipIf(!canRun)("evaluateIntent", async () => {
    try {
      await client.evaluateIntent("fake-intent", entityId);
    } catch (e: any) {
      expect(e.message).not.toMatch(/Validation error/);
      expect(e.message).not.toMatch(/Server error/);
    }
  });

  it.skipIf(!canRun)("authorizeIntent", async () => {
    try {
      await client.authorizeIntent("fake-intent", entityId);
    } catch (e: any) {
      expect(e.message).not.toMatch(/Validation error/);
      expect(e.message).not.toMatch(/Server error/);
    }
  });
});

// ── Governance ─────────────────────────────────────────────────────

describe("governance", () => {
  let client: CorpAPIClient;
  let entityId: string;

  beforeAll(async () => {
    if (skipIfNoServer()) return;
    ({ client } = await freshClient("Governance Test"));
    const formation = await client.createFormation({
      entity_type: "corporation",
      legal_name: "GovTest Corp",
      jurisdiction: "Delaware",
      members: [{ name: "Founder", investor_type: "natural_person", email: "f@test.com", ownership_pct: 100 }],
    });
    entityId = (formation.entity_id ?? formation.id) as string;
  });

  it.skipIf(!canRun)("scheduleMeeting", async () => {
    try {
      await client.scheduleMeeting({ entity_id: entityId, title: "Board Meeting", body_type: "board" });
    } catch (e: any) {
      expect(e.message).not.toMatch(/Validation error/);
      expect(e.message).not.toMatch(/Server error/);
    }
  });

  it.skipIf(!canRun)("conveneMeeting", async () => {
    try {
      await client.conveneMeeting("fake-meeting", entityId, { quorum_present: true });
    } catch (e: any) {
      expect(e.message).not.toMatch(/Validation error/);
      expect(e.message).not.toMatch(/Server error/);
    }
  });

  it.skipIf(!canRun)("castVote", async () => {
    try {
      await client.castVote(entityId, "fake-meeting", "fake-item", { vote: "yes", voter_id: "voter_1" });
    } catch (e: any) {
      expect(e.message).not.toMatch(/Validation error/);
      expect(e.message).not.toMatch(/Server error/);
    }
  });

  it.skipIf(!canRun)("getMeetingResolutions", async () => {
    try {
      await client.getMeetingResolutions("fake-meeting", "fake-entity");
    } catch (e: any) {
      expect(e.message).not.toMatch(/Validation error/);
      expect(e.message).not.toMatch(/Server error/);
    }
  });
});

// ── Contacts ───────────────────────────────────────────────────────

describe("contacts", () => {
  let client: CorpAPIClient;
  let contactId: string;
  let entityId: string;

  beforeAll(async () => {
    if (skipIfNoServer()) return;
    ({ client } = await freshClient("Contacts Test"));
    const formation = await client.createFormation({
      entity_type: "llc",
      legal_name: "Contacts Test LLC",
      jurisdiction: "US-WY",
      members: [{ name: "Owner", investor_type: "natural_person", email: "o@test.com", ownership_pct: 100 }],
    });
    entityId = (formation.entity_id ?? formation.id) as string;
  });

  it.skipIf(!canRun)("createContact", async () => {
    const contact = await client.createContact({
      entity_id: entityId,
      name: "Integration Contact",
      email: "contact@integration.test",
    });
    expectStructuredResponse(contact);
    contactId = (contact.id ?? contact.contact_id) as string;
    expect(contactId).toBeDefined();
  });

  it.skipIf(!canRun)("getContact", async () => {
    const contact = await client.getContact(contactId, entityId);
    expectStructuredResponse(contact);
  });

  it.skipIf(!canRun)("updateContact", async () => {
    const updated = await client.updateContact(contactId, { entity_id: entityId, name: "Updated Contact" });
    expectStructuredResponse(updated);
  });

  it.skipIf(!canRun)("getContactProfile", async () => {
    try {
      const profile = await client.getContactProfile(contactId, entityId);
      expectStructuredResponse(profile);
    } catch (e: any) {
      expect(e.message).not.toMatch(/Validation error/);
      expect(e.message).not.toMatch(/Server error/);
    }
  });

  it.skipIf(!canRun)("getNotificationPrefs", async () => {
    try {
      const prefs = await client.getNotificationPrefs(contactId);
      expectStructuredResponse(prefs);
    } catch (e: any) {
      expect(e.message).not.toMatch(/Validation error/);
      expect(e.message).not.toMatch(/Server error/);
    }
  });

  it.skipIf(!canRun)("updateNotificationPrefs", async () => {
    try {
      const prefs = await client.updateNotificationPrefs(contactId, { email: true });
      expectStructuredResponse(prefs);
    } catch (e: any) {
      expect(e.message).not.toMatch(/Validation error/);
      expect(e.message).not.toMatch(/Server error/);
    }
  });
});

// ── Agents ─────────────────────────────────────────────────────────

describe("agents", () => {
  let client: CorpAPIClient;
  let agentId: string;

  beforeAll(async () => {
    if (skipIfNoServer()) return;
    ({ client } = await freshClient("Agents Test"));
  });

  it.skipIf(!canRun)("createAgent", async () => {
    const agent = await client.createAgent({
      name: "Test Agent",
      model: "claude-sonnet-4-20250514",
      system_prompt: "You are a test agent.",
    });
    expectStructuredResponse(agent);
    agentId = (agent.id ?? agent.agent_id) as string;
    expect(agentId).toBeDefined();
  });

  it.skipIf(!canRun)("getAgent", async () => {
    const agent = await client.getAgent(agentId);
    expectStructuredResponse(agent);
  });

  it.skipIf(!canRun)("updateAgent", async () => {
    const updated = await client.updateAgent(agentId, { name: "Updated Agent" });
    expectStructuredResponse(updated);
  });

  it.skipIf(!canRun)("sendAgentMessage", async () => {
    try {
      const result = await client.sendAgentMessage(agentId, "Hello from integration test");
      expectStructuredResponse(result);
    } catch (e: any) {
      // May fail without Redis — that's OK
      expect(e.message).not.toMatch(/Validation error/);
    }
  });

  // listAgentExecutions and getAgentUsage: no list endpoint exists.

  it.skipIf(!canRun)("addAgentSkill", async () => {
    try {
      const result = await client.addAgentSkill(agentId, { name: "test_skill", description: "A test skill" });
      expectStructuredResponse(result);
    } catch (e: any) {
      expect(e.message).not.toMatch(/Validation error/);
      expect(e.message).not.toMatch(/Server error/);
    }
  });

  it.skipIf(!canRun)("deleteAgent", async () => {
    await client.deleteAgent(agentId);
    // Verify deletion — agent list should not include the deleted agent
    const agents = await client.listAgents();
    const found = (agents as ApiRecord[]).find((a) => (a.id ?? a.agent_id) === agentId);
    expect(found).toBeUndefined();
  });
});

// ── Demo ───────────────────────────────────────────────────────────

describe("demo", () => {
  it.skipIf(!canRun)("seedDemo", async () => {
    const { client } = await freshClient("Demo Seed Test");
    const result = await client.seedDemo("integration-test-corp");
    expectStructuredResponse(result);
  });
});
