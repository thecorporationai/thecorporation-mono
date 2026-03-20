import { loadConfig, requireConfig, resolveEntityId, getActiveEntityId } from "../config.js";
import { CorpAPIClient } from "../api-client.js";
import { printError, printJson, printNextSteps } from "../output.js";
import { withSpinner } from "../spinner.js";
import type { NextStepsResponse, NextStepItem } from "@thecorporation/corp-tools";

interface NextOpts {
  entityId?: string;
  workspace?: boolean;
  json?: boolean;
}

function localChecks(): NextStepItem[] {
  const items: NextStepItem[] = [];
  let cfg;
  try {
    cfg = loadConfig();
  } catch {
    items.push({
      category: "setup",
      title: "Run initial setup",
      description: "No configuration found",
      command: "npx corp setup",
      urgency: "critical",
    });
    return items;
  }

  if (!cfg.api_key) {
    items.push({
      category: "setup",
      title: "Run setup to configure API key",
      description: "No API key configured",
      command: "npx corp setup",
      urgency: "critical",
    });
    return items;
  }

  if (!cfg.workspace_id) {
    items.push({
      category: "setup",
      title: "Claim a workspace",
      description: "No workspace configured",
      command: "npx corp claim <code>",
      urgency: "critical",
    });
    return items;
  }

  if (!getActiveEntityId(cfg)) {
    items.push({
      category: "setup",
      title: "Set an active entity",
      description: "No active entity — set one to get entity-specific recommendations",
      command: "npx corp use <entity-name>",
      urgency: "high",
    });
  }

  return items;
}

export async function nextCommand(opts: NextOpts): Promise<void> {
  if (opts.entityId && opts.workspace) {
    printError("--entity-id and --workspace are mutually exclusive");
    process.exit(1);
  }

  const localItems = localChecks();
  const hasCriticalLocal = localItems.some((i) => i.urgency === "critical");

  if (hasCriticalLocal) {
    const top = localItems[0];
    const backlog = localItems.slice(1);
    const summary = { critical: 0, high: 0, medium: 0, low: 0 };
    for (const item of [top, ...backlog]) {
      const key = item.urgency as keyof typeof summary;
      if (key in summary) summary[key]++;
    }
    const response = { top, backlog, summary };
    if (opts.json) {
      printJson(response);
    } else {
      printNextSteps(response);
    }
    return;
  }

  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);

  try {
    let data: NextStepsResponse;
    if (opts.workspace) {
      data = await withSpinner("Loading", () => client.getWorkspaceNextSteps(), opts.json);
    } else {
      const entityId = resolveEntityId(cfg, opts.entityId);
      data = await withSpinner("Loading", () => client.getEntityNextSteps(entityId), opts.json);
    }

    // Merge non-critical local items into backlog
    if (localItems.length > 0) {
      data.backlog.push(...localItems);
      const all = [data.top, ...data.backlog].filter(Boolean) as NextStepItem[];
      data.summary = { critical: 0, high: 0, medium: 0, low: 0 };
      for (const item of all) {
        const key = item.urgency as keyof typeof data.summary;
        if (key in data.summary) data.summary[key]++;
      }
    }

    if (opts.json) {
      printJson(data);
    } else {
      printNextSteps(data);
    }
  } catch (err) {
    printError(`Failed to fetch next steps: ${err}`);
    process.exit(1);
  }
}
