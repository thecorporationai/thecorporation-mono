import { requireConfig, resolveEntityId } from "../config.js";
import { CorpAPIClient } from "../api-client.js";
import { printContactsTable, printError, printSuccess, printJson } from "../output.js";
import chalk from "chalk";
import type { ApiRecord } from "../types.js";

export async function contactsListCommand(opts: { entityId?: string; json?: boolean }): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const eid = resolveEntityId(cfg, opts.entityId);
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    const contacts = await client.listContacts(eid);
    if (opts.json) printJson(contacts);
    else if (contacts.length === 0) console.log("No contacts found.");
    else printContactsTable(contacts);
  } catch (err) {
    printError(`Failed to fetch contacts: ${err}`);
    process.exit(1);
  }
}

export async function contactsShowCommand(contactId: string, opts: { json?: boolean }): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    const profile = await client.getContactProfile(contactId);
    if (opts.json) {
      printJson(profile);
    } else {
      const contact = (profile.contact ?? profile) as ApiRecord;
      console.log(chalk.cyan("─".repeat(40)));
      console.log(chalk.cyan.bold("  Contact Profile"));
      console.log(chalk.cyan("─".repeat(40)));
      console.log(`  ${chalk.bold("Name:")} ${contact.name ?? "N/A"}`);
      console.log(`  ${chalk.bold("Email:")} ${contact.email ?? "N/A"}`);
      console.log(`  ${chalk.bold("Category:")} ${contact.category ?? "N/A"}`);
      if (contact.phone) console.log(`  ${chalk.bold("Phone:")} ${contact.phone}`);
      if (contact.notes) console.log(`  ${chalk.bold("Notes:")} ${contact.notes}`);
      const holdings = profile.equity_holdings as ApiRecord[] | undefined;
      if (holdings?.length) {
        console.log(`\n  ${chalk.bold("Equity Holdings:")}`);
        for (const h of holdings) console.log(`    ${h.share_class ?? "?"}: ${h.shares ?? "?"} shares`);
      }
      const obls = profile.obligations as unknown[];
      if (obls?.length) console.log(`\n  ${chalk.bold("Obligations:")} ${obls.length}`);
      console.log(chalk.cyan("─".repeat(40)));
    }
  } catch (err) {
    printError(`Failed to fetch contact: ${err}`);
    process.exit(1);
  }
}

export async function contactsAddCommand(opts: {
  entityId?: string;
  name: string;
  email: string;
  type?: string;
  category?: string;
  phone?: string;
  notes?: string;
}): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const eid = resolveEntityId(cfg, opts.entityId);
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    const data: ApiRecord = {
      entity_id: eid,
      contact_type: opts.type ?? "individual",
      name: opts.name,
      email: opts.email,
      category: opts.category ?? "employee",
    };
    if (opts.phone) data.phone = opts.phone;
    if (opts.notes) data.notes = opts.notes;
    const result = await client.createContact(data);
    printSuccess(`Contact created: ${result.contact_id ?? result.id ?? "OK"}`);
  } catch (err) {
    printError(`Failed to create contact: ${err}`);
    process.exit(1);
  }
}

export async function contactsEditCommand(
  contactId: string,
  opts: { name?: string; email?: string; category?: string; phone?: string; notes?: string }
): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    const data: ApiRecord = {};
    if (opts.name != null) data.name = opts.name;
    if (opts.email != null) data.email = opts.email;
    if (opts.category != null) data.category = opts.category;
    if (opts.phone != null) data.phone = opts.phone;
    if (opts.notes != null) data.notes = opts.notes;
    if (Object.keys(data).length === 0) {
      console.log("No fields to update.");
      return;
    }
    await client.updateContact(contactId, data);
    printSuccess("Contact updated.");
  } catch (err) {
    printError(`Failed to update contact: ${err}`);
    process.exit(1);
  }
}
