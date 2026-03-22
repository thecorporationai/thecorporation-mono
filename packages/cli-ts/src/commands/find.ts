import chalk from "chalk";
import Table from "cli-table3";
import { requireConfig } from "../config.js";
import { CorpAPIClient } from "../api-client.js";
import { printError, printJson } from "../output.js";
import { ReferenceResolver, type ResourceKind } from "../references.js";
import { KINDS, ENTITY_SCOPED_KINDS } from "../resource-kinds.js";

export async function findCommand(
  kind: string,
  query: string,
  opts: { entityId?: string; bodyId?: string; meetingId?: string; json?: boolean },
): Promise<void> {
  const normalizedKind = kind.trim().toLowerCase() as ResourceKind;
  if (!KINDS.has(normalizedKind)) {
    throw new Error(`Unsupported find kind: ${kind}. Supported: ${[...KINDS].join(", ")}`);
  }

  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);

  try {
    const entityId = ENTITY_SCOPED_KINDS.has(normalizedKind) || opts.entityId || opts.bodyId || opts.meetingId
      ? await resolver.resolveEntity(opts.entityId)
      : undefined;
    const bodyId = entityId && opts.bodyId ? await resolver.resolveBody(entityId, opts.bodyId) : undefined;
    const meetingId = entityId && opts.meetingId
      ? await resolver.resolveMeeting(entityId, opts.meetingId, bodyId)
      : undefined;

    const matches = await resolver.find(normalizedKind, query, { entityId, bodyId, meetingId });
    if (opts.json) {
      printJson({
        kind: normalizedKind,
        query,
        ...(entityId ? { entity_id: entityId } : {}),
        ...(bodyId ? { body_id: bodyId } : {}),
        ...(meetingId ? { meeting_id: meetingId } : {}),
        matches: matches.map((match) => ({
          kind: match.kind,
          id: match.id,
          short_id: match.short_id,
          alias: match.alias,
          label: match.label,
        })),
      });
      return;
    }

    if (matches.length === 0) {
      console.log(`No ${normalizedKind.replaceAll("_", " ")} matches for "${query}".`);
      return;
    }

    const table = new Table({
      head: [
        chalk.dim("Short"),
        chalk.dim("Alias"),
        chalk.dim("Label"),
        chalk.dim("ID"),
      ],
    });
    for (const match of matches) {
      table.push([
        match.short_id,
        match.alias ?? "",
        match.label,
        match.id,
      ]);
    }
    console.log(table.toString());
  } catch (err) {
    throw new Error(`Failed to find references: ${err}`);
  }
}
