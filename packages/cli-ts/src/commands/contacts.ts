import { requireConfig } from "../config.js";
import { CorpAPIClient } from "../api-client.js";
import { printContactsTable, printError, printJson, printReferenceSummary, printWriteResult } from "../output.js";
import { ReferenceResolver } from "../references.js";
import chalk from "chalk";
import type { ApiRecord } from "../types.js";

export async function contactsListCommand(opts: { entityId?: string; json?: boolean }): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const eid = await resolver.resolveEntity(opts.entityId);
    const contacts = await client.listContacts(eid);
    await resolver.stabilizeRecords("contact", contacts, eid);
    if (opts.json) printJson(contacts);
    else if (contacts.length === 0) console.log("No contacts found.");
    else printContactsTable(contacts);
  } catch (err) {
    printError(`Failed to fetch contacts: ${err}`);
    process.exit(1);
  }
}

export async function contactsShowCommand(contactId: string, opts: { entityId?: string; json?: boolean; quiet?: boolean }): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const eid = await resolver.resolveEntity(opts.entityId);
    const resolvedContactId = await resolver.resolveContact(eid, contactId);
    const profile = await client.getContactProfile(resolvedContactId, eid);
    const contact = await resolver.stabilizeRecord("contact", (profile.contact ?? profile) as ApiRecord, eid);
    if (opts.quiet) {
      console.log(String(contact.contact_id ?? resolvedContactId));
      return;
    }
    if (opts.json) {
      printJson(profile);
    } else {
      console.log(chalk.cyan("─".repeat(40)));
      console.log(chalk.cyan.bold("  Contact Profile"));
      console.log(chalk.cyan("─".repeat(40)));
      console.log(`  ${chalk.bold("Name:")} ${contact.name ?? "N/A"}`);
      console.log(`  ${chalk.bold("Email:")} ${contact.email ?? "N/A"}`);
      console.log(`  ${chalk.bold("Category:")} ${contact.category ?? "N/A"}`);
      if (contact.contact_type) console.log(`  ${chalk.bold("Type:")} ${contact.contact_type}`);
      if (contact.status) console.log(`  ${chalk.bold("Status:")} ${contact.status}`);
      if (contact.cap_table_access) console.log(`  ${chalk.bold("Cap Table Access:")} ${contact.cap_table_access}`);
      if (contact.created_at) console.log(`  ${chalk.bold("Created:")} ${contact.created_at}`);
      printReferenceSummary("contact", contact, { showReuseHint: true });
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
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const eid = await resolver.resolveEntity(opts.entityId);
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
    await resolver.stabilizeRecord("contact", result, eid);
    resolver.rememberFromRecord("contact", result, eid);
    printWriteResult(
      result,
      `Contact created: ${result.contact_id ?? result.id ?? "OK"}`,
      { jsonOnly: opts.json, referenceKind: "contact", showReuseHint: true },
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
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const eid = await resolver.resolveEntity(opts.entityId);
    const resolvedContactId = await resolver.resolveContact(eid, contactId);
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
    const result = await client.updateContact(resolvedContactId, data);
    resolver.remember("contact", resolvedContactId, eid);
    printWriteResult(result, "Contact updated.", opts.json);
  } catch (err) {
    printError(`Failed to update contact: ${err}`);
    process.exit(1);
  }
}
