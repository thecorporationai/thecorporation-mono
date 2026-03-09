import assert from "node:assert/strict";
import test from "node:test";
import { Command } from "commander";

import { inheritOption } from "../src/command-options.ts";

test("nested --json flags inherit from the parent command", () => {
  const program = new Command();
  let jsonOutput;

  const entities = program.command("entities").option("--json");
  entities
    .command("show <entity-id>")
    .option("--json")
    .action((_entityId, opts, cmd) => {
      jsonOutput = inheritOption(opts.json, cmd.parent.opts().json);
    });

  program.exitOverride();
  program.parse(["node", "cli", "entities", "show", "ent_123", "--json"]);

  assert.equal(jsonOutput, true);
});

test("work-item create inherits --category when commander assigns it to the parent command", () => {
  const program = new Command();
  let category;

  const workItems = program.command("work-items").option("--category <category>");
  workItems
    .command("create")
    .requiredOption("--title <title>")
    .option("--category <category>")
    .action((opts, cmd) => {
      category = inheritOption(opts.category, cmd.parent.opts().category);
    });

  program.exitOverride();
  program.parse([
    "node",
    "cli",
    "work-items",
    "create",
    "--title",
    "Matrix Item",
    "--category",
    "qa",
  ]);

  assert.equal(category, "qa");
});
