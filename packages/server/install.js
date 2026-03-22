#!/usr/bin/env node
import { platform, arch } from "node:os";
import { execSync } from "node:child_process";
import { existsSync } from "node:fs";
import { resolve, join } from "node:path";
import { fileURLToPath } from "node:url";
import { createRequire } from "node:module";

const require = createRequire(import.meta.url);

const __dirname = fileURLToPath(new URL(".", import.meta.url));

const PLATFORM_PACKAGES = {
  "linux-x64": "@thecorporation/server-linux-x64-gnu",
  "linux-arm64": "@thecorporation/server-linux-arm64-gnu",
  "darwin-x64": "@thecorporation/server-darwin-x64",
  "darwin-arm64": "@thecorporation/server-darwin-arm64",
  "win32-x64": "@thecorporation/server-win32-x64-msvc",
};

const key = `${platform()}-${arch()}`;
const pkg = PLATFORM_PACKAGES[key];

if (!pkg) {
  console.warn(`@thecorporation/server: unsupported platform ${key}`);
  process.exit(0);
}

// Check if the binary is already available (e.g. installed by parent)
try {
  const pkgDir = resolve(require.resolve(`${pkg}/package.json`), "..");
  const binName = platform() === "win32" ? "api-rs.exe" : "api-rs";
  if (existsSync(join(pkgDir, "bin", binName))) {
    process.exit(0);
  }
} catch {
  // Not installed yet — continue
}

console.log(`@thecorporation/server: installing ${pkg} for ${key}...`);
try {
  execSync(`npm install ${pkg}`, { stdio: "inherit", cwd: __dirname });
} catch {
  console.warn(`@thecorporation/server: failed to install ${pkg} (non-fatal)`);
}
