import chalk from "chalk";
import Table from "cli-table3";
import { printError, printSuccess, printWarning, printJson, printWriteResult, printDryRun } from "./output.js";
import type { OutputWriter } from "./registry/types.js";

export function createWriter(): OutputWriter {
  return {
    writeln(text = "") {
      console.log(text);
    },
    json(data) {
      printJson(data);
    },
    table(title, columns, rows) {
      if (rows.length === 0) {
        console.log(`No ${title.toLowerCase()} found.`);
        return;
      }
      console.log(`\n${chalk.bold(title)}`);
      const table = new Table({ head: columns.map((c) => chalk.dim(c)) });
      for (const row of rows) table.push(row.map((cell) => String(cell ?? "")));
      console.log(table.toString());
    },
    panel(title, color, lines) {
      const colorFn = (chalk as unknown as Record<string, typeof chalk.blue>)[color] || chalk.blue;
      const w = 50;
      console.log(colorFn("─".repeat(w)));
      console.log(colorFn.bold(`  ${title}`));
      console.log(colorFn("─".repeat(w)));
      for (const l of lines) console.log(`  ${l}`);
      console.log(colorFn("─".repeat(w)));
    },
    error(msg) {
      printError(msg);
    },
    success(msg) {
      printSuccess(msg);
    },
    warning(msg) {
      printWarning(msg);
    },
    writeResult(result, message, options) {
      printWriteResult(result, message, options ?? {});
    },
    quietId(id) {
      console.log(id);
    },
    dryRun(operation, payload) {
      printDryRun(operation, payload);
    },
  };
}
