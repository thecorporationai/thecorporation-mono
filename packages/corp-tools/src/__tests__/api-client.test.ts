import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { CorpAPIClient, provisionWorkspace } from "../api-client.js";

// ── provisionWorkspace ──────────────────────────────────────────────

describe("provisionWorkspace", () => {
  const originalFetch = globalThis.fetch;

  afterEach(() => {
    globalThis.fetch = originalFetch;
  });

  it("includes name in body when provided", async () => {
    let capturedBody: string | undefined;
    globalThis.fetch = vi.fn(async (_url: string | URL | Request, init?: RequestInit) => {
      capturedBody = init?.body as string;
      return new Response(JSON.stringify({ workspace_id: "ws_1", api_key: "sk_test" }), {
        status: 200,
        headers: { "Content-Type": "application/json" },
      });
    });

    await provisionWorkspace("http://localhost:8000", "My Workspace");
    const parsed = JSON.parse(capturedBody!);
    expect(parsed).toEqual({ name: "My Workspace" });
  });

  it("sends empty body when name is not provided", async () => {
    let capturedBody: string | undefined;
    globalThis.fetch = vi.fn(async (_url: string | URL | Request, init?: RequestInit) => {
      capturedBody = init?.body as string;
      return new Response(JSON.stringify({ workspace_id: "ws_1", api_key: "sk_test" }), {
        status: 200,
        headers: { "Content-Type": "application/json" },
      });
    });

    await provisionWorkspace("http://localhost:8000");
    const parsed = JSON.parse(capturedBody!);
    expect(parsed).toEqual({});
  });

  it("strips trailing slash from API URL", async () => {
    let capturedUrl: string | undefined;
    globalThis.fetch = vi.fn(async (url: string | URL | Request, _init?: RequestInit) => {
      capturedUrl = url as string;
      return new Response(JSON.stringify({ workspace_id: "ws_1" }), {
        status: 200,
        headers: { "Content-Type": "application/json" },
      });
    });

    await provisionWorkspace("http://localhost:8000///", "test");
    expect(capturedUrl).toBe("http://localhost:8000/v1/workspaces/provision");
  });

  it("throws on non-ok response", async () => {
    globalThis.fetch = vi.fn(async () => {
      return new Response("Bad Request", { status: 422, statusText: "Unprocessable Entity" });
    });

    await expect(provisionWorkspace("http://localhost:8000", "test")).rejects.toThrow("Provision failed: 422");
  });
});

// ── CorpAPIClient construction ──────────────────────────────────────

describe("CorpAPIClient", () => {
  it("strips trailing slash from API URL", () => {
    const client = new CorpAPIClient("http://localhost:8000/", "sk_key", "ws_1");
    expect(client.apiUrl).toBe("http://localhost:8000");
  });

  it("stores apiKey and workspaceId", () => {
    const client = new CorpAPIClient("http://localhost:8000", "sk_abc", "ws_xyz");
    expect(client.apiKey).toBe("sk_abc");
    expect(client.workspaceId).toBe("ws_xyz");
  });
});

// ── CorpAPIClient HTTP methods ──────────────────────────────────────

describe("CorpAPIClient HTTP methods", () => {
  const originalFetch = globalThis.fetch;
  let capturedRequests: { url: string; method: string; headers: Record<string, string>; body?: string }[];
  let client: CorpAPIClient;

  beforeEach(() => {
    capturedRequests = [];
    globalThis.fetch = vi.fn(async (url: string | URL | Request, init?: RequestInit) => {
      const req = {
        url: url as string,
        method: init?.method ?? "GET",
        headers: Object.fromEntries(Object.entries(init?.headers ?? {})),
        body: init?.body as string | undefined,
      };
      capturedRequests.push(req);
      return new Response(JSON.stringify({ ok: true }), {
        status: 200,
        headers: { "Content-Type": "application/json" },
      });
    });
    client = new CorpAPIClient("http://localhost:8000", "sk_test_key", "ws_test");
  });

  afterEach(() => {
    globalThis.fetch = originalFetch;
  });

  it("sets correct Authorization header", async () => {
    await client.getStatus();
    expect(capturedRequests[0].headers["Authorization"]).toBe("Bearer sk_test_key");
  });

  it("sets Content-Type and Accept headers", async () => {
    await client.getStatus();
    expect(capturedRequests[0].headers["Content-Type"]).toBe("application/json");
    expect(capturedRequests[0].headers["Accept"]).toBe("application/json");
  });

  it("getStatus uses correct path", async () => {
    await client.getStatus();
    expect(capturedRequests[0].url).toBe("http://localhost:8000/v1/workspaces/ws_test/status");
    expect(capturedRequests[0].method).toBe("GET");
  });

  it("listEntities uses correct path", async () => {
    await client.listEntities();
    expect(capturedRequests[0].url).toBe("http://localhost:8000/v1/workspaces/ws_test/entities");
  });

  it("listContacts uses correct path", async () => {
    await client.listContacts();
    expect(capturedRequests[0].url).toBe("http://localhost:8000/v1/workspaces/ws_test/contacts");
  });

  it("createContact sends POST with body", async () => {
    await client.createContact({ name: "Alice", email: "alice@test.com" });
    expect(capturedRequests[0].method).toBe("POST");
    expect(capturedRequests[0].url).toBe("http://localhost:8000/v1/workspaces/ws_test/contacts");
    expect(JSON.parse(capturedRequests[0].body!)).toEqual({ name: "Alice", email: "alice@test.com" });
  });

  it("updateContact sends PATCH", async () => {
    await client.updateContact("c_1", { name: "Bob" });
    expect(capturedRequests[0].method).toBe("PATCH");
    expect(capturedRequests[0].url).toBe("http://localhost:8000/v1/contacts/c_1");
  });

  it("deleteAgent sends DELETE", async () => {
    await client.deleteAgent("agent_1");
    expect(capturedRequests[0].method).toBe("DELETE");
    expect(capturedRequests[0].url).toBe("http://localhost:8000/v1/agents/agent_1");
  });

  it("getObligations includes tier param when provided", async () => {
    await client.getObligations("tier1");
    expect(capturedRequests[0].url).toContain("tier=tier1");
  });

  it("createFormation sends POST to /v1/formations", async () => {
    await client.createFormation({ entity_type: "llc", legal_name: "Test LLC" });
    expect(capturedRequests[0].method).toBe("POST");
    expect(capturedRequests[0].url).toBe("http://localhost:8000/v1/formations");
  });

  it("seedDemo sends workspace_id in body", async () => {
    await client.seedDemo("demo-corp");
    const body = JSON.parse(capturedRequests[0].body!);
    expect(body.name).toBe("demo-corp");
    expect(body.workspace_id).toBe("ws_test");
  });

  it("throws SessionExpiredError on 401", async () => {
    globalThis.fetch = vi.fn(async () => {
      return new Response("Unauthorized", { status: 401, statusText: "Unauthorized" });
    });
    const { SessionExpiredError } = await import("../api-client.js");
    await expect(client.getStatus()).rejects.toThrow(SessionExpiredError);
  });
});
