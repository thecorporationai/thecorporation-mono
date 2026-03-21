import type { CommandDef, CommandContext } from "./types.js";
import { printEntitiesTable, printContactsTable, printError, printJson, printReferenceSummary, printSuccess, printWriteResult } from "../output.js";
import { withSpinner } from "../spinner.js";
import { confirm } from "@inquirer/prompts";
import chalk from "chalk";
import type { ApiRecord } from "../types.js";

// ── Entity handlers ────────────────────────────────────────────

async function entitiesHandler(ctx: CommandContext): Promise<void> {
  const jsonOutput = !!ctx.opts.json;
  try {
    const entities = await withSpinner("Loading", () => ctx.client.listEntities(), jsonOutput);
    await ctx.resolver.stabilizeRecords("entity", entities);
    if (jsonOutput) {
      printJson(entities);
    } else if (entities.length === 0) {
      console.log("No entities found.");
    } else {
      printEntitiesTable(entities);
    }
  } catch (err) {
    printError(`Failed to fetch entities: ${err}`);
    process.exit(1);
  }
}

async function entitiesShowHandler(ctx: CommandContext): Promise<void> {
  const entityRef = ctx.positional[0];
  const jsonOutput = !!ctx.opts.json;
  try {
    const resolvedEntityId = await ctx.resolver.resolveEntity(entityRef);
    const entities = await ctx.client.listEntities();
    const entity = entities.find((e: ApiRecord) => e.entity_id === resolvedEntityId);
    if (!entity) {
      printError(`Entity not found: ${entityRef}`);
      process.exit(1);
    }
    await ctx.resolver.stabilizeRecord("entity", entity);
    if (jsonOutput) {
      printJson(entity);
    } else {
      console.log(chalk.blue("\u2500".repeat(40)));
      console.log(chalk.blue.bold("  Entity Detail"));
      console.log(chalk.blue("\u2500".repeat(40)));
      console.log(`  ${chalk.bold("Name:")} ${entity.legal_name ?? entity.name ?? "N/A"}`);
      console.log(`  ${chalk.bold("Type:")} ${entity.entity_type ?? "N/A"}`);
      console.log(`  ${chalk.bold("Jurisdiction:")} ${entity.jurisdiction ?? "N/A"}`);
      console.log(`  ${chalk.bold("Status:")} ${entity.formation_status ?? entity.status ?? "N/A"}`);
      console.log(`  ${chalk.bold("State:")} ${entity.formation_state ?? "N/A"}`);
      printReferenceSummary("entity", entity, { showReuseHint: true });
      if (entity.formation_date) console.log(`  ${chalk.bold("Formation Date:")} ${entity.formation_date}`);
      if (entity.ein) console.log(`  ${chalk.bold("EIN:")} ${entity.ein}`);
      console.log(chalk.blue("\u2500".repeat(40)));
    }
  } catch (err) {
    printError(`Failed to fetch entities: ${err}`);
    process.exit(1);
  }
}

async function entitiesConvertHandler(ctx: CommandContext): Promise<void> {
  const entityRef = ctx.positional[0];
  try {
    const resolvedEntityId = await ctx.resolver.resolveEntity(entityRef);
    const data: Record<string, string> = { target_type: ctx.opts.to as string };
    if (ctx.opts.jurisdiction) data.new_jurisdiction = ctx.opts.jurisdiction as string;
    const result = await ctx.client.convertEntity(resolvedEntityId, data);
    printSuccess(`Entity conversion initiated: ${result.conversion_id ?? "OK"}`);
    printJson(result);
  } catch (err) {
    printError(`Failed to convert entity: ${err}`);
    process.exit(1);
  }
}

async function entitiesDissolveHandler(ctx: CommandContext): Promise<void> {
  const entityRef = ctx.positional[0];
  try {
    const resolvedEntityId = await ctx.resolver.resolveEntity(entityRef);
    if (!ctx.opts.yes) {
      const ok = await confirm({
        message: `Dissolve entity ${entityRef}? This cannot be undone.`,
        default: false,
      });
      if (!ok) {
        console.log("Cancelled.");
        return;
      }
    }
    const data: Record<string, string> = { reason: ctx.opts.reason as string };
    if (ctx.opts.effectiveDate) data.effective_date = ctx.opts.effectiveDate as string;
    const result = await ctx.client.dissolveEntity(resolvedEntityId, data);
    printSuccess(`Dissolution initiated: ${result.dissolution_id ?? "OK"}`);
    printJson(result);
  } catch (err) {
    const msg = String(err);
    if (msg.includes("InvalidTransition") || msg.includes("422")) {
      printError(`Cannot dissolve entity: only entities with 'active' status can be dissolved. Check the entity's current status with: corp entities show ${entityRef}`);
    } else {
      printError(`Failed to dissolve entity: ${err}`);
    }
    process.exit(1);
  }
}

// ── Contact handlers ───────────────────────────────────────────

async function contactsHandler(ctx: CommandContext): Promise<void> {
  const jsonOutput = !!ctx.opts.json;
  try {
    const eid = await ctx.resolver.resolveEntity(ctx.entityId);
    const contacts = await ctx.client.listContacts(eid);
    await ctx.resolver.stabilizeRecords("contact", contacts, eid);
    if (jsonOutput) printJson(contacts);
    else if (contacts.length === 0) console.log("No contacts found.");
    else printContactsTable(contacts);
  } catch (err) {
    printError(`Failed to fetch contacts: ${err}`);
    process.exit(1);
  }
}

async function contactsShowHandler(ctx: CommandContext): Promise<void> {
  const contactRef = ctx.positional[0];
  const jsonOutput = !!ctx.opts.json;
  try {
    const eid = await ctx.resolver.resolveEntity(ctx.entityId);
    const resolvedContactId = await ctx.resolver.resolveContact(eid, contactRef);
    const profile = await ctx.client.getContactProfile(resolvedContactId, eid);
    const contact = await ctx.resolver.stabilizeRecord("contact", (profile.contact ?? profile) as ApiRecord, eid);
    if (jsonOutput) {
      printJson(profile);
    } else {
      console.log(chalk.cyan("\u2500".repeat(40)));
      console.log(chalk.cyan.bold("  Contact Profile"));
      console.log(chalk.cyan("\u2500".repeat(40)));
      console.log(`  ${chalk.bold("Name:")} ${contact.name ?? "N/A"}`);
      console.log(`  ${chalk.bold("Email:")} ${contact.email ?? "N/A"}`);
      console.log(`  ${chalk.bold("Category:")} ${contact.category ?? "N/A"}`);
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
      console.log(chalk.cyan("\u2500".repeat(40)));
    }
  } catch (err) {
    printError(`Failed to fetch contact: ${err}`);
    process.exit(1);
  }
}

async function contactsAddHandler(ctx: CommandContext): Promise<void> {
  const jsonOutput = !!ctx.opts.json;
  try {
    const eid = await ctx.resolver.resolveEntity(ctx.entityId);
    const data: ApiRecord = {
      entity_id: eid,
      contact_type: (ctx.opts.type as string) ?? "individual",
      name: ctx.opts.name as string,
      email: ctx.opts.email as string,
      category: (ctx.opts.category as string) ?? "employee",
    };
    if (ctx.opts.phone) data.phone = ctx.opts.phone;
    if (ctx.opts.notes) data.notes = ctx.opts.notes;
    if (ctx.opts.mailingAddress ?? ctx.opts.address) data.mailing_address = ctx.opts.mailingAddress ?? ctx.opts.address;
    if (ctx.opts.capTableAccess) data.cap_table_access = ctx.opts.capTableAccess;
    const result = await ctx.client.createContact(data);
    await ctx.resolver.stabilizeRecord("contact", result, eid);
    ctx.resolver.rememberFromRecord("contact", result, eid);
    printWriteResult(
      result,
      `Contact created: ${result.contact_id ?? result.id ?? "OK"}`,
      { jsonOnly: jsonOutput, referenceKind: "contact", showReuseHint: true },
    );
  } catch (err) {
    printError(`Failed to create contact: ${err}`);
    process.exit(1);
  }
}

async function contactsEditHandler(ctx: CommandContext): Promise<void> {
  const contactRef = ctx.positional[0];
  const jsonOutput = !!ctx.opts.json;
  try {
    const eid = await ctx.resolver.resolveEntity(ctx.entityId);
    const resolvedContactId = await ctx.resolver.resolveContact(eid, contactRef);
    const data: ApiRecord = { entity_id: eid };
    let hasUpdates = false;
    if (ctx.opts.name != null) { data.name = ctx.opts.name; hasUpdates = true; }
    if (ctx.opts.email != null) { data.email = ctx.opts.email; hasUpdates = true; }
    if (ctx.opts.category != null) { data.category = ctx.opts.category; hasUpdates = true; }
    if (ctx.opts.phone != null) { data.phone = ctx.opts.phone; hasUpdates = true; }
    if (ctx.opts.notes != null) { data.notes = ctx.opts.notes; hasUpdates = true; }
    if (ctx.opts.capTableAccess != null) { data.cap_table_access = ctx.opts.capTableAccess; hasUpdates = true; }
    if (ctx.opts.mailingAddress != null || ctx.opts.address != null) {
      data.mailing_address = ctx.opts.mailingAddress ?? ctx.opts.address;
      hasUpdates = true;
    }
    if (!hasUpdates) {
      console.log("No fields to update.");
      return;
    }
    const result = await ctx.client.updateContact(resolvedContactId, data);
    ctx.resolver.remember("contact", resolvedContactId, eid);
    printWriteResult(result, "Contact updated.", jsonOutput);
  } catch (err) {
    printError(`Failed to update contact: ${err}`);
    process.exit(1);
  }
}

// ── Command definitions ────────────────────────────────────────

export const entityCommands: CommandDef[] = [
  // ── entities ──
  {
    name: "entities",
    description: "List all entities",
    handler: entitiesHandler,
    examples: ["corp entities"],
  },
  {
    name: "entities show",
    description: "Show entity detail",
    args: [{ name: "entity-ref", required: true }],
    handler: entitiesShowHandler,
    examples: ["corp entities show"],
  },
  {
    name: "entities convert",
    description: "Convert entity to a different type",
    route: { method: "POST", path: "/v1/entities/{pos}/convert" },
    args: [{ name: "entity-ref", required: true }],
    options: [
      { flags: "--to <type>", description: "Target entity type (llc, c_corp)", required: true },
      { flags: "--jurisdiction <jurisdiction>", description: "New jurisdiction" },
    ],
    handler: entitiesConvertHandler,
    examples: ["corp entities convert <entity-ref> --to 'type'", "corp entities convert --json"],
  },
  {
    name: "entities dissolve",
    description: "Dissolve an entity",
    route: { method: "POST", path: "/v1/entities/{pos}/dissolve" },
    args: [{ name: "entity-ref", required: true }],
    options: [
      { flags: "--reason <reason>", description: "Dissolution reason", required: true },
      { flags: "--effective-date <date>", description: "Effective date (ISO 8601)" },
      { flags: "--yes, -y", description: "Skip confirmation prompt" },
    ],
    handler: entitiesDissolveHandler,
    examples: ["corp entities dissolve <entity-ref> --reason 'reason'", "corp entities dissolve --json"],
  },

  // ── contacts ──
  {
    name: "contacts",
    description: "Contact management",
    route: { method: "GET", path: "/v1/entities/{eid}/contacts" },
    entity: true,
    display: {
      title: "Contacts",
      cols: ["name>Name", "email>Email", "category>Category", "#contact_id>ID"],
    },
    handler: contactsHandler,
    examples: ["corp contacts", "corp contacts --json"],
  },
  {
    name: "contacts show",
    description: "Show contact detail/profile",
    route: { method: "GET", path: "/v1/contacts/{pos}/profile" },
    entity: "query",
    display: { title: "Contact Profile" },
    args: [{ name: "contact-ref", required: true }],
    handler: contactsShowHandler,
    examples: ["corp contacts show", "corp contacts show --json"],
  },
  {
    name: "contacts add",
    description: "Add a new contact",
    route: { method: "POST", path: "/v1/contacts" },
    entity: true,
    options: [
      { flags: "--name <name>", description: "Contact name", required: true },
      { flags: "--email <email>", description: "Contact email", required: true },
      { flags: "--type <type>", description: "Contact type (individual, organization)", default: "individual" },
      { flags: "--category <category>", description: "Category (employee, contractor, board_member, investor, law_firm, valuation_firm, accounting_firm, officer, founder, member, other)" },
      { flags: "--cap-table-access <level>", description: "Cap table access (none, summary, detailed)" },
      { flags: "--address <address>", description: "Mailing address" },
      { flags: "--mailing-address <address>", description: "Alias for --address" },
      { flags: "--phone <phone>", description: "Phone number" },
      { flags: "--notes <notes>", description: "Additional notes" },
    ],
    handler: contactsAddHandler,
    produces: { kind: "contact" },
    successTemplate: "Contact created: {name}",
    examples: ["corp contacts add --name 'name' --email 'email'", "corp contacts add --json"],
  },
  {
    name: "contacts edit",
    description: "Edit an existing contact",
    route: { method: "PATCH", path: "/v1/contacts/{pos}" },
    entity: true,
    args: [{ name: "contact-ref", required: true }],
    options: [
      { flags: "--name <name>", description: "Contact name" },
      { flags: "--email <email>", description: "Contact email" },
      { flags: "--category <category>", description: "Contact category" },
      { flags: "--cap-table-access <level>", description: "Cap table access (none, summary, detailed)" },
      { flags: "--address <address>", description: "Mailing address" },
      { flags: "--mailing-address <address>", description: "Alias for --address" },
      { flags: "--phone <phone>", description: "Phone number" },
      { flags: "--notes <notes>", description: "Additional notes" },
    ],
    handler: contactsEditHandler,
    examples: ["corp contacts edit <contact-ref>", "corp contacts edit --json"],
  },

  // ── Auto-generated from OpenAPI ──────────────────────────────
  {
    name: "contacts notification-prefs",
    description: "View notification preferences for a contact",
    route: { method: "GET", path: "/v1/contacts/{pos}/notification-prefs" },
    args: [{ name: "contact-id", required: true, description: "Contact ID" }],
    display: { title: "Contacts Notification Prefs", cols: ["#contact_id>ID", "email_enabled>Email Enabled", "sms_enabled>Sms Enabled", "@updated_at>Updated At", "webhook_enabled>Webhook Enabled"] },
    examples: ["corp contacts notification-prefs"],
  },
  {
    name: "contacts update-notification-prefs",
    description: "View notification preferences for a contact",
    route: { method: "PATCH", path: "/v1/contacts/{pos}/notification-prefs" },
    args: [{ name: "contact-id", required: true, description: "Contact ID" }],
    options: [
      { flags: "--email-enabled <email-enabled>", description: "Email Enabled" },
      { flags: "--sms-enabled <sms-enabled>", description: "Sms Enabled" },
      { flags: "--webhook-enabled <webhook-enabled>", description: "Webhook Enabled" },
    ],
    examples: ["corp contacts notification-prefs <contact-id>", "corp contacts notification-prefs --json"],
    successTemplate: "Notification Prefs updated",
  },

];