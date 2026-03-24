import { defineConfig } from "tsup";

export default defineConfig({
  entry: [
    "src/index.ts",
    "src/client.ts",
    "src/server.ts",
    "src/cli.ts",
    "src/bin.ts",
  ],
  format: ["esm"],
  target: "node20",
  outDir: "dist",
  clean: true,
  dts: true,
  sourcemap: true,
  splitting: false,
});
