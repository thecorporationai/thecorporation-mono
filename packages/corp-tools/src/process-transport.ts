import { execFileSync } from "node:child_process";
import { existsSync } from "node:fs";
import { resolve } from "node:path";
import { createRequire } from "node:module";
import { ensureEnvFile, loadEnvFile } from "./env.js";

const require = createRequire(import.meta.url);

let cachedBinaryPath: string | undefined;

export function resolveBinaryPath(processUrl: string): string {
  if (cachedBinaryPath !== undefined) return cachedBinaryPath;

  const parsed = new URL(processUrl);
  if (parsed.pathname && parsed.pathname !== "/") {
    cachedBinaryPath = parsed.pathname;
    return cachedBinaryPath;
  }

  const envBin = process.env.CORP_SERVER_BIN;
  if (envBin && existsSync(envBin)) {
    cachedBinaryPath = envBin;
    return cachedBinaryPath;
  }

  try {
    const server = require("@thecorporation/server");
    const pkgPath: string | undefined = server.getBinaryPath?.();
    if (pkgPath) {
      cachedBinaryPath = pkgPath;
      return pkgPath;
    }
  } catch {
    // Package not installed
  }

  cachedBinaryPath = resolve("services/api-rs/target/release/api-rs");
  return cachedBinaryPath;
}

export function parseStatusFromStderr(stderr: string): number | null {
  const lines = stderr.split("\n");
  for (let i = lines.length - 1; i >= 0; i--) {
    const match = lines[i].match(/^HTTP (\d+)$/);
    if (match) return parseInt(match[1], 10);
  }
  return null;
}

const STATUS_TEXT: Record<number, string> = {
  200: "OK", 201: "Created", 204: "No Content",
  400: "Bad Request", 401: "Unauthorized", 403: "Forbidden",
  404: "Not Found", 409: "Conflict", 422: "Unprocessable Entity",
  500: "Internal Server Error",
};

export function buildProcessResponse(status: number, body: string): Response {
  return {
    status,
    ok: status >= 200 && status < 300,
    statusText: STATUS_TEXT[status] ?? String(status),
    headers: new Headers({ "content-type": "application/json" }),
    json: async () => JSON.parse(body),
    text: async () => body,
    body: null,
    bodyUsed: false,
    redirected: false,
    type: "basic" as ResponseType,
    url: "",
    clone: () => buildProcessResponse(status, body),
    arrayBuffer: async () => new TextEncoder().encode(body).buffer as ArrayBuffer,
    blob: async () => new Blob([body]),
    formData: async () => { throw new Error("not supported"); },
    bytes: async () => new TextEncoder().encode(body),
  } as Response;
}

let envLoaded = false;

function ensureEnv(): void {
  if (envLoaded) return;
  const envPath = resolve(process.cwd(), ".env");
  ensureEnvFile(envPath);
  loadEnvFile(envPath);
  envLoaded = true;
}

export function processRequest(
  processUrl: string,
  method: string,
  pathWithQuery: string,
  headers: Record<string, string>,
  body?: string,
): Response {
  ensureEnv();

  const binPath = resolveBinaryPath(processUrl);
  const args = ["--skip-validation", "call", method, pathWithQuery];

  for (const [key, value] of Object.entries(headers)) {
    args.push("-H", `${key}: ${value}`);
  }

  if (body) {
    args.push("--stdin");
  }

  try {
    const stdout = execFileSync(binPath, args, {
      input: body ?? undefined,
      stdio: ["pipe", "pipe", "pipe"],
      maxBuffer: 10 * 1024 * 1024,
      env: process.env,
    });

    return buildProcessResponse(200, stdout.toString("utf-8"));
  } catch (err: unknown) {
    const execErr = err as { status?: number; stdout?: Buffer; stderr?: Buffer; message?: string };

    if (execErr.stdout === undefined && execErr.stderr === undefined) {
      throw new Error(
        "No api-rs binary found. Install @thecorporation/server or set CORP_SERVER_BIN.\n" +
        `Attempted: ${binPath}\n` +
        `Error: ${execErr.message ?? String(err)}`,
      );
    }

    const stderr = execErr.stderr?.toString("utf-8") ?? "";
    const stdout = execErr.stdout?.toString("utf-8") ?? "";
    const status = parseStatusFromStderr(stderr);

    if (status !== null) {
      return buildProcessResponse(status, stdout);
    }

    throw new Error(`api-rs process failed:\n${stderr || stdout || String(err)}`);
  }
}

export function resetCache(): void {
  cachedBinaryPath = undefined;
  envLoaded = false;
}
