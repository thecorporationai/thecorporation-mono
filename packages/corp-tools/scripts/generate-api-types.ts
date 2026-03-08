/**
 * Generate TypeScript types and runtime enum constants from the OpenAPI spec.
 *
 * Produces:
 *   src/api-types.generated.ts   — full path/schema types via openapi-typescript
 *   src/api-enums.generated.ts   — const arrays + type aliases for every enum
 *
 * Usage:  npx tsx scripts/generate-api-types.ts
 */

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import openapiTS, { astToString } from "openapi-typescript";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const SPEC_PATH = path.resolve(__dirname, "../../../services/api-rs/openapi.json");
const OUT_DIR = path.resolve(__dirname, "../src");

// ── 1. Generate full types via openapi-typescript ────────────────────

async function generateFullTypes(): Promise<void> {
  const spec = new URL(`file://${SPEC_PATH}`);
  const ast = await openapiTS(spec, {
    exportType: true,
    enum: false, // keep union types, not TS enums
  });
  const content = astToString(ast);
  const outPath = path.join(OUT_DIR, "api-types.generated.ts");
  fs.writeFileSync(outPath, content, "utf-8");
  console.log(`wrote ${outPath} (${(content.length / 1024).toFixed(1)} KB)`);
}

// ── 2. Generate runtime enum constants ───────────────────────────────

function toPascalCase(s: string): string {
  return s
    .replace(/(?:^|[_-])([a-z])/g, (_, c) => c.toUpperCase())
    .replace(/^[a-z]/, (c) => c.toUpperCase());
}

function generateEnums(): void {
  const spec = JSON.parse(fs.readFileSync(SPEC_PATH, "utf-8"));
  const schemas = spec?.components?.schemas ?? {};

  const lines: string[] = [
    "// AUTO-GENERATED from OpenAPI spec — do not edit",
    "// Regenerate with: npm run generate:types",
    "",
  ];

  const enumNames = new Set<string>();

  for (const [name, schema] of Object.entries<any>(schemas)) {
    if (!schema.enum || !Array.isArray(schema.enum)) continue;
    const pascalName = toPascalCase(name);

    // Avoid duplicate names
    if (enumNames.has(pascalName)) continue;
    enumNames.add(pascalName);

    const values = schema.enum as string[];
    const literal = JSON.stringify(values);
    lines.push(`export const ${pascalName} = ${literal} as const;`);
    lines.push(`export type ${pascalName} = (typeof ${pascalName})[number];`);
    lines.push("");
  }

  const content = lines.join("\n");
  const outPath = path.join(OUT_DIR, "api-enums.generated.ts");
  fs.writeFileSync(outPath, content, "utf-8");
  console.log(`wrote ${outPath} (${enumNames.size} enums)`);
}

// ── Main ─────────────────────────────────────────────────────────────

async function main(): Promise<void> {
  if (!fs.existsSync(SPEC_PATH)) {
    console.error(`OpenAPI spec not found at ${SPEC_PATH}`);
    console.error("Run: cd services/api-rs && cargo run -- dump-open-api 2>/dev/null > openapi.json");
    process.exit(1);
  }

  await generateFullTypes();
  generateEnums();
  console.log("done");
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
