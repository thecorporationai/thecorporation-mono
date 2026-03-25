#!/usr/bin/env node

/**
 * Postinstall hook — ensures the correct platform-specific binary package is
 * available.  The platform packages are listed as optionalDependencies so npm
 * will attempt to install the matching one automatically.  This hook verifies
 * that the binary actually exists and prints a helpful message if it doesn't.
 */

import { platform, arch } from "node:os";
import { chmodSync, existsSync } from "node:fs";
import { resolve, join } from "node:path";
import { createRequire } from "node:module";
import { fileURLToPath } from "node:url";

const require = createRequire(import.meta.url);
const __dirname = fileURLToPath(new URL(".", import.meta.url));

const PLATFORM_PACKAGES = {
  "linux-x64":    "@thecorporation/corp-linux-x64-gnu",
  "linux-arm64":  "@thecorporation/corp-linux-arm64-gnu",
  "darwin-x64":   "@thecorporation/corp-darwin-x64",
  "darwin-arm64": "@thecorporation/corp-darwin-arm64",
  "win32-x64":    "@thecorporation/corp-win32-x64-msvc",
};

const key = `${platform()}-${arch()}`;
const pkg = PLATFORM_PACKAGES[key];

function ensureExecutable(binPath) {
  if (platform() === "win32") return;
  try {
    chmodSync(binPath, 0o755);
  } catch (error) {
    const msg = error instanceof Error ? error.message : String(error);
    console.warn(`@thecorporation/corp: failed to mark ${binPath} executable: ${msg}`);
  }
}

if (!pkg) {
  console.warn(`@thecorporation/corp: no prebuilt binary for ${key}`);
  console.warn("You can build from source: cargo build -p corp-server -p corp-cli --release");
  process.exit(0);
}

try {
  const pkgDir = resolve(require.resolve(`${pkg}/package.json`), "..");
  const serverBin = platform() === "win32" ? "corp-server.exe" : "corp-server";
  const cliBin = platform() === "win32" ? "corp.exe" : "corp";

  const serverPath = join(pkgDir, "bin", serverBin);
  const cliPath = join(pkgDir, "bin", cliBin);

  if (existsSync(serverPath) && existsSync(cliPath)) {
    ensureExecutable(serverPath);
    ensureExecutable(cliPath);
    process.exit(0);
  }
  console.warn(`@thecorporation/corp: binary package ${pkg} installed but binaries missing`);
} catch {
  console.warn(`@thecorporation/corp: platform package ${pkg} not available`);
  console.warn("Build from source: cargo build -p corp-server -p corp-cli --release");
}
