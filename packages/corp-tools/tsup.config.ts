import { defineConfig } from "tsup";

export default defineConfig([
  // Node entry — existing behavior
  {
    entry: ["src/index.ts"],
    format: ["esm"],
    target: "node20",
    outDir: "dist",
    clean: true,
    dts: true,
    sourcemap: true,
    splitting: false,
  },
  // Browser entry — self-contained bundle for web terminal
  {
    entry: { browser: "src/browser.ts" },
    format: ["esm"],
    target: "es2022",
    outDir: "dist",
    dts: true,
    sourcemap: true,
    splitting: false,
    noExternal: [/.*/],  // Bundle everything into a single file
    platform: "browser",
  },
]);
