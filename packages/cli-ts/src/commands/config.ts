import { configForDisplay, getValue, loadConfig, requireConfig, setValue, updateConfig } from "../config.js";
import { CorpAPIClient } from "../api-client.js";
import { printError, printJson } from "../output.js";
import { ReferenceResolver } from "../references.js";

function looksLikeCanonicalId(value: string): boolean {
  const trimmed = value.trim();
  return /^[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i.test(trimmed)
    || /^ent_[a-z0-9-]+$/i.test(trimmed);
}

export async function configSetCommand(
  key: string,
  value: string,
  options: { force?: boolean } = {},
): Promise<void> {
  let resolvedValue = value;
  try {
    if (key === "active_entity_id" && !looksLikeCanonicalId(value)) {
      const cfg = requireConfig("api_url", "api_key", "workspace_id");
      const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
      const resolver = new ReferenceResolver(client, cfg);
      resolvedValue = await resolver.resolveEntity(value);
    }
    updateConfig((cfg) => {
      setValue(cfg as unknown as Record<string, unknown>, key, resolvedValue, {
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
  if (key === "active_entity_id") {
    console.log(`${key} updated to ${resolvedValue}.`);
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
