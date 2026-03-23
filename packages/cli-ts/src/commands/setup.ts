import { input, confirm, select } from "@inquirer/prompts";
import { homedir } from "node:os";
import { join } from "node:path";
import { existsSync, mkdirSync, readdirSync } from "node:fs";
import { loadConfig, saveConfig, validateApiUrl } from "../config.js";
import { provisionWorkspace } from "../api-client.js";
import {
  generateSecret,
  generateFernetKey,
  processRequest,
} from "@thecorporation/corp-tools";
import { printSuccess, printError } from "../output.js";

const CLOUD_API_URL = "https://api.thecorporation.ai";
const DEFAULT_DATA_DIR = join(homedir(), ".corp", "data");

// ── Magic link auth (unchanged) ─────────────────────────────────────

async function requestMagicLink(
  apiUrl: string,
  email: string,
  tosAccepted: boolean
): Promise<void> {
  const resp = await fetch(`${apiUrl}/v1/auth/magic-link`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ email, tos_accepted: tosAccepted, client: "cli" }),
  });
  if (!resp.ok) {
    const data = await resp.json().catch(() => ({}));
    const detail =
      (data as Record<string, unknown>)?.error ??
      (data as Record<string, unknown>)?.message ??
      resp.statusText;
    throw new Error(
      typeof detail === "string" ? detail : JSON.stringify(detail)
    );
  }
}

async function verifyMagicLinkCode(
  apiUrl: string,
  code: string
): Promise<{ api_key: string; workspace_id: string }> {
  const resp = await fetch(`${apiUrl}/v1/auth/magic-link/verify`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ code, client: "cli" }),
  });
  const data = (await resp.json().catch(() => ({}))) as Record<string, unknown>;
  if (!resp.ok) {
    const detail = data?.error ?? data?.message ?? resp.statusText;
    throw new Error(
      typeof detail === "string" ? detail : JSON.stringify(detail)
    );
  }
  if (!data.api_key || !data.workspace_id) {
    throw new Error("Unexpected response — missing api_key or workspace_id");
  }
  return {
    api_key: data.api_key as string,
    workspace_id: data.workspace_id as string,
  };
}

function isCloudApi(url: string): boolean {
  return url.replace(/\/+$/, "").includes("thecorporation.ai");
}

async function magicLinkAuth(
  apiUrl: string,
  email: string
): Promise<{ api_key: string; workspace_id: string }> {
  console.log("\nSending magic link to " + email + "...");
  await requestMagicLink(apiUrl, email, true);
  console.log("Check your email for a sign-in link from TheCorporation.");
  console.log(
    "Copy the code from the URL (the ?code=... part) and paste it below.\n"
  );

  const code = await input({ message: "Paste your magic link code" });
  const trimmed = code.trim().replace(/^.*[?&]code=/, "");
  if (!trimmed) {
    throw new Error("No code provided");
  }

  console.log("Verifying...");
  return verifyMagicLinkCode(apiUrl, trimmed);
}

// ── Local provisioning ──────────────────────────────────────────────

function setupDataDir(dirPath: string): { isNew: boolean } {
  if (!existsSync(dirPath)) {
    mkdirSync(dirPath, { recursive: true });
    return { isNew: true };
  }
  try {
    const entries = readdirSync(dirPath);
    if (entries.length > 0) {
      console.log("Found existing data, reusing.");
      return { isNew: false };
    }
  } catch {
    // empty or unreadable
  }
  return { isNew: true };
}

async function localProvision(
  dataDir: string,
  name: string
): Promise<{ api_key: string; workspace_id: string }> {
  const resp = processRequest(
    "process://",
    "POST",
    "/v1/workspaces/provision",
    { "Content-Type": "application/json" },
    JSON.stringify({ name }),
    { dataDir },
  );

  if (!resp.ok) {
    const detail = await resp.text();
    throw new Error(`Provision failed: HTTP ${resp.status} — ${detail}`);
  }

  const body = (await resp.json()) as Record<string, unknown>;
  if (!body.api_key || !body.workspace_id) {
    throw new Error("Provision response missing api_key or workspace_id");
  }
  return {
    api_key: body.api_key as string,
    workspace_id: body.workspace_id as string,
  };
}

// ── Main setup ──────────────────────────────────────────────────────

export async function setupCommand(): Promise<void> {
  const cfg = loadConfig();
  console.log("Welcome to corp — corporate governance from the terminal.\n");

  // User info
  console.log("--- User Info ---");
  const user = cfg.user ?? { name: "", email: "" };
  user.name = await input({
    message: "Your name",
    default: user.name || undefined,
  });
  user.email = await input({
    message: "Your email",
    default: user.email || undefined,
  });
  cfg.user = user;

  // Hosting mode
  console.log("\n--- Hosting Mode ---");
  const hostingMode = await select({
    message: "How would you like to run corp?",
    choices: [
      { value: "local", name: "Local (your machine)" },
      { value: "cloud", name: "TheCorporation cloud" },
      { value: "self-hosted", name: "Self-hosted server (custom URL)" },
    ],
    default: cfg.hosting_mode || "local",
  });
  cfg.hosting_mode = hostingMode;

  if (hostingMode === "local") {
    // Data directory
    const dataDir = await input({
      message: "Data directory",
      default: cfg.data_dir || DEFAULT_DATA_DIR,
    });
    cfg.data_dir = dataDir;
    cfg.api_url = "process://";

    const { isNew } = setupDataDir(dataDir);

    // Generate server secrets
    const serverSecrets = {
      jwt_secret: generateSecret(),
      secrets_master_key: generateFernetKey(),
      internal_worker_token: generateSecret(),
    };

    // Inject into process.env so processRequest can use them immediately
    process.env.JWT_SECRET = serverSecrets.jwt_secret;
    process.env.SECRETS_MASTER_KEY = serverSecrets.secrets_master_key;
    process.env.INTERNAL_WORKER_TOKEN = serverSecrets.internal_worker_token;

    // Store secrets via _server_secrets (picked up by serializeAuth)
    (cfg as Record<string, unknown>)._server_secrets = serverSecrets;

    if (isNew || !cfg.workspace_id) {
      console.log("\nProvisioning workspace...");
      try {
        const result = await localProvision(dataDir, `${user.name}'s workspace`);
        cfg.api_key = result.api_key;
        cfg.workspace_id = result.workspace_id;
        printSuccess(`Local workspace ready: ${result.workspace_id}`);
      } catch (err) {
        printError(`Workspace provisioning failed: ${err}`);
        console.log("You can retry with 'corp setup'.");
      }
    } else {
      console.log("\nExisting workspace found.");
    }
  } else if (hostingMode === "cloud") {
    cfg.api_url = CLOUD_API_URL;
    cfg.data_dir = "";

    const needsAuth = !cfg.api_key || !cfg.workspace_id;
    if (needsAuth) {
      try {
        const result = await magicLinkAuth(cfg.api_url, user.email);
        cfg.api_key = result.api_key;
        cfg.workspace_id = result.workspace_id;
        printSuccess(`Authenticated. Workspace: ${result.workspace_id}`);
      } catch (err) {
        printError(`Authentication failed: ${err}`);
        console.log(
          "You can manually set credentials with: corp config set api_key <key>"
        );
      }
    } else {
      console.log("\nExisting credentials found. Run 'corp status' to verify.");
    }
  } else {
    // Self-hosted
    const url = await input({
      message: "Server URL",
      default:
        cfg.api_url !== CLOUD_API_URL && !cfg.api_url.startsWith("process://")
          ? cfg.api_url
          : undefined,
    });
    try {
      cfg.api_url = validateApiUrl(url);
    } catch (err) {
      printError(`Invalid URL: ${err}`);
      process.exit(1);
    }
    cfg.data_dir = "";

    const needsAuth = !cfg.api_key || !cfg.workspace_id;
    if (needsAuth) {
      console.log("\nProvisioning workspace...");
      try {
        const result = await provisionWorkspace(
          cfg.api_url,
          `${user.name}'s workspace`
        );
        cfg.api_key = result.api_key as string;
        cfg.workspace_id = result.workspace_id as string;
        console.log(`Workspace provisioned: ${result.workspace_id}`);
      } catch (err) {
        printError(`Auto-provision failed: ${err}`);
        console.log(
          "You can manually set credentials with: corp config set api_key <key>"
        );
      }
    }
  }

  saveConfig(cfg);
  console.log("\nSettings saved to ~/.corp/config.json");
  console.log("Credentials saved to ~/.corp/auth.json");
  console.log("Run 'corp status' to verify your connection.");
}
