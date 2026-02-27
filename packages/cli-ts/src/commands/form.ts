import { input, select } from "@inquirer/prompts";
import { requireConfig } from "../config.js";
import { CorpAPIClient } from "../api-client.js";
import { printError, printSuccess } from "../output.js";
import type { ApiRecord } from "../types.js";

export async function formCommand(opts: {
  type?: string; name?: string; jurisdiction?: string; member?: string[];
}): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    let serverCfg: ApiRecord = {};
    try { serverCfg = await client.getConfig(); } catch { /* ignore */ }

    let entityType = opts.type;
    if (!entityType) {
      const types = (serverCfg.entity_types ?? ["llc", "c_corp", "s_corp"]) as string[];
      entityType = await select({ message: "Entity type", choices: types.map((t) => ({ value: t, name: t })) });
    }

    let name = opts.name;
    if (!name) {
      name = await input({ message: "Entity name" });
    }

    let jurisdiction = opts.jurisdiction;
    if (!jurisdiction) {
      const jurisdictions = (serverCfg.jurisdictions ?? ["DE", "WY", "NV", "CA"]) as string[];
      jurisdiction = await select({
        message: "Jurisdiction",
        choices: jurisdictions.map((j) => ({ value: j, name: j })),
        default: "DE",
      });
    }

    const parsedMembers: ApiRecord[] = [];
    if (opts.member && opts.member.length > 0) {
      for (const m of opts.member) {
        const parts = m.split(",").map((p) => p.trim());
        if (parts.length < 3) {
          printError(`Invalid member format: ${m}. Expected: name,email,role[,ownership_pct]`);
          process.exit(1);
        }
        const member: ApiRecord = { name: parts[0], email: parts[1], role: parts[2] };
        if (parts.length >= 4) member.ownership_pct = parseFloat(parts[3]);
        parsedMembers.push(member);
      }
    } else {
      console.log("Add members (leave name blank to finish):");
      while (true) {
        const mname = await input({ message: "  Member name (blank to finish)", default: "" });
        if (!mname) break;
        const memail = await input({ message: "  Email" });
        const mrole = await input({ message: "  Role", default: "founder" });
        const mpctStr = await input({ message: "  Ownership %", default: "0" });
        parsedMembers.push({
          name: mname, email: memail, role: mrole, ownership_pct: parseFloat(mpctStr),
        });
      }
    }

    for (const m of parsedMembers) {
      if (!m.investor_type) m.investor_type = "natural_person";
    }

    const result = await client.createFormation({
      entity_type: entityType, legal_name: name, jurisdiction, members: parsedMembers,
    });
    printSuccess(`Formation created: ${result.formation_id ?? result.id ?? "OK"}`);
    if (result.entity_id) console.log(`Entity ID: ${result.entity_id}`);
  } catch (err) {
    if (err instanceof Error && err.message.includes("exit")) throw err;
    printError(`Failed to create formation: ${err}`);
    process.exit(1);
  }
}
