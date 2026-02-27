# @thecorporation/server

Pre-built binaries for the Corporation API server. Wraps the Rust backend as a Node.js child process with automatic platform detection.

Part of [The Corporation](https://thecorporation.ai) — agent-native corporate infrastructure.

## Install

```bash
npm install @thecorporation/server
```

The correct binary for your platform is installed automatically via `optionalDependencies`.

**Supported platforms:** macOS (Apple Silicon, Intel), Linux (x64, arm64), Windows (x64).

## Usage

```js
import { startServer, isAvailable, getBinaryPath } from "@thecorporation/server";

// Check if a binary exists for this platform
if (isAvailable()) {
  const child = startServer({
    port: 8000,
    dataDir: "./data/repos",
  });
}
```

### `startServer(options?)`

Spawns the server as a child process and returns a `ChildProcess`.

| Option | Env var | Default |
|---|---|---|
| `port` | `PORT` | `8000` |
| `dataDir` | `DATA_DIR` | `./data/repos` |
| `redisUrl` | `REDIS_URL` | — |
| `jwtPrivateKeyPem` | `JWT_PRIVATE_KEY_PEM` | — |
| `jwtPublicKeyPem` | `JWT_PUBLIC_KEY_PEM` | — |
| `stripeSecretKey` | `STRIPE_SECRET_KEY` | — |
| `stripeWebhookSecret` | `STRIPE_WEBHOOK_SECRET` | — |
| `commitSigningKey` | `COMMIT_SIGNING_KEY` | — |
| `stdio` | — | `"inherit"` |

### `getBinaryPath()`

Returns the resolved binary path, or `null` if unavailable. Resolution order:

1. `CORP_SERVER_BIN` environment variable
2. Platform-specific npm optional dependency
3. Local dev build at `services/api-rs/target/release/api-rs`

### `isAvailable()`

Returns `true` if a binary exists for the current platform.

## Links

- [thecorporation.ai](https://thecorporation.ai)
- [GitHub](https://github.com/thecorporationai/thecorporation-mono/tree/main/packages/server)

## License

MIT
