import { describe, it, expect, vi, afterEach } from "vitest";
import { CorpAPIClient } from "../api-client.js";

vi.mock("../process-transport.js", () => ({
  processRequest: vi.fn(() => ({
    status: 200,
    ok: true,
    statusText: "OK",
    headers: new Headers(),
    json: async () => ({ status: "ok" }),
    text: async () => '{"status":"ok"}',
  })),
}));

describe("CorpAPIClient transport dispatch", () => {
  afterEach(() => {
    vi.clearAllMocks();
  });

  it("uses processRequest for process:// URLs", async () => {
    const client = new CorpAPIClient("process://", "key", "ws");
    const result = await client.getStatus();
    const { processRequest } = await import("../process-transport.js");
    expect(processRequest).toHaveBeenCalledWith(
      "process://",
      "GET",
      expect.stringContaining("/v1/workspaces/ws/status"),
      expect.objectContaining({ Authorization: "Bearer key" }),
      undefined,
    );
  });

  it("does not call processRequest for https:// URLs", async () => {
    const client = new CorpAPIClient("https://api.example.com", "key", "ws");
    try {
      await client.getStatus();
    } catch {
      // Expected — no real server
    }
    const { processRequest } = await import("../process-transport.js");
    expect(processRequest).not.toHaveBeenCalled();
  });

  it("getPreviewPdfUrl throws for process:// URLs", () => {
    const client = new CorpAPIClient("process://", "key", "ws");
    expect(() => client.getPreviewPdfUrl("ent1", "doc1")).toThrow("not available in process transport");
  });

  it("getPreviewPdfUrl works for https:// URLs", () => {
    const client = new CorpAPIClient("https://api.example.com", "key", "ws");
    const url = client.getPreviewPdfUrl("ent1", "doc1");
    expect(url).toContain("https://api.example.com");
    expect(url).toContain("entity_id=ent1");
  });
});
