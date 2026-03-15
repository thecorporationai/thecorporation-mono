import { requireConfig, saveConfig, setActiveEntityId } from "../config.js";
import { CorpAPIClient } from "../api-client.js";
import { printError, printSuccess } from "../output.js";
import { ReferenceResolver, getReferenceAlias } from "../references.js";

export async function useCommand(entityRef: string): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const entityId = await resolver.resolveEntity(entityRef);
    setActiveEntityId(cfg, entityId);
    saveConfig(cfg);
    const alias = getReferenceAlias("entity", { entity_id: entityId }) ?? entityId;
    printSuccess(`Active entity set to ${alias} (${entityId})`);
  } catch (err) {
    printError(`Failed to resolve entity: ${err}`);
    process.exit(1);
  }
}
