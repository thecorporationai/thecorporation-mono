/**
 * @module server
 *
 * Binary resolution and process management for the corp-server and corp CLI.
 *
 * ```ts
 * import { startServer, getBinaryPath } from "@thecorporation/corp/server";
 *
 * const proc = await startServer({ port: 8000, dataDir: "./data" });
 * // ... use the server ...
 * proc.kill();
 * ```
 */

import { platform, arch } from "node:os";
import { existsSync } from "node:fs";
import { resolve, join } from "node:path";
import { spawn, type ChildProcess } from "node:child_process";
import { createRequire } from "node:module";

const require = createRequire(import.meta.url);

// ── Platform package map ─────────────────────────────────────────────────────

const PLATFORM_PACKAGES: Record<string, string> = {
  "linux-x64":    "@thecorporation/corp-linux-x64-gnu",
  "linux-arm64":  "@thecorporation/corp-linux-arm64-gnu",
  "darwin-x64":   "@thecorporation/corp-darwin-x64",
  "darwin-arm64": "@thecorporation/corp-darwin-arm64",
  "win32-x64":    "@thecorporation/corp-win32-x64-msvc",
};

const EXE = platform() === "win32" ? ".exe" : "";

// ── Binary resolution ────────────────────────────────────────────────────────

let cachedServerPath: string | undefined;
let cachedCliPath: string | undefined;

/**
 * Resolve the path to the `corp-server` binary.
 *
 * Resolution order:
 * 1. `CORP_SERVER_BIN` environment variable
 * 2. Platform-specific npm package (`@thecorporation/corp-{platform}`)
 * 3. Local cargo build (`target/release/corp-server`)
 * 4. Local cargo build (`target/debug/corp-server`)
 */
export function getServerBinaryPath(): string {
  if (cachedServerPath) return cachedServerPath;

  // 1. Environment variable
  const envBin = process.env.CORP_SERVER_BIN;
  if (envBin && existsSync(envBin)) {
    cachedServerPath = envBin;
    return envBin;
  }

  // 2. Platform npm package
  const key = `${platform()}-${arch()}`;
  const pkg = PLATFORM_PACKAGES[key];
  if (pkg) {
    try {
      const pkgDir = resolve(require.resolve(`${pkg}/package.json`), "..");
      const binPath = join(pkgDir, "bin", `corp-server${EXE}`);
      if (existsSync(binPath)) {
        cachedServerPath = binPath;
        return binPath;
      }
    } catch {
      // Package not installed
    }
  }

  // 3. Release build
  const releasePath = resolve("target/release", `corp-server${EXE}`);
  if (existsSync(releasePath)) {
    cachedServerPath = releasePath;
    return releasePath;
  }

  // 4. Debug build
  const debugPath = resolve("target/debug", `corp-server${EXE}`);
  if (existsSync(debugPath)) {
    cachedServerPath = debugPath;
    return debugPath;
  }

  throw new Error(
    "corp-server binary not found. Install @thecorporation/corp or build from source:\n" +
    "  cargo build -p corp-server --release"
  );
}

/**
 * Resolve the path to the `corp` CLI binary.
 *
 * Same resolution order as `getServerBinaryPath()`, but looks for `corp`.
 */
export function getCliBinaryPath(): string {
  if (cachedCliPath) return cachedCliPath;

  const envBin = process.env.CORP_CLI_BIN;
  if (envBin && existsSync(envBin)) {
    cachedCliPath = envBin;
    return envBin;
  }

  const key = `${platform()}-${arch()}`;
  const pkg = PLATFORM_PACKAGES[key];
  if (pkg) {
    try {
      const pkgDir = resolve(require.resolve(`${pkg}/package.json`), "..");
      const binPath = join(pkgDir, "bin", `corp${EXE}`);
      if (existsSync(binPath)) {
        cachedCliPath = binPath;
        return binPath;
      }
    } catch {
      // Package not installed
    }
  }

  const releasePath = resolve("target/release", `corp${EXE}`);
  if (existsSync(releasePath)) {
    cachedCliPath = releasePath;
    return releasePath;
  }

  const debugPath = resolve("target/debug", `corp${EXE}`);
  if (existsSync(debugPath)) {
    cachedCliPath = debugPath;
    return debugPath;
  }

  throw new Error(
    "corp CLI binary not found. Install @thecorporation/corp or build from source:\n" +
    "  cargo build -p corp-cli --release"
  );
}

/** Reset the cached binary paths (useful for testing). */
export function resetCache(): void {
  cachedServerPath = undefined;
  cachedCliPath = undefined;
}

// ── Server process management ────────────────────────────────────────────────

export interface StartServerOptions {
  /** Port to listen on (default: 8000). */
  port?: number;
  /** Root data directory for git repos. */
  dataDir?: string;
  /** JWT signing secret. */
  jwtSecret?: string;
  /** Storage backend: "git" or "kv" (default: "git"). */
  storageBackend?: "git" | "kv";
  /** Redis URL (required when storageBackend is "kv"). */
  redisUrl?: string;
  /** S3 bucket name for KV durability. */
  s3Bucket?: string;
  /** stdio handling. */
  stdio?: "inherit" | "pipe" | "ignore";
  /** Additional environment variables. */
  env?: Record<string, string>;
}

/**
 * Start a `corp-server` process.
 *
 * Returns the `ChildProcess` handle. The server is ready when it starts
 * accepting connections on the configured port.
 */
export function startServer(opts: StartServerOptions = {}): ChildProcess {
  const bin = getServerBinaryPath();

  const env: Record<string, string> = {
    ...process.env as Record<string, string>,
    CORP_DATA_DIR: opts.dataDir ?? "./data",
    CORP_JWT_SECRET: opts.jwtSecret ?? "dev-secret-change-in-production",
    CORP_STORAGE_BACKEND: opts.storageBackend ?? "git",
    PORT: String(opts.port ?? 8000),
    RUST_LOG: "corp_server=info",
    ...opts.env,
  };

  if (opts.redisUrl) env.CORP_REDIS_URL = opts.redisUrl;
  if (opts.s3Bucket) env.CORP_S3_BUCKET = opts.s3Bucket;

  return spawn(bin, [], {
    env,
    stdio: opts.stdio ?? "inherit",
  });
}

/**
 * Start a corp-server and wait until it's accepting connections.
 *
 * Polls the health endpoint until it returns 200, then resolves.
 * Rejects after `timeoutMs` (default: 10_000).
 */
export async function startServerAndWait(
  opts: StartServerOptions = {},
  timeoutMs = 10_000,
): Promise<ChildProcess> {
  const proc = startServer({ ...opts, stdio: opts.stdio ?? "pipe" });
  const port = opts.port ?? 8000;
  const url = `http://127.0.0.1:${port}/health`;

  const deadline = Date.now() + timeoutMs;

  while (Date.now() < deadline) {
    try {
      const resp = await fetch(url);
      if (resp.ok) return proc;
    } catch {
      // Not ready yet
    }
    await new Promise((r) => setTimeout(r, 100));
  }

  proc.kill();
  throw new Error(`corp-server did not start within ${timeoutMs}ms`);
}

// ── CLI process helpers ──────────────────────────────────────────────────────

export interface CliRunOptions {
  /** API URL to pass to the CLI. */
  apiUrl?: string;
  /** API key to pass to the CLI. */
  apiKey?: string;
  /** Output as JSON. */
  json?: boolean;
  /** Additional environment variables. */
  env?: Record<string, string>;
}

export interface CliResult {
  stdout: string;
  stderr: string;
  exitCode: number | null;
}

/**
 * Run a `corp` CLI command and return the result.
 *
 * ```ts
 * const result = await runCli(["entities", "list"], { apiUrl: "http://localhost:8000" });
 * console.log(result.stdout);
 * ```
 */
export function runCli(args: string[], opts: CliRunOptions = {}): Promise<CliResult> {
  return new Promise((resolve) => {
    const bin = getCliBinaryPath();
    const fullArgs: string[] = [];

    if (opts.apiUrl) fullArgs.push("--api-url", opts.apiUrl);
    if (opts.apiKey) fullArgs.push("--api-key", opts.apiKey);
    if (opts.json) fullArgs.push("--json");
    fullArgs.push(...args);

    const proc = spawn(bin, fullArgs, {
      env: { ...process.env, ...opts.env },
      stdio: "pipe",
    });

    let stdout = "";
    let stderr = "";

    proc.stdout?.on("data", (d) => { stdout += d.toString(); });
    proc.stderr?.on("data", (d) => { stderr += d.toString(); });

    proc.on("close", (code) => {
      resolve({ stdout, stderr, exitCode: code });
    });
  });
}

/**
 * Run a `corp` CLI command, parse JSON output, and throw on failure.
 */
export async function runCliJson<T = unknown>(args: string[], opts: CliRunOptions = {}): Promise<T> {
  const result = await runCli(args, { ...opts, json: true });

  if (result.exitCode !== 0) {
    throw new Error(`corp ${args.join(" ")} failed (exit ${result.exitCode}):\n${result.stderr || result.stdout}`);
  }

  // Parse first JSON value from output (may have trailing human messages).
  const match = result.stdout.match(/[\[{]/);
  if (!match || match.index === undefined) {
    throw new Error(`No JSON in output of corp ${args.join(" ")}:\n${result.stdout}`);
  }

  // Use a streaming approach to handle pretty-printed JSON with trailing text.
  const jsonStart = result.stdout.substring(match.index);
  return JSON.parse(jsonStart) as T;
}
