import { requireConfig, resolveEntityId } from "../config.js";
import { CorpAPIClient } from "../api-client.js";
import { printContactsTable, printError, printJson, printWriteResult } from "../output.js";
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

export async function contactsShowCommand(contactId: string, opts: { entityId?: string; json?: boolean }): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const eid = resolveEntityId(cfg, opts.entityId);
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    const profile = await client.getContactProfile(contactId, eid);
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
  capTableAccess?: string;
  phone?: string;
  notes?: string;
  mailingAddress?: string;
  address?: string;
  json?: boolean;
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
    if (opts.mailingAddress ?? opts.address) data.mailing_address = opts.mailingAddress ?? opts.address;
    if (opts.capTableAccess) data.cap_table_access = opts.capTableAccess;
    const result = await client.createContact(data);
    printWriteResult(
      result,
      `Contact created: ${result.contact_id ?? result.id ?? "OK"}`,
      opts.json,
    );
  } catch (err) {
    printError(`Failed to create contact: ${err}`);
    process.exit(1);
  }
}

export async function contactsEditCommand(
  contactId: string,
  opts: {
    entityId?: string;
    name?: string;
    email?: string;
    category?: string;
    capTableAccess?: string;
    phone?: string;
    notes?: string;
    mailingAddress?: string;
    address?: string;
    json?: boolean;
  }
): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const eid = resolveEntityId(cfg, opts.entityId);
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    const data: ApiRecord = { entity_id: eid };
    let hasUpdates = false;
    if (opts.name != null) {
      data.name = opts.name;
      hasUpdates = true;
    }
    if (opts.email != null) {
      data.email = opts.email;
      hasUpdates = true;
    }
    if (opts.category != null) {
      data.category = opts.category;
      hasUpdates = true;
    }
    if (opts.phone != null) {
      data.phone = opts.phone;
      hasUpdates = true;
    }
    if (opts.notes != null) {
      data.notes = opts.notes;
      hasUpdates = true;
    }
    if (opts.capTableAccess != null) {
      data.cap_table_access = opts.capTableAccess;
      hasUpdates = true;
    }
    if (opts.mailingAddress != null || opts.address != null) {
      data.mailing_address = opts.mailingAddress ?? opts.address;
      hasUpdates = true;
    }
    if (!hasUpdates) {
      console.log("No fields to update.");
      return;
    }
    const result = await client.updateContact(contactId, data);
    printWriteResult(result, "Contact updated.", opts.json);
  } catch (err) {
    printError(`Failed to update contact: ${err}`);
    process.exit(1);
  }
}
