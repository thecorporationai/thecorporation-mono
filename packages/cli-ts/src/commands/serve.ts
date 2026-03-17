import { resolve } from "node:path";
import type { ChildProcess } from "node:child_process";
import { ensureEnvFile, loadEnvFile } from "@thecorporation/corp-tools";

interface ServeOptions {
  port: string;
  dataDir: string;
}

export async function serveCommand(opts: ServeOptions): Promise<void> {
  let server: { getBinaryPath: () => string | null; isAvailable: () => boolean; startServer: (options: Record<string, unknown>) => ChildProcess };
  try {
    // @ts-expect-error — optional dependency, handled by catch block
    server = await import("@thecorporation/server");
  } catch {
    console.error(
      "Error: @thecorporation/server is not installed.\n\n" +
      "Install it with:\n" +
      "  npm install @thecorporation/server\n\n" +
      "Or run the Rust binary directly:\n" +
      "  cd services/api-rs && cargo run"
    );
    process.exit(1);
  }

  if (!server.isAvailable()) {
    console.error(
      "Error: No server binary available for this platform.\n\n" +
      "Pre-built binaries are available for:\n" +
      "  - linux-x64, linux-arm64\n" +
      "  - darwin-x64, darwin-arm64\n" +
      "  - win32-x64\n\n" +
      "You can build from source:\n" +
      "  cd services/api-rs && cargo build --release"
    );
    process.exit(1);
  }

  const port = parseInt(opts.port, 10);
  if (isNaN(port) || port > 65535) {
    console.error(`Error: Invalid port "${opts.port}"`);
    process.exit(1);
  }

  // Load .env file, generating one if it doesn't exist
  const envPath = resolve(process.cwd(), ".env");
  ensureEnvFile(envPath);
  loadEnvFile(envPath);

  const localUrl = `http://localhost:${port}`;
  console.log(`Starting server on port ${port}...`);
  console.log(`Data directory: ${opts.dataDir}`);
  console.log(`CLI API URL remains unchanged.`);
  console.log(`  Use CORP_API_URL=${localUrl} for commands against this local server.\n`);

  const child = server.startServer({
    port,
    dataDir: opts.dataDir,
  });

  const shutdown = () => {
    console.log("\nShutting down server...");
    child.kill("SIGTERM");
  };

  process.on("SIGINT", shutdown);
  process.on("SIGTERM", shutdown);

  child.on("exit", (code) => {
    process.exit(code ?? 0);
  });
}
