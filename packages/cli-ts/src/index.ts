import { Command, Option } from "commander";
import { createRequire } from "node:module";
import { inheritOption } from "./command-options.js";

const require = createRequire(import.meta.url);
const pkg = require("../package.json");
const TAX_DOCUMENT_TYPE_CHOICES = [
  "1120",
  "1120s",
  "1065",
  "franchise_tax",
  "annual_report",
  "83b",
  "form_1120",
  "form_1120s",
  "form_1065",
  "1099_nec",
  "form_1099_nec",
  "k1",
  "form_k1",
  "941",
  "form_941",
  "w2",
  "form_w2",
] as const;
const FINALIZE_ITEM_STATUS_CHOICES = [
  "discussed",
  "voted",
  "tabled",
  "withdrawn",
] as const;

const program = new Command();
program
  .name("corp")
  .description("corp — Corporate governance from the terminal")
  .version(pkg.version);

// --- setup ---
program
  .command("setup")
  .description("Interactive setup wizard")
  .action(async () => {
    const { setupCommand } = await import("./commands/setup.js");
    await setupCommand();
  });

// --- status ---
program
  .command("status")
  .description("Workspace summary")
  .option("--json", "Output as JSON")
  .action(async (opts) => {
    const { statusCommand } = await import("./commands/status.js");
    await statusCommand(opts);
  });

program
  .command("context")
  .alias("whoami")
  .description("Show the active workspace, user, and entity context")
  .option("--json", "Output as JSON")
  .action(async (opts) => {
    const { contextCommand } = await import("./commands/context.js");
    await contextCommand(opts);
  });

program
  .command("use <entity-ref>")
  .description("Set the active entity by name, short ID, or reference")
  .action(async (entityRef: string) => {
    const { useCommand } = await import("./commands/use.js");
    await useCommand(entityRef);
  });

program
  .command("schema")
  .description("Dump the CLI command catalog as JSON")
  .option("--compact", "Emit compact JSON")
  .action(async (opts) => {
    const { schemaCommand } = await import("./commands/schema.js");
    schemaCommand(program, opts);
  });

// --- config ---
const configCmd = program.command("config").description("Manage configuration");
configCmd
  .command("set <key> <value>")
  .description("Set a config value (dot-path)")
  .option("--force", "Allow updating a security-sensitive config key")
  .action(async (key: string, value: string, opts: { force?: boolean }) => {
    const { configSetCommand } = await import("./commands/config.js");
    await configSetCommand(key, value, opts);
  });
configCmd
  .command("get <key>")
  .description("Get a config value (dot-path)")
  .action(async (key: string) => {
    const { configGetCommand } = await import("./commands/config.js");
    configGetCommand(key);
  });
configCmd
  .command("list")
  .description("List all config (API keys masked)")
  .action(async () => {
    const { configListCommand } = await import("./commands/config.js");
    configListCommand();
  });

program
  .command("resolve <kind> <ref>")
  .description("Resolve a human-friendly reference to a canonical ID")
  .option("--entity-id <ref>", "Entity reference for entity-scoped resources")
  .option("--body-id <ref>", "Governance body reference for body-scoped resources")
  .option("--meeting-id <ref>", "Meeting reference for meeting-scoped resources")
  .action(async (kind: string, ref: string, opts) => {
    const { resolveCommand } = await import("./commands/resolve.js");
    await resolveCommand(kind, ref, opts);
  });

program
  .command("find <kind> <query>")
  .description("List matching references for a resource kind")
  .option("--entity-id <ref>", "Entity reference for entity-scoped resources")
  .option("--body-id <ref>", "Governance body reference for body-scoped resources")
  .option("--meeting-id <ref>", "Meeting reference for meeting-scoped resources")
  .option("--json", "Output as JSON")
  .action(async (kind: string, query: string, opts) => {
    const { findCommand } = await import("./commands/find.js");
    await findCommand(kind, query, opts);
  });

// --- obligations ---
program
  .command("obligations")
  .description("List obligations with urgency tiers")
  .option("--tier <tier>", "Filter by urgency tier")
  .option("--json", "Output as JSON")
  .action(async (opts) => {
    const { obligationsCommand } = await import("./commands/obligations.js");
    await obligationsCommand(opts);
  });

// --- digest ---
program
  .command("digest")
  .description("View or trigger daily digests")
  .option("--trigger", "Trigger digest now")
  .option("--key <key>", "Get specific digest by key")
  .option("--json", "Output as JSON")
  .action(async (opts) => {
    const { digestCommand } = await import("./commands/digest.js");
    await digestCommand(opts);
  });

// --- link ---
program
  .command("link")
  .description("Link workspace to an external provider")
  .requiredOption("--external-id <id>", "External ID to link")
  .requiredOption("--provider <provider>", "Provider name (e.g. stripe, github)")
  .action(async (opts) => {
    const { linkCommand } = await import("./commands/link.js");
    await linkCommand(opts);
  });

// --- claim ---
program
  .command("claim <code>")
  .description("Redeem a claim code to join a workspace")
  .action(async (code: string) => {
    const { claimCommand } = await import("./commands/claim.js");
    await claimCommand(code);
  });

// --- chat ---
program
  .command("chat")
  .description("Interactive LLM chat session")
  .action(async () => {
    const { chatCommand } = await import("./chat.js");
    await chatCommand();
  });

// --- entities ---
const entitiesCmd = program
  .command("entities")
  .description("List entities, show detail, convert, or dissolve")
  .option("--json", "Output as JSON")
  .action(async (opts) => {
    const { entitiesCommand } = await import("./commands/entities.js");
    await entitiesCommand(opts);
  });
entitiesCmd
  .command("show <entity-ref>")
  .option("--json", "Output as JSON")
  .description("Show entity detail")
  .action(async (entityId: string, opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { entitiesShowCommand } = await import("./commands/entities.js");
    await entitiesShowCommand(entityId, {
      ...opts,
      json: inheritOption(opts.json, parent.json),
    });
  });
entitiesCmd
  .command("convert <entity-ref>")
  .requiredOption("--to <type>", "Target entity type (llc, c_corp)")
  .option("--jurisdiction <jurisdiction>", "New jurisdiction")
  .description("Convert entity to a different type")
  .action(async (entityId: string, opts) => {
    const { entitiesConvertCommand } = await import("./commands/entities.js");
    await entitiesConvertCommand(entityId, opts);
  });
entitiesCmd
  .command("dissolve <entity-ref>")
  .requiredOption("--reason <reason>", "Dissolution reason")
  .option("--effective-date <date>", "Effective date (ISO 8601)")
  .option("--yes, -y", "Skip confirmation prompt")
  .description("Dissolve an entity")
  .action(async (entityId: string, opts) => {
    const { entitiesDissolveCommand } = await import("./commands/entities.js");
    await entitiesDissolveCommand(entityId, opts);
  });

// --- contacts ---
const contactsCmd = program
  .command("contacts")
  .description("Contact management")
  .option("--entity-id <ref>", "Entity reference (ID, short ID, @last, or unique name)")
  .option("--json", "Output as JSON")
  .action(async (opts) => {
    const { contactsListCommand } = await import("./commands/contacts.js");
    await contactsListCommand(opts);
  });
contactsCmd
  .command("show <contact-ref>")
  .option("--json", "Output as JSON")
  .description("Show contact detail/profile")
  .action(async (contactId: string, opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { contactsShowCommand } = await import("./commands/contacts.js");
    await contactsShowCommand(contactId, {
      ...opts,
      entityId: parent.entityId,
      json: inheritOption(opts.json, parent.json),
    });
  });
contactsCmd
  .command("add")
  .requiredOption("--name <name>", "Contact name")
  .requiredOption("--email <email>", "Contact email")
  .option("--type <type>", "Contact type (individual, organization)", "individual")
  .option("--category <category>", "Category (employee, contractor, board_member, investor, law_firm, valuation_firm, accounting_firm, officer, founder, member, other)")
  .option("--cap-table-access <level>", "Cap table access (none, summary, detailed)")
  .option("--address <address>", "Mailing address")
  .option("--mailing-address <address>", "Alias for --address")
  .option("--phone <phone>", "Phone number")
  .option("--notes <notes>", "Notes")
  .option("--json", "Output as JSON")
  .description("Add a new contact")
  .action(async (opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { contactsAddCommand } = await import("./commands/contacts.js");
    await contactsAddCommand({
      ...opts,
      entityId: parent.entityId,
      json: inheritOption(opts.json, parent.json),
    });
  });
contactsCmd
  .command("edit <contact-ref>")
  .option("--name <name>", "Contact name")
  .option("--email <email>", "Contact email")
  .option("--category <category>", "Contact category")
  .option("--cap-table-access <level>", "Cap table access (none, summary, detailed)")
  .option("--address <address>", "Mailing address")
  .option("--mailing-address <address>", "Alias for --address")
  .option("--phone <phone>", "Phone number")
  .option("--notes <notes>", "Notes")
  .option("--json", "Output as JSON")
  .description("Edit an existing contact")
  .action(async (contactId: string, opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { contactsEditCommand } = await import("./commands/contacts.js");
    await contactsEditCommand(contactId, {
      ...opts,
      entityId: parent.entityId,
      json: inheritOption(opts.json, parent.json),
    });
  });

// --- cap-table ---
const capTableCmd = program
  .command("cap-table")
  .description("Cap table, equity grants, SAFEs, transfers, and valuations")
  .option("--entity-id <ref>", "Entity reference (ID, short ID, @last, or unique name)")
  .option("--json", "Output as JSON")
  .action(async (opts) => {
    const { capTableCommand } = await import("./commands/cap-table.js");
    await capTableCommand(opts);
  });
capTableCmd.command("safes")
  .option("--entity-id <ref>", "Entity reference")
  .option("--json", "Output as JSON")
  .description("SAFE notes")
  .action(async (opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { safesCommand } = await import("./commands/cap-table.js");
    await safesCommand({
      entityId: opts.entityId ?? parent.entityId,
      json: inheritOption(opts.json, parent.json),
    });
  });
capTableCmd.command("transfers")
  .option("--entity-id <ref>", "Entity reference")
  .option("--json", "Output as JSON")
  .description("Share transfers")
  .action(async (opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { transfersCommand } = await import("./commands/cap-table.js");
    await transfersCommand({
      entityId: opts.entityId ?? parent.entityId,
      json: inheritOption(opts.json, parent.json),
    });
  });
capTableCmd.command("instruments")
  .option("--entity-id <ref>", "Entity reference")
  .option("--json", "Output as JSON")
  .description("Cap table instruments")
  .action(async (opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { instrumentsCommand } = await import("./commands/cap-table.js");
    await instrumentsCommand({
      entityId: opts.entityId ?? parent.entityId,
      json: inheritOption(opts.json, parent.json),
    });
  });
capTableCmd.command("share-classes")
  .option("--entity-id <ref>", "Entity reference")
  .option("--json", "Output as JSON")
  .description("Share classes")
  .action(async (opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { shareClassesCommand } = await import("./commands/cap-table.js");
    await shareClassesCommand({
      entityId: opts.entityId ?? parent.entityId,
      json: inheritOption(opts.json, parent.json),
    });
  });
capTableCmd.command("rounds")
  .option("--entity-id <ref>", "Entity reference")
  .option("--json", "Output as JSON")
  .description("Staged equity rounds")
  .action(async (opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { roundsCommand } = await import("./commands/cap-table.js");
    await roundsCommand({
      entityId: opts.entityId ?? parent.entityId,
      json: inheritOption(opts.json, parent.json),
    });
  });
capTableCmd.command("valuations")
  .option("--entity-id <ref>", "Entity reference")
  .option("--json", "Output as JSON")
  .description("Valuations history")
  .action(async (opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { valuationsCommand } = await import("./commands/cap-table.js");
    await valuationsCommand({
      entityId: opts.entityId ?? parent.entityId,
      json: inheritOption(opts.json, parent.json),
    });
  });
capTableCmd.command("409a")
  .option("--entity-id <ref>", "Entity reference")
  .option("--json", "Output as JSON")
  .description("Current 409A valuation")
  .action(async (opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { fourOhNineACommand } = await import("./commands/cap-table.js");
    await fourOhNineACommand({
      entityId: opts.entityId ?? parent.entityId,
      json: inheritOption(opts.json, parent.json),
    });
  });
capTableCmd
  .command("create-instrument")
  .requiredOption("--kind <kind>", "Instrument kind (common_equity, preferred_equity, membership_unit, option_grant, safe)")
  .requiredOption("--symbol <symbol>", "Instrument symbol")
  .option("--issuer-legal-entity-id <ref>", "Issuer legal entity reference (ID, short ID, @last, or unique name)")
  .option("--authorized-units <n>", "Authorized units", parseInt)
  .option("--issue-price-cents <n>", "Issue price in cents", parseInt)
  .option("--terms-json <json>", "JSON object of instrument terms")
  .option("--json", "Output as JSON")
  .option("--dry-run", "Show the request without creating the instrument")
  .description("Create a cap table instrument")
  .action(async (opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { createInstrumentCommand } = await import("./commands/cap-table.js");
    await createInstrumentCommand({
      ...opts,
      entityId: parent.entityId,
      json: inheritOption(opts.json, parent.json),
    });
  });
capTableCmd
  .command("issue-equity")
  .requiredOption("--grant-type <type>", "Grant type (common, preferred, membership_unit, stock_option, iso, nso, rsa)")
  .requiredOption("--shares <n>", "Number of shares", parseInt)
  .requiredOption("--recipient <name>", "Recipient name")
  .option("--email <email>", "Recipient email (auto-creates contact if needed)")
  .option("--instrument-id <ref>", "Instrument reference (ID, short ID, symbol, or @last)")
  .option("--meeting-id <ref>", "Board meeting reference required when a board approval already exists or is being recorded")
  .option("--resolution-id <ref>", "Board resolution reference required when issuing under a board-governed entity")
  .option("--json", "Output as JSON")
  .option("--dry-run", "Show the request without creating the round")
  .description("Issue an equity grant (creates a round, adds security, and issues it)")
  .action(async (opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { issueEquityCommand } = await import("./commands/cap-table.js");
    await issueEquityCommand({
      ...opts,
      entityId: parent.entityId,
      json: inheritOption(opts.json, parent.json),
    });
  });
capTableCmd
  .command("issue-safe")
  .requiredOption("--investor <name>", "Investor name")
  .requiredOption("--amount-cents <n>", "Principal amount in cents (e.g. 5000000000 = $50M)", parseInt)
  .option("--amount <n>", "", parseInt)
  .option("--safe-type <type>", "SAFE type", "post_money")
  .requiredOption("--valuation-cap-cents <n>", "Valuation cap in cents (e.g. 1000000000 = $10M)", parseInt)
  .option("--valuation-cap <n>", "", parseInt)
  .option("--meeting-id <ref>", "Board meeting reference required when issuing under a board-governed entity")
  .option("--resolution-id <ref>", "Board resolution reference required when issuing under a board-governed entity")
  .option("--json", "Output as JSON")
  .option("--dry-run", "Show the request without creating the round")
  .description("Issue a SAFE note")
  .action(async (opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { issueSafeCommand } = await import("./commands/cap-table.js");
    await issueSafeCommand({
      ...opts,
      amountCents: opts.amountCents ?? opts.amount,
      valuationCapCents: opts.valuationCapCents ?? opts.valuationCap,
      entityId: parent.entityId,
      json: inheritOption(opts.json, parent.json),
    });
  });
capTableCmd
  .command("transfer")
  .requiredOption("--from <ref>", "Source contact reference (from_contact_id)")
  .requiredOption("--to <ref>", "Destination contact reference (to_contact_id)")
  .requiredOption("--shares <n>", "Number of shares to transfer", parseInt)
  .requiredOption("--share-class-id <ref>", "Share class reference")
  .requiredOption("--governing-doc-type <type>", "Governing doc type (bylaws, operating_agreement, shareholder_agreement, other)")
  .requiredOption("--transferee-rights <rights>", "Transferee rights (full_member, economic_only, limited)")
  .option("--prepare-intent-id <id>", "Prepare intent ID (auto-created if omitted)")
  .option("--type <type>", "Transfer type (gift, trust_transfer, secondary_sale, estate, other)", "secondary_sale")
  .option("--price-per-share-cents <n>", "Price per share in cents", parseInt)
  .option("--relationship <rel>", "Relationship to holder")
  .option("--json", "Output as JSON")
  .option("--dry-run", "Show the request without creating the workflow")
  .description("Create a share transfer workflow")
  .action(async (opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { transferSharesCommand } = await import("./commands/cap-table.js");
    await transferSharesCommand({
      ...opts,
      entityId: parent.entityId,
      json: inheritOption(opts.json, parent.json),
    });
  });
capTableCmd
  .command("distribute")
  .requiredOption("--amount-cents <n>", "Total distribution amount in cents (e.g. 100000 = $1,000.00)", parseInt)
  .option("--amount <n>", "", parseInt)
  .option("--type <type>", "Distribution type (dividend, return, liquidation)", "dividend")
  .requiredOption("--description <desc>", "Distribution description")
  .option("--json", "Output as JSON")
  .option("--dry-run", "Show the request without calculating the distribution")
  .description("Calculate a distribution")
  .action(async (opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { distributeCommand } = await import("./commands/cap-table.js");
    await distributeCommand({
      ...opts,
      amountCents: opts.amountCents ?? opts.amount,
      entityId: parent.entityId,
      json: inheritOption(opts.json, parent.json),
    });
  });

capTableCmd
  .command("start-round")
  .requiredOption("--name <name>", "Round name")
  .requiredOption("--issuer-legal-entity-id <ref>", "Issuer legal entity reference")
  .option("--json", "Output as JSON")
  .option("--dry-run", "Show the request without creating the round")
  .description("Start a staged equity round")
  .action(async (opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { startRoundCommand } = await import("./commands/cap-table.js");
    await startRoundCommand({
      ...opts,
      entityId: parent.entityId,
      json: inheritOption(opts.json, parent.json),
    });
  });
capTableCmd
  .command("add-security")
  .requiredOption("--round-id <ref>", "Round reference")
  .requiredOption("--instrument-id <ref>", "Instrument reference")
  .requiredOption("--quantity <n>", "Number of shares/units", parseInt)
  .requiredOption("--recipient-name <name>", "Recipient display name")
  .option("--holder-id <ref>", "Existing holder reference")
  .option("--email <email>", "Recipient email (to find or create holder)")
  .option("--principal-cents <n>", "Principal amount in cents", parseInt)
  .option("--grant-type <type>", "Grant type")
  .option("--json", "Output as JSON")
  .option("--dry-run", "Show the request without adding the security")
  .description("Add a security to a staged equity round")
  .action(async (opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { addSecurityCommand } = await import("./commands/cap-table.js");
    await addSecurityCommand({
      ...opts,
      entityId: parent.entityId,
      json: inheritOption(opts.json, parent.json),
    });
  });
capTableCmd
  .command("issue-round")
  .option("--meeting-id <ref>", "Board meeting reference required when issuing under a board-governed entity")
  .option("--resolution-id <ref>", "Board resolution reference required when issuing under a board-governed entity")
  .option("--json", "Output as JSON")
  .option("--dry-run", "Show the request without issuing the round")
  .requiredOption("--round-id <ref>", "Round reference")
  .description("Issue all securities and close a staged round")
  .action(async (opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { issueRoundCommand } = await import("./commands/cap-table.js");
    await issueRoundCommand({
      ...opts,
      entityId: parent.entityId,
      json: inheritOption(opts.json, parent.json),
    });
  });
capTableCmd
  .command("create-valuation")
  .requiredOption("--type <type>", "Valuation type (four_oh_nine_a, fair_market_value, etc.)")
  .requiredOption("--date <date>", "Effective date (ISO 8601)")
  .requiredOption("--methodology <method>", "Methodology (income, market, asset, backsolve, hybrid)")
  .option("--fmv <cents>", "FMV per share in cents", parseInt)
  .option("--enterprise-value <cents>", "Enterprise value in cents", parseInt)
  .option("--json", "Output as JSON")
  .option("--dry-run", "Show the request without creating the valuation")
  .description("Create a valuation")
  .action(async (opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { createValuationCommand } = await import("./commands/cap-table.js");
    await createValuationCommand({
      ...opts,
      entityId: parent.entityId,
      json: inheritOption(opts.json, parent.json),
    });
  });
capTableCmd
  .command("submit-valuation <valuation-ref>")
  .option("--json", "Output as JSON")
  .option("--dry-run", "Show the request without submitting the valuation")
  .description("Submit a valuation for board approval")
  .action(async (valuationId: string, opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { submitValuationCommand } = await import("./commands/cap-table.js");
    await submitValuationCommand({
      ...opts,
      valuationId,
      entityId: parent.entityId,
      json: inheritOption(opts.json, parent.json),
    });
  });
capTableCmd
  .command("approve-valuation <valuation-ref>")
  .option("--resolution-id <ref>", "Resolution reference from the board vote")
  .option("--json", "Output as JSON")
  .option("--dry-run", "Show the request without approving the valuation")
  .description("Approve a valuation")
  .action(async (valuationId: string, opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { approveValuationCommand } = await import("./commands/cap-table.js");
    await approveValuationCommand({
      ...opts,
      valuationId,
      entityId: parent.entityId,
      json: inheritOption(opts.json, parent.json),
    });
  });

// --- finance ---
const financeCmd = program
  .command("finance")
  .description("Invoicing, payroll, payments, banking")
  .option("--entity-id <ref>", "Entity reference (ID, short ID, @last, or unique name)")
  .option("--json", "Output as JSON")
  .action(async (opts) => {
    const { financeSummaryCommand } = await import("./commands/finance.js");
    await financeSummaryCommand(opts);
  });
financeCmd
  .command("invoices")
  .option("--json", "Output as JSON")
  .description("List invoices")
  .action(async (opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { financeInvoicesCommand } = await import("./commands/finance.js");
    await financeInvoicesCommand({
      ...opts,
      entityId: parent.entityId,
      json: inheritOption(opts.json, parent.json),
    });
  });
financeCmd
  .command("invoice")
  .requiredOption("--customer <name>", "Customer name")
  .requiredOption("--amount-cents <n>", "Amount in cents (e.g. 500000 = $5,000.00)", parseInt)
  .option("--amount <n>", "", parseInt)
  .requiredOption("--due-date <date>", "Due date (ISO 8601)")
  .option("--description <desc>", "Description", "Services rendered")
  .option("--json", "Output as JSON")
  .description("Create an invoice")
  .action(async (opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { financeInvoiceCommand } = await import("./commands/finance.js");
    await financeInvoiceCommand({
      ...opts,
      amountCents: opts.amountCents ?? opts.amount,
      entityId: parent.entityId,
      json: inheritOption(opts.json, parent.json),
    });
  });
financeCmd
  .command("payroll-runs")
  .option("--json", "Output as JSON")
  .description("List payroll runs")
  .action(async (opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { financePayrollRunsCommand } = await import("./commands/finance.js");
    await financePayrollRunsCommand({
      ...opts,
      entityId: parent.entityId,
      json: inheritOption(opts.json, parent.json),
    });
  });
financeCmd
  .command("payroll")
  .requiredOption("--period-start <date>", "Pay period start")
  .requiredOption("--period-end <date>", "Pay period end")
  .option("--json", "Output as JSON")
  .description("Run payroll")
  .action(async (opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { financePayrollCommand } = await import("./commands/finance.js");
    await financePayrollCommand({
      ...opts,
      entityId: parent.entityId,
      json: inheritOption(opts.json, parent.json),
    });
  });
financeCmd
  .command("payments")
  .option("--json", "Output as JSON")
  .description("List payments")
  .action(async (opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { financePaymentsCommand } = await import("./commands/finance.js");
    await financePaymentsCommand({
      ...opts,
      entityId: parent.entityId,
      json: inheritOption(opts.json, parent.json),
    });
  });
financeCmd
  .command("pay")
  .requiredOption("--amount-cents <n>", "Amount in cents (e.g. 500000 = $5,000.00)", parseInt)
  .option("--amount <n>", "", parseInt)
  .requiredOption("--recipient <name>", "Recipient name")
  .option("--method <method>", "Payment method", "ach")
  .option("--json", "Output as JSON")
  .description("Submit a payment")
  .action(async (opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { financePayCommand } = await import("./commands/finance.js");
    await financePayCommand({
      ...opts,
      amountCents: opts.amountCents ?? opts.amount,
      entityId: parent.entityId,
      json: inheritOption(opts.json, parent.json),
    });
  });
financeCmd
  .command("bank-accounts")
  .option("--json", "Output as JSON")
  .description("List bank accounts")
  .action(async (opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { financeBankAccountsCommand } = await import("./commands/finance.js");
    await financeBankAccountsCommand({
      ...opts,
      entityId: parent.entityId,
      json: inheritOption(opts.json, parent.json),
    });
  });
financeCmd
  .command("open-account")
  .option("--institution <name>", "Banking institution", "Mercury")
  .option("--json", "Output as JSON")
  .description("Open a business bank account")
  .action(async (opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { financeOpenAccountCommand } = await import("./commands/finance.js");
    await financeOpenAccountCommand({
      ...opts,
      entityId: parent.entityId,
      json: inheritOption(opts.json, parent.json),
    });
  });
financeCmd
  .command("activate-account <account-ref>")
  .option("--json", "Output as JSON")
  .description("Activate a bank account (transitions from pending_review to active)")
  .action(async (accountRef: string, opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { financeActivateAccountCommand } = await import("./commands/finance.js");
    await financeActivateAccountCommand(accountRef, {
      ...opts,
      entityId: parent.entityId,
      json: inheritOption(opts.json, parent.json),
    });
  });
financeCmd
  .command("classifications")
  .option("--json", "Output as JSON")
  .description("List contractor classifications")
  .action(async (opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { financeClassificationsCommand } = await import("./commands/finance.js");
    await financeClassificationsCommand({
      ...opts,
      entityId: parent.entityId,
      json: inheritOption(opts.json, parent.json),
    });
  });
financeCmd
  .command("classify-contractor")
  .requiredOption("--name <name>", "Contractor name")
  .requiredOption("--state <code>", "US state code")
  .requiredOption("--hours <n>", "Hours per week", parseInt)
  .option("--exclusive", "Exclusive client", false)
  .requiredOption("--duration <n>", "Duration in months", parseInt)
  .option("--provides-tools", "Company provides tools", false)
  .option("--json", "Output as JSON")
  .description("Analyze contractor classification risk")
  .action(async (opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { financeClassifyContractorCommand } = await import("./commands/finance.js");
    await financeClassifyContractorCommand({
      ...opts,
      entityId: parent.entityId,
      json: inheritOption(opts.json, parent.json),
    });
  });
financeCmd
  .command("reconciliations")
  .option("--json", "Output as JSON")
  .description("List reconciliations")
  .action(async (opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { financeReconciliationsCommand } = await import("./commands/finance.js");
    await financeReconciliationsCommand({
      ...opts,
      entityId: parent.entityId,
      json: inheritOption(opts.json, parent.json),
    });
  });
financeCmd
  .command("reconcile")
  .requiredOption("--start-date <date>", "Period start")
  .requiredOption("--end-date <date>", "Period end")
  .option("--json", "Output as JSON")
  .description("Reconcile ledger")
  .action(async (opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { financeReconcileCommand } = await import("./commands/finance.js");
    await financeReconcileCommand({
      ...opts,
      entityId: parent.entityId,
      json: inheritOption(opts.json, parent.json),
    });
  });
financeCmd
  .command("distributions")
  .option("--json", "Output as JSON")
  .description("List distributions")
  .action(async (opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { financeDistributionsCommand } = await import("./commands/finance.js");
    await financeDistributionsCommand({
      ...opts,
      entityId: parent.entityId,
      json: inheritOption(opts.json, parent.json),
    });
  });

// --- governance ---
const governanceCmd = program
  .command("governance")
  .description("Governance bodies, seats, meetings, resolutions")
  .option("--entity-id <ref>", "Entity reference (overrides active entity)")
  .option("--json", "Output as JSON")
  .action(async (opts) => {
    const { governanceListCommand } = await import("./commands/governance.js");
    await governanceListCommand(opts);
  });
governanceCmd
  .command("create-body")
  .requiredOption("--name <name>", "Body name (e.g. 'Board of Directors')")
  .requiredOption("--body-type <type>", "Body type (board_of_directors, llc_member_vote)")
  .option("--quorum <rule>", "Quorum rule (majority, supermajority, unanimous)", "majority")
  .option("--voting <method>", "Voting method (per_capita, per_unit)", "per_capita")
  .option("--json", "Output as JSON")
  .option("--dry-run", "Show the request without creating the governance body")
  .description("Create a governance body")
  .action(async (opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { governanceCreateBodyCommand } = await import("./commands/governance.js");
    await governanceCreateBodyCommand({
      ...opts,
      entityId: parent.entityId,
      json: inheritOption(opts.json, parent.json),
    });
  });
governanceCmd
  .command("add-seat <body-ref>")
  .requiredOption("--holder <contact-ref>", "Contact reference for the seat holder")
  .option("--role <role>", "Seat role (chair, member, officer, observer)", "member")
  .option("--json", "Output as JSON")
  .option("--dry-run", "Show the request without adding the seat")
  .description("Add a seat to a governance body")
  .action(async (bodyId: string, opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { governanceAddSeatCommand } = await import("./commands/governance.js");
    await governanceAddSeatCommand(bodyId, {
      ...opts,
      entityId: parent.entityId,
      json: inheritOption(opts.json, parent.json),
    });
  });
governanceCmd
  .command("seats <body-ref>")
  .description("Seats for a governance body")
  .action(async (bodyId: string, _opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { governanceSeatsCommand } = await import("./commands/governance.js");
    await governanceSeatsCommand(bodyId, parent);
  });
governanceCmd
  .command("meetings <body-ref>")
  .description("Meetings for a governance body")
  .action(async (bodyId: string, _opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { governanceMeetingsCommand } = await import("./commands/governance.js");
    await governanceMeetingsCommand(bodyId, parent);
  });
governanceCmd
  .command("resolutions <meeting-ref>")
  .description("Resolutions for a meeting")
  .action(async (meetingId: string, _opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { governanceResolutionsCommand } = await import("./commands/governance.js");
    await governanceResolutionsCommand(meetingId, parent);
  });
governanceCmd
  .command("convene")
  .requiredOption("--body <ref>", "Governance body reference")
  .requiredOption("--type <type>", "Meeting type (board_meeting, shareholder_meeting, member_meeting, written_consent)")
  .requiredOption("--title <title>", "Meeting title")
  .option("--date <date>", "Meeting date (ISO 8601)")
  .option("--agenda <item>", "Agenda item (repeatable)", (v: string, a: string[]) => [...a, v], [] as string[])
  .option("--json", "Output as JSON")
  .option("--dry-run", "Show the request without scheduling the meeting")
  .description("Convene a governance meeting")
  .action(async (opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { governanceConveneCommand } = await import("./commands/governance.js");
    await governanceConveneCommand({
      ...opts,
      meetingType: opts.type,
      entityId: parent.entityId,
      json: inheritOption(opts.json, parent.json),
    });
  });
governanceCmd
  .command("open <meeting-ref>")
  .requiredOption("--present-seat <ref>", "Seat reference present at the meeting (repeatable)", (v: string, a?: string[]) => [...(a ?? []), v])
  .option("--json", "Output as JSON")
  .option("--dry-run", "Show the request without opening the meeting")
  .description("Open a scheduled meeting for voting")
  .action(async (meetingId: string, opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { governanceOpenMeetingCommand } = await import("./commands/governance.js");
    await governanceOpenMeetingCommand(meetingId, {
      ...opts,
      entityId: parent.entityId,
      json: inheritOption(opts.json, parent.json),
    });
  });
governanceCmd
  .command("vote <meeting-ref> <item-ref>")
  .requiredOption("--voter <ref>", "Voter contact reference")
  .addOption(new Option("--vote <value>", "Vote (for, against, abstain, recusal)").choices(["for", "against", "abstain", "recusal"]).makeOptionMandatory())
  .option("--json", "Output as JSON")
  .option("--dry-run", "Show the request without casting the vote")
  .description("Cast a vote on an agenda item")
  .action(async (meetingId: string, itemId: string, opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { governanceVoteCommand } = await import("./commands/governance.js");
    await governanceVoteCommand(meetingId, itemId, {
      ...opts,
      entityId: parent.entityId,
      json: inheritOption(opts.json, parent.json),
    });
  });
governanceCmd
  .command("notice <meeting-ref>")
  .option("--json", "Output as JSON")
  .option("--dry-run", "Show the request without sending notices")
  .description("Send meeting notice")
  .action(async (meetingId: string, opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { sendNoticeCommand } = await import("./commands/governance.js");
    await sendNoticeCommand(meetingId, {
      ...opts,
      entityId: parent.entityId,
      json: inheritOption(opts.json, parent.json),
    });
  });
governanceCmd
  .command("adjourn <meeting-ref>")
  .option("--json", "Output as JSON")
  .option("--dry-run", "Show the request without adjourning the meeting")
  .description("Adjourn a meeting")
  .action(async (meetingId: string, opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { adjournMeetingCommand } = await import("./commands/governance.js");
    await adjournMeetingCommand(meetingId, {
      ...opts,
      entityId: parent.entityId,
      json: inheritOption(opts.json, parent.json),
    });
  });
governanceCmd
  .command("reopen <meeting-ref>")
  .option("--json", "Output as JSON")
  .option("--dry-run", "Show the request without re-opening the meeting")
  .description("Re-open an adjourned meeting")
  .action(async (meetingId: string, opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { reopenMeetingCommand } = await import("./commands/governance.js");
    await reopenMeetingCommand(meetingId, {
      ...opts,
      entityId: parent.entityId,
      json: inheritOption(opts.json, parent.json),
    });
  });
governanceCmd
  .command("cancel <meeting-ref>")
  .option("--json", "Output as JSON")
  .option("--dry-run", "Show the request without cancelling the meeting")
  .option("--yes, -y", "Skip confirmation prompt")
  .description("Cancel a meeting")
  .action(async (meetingId: string, opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { cancelMeetingCommand } = await import("./commands/governance.js");
    await cancelMeetingCommand(meetingId, {
      ...opts,
      entityId: parent.entityId,
      json: inheritOption(opts.json, parent.json),
    });
  });
governanceCmd
  .command("agenda-items <meeting-ref>")
  .description("List agenda items for a meeting")
  .action(async (meetingId: string, _opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { listAgendaItemsCommand } = await import("./commands/governance.js");
    await listAgendaItemsCommand(meetingId, { entityId: parent.entityId, json: parent.json });
  });
governanceCmd
  .command("finalize-item <meeting-ref> <item-ref>")
  .addOption(
    new Option(
      "--status <status>",
      `Status (${FINALIZE_ITEM_STATUS_CHOICES.join(", ")})`,
    )
      .choices([...FINALIZE_ITEM_STATUS_CHOICES])
      .makeOptionMandatory(),
  )
  .option("--json", "Output as JSON")
  .option("--dry-run", "Show the request without finalizing the item")
  .description("Finalize an agenda item")
  .action(async (meetingId: string, itemId: string, opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { finalizeAgendaItemCommand } = await import("./commands/governance.js");
    await finalizeAgendaItemCommand(meetingId, itemId, {
      ...opts,
      entityId: parent.entityId,
      json: inheritOption(opts.json, parent.json),
    });
  });
governanceCmd
  .command("resolve <meeting-ref> <item-ref>")
  .requiredOption("--text <resolution_text>", "Resolution text")
  .option("--json", "Output as JSON")
  .option("--dry-run", "Show the request without computing the resolution")
  .description("Compute a resolution for an agenda item")
  .action(async (meetingId: string, itemId: string, opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { computeResolutionCommand } = await import("./commands/governance.js");
    await computeResolutionCommand(meetingId, itemId, {
      ...opts,
      entityId: parent.entityId,
      json: inheritOption(opts.json, parent.json),
    });
  });
governanceCmd
  .command("written-consent")
  .requiredOption("--body <ref>", "Governance body reference")
  .requiredOption("--title <title>", "Title")
  .requiredOption("--description <desc>", "Description")
  .option("--json", "Output as JSON")
  .option("--dry-run", "Show the request without creating the written consent")
  .description("Create a written consent action")
  .action(async (opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { writtenConsentCommand } = await import("./commands/governance.js");
    await writtenConsentCommand({
      ...opts,
      entityId: parent.entityId,
      json: inheritOption(opts.json, parent.json),
    });
  });

// --- documents ---
const documentsCmd = program
  .command("documents")
  .description("Documents and signing")
  .option("--entity-id <ref>", "Entity reference (ID, short ID, @last, or unique name)")
  .option("--json", "Output as JSON")
  .action(async (opts) => {
    const { documentsListCommand } = await import("./commands/documents.js");
    await documentsListCommand(opts);
  });
documentsCmd
  .command("signing-link <doc-ref>")
  .option("--entity-id <ref>", "Entity reference (overrides active entity and parent command)")
  .description("Get a signing link for a document")
  .action(async (docId: string, opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { documentsSigningLinkCommand } = await import("./commands/documents.js");
    await documentsSigningLinkCommand(docId, { entityId: opts.entityId ?? parent.entityId });
  });
documentsCmd
  .command("sign <doc-ref>")
  .option("--entity-id <ref>", "Entity reference (overrides active entity and parent command)")
  .option("--signer-name <name>", "Manual signer name")
  .option("--signer-role <role>", "Manual signer role")
  .option("--signer-email <email>", "Manual signer email")
  .option("--signature-text <text>", "Manual signature text (defaults to signer name)")
  .option("--json", "Output as JSON")
  .description("Sign a formation document, or auto-sign all missing required signatures")
  .action(async (docId: string, opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { documentsSignCommand } = await import("./commands/documents.js");
    await documentsSignCommand(docId, {
      ...opts,
      entityId: opts.entityId ?? parent.entityId,
      json: inheritOption(opts.json, parent.json),
    });
  });
documentsCmd
  .command("sign-all")
  .option("--entity-id <ref>", "Entity reference (overrides active entity and parent command)")
  .option("--json", "Output as JSON")
  .description("Auto-sign all outstanding formation documents for an entity")
  .action(async (opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { documentsSignAllCommand } = await import("./commands/documents.js");
    await documentsSignAllCommand({
      ...opts,
      entityId: opts.entityId ?? parent.entityId,
      json: inheritOption(opts.json, parent.json),
    });
  });
documentsCmd
  .command("generate")
  .requiredOption("--template <type>", "Template type (consulting_agreement, employment_offer, contractor_agreement, nda, custom)")
  .requiredOption("--counterparty <name>", "Counterparty name")
  .option("--effective-date <date>", "Effective date (ISO 8601, defaults to today)")
  .option("--base-salary <amount>", "Employment offer base salary (for employment_offer)")
  .option("--param <key=value>", "Additional template parameter (repeatable)", (value: string, values: string[]) => [...values, value], [] as string[])
  .option("--json", "Output as JSON")
  .description("Generate a contract from a template")
  .action(async (opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { documentsGenerateCommand } = await import("./commands/documents.js");
    await documentsGenerateCommand({
      ...opts,
      entityId: parent.entityId,
      json: inheritOption(opts.json, parent.json),
    });
  });
documentsCmd
  .command("preview-pdf")
  .option("--definition-id <id>", "AST document definition ID (e.g. 'bylaws')")
  .option("--document-id <id>", "Deprecated alias for --definition-id")
  .description("Validate and print the authenticated PDF preview URL for a governance document")
  .action(async (opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { documentsPreviewPdfCommand } = await import("./commands/documents.js");
    await documentsPreviewPdfCommand({
      ...opts,
      documentId: opts.definitionId ?? opts.documentId,
      entityId: parent.entityId,
    });
  });

// --- tax ---
const taxCmd = program
  .command("tax")
  .description("Tax filings and deadline tracking")
  .option("--entity-id <ref>", "Entity reference (ID, short ID, @last, or unique name)")
  .option("--json", "Output as JSON")
  .action(async (opts) => {
    const { taxSummaryCommand } = await import("./commands/tax.js");
    await taxSummaryCommand(opts);
  });
taxCmd
  .command("filings")
  .option("--json", "Output as JSON")
  .description("List tax filings")
  .action(async (opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { taxFilingsCommand } = await import("./commands/tax.js");
    await taxFilingsCommand({
      ...opts,
      entityId: parent.entityId,
      json: inheritOption(opts.json, parent.json),
    });
  });
taxCmd
  .command("file")
  .addOption(new Option("--type <type>", `Document type (${TAX_DOCUMENT_TYPE_CHOICES.join(", ")})`).choices([...TAX_DOCUMENT_TYPE_CHOICES]).makeOptionMandatory())
  .requiredOption("--year <year>", "Tax year", parseInt)
  .option("--json", "Output as JSON")
  .description("File a tax document")
  .action(async (opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { taxFileCommand } = await import("./commands/tax.js");
    await taxFileCommand({
      ...opts,
      entityId: parent.entityId,
      json: inheritOption(opts.json, parent.json),
    });
  });
taxCmd
  .command("deadlines")
  .option("--json", "Output as JSON")
  .description("List tracked deadlines")
  .action(async (opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { taxDeadlinesCommand } = await import("./commands/tax.js");
    await taxDeadlinesCommand({
      ...opts,
      entityId: parent.entityId,
      json: inheritOption(opts.json, parent.json),
    });
  });
taxCmd
  .command("deadline")
  .requiredOption("--type <type>", "Deadline type")
  .requiredOption("--due-date <date>", "Due date (ISO 8601)")
  .requiredOption("--description <desc>", "Description")
  .option("--recurrence <recurrence>", "Recurrence (e.g. annual; 'yearly' is normalized)")
  .option("--json", "Output as JSON")
  .description("Track a compliance deadline")
  .action(async (opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { taxDeadlineCommand } = await import("./commands/tax.js");
    await taxDeadlineCommand({
      ...opts,
      entityId: parent.entityId,
      json: inheritOption(opts.json, parent.json),
    });
  });

// --- agents ---
const agentsCmd = program
  .command("agents")
  .description("Agent management")
  .option("--json", "Output as JSON")
  .action(async (opts) => {
    const { agentsListCommand } = await import("./commands/agents.js");
    await agentsListCommand(opts);
  });
agentsCmd.command("show <agent-ref>").option("--json", "Output as JSON").description("Show agent detail")
  .action(async (agentId: string, opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { agentsShowCommand } = await import("./commands/agents.js");
    await agentsShowCommand(agentId, {
      ...opts,
      json: inheritOption(opts.json, parent.json),
    });
  });
agentsCmd.command("create").requiredOption("--name <name>", "Agent name")
  .requiredOption("--prompt <prompt>", "System prompt").option("--model <model>", "Model")
  .option("--json", "Output as JSON")
  .description("Create a new agent")
  .action(async (opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { agentsCreateCommand } = await import("./commands/agents.js");
    await agentsCreateCommand({
      ...opts,
      json: inheritOption(opts.json, parent.json),
    });
  });
agentsCmd.command("pause <agent-ref>").option("--json", "Output as JSON").description("Pause an agent")
  .action(async (agentId: string, opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { agentsPauseCommand } = await import("./commands/agents.js");
    await agentsPauseCommand(agentId, {
      json: inheritOption(opts.json, parent.json),
    });
  });
agentsCmd.command("resume <agent-ref>").option("--json", "Output as JSON").description("Resume a paused agent")
  .action(async (agentId: string, opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { agentsResumeCommand } = await import("./commands/agents.js");
    await agentsResumeCommand(agentId, {
      json: inheritOption(opts.json, parent.json),
    });
  });
agentsCmd.command("delete <agent-ref>").option("--json", "Output as JSON")
  .option("--yes, -y", "Skip confirmation prompt")
  .description("Delete an agent")
  .action(async (agentId: string, opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { agentsDeleteCommand } = await import("./commands/agents.js");
    await agentsDeleteCommand(agentId, {
      json: inheritOption(opts.json, parent.json),
      yes: opts.yes,
    });
  });
agentsCmd.command("message <agent-ref>").option("--body <text>", "Message text")
  .option("--body-file <path>", "Read the message body from a file")
  .option("--json", "Output as JSON")
  .description("Send a message to an agent")
  .action(async (agentId: string, opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { agentsMessageCommand } = await import("./commands/agents.js");
    await agentsMessageCommand(agentId, {
      ...opts,
      json: inheritOption(opts.json, parent.json),
    });
  });
agentsCmd.command("skill <agent-ref>").requiredOption("--name <name>", "Skill name")
  .requiredOption("--description <desc>", "Skill description").option("--instructions <text>", "Instructions")
  .option("--instructions-file <path>", "Read skill instructions from a file")
  .option("--json", "Output as JSON")
  .description("Add a skill to an agent")
  .action(async (agentId: string, opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { agentsSkillCommand } = await import("./commands/agents.js");
    await agentsSkillCommand(agentId, {
      ...opts,
      json: inheritOption(opts.json, parent.json),
    });
  });

// --- work-items ---
const workItemsCmd = program
  .command("work-items")
  .description("Long-term work item coordination")
  .option("--entity-id <ref>", "Entity reference (ID, short ID, @last, or unique name)")
  .option("--json", "Output as JSON")
  .option("--status <status>", "Filter by status (open, claimed, completed, cancelled)")
  .option("--category <category>", "Filter by category")
  .action(async (opts) => {
    const { workItemsListCommand } = await import("./commands/work-items.js");
    await workItemsListCommand(opts);
  });
workItemsCmd
  .command("show <item-ref>")
  .option("--json", "Output as JSON")
  .description("Show work item detail")
  .action(async (itemId: string, opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { workItemsShowCommand } = await import("./commands/work-items.js");
    await workItemsShowCommand(itemId, {
      ...opts,
      entityId: parent.entityId,
      json: inheritOption(opts.json, parent.json),
    });
  });
workItemsCmd
  .command("create")
  .requiredOption("--title <title>", "Work item title")
  .requiredOption("--category <category>", "Work item category")
  .option("--description <desc>", "Description")
  .option("--deadline <date>", "Deadline (YYYY-MM-DD)")
  .option("--asap", "Mark as ASAP priority")
  .option("--created-by <name>", "Creator identifier")
  .option("--json", "Output as JSON")
  .description("Create a new work item")
  .action(async (opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { workItemsCreateCommand } = await import("./commands/work-items.js");
    await workItemsCreateCommand({
      ...opts,
      category: inheritOption(opts.category, parent.category),
      entityId: parent.entityId,
      json: inheritOption(opts.json, parent.json),
    });
  });
workItemsCmd
  .command("claim <item-ref>")
  .option("--by <name>", "Agent or user claiming the item (required)")
  .option("--claimer <name>", "Alias for --by")
  .option("--ttl <seconds>", "Auto-release TTL in seconds", parseInt)
  .option("--json", "Output as JSON")
  .description("Claim a work item")
  .action(async (itemId: string, opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { workItemsClaimCommand } = await import("./commands/work-items.js");
    const claimedBy = opts.by ?? opts.claimer;
    if (!claimedBy) {
      cmd.error("required option '--by <name>' not specified");
      return;
    }
    await workItemsClaimCommand(itemId, {
      claimedBy,
      ttl: opts.ttl,
      entityId: parent.entityId,
      json: inheritOption(opts.json, parent.json),
    });
  });
workItemsCmd
  .command("complete <item-ref>")
  .option("--by <name>", "Agent or user completing the item (required)")
  .option("--completed-by <name>", "Alias for --by")
  .option("--result <text>", "Completion result or notes")
  .option("--notes <text>", "Alias for --result")
  .option("--json", "Output as JSON")
  .description("Mark a work item as completed")
  .action(async (itemId: string, opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { workItemsCompleteCommand } = await import("./commands/work-items.js");
    const completedBy = opts.by ?? opts.completedBy;
    if (!completedBy) {
      cmd.error("required option '--by <name>' not specified");
      return;
    }
    await workItemsCompleteCommand(itemId, {
      completedBy,
      result: opts.result ?? opts.notes,
      entityId: parent.entityId,
      json: inheritOption(opts.json, parent.json),
    });
  });
workItemsCmd
  .command("release <item-ref>")
  .option("--json", "Output as JSON")
  .description("Release a claimed work item")
  .action(async (itemId: string, opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { workItemsReleaseCommand } = await import("./commands/work-items.js");
    await workItemsReleaseCommand(itemId, {
      entityId: parent.entityId,
      json: inheritOption(opts.json, parent.json),
    });
  });
workItemsCmd
  .command("cancel <item-ref>")
  .option("--json", "Output as JSON")
  .option("--yes, -y", "Skip confirmation prompt")
  .description("Cancel a work item")
  .action(async (itemId: string, opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { workItemsCancelCommand } = await import("./commands/work-items.js");
    await workItemsCancelCommand(itemId, {
      entityId: parent.entityId,
      json: inheritOption(opts.json, parent.json),
      yes: opts.yes,
    });
  });

// --- services ---
const servicesCmd = program
  .command("services")
  .description("Service catalog and fulfillment")
  .option("--entity-id <ref>", "Entity reference (ID, short ID, @last, or unique name)")
  .option("--json", "Output as JSON")
  .action(async (opts) => {
    const { servicesCatalogCommand } = await import("./commands/services.js");
    await servicesCatalogCommand({ json: opts.json });
  });
servicesCmd
  .command("catalog")
  .option("--json", "Output as JSON")
  .description("List the service catalog")
  .action(async (opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { servicesCatalogCommand } = await import("./commands/services.js");
    await servicesCatalogCommand({
      json: inheritOption(opts.json, parent.json),
    });
  });
servicesCmd
  .command("buy <slug>")
  .option("--json", "Output as JSON")
  .option("--dry-run", "Show the request without executing")
  .description("Purchase a service from the catalog")
  .action(async (slug: string, opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { servicesBuyCommand } = await import("./commands/services.js");
    await servicesBuyCommand(slug, {
      ...opts,
      entityId: parent.entityId,
      json: inheritOption(opts.json, parent.json),
    });
  });
servicesCmd
  .command("list")
  .option("--json", "Output as JSON")
  .description("List service requests for an entity")
  .action(async (opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { servicesListCommand } = await import("./commands/services.js");
    await servicesListCommand({
      ...opts,
      entityId: parent.entityId,
      json: inheritOption(opts.json, parent.json),
    });
  });
servicesCmd
  .command("show <ref>")
  .option("--json", "Output as JSON")
  .description("Show service request detail")
  .action(async (ref_: string, opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { servicesShowCommand } = await import("./commands/services.js");
    await servicesShowCommand(ref_, {
      ...opts,
      entityId: parent.entityId,
      json: inheritOption(opts.json, parent.json),
    });
  });
servicesCmd
  .command("fulfill <ref>")
  .option("--note <note>", "Fulfillment note")
  .option("--json", "Output as JSON")
  .description("Mark a service request as fulfilled (operator)")
  .action(async (ref_: string, opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { servicesFulfillCommand } = await import("./commands/services.js");
    await servicesFulfillCommand(ref_, {
      ...opts,
      entityId: parent.entityId,
      json: inheritOption(opts.json, parent.json),
    });
  });
servicesCmd
  .command("cancel <ref>")
  .option("--json", "Output as JSON")
  .description("Cancel a service request")
  .action(async (ref_: string, opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { servicesCancelCommand } = await import("./commands/services.js");
    await servicesCancelCommand(ref_, {
      ...opts,
      entityId: parent.entityId,
      json: inheritOption(opts.json, parent.json),
    });
  });

// --- billing ---
const billingCmd = program
  .command("billing")
  .description("Billing status, plans, and subscription management")
  .option("--json", "Output as JSON")
  .action(async (opts) => {
    const { billingCommand } = await import("./commands/billing.js");
    await billingCommand(opts);
  });
billingCmd.command("portal").description("Open Stripe Customer Portal")
  .action(async () => {
    const { billingPortalCommand } = await import("./commands/billing.js");
    await billingPortalCommand();
  });
billingCmd.command("upgrade").option("--plan <plan>", "Plan ID to upgrade to (free, pro, enterprise)", "pro")
  .description("Open Stripe Checkout to upgrade your plan")
  .action(async (opts) => {
    const { billingUpgradeCommand } = await import("./commands/billing.js");
    await billingUpgradeCommand(opts);
  });

// --- approvals ---
// The approval system is integrated into governance meetings (vote on agenda items)
// and execution intents. There is no standalone /v1/approvals endpoint.
program
  .command("approvals")
  .description("Approvals are managed through governance meetings and execution intents")
  .action(async () => {
    const { approvalsListCommand } = await import("./commands/approvals.js");
    await approvalsListCommand({});
  });

// --- form ---
const formCmd = program
  .command("form")
  .description("Form a new entity with founders and cap table")
  .option("--type <type>", "Entity type (llc, c_corp)")
  .option("--name <name>", "Legal name")
  .option("--jurisdiction <jurisdiction>", "Jurisdiction (e.g. US-DE, US-WY)")
  .option("--member <member>", "Founder as 'name,email,role[,pct[,address[,officer_title[,is_incorporator]]]]' with address as street|city|state|zip, or key=value pairs like 'name=...,email=...,role=...,officer_title=cto,is_incorporator=true,address=street|city|state|zip' (repeatable)", (v: string, a: string[]) => [...a, v], [] as string[])
  .option("--member-json <json>", "Founder JSON object (repeatable)", (v: string, a: string[]) => [...a, v], [] as string[])
  .option("--members-file <path>", "Path to a JSON array of founders or {\"members\": [...]}")
  .option("--address <address>", "Company address as 'street,city,state,zip'")
  .option("--fiscal-year-end <date>", "Fiscal year end (MM-DD)", "12-31")
  .option("--s-corp", "Elect S-Corp status")
  .option("--transfer-restrictions", "Enable transfer restrictions")
  .option("--rofr", "Enable right of first refusal")
  .option("--json", "Output as JSON")
  .option("--dry-run", "Show the request without creating the entity")
  .action(async (opts) => {
    const { formCommand } = await import("./commands/form.js");
    await formCommand(opts);
  });
formCmd.command("create")
  .description("Create a pending entity (staged flow step 1)")
  .requiredOption("--type <type>", "Entity type (llc, c_corp)")
  .requiredOption("--name <name>", "Legal name")
  .option("--jurisdiction <jurisdiction>", "Jurisdiction (e.g. US-DE, US-WY)")
  .option("--registered-agent-name <name>", "Registered agent legal name")
  .option("--registered-agent-address <address>", "Registered agent address line")
  .option("--formation-date <date>", "Formation date (RFC3339 or YYYY-MM-DD)")
  .option("--fiscal-year-end <date>", "Fiscal year end (MM-DD)")
  .option("--s-corp", "Elect S-Corp status")
  .option("--transfer-restrictions", "Enable transfer restrictions")
  .option("--rofr", "Enable right of first refusal")
  .option("--company-address <address>", "Company address as 'street,city,state,zip'")
  .option("--json", "Output as JSON")
  .option("--dry-run", "Show the request without creating the pending entity")
  .action(async (opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { formCreateCommand } = await import("./commands/form.js");
    await formCreateCommand({
      ...opts,
      jurisdiction: inheritOption(opts.jurisdiction, parent.jurisdiction),
      fiscalYearEnd: inheritOption(opts.fiscalYearEnd, parent.fiscalYearEnd),
      sCorp: inheritOption(opts.sCorp, parent.sCorp),
      transferRestrictions: inheritOption(opts.transferRestrictions, parent.transferRestrictions),
      rofr: inheritOption(opts.rofr, parent.rofr),
      json: inheritOption(opts.json, parent.json),
      dryRun: inheritOption(opts.dryRun, parent.dryRun),
    });
  });
formCmd.command("add-founder <entity-ref>")
  .description("Add a founder to a pending entity (staged flow step 2)")
  .requiredOption("--name <name>", "Founder name")
  .requiredOption("--email <email>", "Founder email")
  .requiredOption("--role <role>", "Role: director|officer|manager|member|chair")
  .requiredOption("--pct <pct>", "Ownership percentage")
  .addOption(new Option("--officer-title <title>", "Officer title (corporations only)").choices(["ceo", "cfo", "cto", "coo", "secretary", "treasurer", "president", "vp", "other"]))
  .option("--incorporator", "Mark as sole incorporator (corporations only)")
  .option("--address <address>", "Founder address as 'street,city,state,zip'")
  .option("--json", "Output as JSON")
  .option("--dry-run", "Show the request without adding the founder")
  .action(async (entityId: string, opts, cmd) => {
    const { formAddFounderCommand } = await import("./commands/form.js");
    await formAddFounderCommand(entityId, {
      ...opts,
      json: inheritOption(opts.json, cmd.parent!.opts().json),
      dryRun: inheritOption(opts.dryRun, cmd.parent!.opts().dryRun),
    });
  });
formCmd.command("finalize <entity-ref>")
  .description("Finalize formation and generate documents + cap table (staged flow step 3)")
  .option("--authorized-shares <count>", "Authorized shares for corporations")
  .option("--par-value <value>", "Par value per share, e.g. 0.0001")
  .option("--board-size <count>", "Board size for corporations")
  .option("--principal-name <name>", "Principal or manager name for LLCs")
  .option("--registered-agent-name <name>", "Registered agent legal name")
  .option("--registered-agent-address <address>", "Registered agent address line")
  .option("--formation-date <date>", "Formation date (RFC3339 or YYYY-MM-DD)")
  .option("--fiscal-year-end <date>", "Fiscal year end (MM-DD)")
  .option("--s-corp", "Elect S-Corp status")
  .option("--transfer-restrictions", "Enable transfer restrictions")
  .option("--rofr", "Enable right of first refusal")
  .option("--company-address <address>", "Company address as 'street,city,state,zip'")
  .option("--incorporator-name <name>", "Incorporator legal name (overrides founder)")
  .option("--incorporator-address <address>", "Incorporator mailing address (overrides founder)")
  .option("--json", "Output as JSON")
  .option("--dry-run", "Show the request without finalizing formation")
  .action(async (entityId: string, opts, cmd) => {
    const { formFinalizeCommand } = await import("./commands/form.js");
    await formFinalizeCommand(entityId, {
      ...opts,
      json: inheritOption(opts.json, cmd.parent!.opts().json),
      dryRun: inheritOption(opts.dryRun, cmd.parent!.opts().dryRun),
    });
  });
formCmd.command("activate <entity-ref>")
  .description("Programmatically sign formation documents and advance an entity to active")
  .option("--evidence-uri <uri>", "Registered-agent consent evidence URI placeholder")
  .option("--evidence-type <type>", "Registered-agent consent evidence type", "generated")
  .option("--filing-id <id>", "External filing identifier to record")
  .option("--receipt-reference <ref>", "External receipt reference to record")
  .option("--ein <ein>", "EIN to confirm (defaults to a deterministic simulated EIN)")
  .option("--json", "Output as JSON")
  .option("--dry-run", "Show the activation plan without mutating")
  .action(async (entityId: string, opts, cmd) => {
    const { formActivateCommand } = await import("./commands/form.js");
    await formActivateCommand(entityId, {
      ...opts,
      json: inheritOption(opts.json, cmd.parent!.opts().json),
      dryRun: inheritOption(opts.dryRun, cmd.parent!.opts().dryRun),
    });
  });

// --- api-keys ---
program
  .command("api-keys")
  .description("List API keys")
  .option("--json", "Output as JSON")
  .action(async (opts) => {
    const { apiKeysCommand } = await import("./commands/api-keys.js");
    await apiKeysCommand(opts);
  });

// --- demo ---
program
  .command("demo")
  .description("Create a usable demo workspace environment")
  .requiredOption("--name <name>", "Corporation name")
  .option("--scenario <scenario>", "Scenario to create (startup, llc, restaurant)", "startup")
  .option("--minimal", "Use the minimal server-side demo seed instead of the full CLI workflow")
  .option("--json", "Output as JSON")
  .action(async (opts) => {
    const { demoCommand } = await import("./commands/demo.js");
    await demoCommand(opts);
  });

// --- feedback ---
program
  .command("feedback")
  .description("Submit feedback to TheCorporation")
  .argument("<message>", "Feedback message")
  .option("--category <category>", "Category (e.g. bug, feature, general)", "general")
  .option("--email <email>", "Your email address (to receive a copy)")
  .action(async (message, opts) => {
    const { feedbackCommand } = await import("./commands/feedback.js");
    await feedbackCommand(message, opts);
  });

// --- serve ---
program
  .command("serve")
  .description("Start the API server locally")
  .option("--port <port>", "Port to listen on", "8000")
  .option("--data-dir <path>", "Data directory", "./data/repos")
  .action(async (opts) => {
    const { serveCommand } = await import("./commands/serve.js");
    await serveCommand(opts);
  });

await program.parseAsync(process.argv);
