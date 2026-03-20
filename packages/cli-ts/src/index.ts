import { createRequire } from "node:module";
import { buildCLI } from "./cli.js";
import { registry } from "./registry/index.js";

const require = createRequire(import.meta.url);
const pkg = require("../package.json");

const program = buildCLI(registry, pkg.version);
program.parseAsync(process.argv).catch((err) => {
  console.error(err);
  process.exit(1);
});
