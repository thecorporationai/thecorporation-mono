# Local-First Setup Flow

**Date:** 2026-03-14
**Status:** Approved

## Problem

`corp setup` currently only supports cloud (magic link auth) or self-hosted HTTP servers. Users cannot run corp locally without first starting a server. The process transport (`process://`) we just built enables serverless local execution, but there's no setup flow to configure it.

## Design

### Setup Flow

`corp setup` adds a hosting mode selection after user info:

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
2. Create directory if it doesn't exist
3. If directory has existing data, reuse it ("Found existing data, reusing.")
4. Generate `JWT_SECRET`, `SECRETS_MASTER_KEY`, `INTERNAL_WORKER_TOKEN` and store in `~/.corp/auth.json` under `server_secrets`
5. Set `api_url = process://` and `data_dir` in config
6. Auto-provision workspace via process transport (internal `POST /v1/workspaces/provision`)
7. Print: "Your local workspace is ready. Run 'corp status' to verify."

**Cloud mode:** Current flow unchanged (magic link auth to `api.thecorporation.ai`).

**Self-hosted mode:** Prompt for server URL, then provision directly (existing non-cloud flow).

### Data Directory

Default: `~/.corp/data`. Stored in config as `data_dir`.

Passed to `api-rs` via:
- `--data-dir` CLI flag on the `call` subcommand (new)
- `DATA_DIR` environment variable (existing but hardcoded to `./data/repos`)
- Flag takes precedence over env var; env var takes precedence over `./data/repos` default

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

**Update `call_oneshot` signature** to accept `data_dir: Option<PathBuf>`. If provided, set `DATA_DIR` env var before calling `init_state()`.

**Update `init_state()`** to read data directory from `DATA_DIR` env var, falling back to `./data/repos`:

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

These secrets are generated once during setup and persist across sessions. They never leave the machine.

### Process Transport Changes (`packages/corp-tools/src/process-transport.ts`)

Before spawning `api-rs call`:
1. Load config to get `data_dir`
2. Load auth.json to get `server_secrets`
3. Inject `JWT_SECRET`, `SECRETS_MASTER_KEY`, `INTERNAL_WORKER_TOKEN` as env vars from `server_secrets`
4. Pass `--data-dir {path}` flag if `data_dir` is set in config

This replaces the current `.env` file loading for process transport. The `.env` approach remains for `corp serve`.

### Config Changes

**New config field:**
- `data_dir`: string — path to data directory (default `~/.corp/data` for local, unused for cloud/self-hosted)

**Existing field gets values:**
- `hosting_mode`: `"local"` | `"cloud"` | `"self-hosted"` (currently exists but unused)

**New allowed config keys:** `data_dir` added to `ALLOWED_CONFIG_KEYS` set.

### Files Changed

| File | Change |
|---|---|
| `services/api-rs/src/main.rs` | Add `--data-dir` to `Call`, read `DATA_DIR` env in `init_state()` |
| `packages/cli-ts/src/commands/setup.ts` | Hosting mode selection, local setup flow with data dir prompt and auto-provision |
| `packages/cli-ts/src/config.ts` | Add `data_dir` to config type and allowed keys |
| `packages/cli-ts/src/types.ts` | Add `data_dir` and `server_secrets` to CorpConfig type |
| `packages/corp-tools/src/process-transport.ts` | Load server_secrets from auth, pass --data-dir to binary |

### Files Not Changed

- `packages/corp-tools/src/api-client.ts` — transport dispatch already works
- `packages/corp-tools/src/env.ts` — still used by `corp serve`, not by process transport in local mode
- Cloud and self-hosted auth flows — unchanged

### Error Handling

- **Binary not found during local setup:** "api-rs binary not found. Install @thecorporation/server or build from source."
- **Data directory not writable:** "Cannot create data directory at {path}. Check permissions."
- **Auto-provision fails:** "Workspace provisioning failed: {error}. You can retry with 'corp setup'."
- **Existing data directory with data:** Not an error — reuse silently with a message.
