#!/usr/bin/env node

/**
 * Thin Node.js wrapper that locates and execs the native `corp` binary.
 *
 * This lets `npx @thecorporation/corp entities list` work without a global
 * cargo install.
 */

import { spawn } from "node:child_process";
import { getCliBinaryPath } from "./server.js";

try {
  const bin = getCliBinaryPath();
  const args = process.argv.slice(2);

  const proc = spawn(bin, args, { stdio: "inherit" });

  proc.on("close", (code) => {
    process.exit(code ?? 1);
  });

  proc.on("error", (err) => {
    console.error(`Failed to start corp: ${err.message}`);
    process.exit(1);
  });
} catch (err: unknown) {
  const msg = err instanceof Error ? err.message : String(err);
  console.error(msg);
  process.exit(1);
}
