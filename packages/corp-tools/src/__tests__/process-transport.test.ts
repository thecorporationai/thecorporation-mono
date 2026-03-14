import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { parseStatusFromStderr, buildProcessResponse, resolveBinaryPath, resetCache } from "../process-transport.js";

describe("parseStatusFromStderr", () => {
  it("extracts status from clean stderr", () => {
    expect(parseStatusFromStderr("HTTP 200")).toBe(200);
  });

  it("extracts status from stderr with tracing output", () => {
    const stderr = [
      "2026-03-14T00:00:00Z  INFO api_rs: secrets encryption enabled",
      "2026-03-14T00:00:00Z  INFO api_rs: storage backend: git",
      "HTTP 404",
    ].join("\n");
    expect(parseStatusFromStderr(stderr)).toBe(404);
  });

  it("returns null when no status line found", () => {
    expect(parseStatusFromStderr("some random error output")).toBeNull();
  });

  it("uses last matching line when multiple present", () => {
    const stderr = "HTTP 200\nHTTP 500";
    expect(parseStatusFromStderr(stderr)).toBe(500);
  });
});

describe("buildProcessResponse", () => {
  it("creates response with correct status and body", async () => {
    const resp = buildProcessResponse(200, '{"status":"ok"}');
    expect(resp.status).toBe(200);
    expect(resp.ok).toBe(true);
    expect(resp.statusText).toBe("OK");
    expect(await resp.json()).toEqual({ status: "ok" });
    expect(await resp.text()).toBe('{"status":"ok"}');
  });

  it("sets ok=false for 4xx/5xx", () => {
    expect(buildProcessResponse(404, "").ok).toBe(false);
    expect(buildProcessResponse(500, "").ok).toBe(false);
  });

  it("sets ok=true for 2xx", () => {
    expect(buildProcessResponse(200, "").ok).toBe(true);
    expect(buildProcessResponse(201, "").ok).toBe(true);
  });
});

describe("resolveBinaryPath", () => {
  const origEnv = process.env.CORP_SERVER_BIN;

  beforeEach(() => {
    resetCache();
  });

  afterEach(() => {
    if (origEnv !== undefined) {
      process.env.CORP_SERVER_BIN = origEnv;
    } else {
      delete process.env.CORP_SERVER_BIN;
    }
    resetCache();
  });

  it("returns custom path from process:///path URL", () => {
    expect(resolveBinaryPath("process:///usr/local/bin/api-rs")).toBe("/usr/local/bin/api-rs");
  });

  it("returns CORP_SERVER_BIN if set and file exists", () => {
    process.env.CORP_SERVER_BIN = process.execPath;
    expect(resolveBinaryPath("process://")).toBe(process.execPath);
  });

  it("skips CORP_SERVER_BIN if file does not exist", () => {
    process.env.CORP_SERVER_BIN = "/nonexistent/path/api-rs";
    const result = resolveBinaryPath("process://");
    expect(result).not.toBe("/nonexistent/path/api-rs");
  });

  it("caches resolved path across calls", () => {
    process.env.CORP_SERVER_BIN = process.execPath;
    const first = resolveBinaryPath("process://");
    delete process.env.CORP_SERVER_BIN;
    const second = resolveBinaryPath("process://");
    expect(first).toBe(second);
  });
});
