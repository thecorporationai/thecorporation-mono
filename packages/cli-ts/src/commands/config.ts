import { configForDisplay, getValue, loadConfig, setValue, updateConfig } from "../config.js";
import { printError, printJson } from "../output.js";

export function configSetCommand(
  key: string,
  value: string,
  options: { force?: boolean } = {},
): void {
  try {
    updateConfig((cfg) => {
      setValue(cfg as unknown as Record<string, unknown>, key, value, {
        forceSensitive: options.force,
      });
    });
  } catch (err) {
    printError(`Failed to update config: ${err}`);
    process.exit(1);
  }

  if (key === "api_key" || key === "llm.api_key") {
    console.log(`${key} updated.`);
    return;
  }
  console.log(`${key} updated.`);
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
