import { spawn, type ChildProcess } from "node:child_process";
import { existsSync } from "node:fs";
import { resolve, join } from "node:path";
import { platform, arch } from "node:os";
import { fileURLToPath } from "node:url";

const __dirname = fileURLToPath(new URL(".", import.meta.url));

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
