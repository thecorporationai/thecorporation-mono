import { spawn, type ChildProcess } from "node:child_process";
import { existsSync } from "node:fs";
import { resolve, join } from "node:path";
import { platform, arch } from "node:os";
import { fileURLToPath } from "node:url";
import { createRequire } from "node:module";

const __dirname = fileURLToPath(new URL(".", import.meta.url));
const require = createRequire(import.meta.url);

/** Platform-to-package mapping */
const PLATFORM_PACKAGES: Record<string, string> = {
  "linux-x64": "@thecorporation/server-linux-x64-gnu",
  "linux-arm64": "@thecorporation/server-linux-arm64-gnu",
  "darwin-x64": "@thecorporation/server-darwin-x64",
  "darwin-arm64": "@thecorporation/server-darwin-arm64",
  "win32-x64": "@thecorporation/server-win32-x64-msvc",
};

function getPlatformKey(): string {
  return `${platform()}-${arch()}`;
}

function getBinaryName(): string {
  return platform() === "win32" ? "api-rs.exe" : "api-rs";
}

function getWorkerBinaryName(): string {
  return platform() === "win32" ? "agent-worker.exe" : "agent-worker";
}

/**
 * Resolve the path to the server binary.
 *
 * Resolution order:
 * 1. CORP_SERVER_BIN environment variable
 * 2. Platform-specific npm package (installed via optionalDependencies)
 * 3. Local dev build at services/api-rs/target/release/api-rs
 */
export function getBinaryPath(): string | null {
  // 1. Explicit env override
  const envBin = process.env.CORP_SERVER_BIN;
  if (envBin && existsSync(envBin)) {
    return envBin;
  }

  // 2. Platform npm package
  const key = getPlatformKey();
  const pkg = PLATFORM_PACKAGES[key];
  if (pkg) {
    try {
      const pkgDir = resolve(require.resolve(`${pkg}/package.json`), "..");
      const binPath = join(pkgDir, "bin", getBinaryName());
      if (existsSync(binPath)) {
        return binPath;
      }
    } catch {
      // Package not installed — fall through
    }
  }

  // 3. Local dev build (monorepo layout)
  const devBuild = resolve(__dirname, "..", "..", "services", "api-rs", "target", "release", getBinaryName());
  if (existsSync(devBuild)) {
    return devBuild;
  }

  // Also try from repo root (when installed as a package)
  const repoRoot = resolve(__dirname, "..", "..", "..");
  const repoDevBuild = join(repoRoot, "services", "api-rs", "target", "release", getBinaryName());
  if (existsSync(repoDevBuild)) {
    return repoDevBuild;
  }

  return null;
}

/**
 * Check if a server binary is available for the current platform.
 */
export function isAvailable(): boolean {
  return getBinaryPath() !== null;
}

export interface StartServerOptions {
  /** Port to listen on (default: 8000) */
  port?: number;
  /** Data directory for git repos (default: ./data/repos) */
  dataDir?: string;
  /** Redis URL for caching */
  redisUrl?: string;
  /** PEM-encoded JWT private key */
  jwtPrivateKeyPem?: string;
  /** PEM-encoded JWT public key */
  jwtPublicKeyPem?: string;
  /** Stripe secret key */
  stripeSecretKey?: string;
  /** Stripe webhook secret */
  stripeWebhookSecret?: string;
  /** PEM-encoded Ed25519 key for signing git commits */
  commitSigningKey?: string;
  /** Inherit stdio from parent process (default: true) */
  stdio?: "inherit" | "pipe" | "ignore";
}

/**
 * Start the API server as a child process.
 *
 * Environment variables match `services/api-rs/src/config.rs`:
 * - PORT, DATA_DIR, REDIS_URL, JWT_PRIVATE_KEY_PEM, JWT_PUBLIC_KEY_PEM,
 *   STRIPE_SECRET_KEY, STRIPE_WEBHOOK_SECRET, COMMIT_SIGNING_KEY
 */
export function startServer(options: StartServerOptions = {}): ChildProcess {
  const binPath = getBinaryPath();
  if (!binPath) {
    throw new Error(
      `No server binary found for platform ${getPlatformKey()}. ` +
      `Set CORP_SERVER_BIN or install the platform-specific package.`
    );
  }

  const env: Record<string, string> = { ...process.env as Record<string, string> };

  if (options.port !== undefined) env.PORT = String(options.port);
  if (options.dataDir !== undefined) env.DATA_DIR = options.dataDir;
  if (options.redisUrl !== undefined) env.REDIS_URL = options.redisUrl;
  if (options.jwtPrivateKeyPem !== undefined) env.JWT_PRIVATE_KEY_PEM = options.jwtPrivateKeyPem;
  if (options.jwtPublicKeyPem !== undefined) env.JWT_PUBLIC_KEY_PEM = options.jwtPublicKeyPem;
  if (options.stripeSecretKey !== undefined) env.STRIPE_SECRET_KEY = options.stripeSecretKey;
  if (options.stripeWebhookSecret !== undefined) env.STRIPE_WEBHOOK_SECRET = options.stripeWebhookSecret;
  if (options.commitSigningKey !== undefined) env.COMMIT_SIGNING_KEY = options.commitSigningKey;

  return spawn(binPath, [], {
    env,
    stdio: options.stdio ?? "inherit",
  });
}

/**
 * Resolve the path to the agent-worker binary.
 *
 * Resolution order:
 * 1. CORP_WORKER_BIN environment variable
 * 2. Local dev build at services/agent-worker/target/release/agent-worker
 *
 * Note: agent-worker does not have its own npm platform packages yet (dev-build
 * only). PLATFORM_PACKAGES maps to server binaries, so we intentionally skip
 * the npm-package lookup here to avoid finding a stale server binary at the
 * wrong path.
 */
export function getWorkerBinaryPath(): string | null {
  // 1. Explicit env override
  const envBin = process.env.CORP_WORKER_BIN;
  if (envBin && existsSync(envBin)) {
    return envBin;
  }

  // 2. Local dev build (monorepo layout)
  const devBuild = resolve(__dirname, "..", "..", "services", "agent-worker", "target", "release", getWorkerBinaryName());
  if (existsSync(devBuild)) {
    return devBuild;
  }

  // Also try from repo root (when installed as a package)
  const repoRoot = resolve(__dirname, "..", "..", "..");
  const repoDevBuild = join(repoRoot, "services", "agent-worker", "target", "release", getWorkerBinaryName());
  if (existsSync(repoDevBuild)) {
    return repoDevBuild;
  }

  return null;
}

export interface StartWorkerOptions {
  /** Redis URL for job queue (default: redis://localhost:6379/0) */
  redisUrl?: string;
  /** Base URL of the API server (default: http://localhost:8000) */
  apiBaseUrl?: string;
  /** Bearer token for authenticated worker -> API calls */
  apiBearerToken?: string;
  /** Docker host socket or URL */
  dockerHost?: string;
  /** Root directory for agent workspaces */
  workspaceRoot?: string;
  /** Docker image for agent runtime containers */
  runtimeImage?: string;
  /** Maximum concurrent agent executions */
  maxConcurrency?: number;
  /** Inherit stdio from parent process (default: true) */
  stdio?: "inherit" | "pipe" | "ignore";
}

/**
 * Start the agent-worker as a child process.
 *
 * Environment variables match `services/agent-worker/src/config.rs`:
 * - REDIS_URL, API_BASE_URL, API_BEARER_TOKEN, DOCKER_HOST,
 *   WORKSPACE_ROOT, RUNTIME_IMAGE, MAX_CONCURRENCY, RUST_LOG
 */
export function startWorker(options: StartWorkerOptions = {}): ChildProcess {
  const binPath = getWorkerBinaryPath();
  if (!binPath) {
    throw new Error(
      `No agent-worker binary found for platform ${getPlatformKey()}. ` +
      `Set CORP_WORKER_BIN or install the platform-specific package.`
    );
  }

  const env: Record<string, string> = { ...process.env as Record<string, string> };

  if (options.redisUrl !== undefined) env.REDIS_URL = options.redisUrl;
  if (options.apiBaseUrl !== undefined) env.API_BASE_URL = options.apiBaseUrl;
  if (options.apiBearerToken !== undefined) env.API_BEARER_TOKEN = options.apiBearerToken;
  if (!env.API_BEARER_TOKEN && env.INTERNAL_WORKER_TOKEN) {
    env.API_BEARER_TOKEN = env.INTERNAL_WORKER_TOKEN;
  }
  if (options.dockerHost !== undefined) env.DOCKER_HOST = options.dockerHost;
  if (options.workspaceRoot !== undefined) env.WORKSPACE_ROOT = options.workspaceRoot;
  if (options.runtimeImage !== undefined) env.RUNTIME_IMAGE = options.runtimeImage;
  if (options.maxConcurrency !== undefined) env.MAX_CONCURRENCY = String(options.maxConcurrency);
  if (!env.RUST_LOG) env.RUST_LOG = "agent_worker=info";

  return spawn(binPath, [], {
    env,
    stdio: options.stdio ?? "inherit",
  });
}
