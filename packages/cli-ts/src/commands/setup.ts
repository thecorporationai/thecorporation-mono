import { input, confirm } from "@inquirer/prompts";
import { loadConfig, saveConfig, validateApiUrl } from "../config.js";
import { provisionWorkspace } from "../api-client.js";
import { printSuccess, printError } from "../output.js";

const CLOUD_API_URL = "https://api.thecorporation.ai";

async function requestMagicLink(
  apiUrl: string,
  email: string,
  tosAccepted: boolean
): Promise<void> {
  const resp = await fetch(`${apiUrl}/v1/auth/magic-link`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ email, tos_accepted: tosAccepted }),
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

export async function setupCommand(): Promise<void> {
  const cfg = loadConfig();
  console.log("Welcome to corp — corporate governance from the terminal.\n");

  // Determine API URL
  const customUrl = process.env.CORP_API_URL;
  if (customUrl) {
    try {
      cfg.api_url = validateApiUrl(customUrl);
    } catch (err) {
      printError(`Invalid CORP_API_URL: ${err}`);
      process.exit(1);
    }
    console.log(`Using API: ${cfg.api_url}\n`);
  } else {
    cfg.api_url = CLOUD_API_URL;
  }

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

  const needsAuth = !cfg.api_key || !cfg.workspace_id;
  const cloud = isCloudApi(cfg.api_url);

  if (needsAuth) {
    if (cloud) {
      // Cloud API — authenticate via magic link
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
      // Self-hosted — provision directly (no auth required)
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
  } else {
    console.log("\nVerifying existing credentials...");
    let keyValid = false;
    try {
      const resp = await fetch(
        `${cfg.api_url.replace(/\/+$/, "")}/v1/workspaces/${cfg.workspace_id}/status`,
        { headers: { Authorization: `Bearer ${cfg.api_key}` } }
      );
      keyValid = resp.status !== 401;
    } catch {
      // network error — treat as potentially valid
    }

    if (keyValid) {
      console.log("Credentials OK.");
    } else {
      console.log("API key is no longer valid.");
      const reauth = await confirm({
        message: cloud
          ? "Re-authenticate via magic link?"
          : "Provision a new workspace? (This will replace your current credentials)",
        default: true,
      });
      if (reauth) {
        try {
          if (cloud) {
            const result = await magicLinkAuth(cfg.api_url, user.email);
            cfg.api_key = result.api_key;
            cfg.workspace_id = result.workspace_id;
            printSuccess(`Authenticated. Workspace: ${result.workspace_id}`);
          } else {
            const result = await provisionWorkspace(
              cfg.api_url,
              `${user.name}'s workspace`
            );
            cfg.api_key = result.api_key as string;
            cfg.workspace_id = result.workspace_id as string;
            console.log(`Workspace provisioned: ${result.workspace_id}`);
          }
        } catch (err) {
          printError(`Authentication failed: ${err}`);
        }
      } else {
        console.log(
          "Keeping existing credentials. You can manually update with: corp config set api_key <key>"
        );
      }
    }
  }

  saveConfig(cfg);
  console.log("\nSettings saved to ~/.corp/config.json");
  console.log("Credentials saved to ~/.corp/auth.json");
  console.log("Run 'corp status' to verify your connection.");
}
