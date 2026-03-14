# Local-First Setup Flow

**Date:** 2026-03-14
**Status:** Approved

## Problem

`corp setup` currently only supports cloud (magic link auth) or self-hosted HTTP servers. Users cannot run corp locally without first starting a server. The process transport (`process://`) we just built enables serverless local execution, but there's no setup flow to configure it.

## Design

### Setup Flow

`corp setup` adds a hosting mode selection after user info (using `select` from `@inquirer/prompts`):

```
Welcome to corp — corporate governance from the terminal.

--- User Info ---
Your name: Alice
Your email: alice@example.com

--- Hosting Mode ---
How would you like to run corp?
  > Local (your machine)
    TheCorporation cloud
    Self-hosted server (custom URL)
```

**Local mode:**
1. Prompt for data directory (default `~/.corp/data`)
2. Create directory if it doesn't exist; if it has existing data, print "Found existing data, reusing."
3. Generate `JWT_SECRET`, `SECRETS_MASTER_KEY`, `INTERNAL_WORKER_TOKEN` using `generateSecret()` and `generateFernetKey()` from `corp-tools/env.ts` (export these). Store in `~/.corp/auth.json` under `server_secrets`.
4. Set `api_url = process://`, `data_dir`, and `hosting_mode = "local"` in config
5. Inject server_secrets into `process.env` and call `processRequest()` directly to `POST /v1/workspaces/provision` (bypassing `provisionWorkspace()` which uses `fetch()`). Pass `--data-dir` to the binary.
6. Save resulting `api_key` and `workspace_id` to config
7. Print: "Your local workspace is ready. Run 'corp status' to verify."

**Re-running setup with existing local data:** If `data_dir` exists and is non-empty, skip provisioning. Print "Existing workspace found." and verify credentials by hitting `GET /health` via process transport. If the workspace_id is set in config, keep it.

**Cloud mode:** Current flow unchanged (magic link auth to `api.thecorporation.ai`).

**Self-hosted mode:** Prompt for server URL, then provision directly (existing non-cloud flow).

### Data Directory

Default: `~/.corp/data`. Stored in config as `data_dir`.

Passed to `api-rs` via:
- `--data-dir` CLI flag on the `call` subcommand (new)
- `DATA_DIR` environment variable (fallback)
- Flag takes precedence. If neither is set, falls back to `./data/repos`.

### Rust Changes (`services/api-rs/src/main.rs`)

**Add `--data-dir` flag to `Call` subcommand:**

```rust
Call {
    method: String,
    path: String,
    #[arg(long = "header", short = 'H')]
    headers: Vec<String>,
    #[arg(long)]
    stdin: bool,
    #[arg(long)]
    data_dir: Option<PathBuf>,
}
```

**Update `call_oneshot` signature** to accept `data_dir: Option<PathBuf>`. If provided, set `DATA_DIR` env var before calling `init_state()`. Use `unsafe { std::env::set_var() }` (required in Rust 2024 edition).

**Update `init_state()`** to read data directory from `DATA_DIR` env var:

```rust
let data_dir = PathBuf::from(
    std::env::var("DATA_DIR").unwrap_or_else(|_| "./data/repos".to_owned())
);
```

### Auth.json Changes

Add `server_secrets` field (only present for local mode):

```json
{
  "api_url": "process://",
  "api_key": "generated-api-key",
  "workspace_id": "generated-workspace-id",
  "server_secrets": {
    "jwt_secret": "64-char-hex",
    "secrets_master_key": "base64url-fernet-key",
    "internal_worker_token": "64-char-hex"
  }
}
```

Type changes:
- Add `server_secrets` to `CorpAuthConfig` type in config.ts
- `serializeAuth()` preserves `server_secrets` if present
- Add `loadServerSecrets(): ServerSecrets | null` export to config.ts — reads auth.json directly, returns `server_secrets` object or null. Does NOT merge into CorpConfig (these are server-internal secrets, not user config).

### Process Transport Changes (`packages/corp-tools/src/process-transport.ts`)

**New `processRequest` signature:**

```typescript
export function processRequest(
  processUrl: string,
  method: string,
  pathWithQuery: string,
  headers: Record<string, string>,
  body?: string,
  options?: { dataDir?: string },
): Response
```

Before spawning `api-rs call`:
1. Call `loadServerSecrets()` from config. If secrets exist, inject `JWT_SECRET`, `SECRETS_MASTER_KEY`, `INTERNAL_WORKER_TOKEN` into `process.env`.
2. If no server_secrets found, fall back to existing `.env` loading (`ensureEnv()`). This preserves backward compat for `.env`-based users.
3. If `options.dataDir` is set, add `--data-dir {path}` to args (position: after `call`, before method).

**`CorpAPIClient.request()` changes:** Load config once (lazy) to get `data_dir`, pass it through `options.dataDir` to `processRequest()`.

**Error handling for binary crashes:** If the process exits non-zero without an `HTTP NNN` line on stderr, and stderr contains "panic" or "INTERNAL_WORKER_TOKEN", produce: "Server configuration incomplete. Run 'corp setup' to configure local mode."

### Config Changes

**CorpConfig type** (`packages/corp-tools/src/types.ts`):
- Add `data_dir: string` (default: `""`)

**config.ts changes:**
- Add `"data_dir"` to `ALLOWED_CONFIG_KEYS`
- Add `data_dir: ""` to `DEFAULTS`
- Add normalization in `normalizeConfig()`: `cfg.data_dir = normalizeString(raw.data_dir) ?? cfg.data_dir`
- Add serialization in `serializeConfig()`: include `data_dir` when non-empty
- Add `case "data_dir"` to `setKnownConfigValue()`: `cfg.data_dir = value.trim()`
- Add `server_secrets` to `CorpAuthConfig` type
- `serializeAuth()` preserves `server_secrets`
- New export: `loadServerSecrets()`

**env.ts changes:**
- Export `generateFernetKey()` and `generateSecret()` (currently private)

### Files Changed

| File | Change |
|---|---|
| `services/api-rs/src/main.rs` | Add `--data-dir` to `Call`, update `call_oneshot` to accept and set it, read `DATA_DIR` env in `init_state()` |
| `packages/cli-ts/src/commands/setup.ts` | Hosting mode selection (import `select`), local setup flow with data dir prompt, direct `processRequest()` call for provisioning |
| `packages/cli-ts/src/config.ts` | Add `data_dir` config field, `server_secrets` auth field, `loadServerSecrets()` export, serialization |
| `packages/corp-tools/src/types.ts` | Add `data_dir: string` to `CorpConfig` |
| `packages/corp-tools/src/process-transport.ts` | Add `options.dataDir` parameter, load server_secrets from auth, fallback to .env, better crash error messages |
| `packages/corp-tools/src/api-client.ts` | Pass `data_dir` from config through to `processRequest()` |
| `packages/corp-tools/src/env.ts` | Export `generateFernetKey()` and `generateSecret()` |

### Error Handling

- **Binary not found:** "No api-rs binary found. Install @thecorporation/server or build from source."
- **Binary crashes (missing env vars):** "Server configuration incomplete. Run 'corp setup' to configure local mode."
- **Data directory not writable:** "Cannot create data directory at {path}. Check permissions."
- **Auto-provision fails:** "Workspace provisioning failed: {error}. You can retry with 'corp setup'."
- **Re-run with existing workspace:** Skip provisioning, verify connection, keep existing workspace.
