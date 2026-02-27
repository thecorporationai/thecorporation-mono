import { configForDisplay, getValue, loadConfig, saveConfig, setValue } from "../config.js";
import { printError, printJson } from "../output.js";

export function configSetCommand(key: string, value: string): void {
  const cfg = loadConfig();
  setValue(cfg as unknown as Record<string, unknown>, key, value);
  saveConfig(cfg);
  console.log(`${key} = ${value}`);
}

export function configGetCommand(key: string): void {
  const cfg = loadConfig();
  const val = getValue(cfg as unknown as Record<string, unknown>, key);
  if (val === undefined) {
    printError(`Key not found: ${key}`);
    process.exit(1);
  }
  if (typeof val === "object" && val !== null) {
    printJson(val);
  } else {
    console.log(String(val));
  }
}

export function configListCommand(): void {
  const cfg = loadConfig();
  printJson(configForDisplay(cfg));
}
