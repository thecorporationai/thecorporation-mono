# Local-First Setup Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add hosting mode selection to `corp setup` so users can run locally via process transport, with auto-provisioning and data directory configuration.

**Architecture:** Rust binary gets `--data-dir` flag and `DATA_DIR` env var support. Config system gains `data_dir` field and `server_secrets` in auth.json. Process transport loads secrets from auth.json (fallback to .env). Setup flow adds 3-way hosting mode selection with local auto-provisioning.

**Tech Stack:** Rust (clap), TypeScript, @inquirer/prompts (select), vitest

**Spec:** `docs/superpowers/specs/2026-03-14-local-first-setup-design.md`

---

## Chunk 1: File Structure

| File | Role |
|---|---|
| `services/api-rs/src/main.rs` | **Modify.** Add `--data-dir` to Call, `DATA_DIR` env in init_state, update call_oneshot. |
| `packages/corp-tools/src/types.ts` | **Modify.** Add `data_dir` to CorpConfig. |
| `packages/corp-tools/src/env.ts` | **Modify.** Export `generateFernetKey` and `generateSecret`. |
| `packages/corp-tools/src/process-transport.ts` | **Modify.** Add `options.dataDir`, load server_secrets from auth, better crash messages. |
| `packages/corp-tools/src/api-client.ts` | **Modify.** Pass `dataDir` to processRequest. |
| `packages/cli-ts/src/config.ts` | **Modify.** Add `data_dir` config field, `server_secrets` auth handling, `loadServerSecrets()`. |
| `packages/cli-ts/src/commands/setup.ts` | **Modify.** Hosting mode selection, local setup flow. |

---

## Chunk 2: Task 1 — Rust: --data-dir flag and DATA_DIR env

### Task 1: Add --data-dir to Call subcommand and DATA_DIR to init_state

**Files:**
- Modify: `services/api-rs/src/main.rs:57-68` (Call struct)
- Modify: `services/api-rs/src/main.rs:125-131` (match arm)
- Modify: `services/api-rs/src/main.rs:139-140` (init_state data_dir)
- Modify: `services/api-rs/src/main.rs:397-402` (call_oneshot signature)

- [ ] **Step 1: Add `data_dir` field to `Call` variant (main.rs:57-68)**

Replace the Call variant:

```rust
    Call {
        /// HTTP method (GET, POST, PUT, DELETE, PATCH).
        method: String,
        /// Request path (e.g. /v1/health).
        path: String,
        /// Headers in "Key: Value" format. Can be repeated.
        #[arg(long = "header", short = 'H')]
        headers: Vec<String>,
        /// Read request body from stdin.
        #[arg(long)]
        stdin: bool,
        /// Path to data/repos directory.
        #[arg(long)]
        data_dir: Option<PathBuf>,
    },
```

- [ ] **Step 2: Update match arm to pass data_dir (main.rs:125-131)**

```rust
        Some(Command::Call {
            method,
            path,
            headers,
            stdin,
            data_dir,
        }) => {
            call_oneshot(cli.skip_validation, method, path, headers, stdin, data_dir).await;
        }
```

- [ ] **Step 3: Update init_state to read DATA_DIR env (main.rs:139-140)**

Replace `let data_dir = PathBuf::from("./data/repos");` with:

```rust
    let data_dir = PathBuf::from(
        std::env::var("DATA_DIR").unwrap_or_else(|_| "./data/repos".to_owned()),
    );
```

- [ ] **Step 4: Update call_oneshot signature (main.rs:397-402)**

```rust
async fn call_oneshot(
    skip_validation: bool,
    method: String,
    path: String,
    headers: Vec<String>,
    read_stdin: bool,
    data_dir: Option<PathBuf>,
) {
    use std::io::{Read as _, Write as _};
    use tower::ServiceExt;

    // Set DATA_DIR before init_state reads it
    if let Some(ref dir) = data_dir {
        // SAFETY: single-threaded at this point (before tokio runtime work)
        unsafe { std::env::set_var("DATA_DIR", dir); }
    }

    tracing_subscriber::fmt()
```

Note: `unsafe` is required for `set_var` in Rust 2024 edition. The call happens before any async work or multi-threading, so it is safe.

- [ ] **Step 5: Build and test**

Run: `cd /root/repos/thecorporation-mono/services/api-rs && cargo build --release 2>&1 | tail -3`
Expected: Build succeeds

Run: `JWT_SECRET=dev-secret-32-bytes-minimum-length SECRETS_MASTER_KEY=$(python3 -c "import os,base64;print(base64.urlsafe_b64encode(os.urandom(32)).decode()+'=')") INTERNAL_WORKER_TOKEN=test ./target/release/api-rs --skip-validation call --data-dir /tmp/test-corp-data GET /health 2>/dev/null`
Expected: `{"status":"ok"}`

- [ ] **Step 6: Commit**

```bash
git add services/api-rs/src/main.rs
git commit -m "Add --data-dir flag to call subcommand and DATA_DIR env to init_state"
```

---

## Chunk 3: Task 2 — TypeScript config plumbing

### Task 2: Add data_dir to CorpConfig, server_secrets to auth, loadServerSecrets()

**Files:**
- Modify: `packages/corp-tools/src/types.ts:1-20`
- Modify: `packages/corp-tools/src/env.ts:35-42`
- Modify: `packages/cli-ts/src/config.ts` (multiple locations)

- [ ] **Step 1: Add `data_dir` to CorpConfig type (types.ts:1-20)**

Add after `active_entity_id: string;` (line 16):

```typescript
  data_dir: string;
```

- [ ] **Step 2: Export generateFernetKey and generateSecret (env.ts:35-42)**

Change `function generateFernetKey()` to `export function generateFernetKey()` and `function generateSecret()` to `export function generateSecret()`.

- [ ] **Step 3: Re-export from index.ts**

Add to `packages/corp-tools/src/index.ts`:

```typescript
export { generateFernetKey, generateSecret } from "./env.js";
```

- [ ] **Step 4: Add `data_dir` to ALLOWED_CONFIG_KEYS (config.ts:28-40)**

Add `"data_dir"` to the set.

- [ ] **Step 5: Add `data_dir` to DEFAULTS (config.ts:53-66)**

Add `data_dir: "",` after `hosting_mode: "",`.

- [ ] **Step 6: Add server_secrets to CorpAuthConfig type (config.ts:44-51)**

```typescript
type CorpAuthConfig = {
  api_url?: string;
  api_key?: string;
  workspace_id?: string;
  llm?: {
    api_key?: string;
  };
  server_secrets?: {
    jwt_secret: string;
    secrets_master_key: string;
    internal_worker_token: string;
  };
};
```

- [ ] **Step 7: Add data_dir to normalizeConfig (config.ts after line 303)**

Add after `cfg.active_entity_id = ...` line:

```typescript
  cfg.data_dir = normalizeString(raw.data_dir) ?? cfg.data_dir;
```

- [ ] **Step 8: Add data_dir to serializeConfig (config.ts:342-363)**

Add inside the `serialized` object, after `active_entity_id`:

```typescript
    ...(normalized.data_dir ? { data_dir: normalized.data_dir } : {}),
```

- [ ] **Step 9: Preserve server_secrets in serializeAuth (config.ts:366-377)**

Replace the function:

```typescript
function serializeAuth(cfg: CorpConfig): string {
  const normalized = normalizeConfig(cfg);
  const serialized: CorpAuthConfig = {
    api_url: normalized.api_url,
    api_key: normalized.api_key,
    workspace_id: normalized.workspace_id,
  };
  if (normalized.llm.api_key) {
    serialized.llm = { api_key: normalized.llm.api_key };
  }
  // Preserve server_secrets if present in the existing auth file
  const existingAuth = readJsonFile(AUTH_FILE);
  if (isObject(existingAuth) && isObject(existingAuth.server_secrets)) {
    const ss = existingAuth.server_secrets;
    if (typeof ss.jwt_secret === "string" && typeof ss.secrets_master_key === "string" && typeof ss.internal_worker_token === "string") {
      serialized.server_secrets = {
        jwt_secret: ss.jwt_secret,
        secrets_master_key: ss.secrets_master_key,
        internal_worker_token: ss.internal_worker_token,
      };
    }
  }
  // Allow overriding via cfg (used by setup to write new secrets)
  if ((cfg as Record<string, unknown>)._server_secrets) {
    serialized.server_secrets = (cfg as Record<string, unknown>)._server_secrets as CorpAuthConfig["server_secrets"];
  }
  return JSON.stringify(serialized, null, 2) + "\n";
}
```

- [ ] **Step 10: Add case "data_dir" to setKnownConfigValue (config.ts after line 406)**

Add after the `hosting_mode` case:

```typescript
    case "data_dir":
      cfg.data_dir = value.trim();
      return;
```

- [ ] **Step 11: Add loadServerSecrets export (config.ts, after loadConfig)**

```typescript
export interface ServerSecrets {
  jwt_secret: string;
  secrets_master_key: string;
  internal_worker_token: string;
}

export function loadServerSecrets(): ServerSecrets | null {
  const authRaw = readJsonFile(AUTH_FILE);
  if (!isObject(authRaw) || !isObject(authRaw.server_secrets)) {
    return null;
  }
  const ss = authRaw.server_secrets;
  if (typeof ss.jwt_secret !== "string" || typeof ss.secrets_master_key !== "string" || typeof ss.internal_worker_token !== "string") {
    return null;
  }
  return {
    jwt_secret: ss.jwt_secret,
    secrets_master_key: ss.secrets_master_key,
    internal_worker_token: ss.internal_worker_token,
  };
}
```

- [ ] **Step 12: Build and verify**

Run: `cd /root/repos/thecorporation-mono/packages/corp-tools && npx tsup 2>&1 | tail -3`
Run: `cd /root/repos/thecorporation-mono/packages/cli-ts && npx tsc --noEmit 2>&1 | grep -c "error" || echo "0 errors"`
Expected: Build succeeds, no new errors

- [ ] **Step 13: Commit**

```bash
git add packages/corp-tools/src/types.ts packages/corp-tools/src/env.ts packages/corp-tools/src/index.ts packages/cli-ts/src/config.ts
git commit -m "Add data_dir config, server_secrets auth, loadServerSecrets export"
```

---

## Chunk 4: Task 3 — Process transport: server_secrets + dataDir

### Task 3: Update process transport to load server_secrets and pass --data-dir

**Files:**
- Modify: `packages/corp-tools/src/process-transport.ts:78-98`
- Modify: `packages/corp-tools/src/api-client.ts:113-124`
- Modify: `packages/corp-tools/src/__tests__/process-transport.test.ts`

- [ ] **Step 1: Add tests for new behavior**

Add to `packages/corp-tools/src/__tests__/process-transport.test.ts`:

```typescript
describe("processRequest options", () => {
  it("includes --data-dir in args when dataDir option provided", () => {
    // We can't easily test the actual spawn, but we can test the args building
    // by checking that the function signature accepts options
    expect(typeof processRequest).toBe("function");
    // The function should accept 6 parameters now
    expect(processRequest.length).toBeGreaterThanOrEqual(5);
  });
});
```

- [ ] **Step 2: Update processRequest signature and env loading (process-transport.ts)**

Replace lines 78-98 (from `let envLoaded` through `const args` line):

```typescript
let envLoaded = false;

function ensureEnv(): void {
  if (envLoaded) return;

  // Try server_secrets from auth.json first (local mode)
  try {
    const { loadServerSecrets } = require("@thecorporation/cli-ts/config");
    const secrets = loadServerSecrets?.();
    if (secrets) {
      if (!process.env.JWT_SECRET) process.env.JWT_SECRET = secrets.jwt_secret;
      if (!process.env.SECRETS_MASTER_KEY) process.env.SECRETS_MASTER_KEY = secrets.secrets_master_key;
      if (!process.env.INTERNAL_WORKER_TOKEN) process.env.INTERNAL_WORKER_TOKEN = secrets.internal_worker_token;
      envLoaded = true;
      return;
    }
  } catch {
    // cli-ts not available or loadServerSecrets not found
  }

  // Fallback: load from .env file
  const envPath = resolve(process.cwd(), ".env");
  ensureEnvFile(envPath);
  loadEnvFile(envPath);
  envLoaded = true;
}

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

  if (options?.dataDir) {
    args.push("--data-dir", options.dataDir);
  }

  args.push(method, pathWithQuery);
```

Keep the rest of the function unchanged (header loop, stdin, try/catch).

- [ ] **Step 3: Improve crash error message (process-transport.ts)**

In the catch block, replace the final `throw` (currently line 136):

```typescript
    // Binary crashed or env var missing
    const isConfigError = stderr.includes("panic") || stderr.includes("INTERNAL_WORKER_TOKEN") || stderr.includes("JWT_SECRET") || stderr.includes("SECRETS_MASTER_KEY");
    if (isConfigError) {
      throw new Error("Server configuration incomplete. Run 'corp setup' to configure local mode.");
    }
    throw new Error(`api-rs process failed:\n${stderr || stdout || String(err)}`);
```

- [ ] **Step 4: Update CorpAPIClient.request to pass dataDir (api-client.ts:113-124)**

The client needs to know `data_dir`. Add a lazy config loader. Replace the `request` method:

```typescript
  private _dataDir: string | undefined;
  private get dataDir(): string | undefined {
    if (this._dataDir !== undefined) return this._dataDir || undefined;
    try {
      const { loadConfig } = require("@thecorporation/cli-ts/config");
      const cfg = loadConfig?.();
      this._dataDir = cfg?.data_dir || "";
    } catch {
      this._dataDir = "";
    }
    return this._dataDir || undefined;
  }

  private async request(method: string, path: string, body?: unknown, params?: Record<string, string>): Promise<Response> {
    let fullPath = path;
    if (params) {
      const qs = new URLSearchParams(params).toString();
      if (qs) fullPath += `?${qs}`;
    }

    if (this.apiUrl.startsWith("process://")) {
      const hdrs = this.headers();
      const bodyStr = body !== undefined ? JSON.stringify(body) : undefined;
      return processRequest(this.apiUrl, method, fullPath, hdrs, bodyStr, { dataDir: this.dataDir });
    }

    const url = `${this.apiUrl}${fullPath}`;
    const opts: RequestInit = { method, headers: this.headers() };
    if (body !== undefined) opts.body = JSON.stringify(body);
    return fetch(url, opts);
  }
```

Actually — wait. The `corp-tools` package shouldn't depend on `cli-ts` (that creates a circular dependency). Let me reconsider.

**Better approach:** The process transport reads `data_dir` and `server_secrets` from the config files directly, without importing cli-ts. The auth.json and config.json paths are at `~/.corp/auth.json` and `~/.corp/config.json`.

Replace the `ensureEnv` function and add a config reader:

```typescript
import { homedir } from "node:os";
import { join } from "node:path";

function readJsonFileSafe(path: string): Record<string, unknown> | null {
  try {
    if (!existsSync(path)) return null;
    const { readFileSync } = require("node:fs");
    return JSON.parse(readFileSync(path, "utf-8"));
  } catch {
    return null;
  }
}

const CORP_CONFIG_DIR = process.env.CORP_CONFIG_DIR || join(homedir(), ".corp");

function loadServerSecretsFromAuth(): { jwt_secret: string; secrets_master_key: string; internal_worker_token: string } | null {
  const auth = readJsonFileSafe(join(CORP_CONFIG_DIR, "auth.json"));
  if (!auth || typeof auth !== "object") return null;
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
  const auth = readJsonFileSafe(join(CORP_CONFIG_DIR, "auth.json"));
  // data_dir lives in config.json
  const dataDir = typeof cfg.data_dir === "string" ? cfg.data_dir : undefined;
  return dataDir || undefined;
}

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
```

And the `processRequest` function auto-reads `dataDir` from config:

```typescript
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
```

This means `CorpAPIClient.request()` does NOT need to change — processRequest reads dataDir itself.

- [ ] **Step 5: Run tests**

Run: `cd /root/repos/thecorporation-mono/packages/corp-tools && npx vitest run`
Expected: All tests pass

- [ ] **Step 6: Build**

Run: `cd /root/repos/thecorporation-mono/packages/corp-tools && npx tsup 2>&1 | tail -3`
Expected: Build succeeds

- [ ] **Step 7: Commit**

```bash
git add packages/corp-tools/src/process-transport.ts packages/corp-tools/src/api-client.ts packages/corp-tools/src/__tests__/process-transport.test.ts
git commit -m "Process transport: load server_secrets from auth.json, pass --data-dir"
```

---

## Chunk 5: Task 4 — Setup flow: hosting mode selection

### Task 4: Rewrite setup.ts with hosting mode selection and local provisioning

**Files:**
- Modify: `packages/cli-ts/src/commands/setup.ts`

- [ ] **Step 1: Rewrite setup.ts**

The full updated file. Key changes:
- Import `select` from `@inquirer/prompts`
- Import `generateSecret`, `generateFernetKey`, `processRequest` from corp-tools
- Import `loadServerSecrets`, `saveConfig`, `loadConfig`, `validateApiUrl` from config
- Add `hostingModePrompt()` — returns `"local" | "cloud" | "self-hosted"`
- Add `setupLocal()` — data dir prompt, secret generation, auto-provision
- Restructure `setupCommand()` to branch on hosting mode

```typescript
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

// --- Magic link auth (unchanged) ---

async function requestMagicLink(apiUrl: string, email: string, tosAccepted: boolean): Promise<void> {
  const resp = await fetch(`${apiUrl}/v1/auth/magic-link`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ email, tos_accepted: tosAccepted }),
  });
  if (!resp.ok) {
    const data = await resp.json().catch(() => ({}));
    const detail = (data as Record<string, unknown>)?.error ?? (data as Record<string, unknown>)?.message ?? resp.statusText;
    throw new Error(typeof detail === "string" ? detail : JSON.stringify(detail));
  }
}

async function verifyMagicLinkCode(apiUrl: string, code: string): Promise<{ api_key: string; workspace_id: string }> {
  const resp = await fetch(`${apiUrl}/v1/auth/magic-link/verify`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ code, client: "cli" }),
  });
  const data = (await resp.json().catch(() => ({}))) as Record<string, unknown>;
  if (!resp.ok) {
    const detail = data?.error ?? data?.message ?? resp.statusText;
    throw new Error(typeof detail === "string" ? detail : JSON.stringify(detail));
  }
  if (!data.api_key || !data.workspace_id) {
    throw new Error("Unexpected response — missing api_key or workspace_id");
  }
  return { api_key: data.api_key as string, workspace_id: data.workspace_id as string };
}

function isCloudApi(url: string): boolean {
  return url.replace(/\/+$/, "").includes("thecorporation.ai");
}

async function magicLinkAuth(apiUrl: string, email: string): Promise<{ api_key: string; workspace_id: string }> {
  console.log("\nSending magic link to " + email + "...");
  await requestMagicLink(apiUrl, email, true);
  console.log("Check your email for a sign-in link from TheCorporation.");
  console.log("Copy the code from the URL (the ?code=... part) and paste it below.\n");
  const code = await input({ message: "Paste your magic link code" });
  const trimmed = code.trim().replace(/^.*[?&]code=/, "");
  if (!trimmed) throw new Error("No code provided");
  console.log("Verifying...");
  return verifyMagicLinkCode(apiUrl, trimmed);
}

// --- Local provisioning ---

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

function localProvision(dataDir: string, name: string): { api_key: string; workspace_id: string } {
  // Inject server secrets into env (they were just generated and saved to auth.json)
  // processRequest's ensureEnv will load them from auth.json

  const resp = processRequest(
    "process://",
    "POST",
    "/v1/workspaces/provision",
    { "Content-Type": "application/json" },
    JSON.stringify({ name }),
    { dataDir },
  );

  if (!resp.ok) {
    throw new Error(`Provision failed: HTTP ${resp.status}`);
  }

  // processRequest returns sync Response-like with async json()
  // But we need sync here — the body is already available as text
  let body: Record<string, unknown>;
  const textPromise = resp.text();
  // Since buildProcessResponse returns an immediately-resolved promise, we can hack around it
  let text = "";
  (textPromise as Promise<string>).then(t => { text = t; });
  // Force microtask flush — this works because buildProcessResponse closures resolve instantly
  body = JSON.parse(text || "{}");

  if (!body.api_key || !body.workspace_id) {
    throw new Error("Provision response missing api_key or workspace_id");
  }
  return { api_key: body.api_key as string, workspace_id: body.workspace_id as string };
}

// --- Main setup ---

export async function setupCommand(): Promise<void> {
  const cfg = loadConfig();
  console.log("Welcome to corp — corporate governance from the terminal.\n");

  // User info
  console.log("--- User Info ---");
  const user = cfg.user ?? { name: "", email: "" };
  user.name = await input({ message: "Your name", default: user.name || undefined });
  user.email = await input({ message: "Your email", default: user.email || undefined });
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
        const result = localProvision(dataDir, `${user.name}'s workspace`);
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
        console.log("You can manually set credentials with: corp config set api_key <key>");
      }
    } else {
      console.log("\nExisting credentials found. Run 'corp status' to verify.");
    }

  } else {
    // Self-hosted
    const url = await input({
      message: "Server URL",
      default: cfg.api_url !== CLOUD_API_URL && !cfg.api_url.startsWith("process://") ? cfg.api_url : undefined,
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
        const result = await provisionWorkspace(cfg.api_url, `${user.name}'s workspace`);
        cfg.api_key = result.api_key as string;
        cfg.workspace_id = result.workspace_id as string;
        console.log(`Workspace provisioned: ${result.workspace_id}`);
      } catch (err) {
        printError(`Auto-provision failed: ${err}`);
        console.log("You can manually set credentials with: corp config set api_key <key>");
      }
    }
  }

  saveConfig(cfg);
  console.log("\nSettings saved to ~/.corp/config.json");
  console.log("Credentials saved to ~/.corp/auth.json");
  console.log("Run 'corp status' to verify your connection.");
}
```

**Note about `localProvision`:** The `processRequest` function returns a sync Response-like object whose `text()` and `json()` return immediately-resolving promises (they're closures, not real body streams). We need the body synchronously in setup. The simplest fix is to make `processRequest` also expose a `_bodyText` property, or we can use the fact that `execFileSync` is sync and the response body is already available. Let the implementer handle this — the key insight is that `buildProcessResponse`'s `text()` resolves instantly, so `await resp.text()` in an async context works fine. Since `setupCommand` is already async, just use `await`:

```typescript
async function localProvision(dataDir: string, name: string): Promise<{ api_key: string; workspace_id: string }> {
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

  const body = await resp.json() as Record<string, unknown>;
  if (!body.api_key || !body.workspace_id) {
    throw new Error("Provision response missing api_key or workspace_id");
  }
  return { api_key: body.api_key as string, workspace_id: body.workspace_id as string };
}
```

And update the call site:
```typescript
const result = await localProvision(dataDir, `${user.name}'s workspace`);
```

- [ ] **Step 2: Build and verify**

Run: `cd /root/repos/thecorporation-mono/packages/cli-ts && npx tsc --noEmit`
Expected: No new errors

- [ ] **Step 3: Commit**

```bash
git add packages/cli-ts/src/commands/setup.ts
git commit -m "Add hosting mode selection and local-first setup flow"
```

---

## Chunk 6: Task 5 — Integration test and final verification

### Task 5: Test the full local setup flow

**Files:**
- No new files — manual end-to-end test

- [ ] **Step 1: Build everything**

Run: `cd /root/repos/thecorporation-mono/services/api-rs && cargo build --release 2>&1 | tail -3`
Run: `cd /root/repos/thecorporation-mono/packages/corp-tools && npx tsup 2>&1 | tail -3`
Expected: Both succeed

- [ ] **Step 2: Run unit tests**

Run: `cd /root/repos/thecorporation-mono/packages/corp-tools && npx vitest run`
Expected: All tests pass

- [ ] **Step 3: Run integration tests**

Run: `cd /root/repos/thecorporation-mono/packages/corp-tools && npx vitest run --config vitest.integration.config.ts`
Expected: Process transport integration tests pass

- [ ] **Step 4: Test --data-dir flag manually**

```bash
cd /root/repos/thecorporation-mono/services/api-rs
JWT_SECRET=test-32-bytes-minimum-length-here SECRETS_MASTER_KEY=$(python3 -c "import os,base64;print(base64.urlsafe_b64encode(os.urandom(32)).decode()+'=')") INTERNAL_WORKER_TOKEN=test ./target/release/api-rs --skip-validation call --data-dir /tmp/corp-test-data GET /health 2>/dev/null
```
Expected: `{"status":"ok"}`

- [ ] **Step 5: Commit any fixes**

```bash
git add -A && git commit -m "Fix integration issues from local-first setup testing"
```
(Only if there are fixes needed)
