import { execFileSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { resolve, join } from "node:path";
import { homedir } from "node:os";
import { createRequire } from "node:module";
import { ensureEnvFile, loadEnvFile } from "./env.js";

const require = createRequire(import.meta.url);

// ── Binary resolution ───────────────────────────────────────────────

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

// ── Stderr parsing ──────────────────────────────────────────────────

export function parseStatusFromStderr(stderr: string): number | null {
  const lines = stderr.split("\n");
  for (let i = lines.length - 1; i >= 0; i--) {
    const match = lines[i].match(/^HTTP (\d+)$/);
    if (match) return parseInt(match[1], 10);
  }
  return null;
}

// ── Response builder ────────────────────────────────────────────────

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

// ── Config reading (direct file access, no cli-ts dependency) ───────

const CORP_CONFIG_DIR = process.env.CORP_CONFIG_DIR || join(homedir(), ".corp");

function readJsonFileSafe(path: string): Record<string, unknown> | null {
  try {
    if (!existsSync(path)) return null;
    return JSON.parse(readFileSync(path, "utf-8"));
  } catch {
    return null;
  }
}

function loadServerSecretsFromAuth(): { jwt_secret: string; secrets_master_key: string; internal_worker_token: string } | null {
  const auth = readJsonFileSafe(join(CORP_CONFIG_DIR, "auth.json"));
  if (!auth) return null;
  const ss = auth.server_secrets;
  if (!ss || typeof ss !== "object") return null;
  const s = ss as Record<string, unknown>;
  if (typeof s.jwt_secret === "string" && typeof s.secrets_master_key === "string" && typeof s.internal_worker_token === "string") {
    return { jwt_secret: s.jwt_secret, secrets_master_key: s.secrets_master_key, internal_worker_token: s.internal_worker_token };
  }
  return null;
}

function loadDataDirFromConfig(): string | undefined {
  const cfg = readJsonFileSafe(join(CORP_CONFIG_DIR, "config.json"));
  if (!cfg) return undefined;
  const dataDir = typeof cfg.data_dir === "string" ? cfg.data_dir : undefined;
  return dataDir || undefined;
}

// ── Env loading ─────────────────────────────────────────────────────

let envLoaded = false;

function ensureEnv(): void {
  if (envLoaded) return;

  // Try server_secrets from auth.json first (local mode)
  const secrets = loadServerSecretsFromAuth();
  if (secrets) {
    if (!process.env.JWT_SECRET) process.env.JWT_SECRET = secrets.jwt_secret;
    if (!process.env.SECRETS_MASTER_KEY) process.env.SECRETS_MASTER_KEY = secrets.secrets_master_key;
    if (!process.env.INTERNAL_WORKER_TOKEN) process.env.INTERNAL_WORKER_TOKEN = secrets.internal_worker_token;
    envLoaded = true;
    return;
  }

  // Fallback: load from .env file
  const envPath = resolve(process.cwd(), ".env");
  ensureEnvFile(envPath);
  loadEnvFile(envPath);
  envLoaded = true;
}

// ── Process request ─────────────────────────────────────────────────

export interface ProcessRequestOptions {
  dataDir?: string;
}

export function processRequest(
  processUrl: string,
  method: string,
  pathWithQuery: string,
  headers: Record<string, string>,
  body?: string,
  options?: ProcessRequestOptions,
): Response {
  ensureEnv();

  const binPath = resolveBinaryPath(processUrl);
  const args = ["--skip-validation", "call"];

  const dataDir = options?.dataDir ?? loadDataDirFromConfig();
  if (dataDir) {
    args.push("--data-dir", dataDir);
  }

  args.push(method, pathWithQuery);

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

    // Binary crashed — detect config errors
    const isConfigError = stderr.includes("panic") ||
      stderr.includes("INTERNAL_WORKER_TOKEN") ||
      stderr.includes("JWT_SECRET") ||
      stderr.includes("SECRETS_MASTER_KEY");
    if (isConfigError) {
      throw new Error("Server configuration incomplete. Run 'corp setup' to configure local mode.");
    }

    throw new Error(`api-rs process failed:\n${stderr || stdout || String(err)}`);
  }
}

export function resetCache(): void {
  cachedBinaryPath = undefined;
  envLoaded = false;
}
