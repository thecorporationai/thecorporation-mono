import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { CorpClient, CorpApiError } from "./client.js";

// ── Helpers ─────────────────────────────────────────────────────────────────

function mockFetch(status: number, body: unknown = {}, ok = status >= 200 && status < 300) {
  return vi.fn().mockResolvedValue({
    ok,
    status,
    json: () => Promise.resolve(body),
    text: () => Promise.resolve(typeof body === "string" ? body : JSON.stringify(body)),
  });
}

let originalFetch: typeof globalThis.fetch;

beforeEach(() => {
  originalFetch = globalThis.fetch;
});

afterEach(() => {
  globalThis.fetch = originalFetch;
});

// ── CorpApiError ────────────────────────────────────────────────────────────

describe("CorpApiError", () => {
  it("is an Error instance", () => {
    const err = new CorpApiError(404, "not found", "/v1/entities/abc");
    expect(err).toBeInstanceOf(Error);
    expect(err.name).toBe("CorpApiError");
  });

  it("formats message as 'status path: body'", () => {
    const err = new CorpApiError(403, "forbidden", "/v1/billing");
    expect(err.message).toBe("403 /v1/billing: forbidden");
    expect(err.status).toBe(403);
    expect(err.path).toBe("/v1/billing");
    expect(err.body).toBe("forbidden");
  });
});

// ── Constructor ─────────────────────────────────────────────────────────────

describe("CorpClient constructor", () => {
  it("strips trailing slashes from baseUrl", async () => {
    const fetch = mockFetch(200, []);
    globalThis.fetch = fetch;
    const client = new CorpClient("http://example.com///", "key");
    await client.entities.list();
    expect(fetch).toHaveBeenCalledOnce();
    const url = fetch.mock.calls[0][0] as string;
    expect(url).toBe("http://example.com/v1/entities");
  });

  it("includes Authorization header when apiKey is provided", async () => {
    const fetch = mockFetch(200, []);
    globalThis.fetch = fetch;
    const client = new CorpClient("http://x", "corp_test_key");
    await client.entities.list();
    const headers = fetch.mock.calls[0][1]?.headers as Record<string, string>;
    expect(headers["Authorization"]).toBe("Bearer corp_test_key");
  });

  it("omits Authorization header when apiKey is not provided", async () => {
    const fetch = mockFetch(200, []);
    globalThis.fetch = fetch;
    const client = new CorpClient("http://x");
    await client.entities.list();
    const headers = fetch.mock.calls[0][1]?.headers as Record<string, string>;
    expect(headers["Authorization"]).toBeUndefined();
  });

  it("sets Content-Type and User-Agent headers", async () => {
    const fetch = mockFetch(200, []);
    globalThis.fetch = fetch;
    const client = new CorpClient("http://x");
    await client.entities.list();
    const headers = fetch.mock.calls[0][1]?.headers as Record<string, string>;
    expect(headers["Content-Type"]).toBe("application/json");
    expect(headers["User-Agent"]).toBe("corp-ts/0.1.0");
  });
});

// ── HTTP methods ────────────────────────────────────────────────────────────

describe("HTTP methods", () => {
  it("GET sends no body", async () => {
    const fetch = mockFetch(200, { id: "1" });
    globalThis.fetch = fetch;
    const client = new CorpClient("http://x");
    await client.get("/test");
    expect(fetch.mock.calls[0][1]?.method).toBe("GET");
    expect(fetch.mock.calls[0][1]?.body).toBeUndefined();
  });

  it("POST sends JSON body", async () => {
    const fetch = mockFetch(200, { ok: true });
    globalThis.fetch = fetch;
    const client = new CorpClient("http://x");
    await client.post("/test", { foo: "bar" });
    expect(fetch.mock.calls[0][1]?.method).toBe("POST");
    expect(fetch.mock.calls[0][1]?.body).toBe('{"foo":"bar"}');
  });

  it("PUT sends JSON body", async () => {
    const fetch = mockFetch(200, {});
    globalThis.fetch = fetch;
    const client = new CorpClient("http://x");
    await client.put("/test", { a: 1 });
    expect(fetch.mock.calls[0][1]?.method).toBe("PUT");
    expect(fetch.mock.calls[0][1]?.body).toBe('{"a":1}');
  });

  it("PATCH sends JSON body", async () => {
    const fetch = mockFetch(200, {});
    globalThis.fetch = fetch;
    const client = new CorpClient("http://x");
    await client.patch("/test", { b: 2 });
    expect(fetch.mock.calls[0][1]?.method).toBe("PATCH");
  });

  it("DELETE sends no body", async () => {
    const fetch = mockFetch(200, {});
    globalThis.fetch = fetch;
    const client = new CorpClient("http://x");
    await client.delete("/test");
    expect(fetch.mock.calls[0][1]?.method).toBe("DELETE");
    expect(fetch.mock.calls[0][1]?.body).toBeUndefined();
  });
});

// ── Response handling ───────────────────────────────────────────────────────

describe("response handling", () => {
  it("parses JSON response on 200", async () => {
    const fetch = mockFetch(200, { entity_id: "abc" });
    globalThis.fetch = fetch;
    const client = new CorpClient("http://x");
    const result = await client.get("/test");
    expect(result).toEqual({ entity_id: "abc" });
  });

  it("returns empty object on 204", async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true,
      status: 204,
    });
    const client = new CorpClient("http://x");
    const result = await client.get("/test");
    expect(result).toEqual({});
  });

  it("throws CorpApiError on non-ok response", async () => {
    const fetch = mockFetch(404, "entity not found", false);
    globalThis.fetch = fetch;
    const client = new CorpClient("http://x");
    await expect(client.get("/v1/entities/bad")).rejects.toThrow(CorpApiError);

    try {
      await client.get("/v1/entities/bad");
    } catch (err) {
      expect(err).toBeInstanceOf(CorpApiError);
      const e = err as CorpApiError;
      expect(e.status).toBe(404);
      expect(e.path).toBe("/v1/entities/bad");
      expect(e.body).toBe("entity not found");
    }
  });

  it("handles text() rejection gracefully during error", async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: false,
      status: 500,
      text: () => Promise.reject(new Error("stream error")),
    });
    const client = new CorpClient("http://x");
    try {
      await client.get("/fail");
    } catch (err) {
      const e = err as CorpApiError;
      expect(e.status).toBe(500);
      expect(e.body).toBe("");
    }
  });
});

// ── Sub-client URL routing ──────────────────────────────────────────────────

describe("sub-client URL routing", () => {
  let client: CorpClient;
  let fetch: ReturnType<typeof mockFetch>;

  beforeEach(() => {
    fetch = mockFetch(200, {});
    globalThis.fetch = fetch;
    client = new CorpClient("http://api.test", "key");
  });

  function lastUrl(): string {
    return fetch.mock.calls.at(-1)![0] as string;
  }
  function lastMethod(): string {
    return fetch.mock.calls.at(-1)![1]?.method as string;
  }

  // Entities
  it("entities.list", async () => {
    await client.entities.list();
    expect(lastUrl()).toBe("http://api.test/v1/entities");
    expect(lastMethod()).toBe("GET");
  });

  it("entities.get", async () => {
    await client.entities.get("ent-1" as any);
    expect(lastUrl()).toBe("http://api.test/v1/entities/ent-1");
  });

  it("entities.create", async () => {
    await client.entities.create({ legal_name: "A", entity_type: "c_corp" as any, jurisdiction: "DE" });
    expect(lastUrl()).toBe("http://api.test/v1/entities");
    expect(lastMethod()).toBe("POST");
  });

  it("entities.dissolve", async () => {
    await client.entities.dissolve("ent-1" as any);
    expect(lastUrl()).toBe("http://api.test/v1/entities/ent-1/dissolve");
    expect(lastMethod()).toBe("POST");
  });

  // Formation
  it("formation.advance", async () => {
    await client.formation.advance("ent-1" as any);
    expect(lastUrl()).toBe("http://api.test/v1/formations/ent-1/advance");
  });

  it("formation.listDocuments", async () => {
    await client.formation.listDocuments("ent-1" as any);
    expect(lastUrl()).toBe("http://api.test/v1/formations/ent-1/documents");
    expect(lastMethod()).toBe("GET");
  });

  it("formation.signDocument uses /v1/documents/ not /v1/formations/", async () => {
    await client.formation.signDocument("doc-1" as any, {
      signer_name: "Jane", signer_role: "Director", signer_email: "j@x.com",
      signature_text: "/s/ Jane", consent_text: "I consent",
    });
    expect(lastUrl()).toBe("http://api.test/v1/documents/doc-1/sign");
  });

  // Equity
  it("equity.getCapTable", async () => {
    await client.equity.getCapTable("ent-1" as any);
    expect(lastUrl()).toBe("http://api.test/v1/entities/ent-1/cap-table");
  });

  it("equity.createGrant", async () => {
    await client.equity.createGrant("ent-1" as any, { shares: 1000 });
    expect(lastUrl()).toBe("http://api.test/v1/entities/ent-1/grants");
    expect(lastMethod()).toBe("POST");
  });

  // Governance
  it("governance.createMeeting", async () => {
    await client.governance.createMeeting("ent-1" as any, {
      body_id: "b-1" as any, meeting_type: "board", title: "Q1",
    });
    expect(lastUrl()).toBe("http://api.test/v1/entities/ent-1/governance/meetings");
  });

  it("governance.castVote", async () => {
    await client.governance.castVote("ent-1" as any, "mtg-1" as any, {
      agenda_item_id: "ai-1" as any, seat_id: "s-1" as any, value: "for",
    });
    expect(lastUrl()).toBe("http://api.test/v1/entities/ent-1/governance/meetings/mtg-1/votes");
  });

  // Treasury
  it("treasury.createInvoice", async () => {
    await client.treasury.createInvoice("ent-1" as any, { amount: 100 });
    expect(lastUrl()).toBe("http://api.test/v1/entities/ent-1/invoices");
    expect(lastMethod()).toBe("POST");
  });

  // Contacts
  it("contacts.update uses PATCH", async () => {
    await client.contacts.update("ent-1" as any, "c-1" as any, { name: "New" });
    expect(lastUrl()).toBe("http://api.test/v1/entities/ent-1/contacts/c-1");
    expect(lastMethod()).toBe("PATCH");
  });

  // Agents (no entity prefix)
  it("agents.list has no entity prefix", async () => {
    await client.agents.list();
    expect(lastUrl()).toBe("http://api.test/v1/agents");
  });

  it("agents.delete uses DELETE method", async () => {
    await client.agents.delete("ag-1" as any);
    expect(lastUrl()).toBe("http://api.test/v1/agents/ag-1");
    expect(lastMethod()).toBe("DELETE");
  });

  // Work Items
  it("workItems.claim", async () => {
    await client.workItems.claim("ent-1" as any, "wi-1" as any, { claimed_by: "me" });
    expect(lastUrl()).toBe("http://api.test/v1/entities/ent-1/work-items/wi-1/claim");
  });

  // Admin
  it("admin.health uses /health not /v1/health", async () => {
    await client.admin.health();
    expect(lastUrl()).toBe("http://api.test/health");
  });

  it("admin.createApiKey", async () => {
    await client.admin.createApiKey({ name: "test" });
    expect(lastUrl()).toBe("http://api.test/v1/api-keys");
    expect(lastMethod()).toBe("POST");
  });
});
