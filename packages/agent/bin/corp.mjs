#!/usr/bin/env node

import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { existsSync, mkdirSync, copyFileSync } from "node:fs";
import { execFileSync } from "node:child_process";

const pkgDir = dirname(dirname(fileURLToPath(import.meta.url)));
const extDir = join(pkgDir, "extension");

// Ensure .pi/extensions/ exists in cwd
const targetExtDir = join(process.cwd(), ".pi", "extensions");
mkdirSync(targetExtDir, { recursive: true });

// Always copy corp.ts (keep current with installed package version)
copyFileSync(join(extDir, "corp.ts"), join(targetExtDir, "corp.ts"));

// Copy AGENT.md only if not present (don't overwrite user customizations)
const targetAgent = join(process.cwd(), ".pi", "AGENT.md");
if (!existsSync(targetAgent)) {
  copyFileSync(join(extDir, "AGENT.md"), targetAgent);
}

// Launch pi, forwarding all arguments
try {
  execFileSync("pi", process.argv.slice(2), { stdio: "inherit" });
} catch (err) {
  if (err.code === "ENOENT") {
    console.error(
      "Error: 'pi' not found. Install the Pi coding agent first:\n\n" +
        "  npm i -g @mariozechner/pi-coding-agent\n"
    );
    process.exit(1);
  }
  // pi exited with non-zero — propagate the exit code
  process.exit(err.status ?? 1);
}
