import {
  TOOL_DEFINITIONS as _TOOL_DEFINITIONS,
  isWriteTool as _isWriteTool,
  executeTool as _executeTool,
} from "@thecorporation/corp-tools";
import type { CorpAPIClient } from "@thecorporation/corp-tools";
import { loadConfig, saveConfig } from "./config.js";
import { join } from "node:path";
import { homedir } from "node:os";

export const TOOL_DEFINITIONS = _TOOL_DEFINITIONS;
export const isWriteTool: (name: string, args?: Record<string, unknown>) => boolean = _isWriteTool;

export async function executeTool(
  name: string,
  args: Record<string, unknown>,
  client: CorpAPIClient,
): Promise<string> {
  return _executeTool(name, args, client, {
    dataDir: join(homedir(), ".corp"),
    onEntityFormed: (entityId) => {
      try {
        const cfg = loadConfig();
        cfg.active_entity_id = entityId;
        saveConfig(cfg);
      } catch { /* ignore */ }
    },
  });
}
