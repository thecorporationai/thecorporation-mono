import { defineConfig } from "tsup";

export default defineConfig({
  entry: ["src/index.ts"],
  format: ["esm"],
  target: "node20",
  outDir: "dist",
  clean: true,
  dts: true,
  sourcemap: true,
  splitting: false,
  banner: {
    js: "#!/usr/bin/env node",
  },
  // Don't bundle dependencies — they'll be in node_modules at runtime
  noExternal: [],
  external: [
    "commander", "chalk", "cli-table3", "ora", "inquirer",
    "@inquirer/prompts", "@anthropic-ai/sdk", "openai",
    "@thecorporation/server",
  ],
});
