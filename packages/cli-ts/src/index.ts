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
  .action(async () => {
    const { statusCommand } = await import("./commands/status.js");
    await statusCommand();
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
  .action(async (key: string, value: string) => {
    const { configSetCommand } = await import("./commands/config.js");
    configSetCommand(key, value);
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
  .command("show <entity-id>")
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
  .command("convert <entity-id>")
  .requiredOption("--to <type>", "Target entity type (llc, c_corp)")
  .option("--jurisdiction <jurisdiction>", "New jurisdiction")
  .description("Convert entity to a different type")
  .action(async (entityId: string, opts) => {
    const { entitiesConvertCommand } = await import("./commands/entities.js");
    await entitiesConvertCommand(entityId, opts);
  });
entitiesCmd
  .command("dissolve <entity-id>")
  .requiredOption("--reason <reason>", "Dissolution reason")
  .option("--effective-date <date>", "Effective date (ISO 8601)")
  .description("Dissolve an entity")
  .action(async (entityId: string, opts) => {
    const { entitiesDissolveCommand } = await import("./commands/entities.js");
    await entitiesDissolveCommand(entityId, opts);
  });

// --- contacts ---
const contactsCmd = program
  .command("contacts")
  .description("Contact management")
  .option("--entity-id <id>", "Entity ID (overrides active entity)")
  .option("--json", "Output as JSON")
  .action(async (opts) => {
    const { contactsListCommand } = await import("./commands/contacts.js");
    await contactsListCommand(opts);
  });
contactsCmd
  .command("show <contact-id>")
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
  .command("edit <contact-id>")
  .option("--name <name>", "Contact name")
  .option("--email <email>", "Contact email")
  .option("--category <category>", "Contact category")
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
  .option("--entity-id <id>", "Entity ID (overrides active entity)")
  .option("--json", "Output as JSON")
  .action(async (opts) => {
    const { capTableCommand } = await import("./commands/cap-table.js");
    await capTableCommand(opts);
  });
capTableCmd.command("safes").description("SAFE notes").action(async (_opts, cmd) => {
  const parent = cmd.parent!.opts();
  const { safesCommand } = await import("./commands/cap-table.js");
  await safesCommand(parent);
});
capTableCmd.command("transfers").description("Share transfers").action(async (_opts, cmd) => {
  const parent = cmd.parent!.opts();
  const { transfersCommand } = await import("./commands/cap-table.js");
  await transfersCommand(parent);
});
capTableCmd.command("valuations").description("Valuations history").action(async (_opts, cmd) => {
  const parent = cmd.parent!.opts();
  const { valuationsCommand } = await import("./commands/cap-table.js");
  await valuationsCommand(parent);
});
capTableCmd.command("409a").description("Current 409A valuation").action(async (_opts, cmd) => {
  const parent = cmd.parent!.opts();
  const { fourOhNineACommand } = await import("./commands/cap-table.js");
  await fourOhNineACommand(parent);
});
capTableCmd
  .command("issue-equity")
  .requiredOption("--grant-type <type>", "Grant type (common, preferred, membership_unit, stock_option, iso, nso, rsa)")
  .requiredOption("--shares <n>", "Number of shares", parseInt)
  .requiredOption("--recipient <name>", "Recipient name")
  .option("--email <email>", "Recipient email (auto-creates contact if needed)")
  .option("--instrument-id <id>", "Instrument ID (auto-detected from cap table if omitted)")
  .option("--meeting-id <id>", "Board meeting ID required when a board approval already exists or is being recorded")
  .option("--resolution-id <id>", "Board resolution ID required when issuing under a board-governed entity")
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
  .requiredOption("--amount <n>", "Principal amount in cents", parseInt)
  .option("--safe-type <type>", "SAFE type", "post_money")
  .requiredOption("--valuation-cap <n>", "Valuation cap in cents", parseInt)
  .option("--meeting-id <id>", "Board meeting ID required when issuing under a board-governed entity")
  .option("--resolution-id <id>", "Board resolution ID required when issuing under a board-governed entity")
  .option("--json", "Output as JSON")
  .option("--dry-run", "Show the request without creating the round")
  .description("Issue a SAFE note")
  .action(async (opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { issueSafeCommand } = await import("./commands/cap-table.js");
    await issueSafeCommand({
      ...opts,
      entityId: parent.entityId,
      json: inheritOption(opts.json, parent.json),
    });
  });
capTableCmd
  .command("transfer")
  .requiredOption("--from <id>", "Source contact ID (from_contact_id)")
  .requiredOption("--to <id>", "Destination contact ID (to_contact_id)")
  .requiredOption("--shares <n>", "Number of shares to transfer", parseInt)
  .requiredOption("--share-class-id <id>", "Share class ID")
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
  .requiredOption("--amount <n>", "Total distribution amount in cents", parseInt)
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
      entityId: parent.entityId,
      json: inheritOption(opts.json, parent.json),
    });
  });

capTableCmd
  .command("start-round")
  .requiredOption("--name <name>", "Round name")
  .requiredOption("--issuer-legal-entity-id <id>", "Issuer legal entity ID")
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
  .requiredOption("--round-id <id>", "Round ID")
  .requiredOption("--instrument-id <id>", "Instrument ID")
  .requiredOption("--quantity <n>", "Number of shares/units", parseInt)
  .requiredOption("--recipient-name <name>", "Recipient display name")
  .option("--holder-id <id>", "Existing holder ID")
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
  .option("--json", "Output as JSON")
  .option("--dry-run", "Show the request without issuing the round")
  .requiredOption("--round-id <id>", "Round ID")
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
  .command("submit-valuation <valuation-id>")
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
  .command("approve-valuation <valuation-id>")
  .option("--resolution-id <id>", "Resolution ID from the board vote")
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
  .option("--entity-id <id>", "Entity ID (overrides active entity)");
financeCmd
  .command("invoice")
  .requiredOption("--customer <name>", "Customer name")
  .requiredOption("--amount <n>", "Amount in cents", parseInt)
  .requiredOption("--due-date <date>", "Due date (ISO 8601)")
  .option("--description <desc>", "Description", "Services rendered")
  .description("Create an invoice")
  .action(async (opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { financeInvoiceCommand } = await import("./commands/finance.js");
    await financeInvoiceCommand({ ...opts, entityId: parent.entityId });
  });
financeCmd
  .command("payroll")
  .requiredOption("--period-start <date>", "Pay period start")
  .requiredOption("--period-end <date>", "Pay period end")
  .description("Run payroll")
  .action(async (opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { financePayrollCommand } = await import("./commands/finance.js");
    await financePayrollCommand({ ...opts, entityId: parent.entityId });
  });
financeCmd
  .command("pay")
  .requiredOption("--amount <n>", "Amount in cents", parseInt)
  .requiredOption("--recipient <name>", "Recipient name")
  .option("--method <method>", "Payment method", "ach")
  .description("Submit a payment")
  .action(async (opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { financePayCommand } = await import("./commands/finance.js");
    await financePayCommand({ ...opts, entityId: parent.entityId });
  });
financeCmd
  .command("open-account")
  .option("--institution <name>", "Banking institution", "Mercury")
  .description("Open a business bank account")
  .action(async (opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { financeOpenAccountCommand } = await import("./commands/finance.js");
    await financeOpenAccountCommand({ ...opts, entityId: parent.entityId });
  });
financeCmd
  .command("classify-contractor")
  .requiredOption("--name <name>", "Contractor name")
  .requiredOption("--state <code>", "US state code")
  .requiredOption("--hours <n>", "Hours per week", parseInt)
  .option("--exclusive", "Exclusive client", false)
  .requiredOption("--duration <n>", "Duration in months", parseInt)
  .option("--provides-tools", "Company provides tools", false)
  .description("Analyze contractor classification risk")
  .action(async (opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { financeClassifyContractorCommand } = await import("./commands/finance.js");
    await financeClassifyContractorCommand({ ...opts, entityId: parent.entityId });
  });
financeCmd
  .command("reconcile")
  .requiredOption("--start-date <date>", "Period start")
  .requiredOption("--end-date <date>", "Period end")
  .description("Reconcile ledger")
  .action(async (opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { financeReconcileCommand } = await import("./commands/finance.js");
    await financeReconcileCommand({ ...opts, entityId: parent.entityId });
  });

// --- governance ---
const governanceCmd = program
  .command("governance")
  .description("Governance bodies, seats, meetings, resolutions")
  .option("--entity-id <id>", "Entity ID (overrides active entity)")
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
  .command("add-seat <body-id>")
  .requiredOption("--holder <contact-id>", "Contact ID for the seat holder")
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
  .command("seats <body-id>")
  .description("Seats for a governance body")
  .action(async (bodyId: string, _opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { governanceSeatsCommand } = await import("./commands/governance.js");
    await governanceSeatsCommand(bodyId, parent);
  });
governanceCmd
  .command("meetings <body-id>")
  .description("Meetings for a governance body")
  .action(async (bodyId: string, _opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { governanceMeetingsCommand } = await import("./commands/governance.js");
    await governanceMeetingsCommand(bodyId, parent);
  });
governanceCmd
  .command("resolutions <meeting-id>")
  .description("Resolutions for a meeting")
  .action(async (meetingId: string, _opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { governanceResolutionsCommand } = await import("./commands/governance.js");
    await governanceResolutionsCommand(meetingId, parent);
  });
governanceCmd
  .command("convene")
  .requiredOption("--body <id>", "Governance body ID")
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
  .command("open <meeting-id>")
  .requiredOption("--present-seat <id>", "Seat ID present at the meeting (repeatable)", (v: string, a?: string[]) => [...(a ?? []), v])
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
  .command("vote <meeting-id> <item-id>")
  .requiredOption("--voter <id>", "Voter contact UUID")
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
  .command("notice <meeting-id>")
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
  .command("adjourn <meeting-id>")
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
  .command("cancel <meeting-id>")
  .option("--json", "Output as JSON")
  .option("--dry-run", "Show the request without cancelling the meeting")
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
  .command("agenda-items <meeting-id>")
  .description("List agenda items for a meeting")
  .action(async (meetingId: string, _opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { listAgendaItemsCommand } = await import("./commands/governance.js");
    await listAgendaItemsCommand(meetingId, { entityId: parent.entityId, json: parent.json });
  });
governanceCmd
  .command("finalize-item <meeting-id> <item-id>")
  .requiredOption("--status <status>", "Status: voted, discussed, tabled, withdrawn")
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
  .command("resolve <meeting-id> <item-id>")
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
  .requiredOption("--body <id>", "Governance body ID")
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
  .option("--entity-id <id>", "Entity ID (overrides active entity)")
  .option("--json", "Output as JSON")
  .action(async (opts) => {
    const { documentsListCommand } = await import("./commands/documents.js");
    await documentsListCommand(opts);
  });
documentsCmd
  .command("signing-link <doc-id>")
  .option("--entity-id <id>", "Entity ID (overrides active entity and parent command)")
  .description("Get a signing link for a document")
  .action(async (docId: string, opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { documentsSigningLinkCommand } = await import("./commands/documents.js");
    await documentsSigningLinkCommand(docId, { entityId: opts.entityId ?? parent.entityId });
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
  .requiredOption("--definition-id <id>", "AST document definition ID (e.g. 'bylaws')")
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
  .option("--entity-id <id>", "Entity ID (overrides active entity)");
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
agentsCmd.command("show <agent-id>").option("--json", "Output as JSON").description("Show agent detail")
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
agentsCmd.command("pause <agent-id>").option("--json", "Output as JSON").description("Pause an agent")
  .action(async (agentId: string, opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { agentsPauseCommand } = await import("./commands/agents.js");
    await agentsPauseCommand(agentId, {
      json: inheritOption(opts.json, parent.json),
    });
  });
agentsCmd.command("resume <agent-id>").option("--json", "Output as JSON").description("Resume a paused agent")
  .action(async (agentId: string, opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { agentsResumeCommand } = await import("./commands/agents.js");
    await agentsResumeCommand(agentId, {
      json: inheritOption(opts.json, parent.json),
    });
  });
agentsCmd.command("delete <agent-id>").option("--json", "Output as JSON").description("Delete an agent")
  .action(async (agentId: string, opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { agentsDeleteCommand } = await import("./commands/agents.js");
    await agentsDeleteCommand(agentId, {
      json: inheritOption(opts.json, parent.json),
    });
  });
agentsCmd.command("message <agent-id>").option("--body <text>", "Message text")
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
agentsCmd.command("skill <agent-id>").requiredOption("--name <name>", "Skill name")
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
  .option("--entity-id <id>", "Entity ID (overrides active entity)")
  .option("--json", "Output as JSON")
  .option("--status <status>", "Filter by status (open, claimed, completed, cancelled)")
  .option("--category <category>", "Filter by category")
  .action(async (opts) => {
    const { workItemsListCommand } = await import("./commands/work-items.js");
    await workItemsListCommand(opts);
  });
workItemsCmd
  .command("show <item-id>")
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
  .option("--category <category>", "Work item category")
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
  .command("claim <item-id>")
  .option("--by <name>", "Agent or user claiming the item")
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
  .command("complete <item-id>")
  .option("--by <name>", "Agent or user completing the item")
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
  .command("release <item-id>")
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
  .command("cancel <item-id>")
  .option("--json", "Output as JSON")
  .description("Cancel a work item")
  .action(async (itemId: string, opts, cmd) => {
    const parent = cmd.parent!.opts();
    const { workItemsCancelCommand } = await import("./commands/work-items.js");
    await workItemsCancelCommand(itemId, {
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
    const { printError } = await import("./output.js");
    printError(
      "Approvals are managed through governance meetings.\n" +
      "  Use: corp governance convene ... to schedule a board meeting\n" +
      "  Use: corp governance vote <meeting-id> <item-id> ... to cast votes"
    );
  });

// --- form ---
const formCmd = program
  .command("form")
  .description("Form a new entity with founders and cap table")
  .option("--entity-type <type>", "Entity type (llc, c_corp)")
  .option("--legal-name <name>", "Legal name")
  .option("--jurisdiction <jurisdiction>", "Jurisdiction (e.g. US-DE, US-WY)")
  .option("--member <member>", "Founder as 'name,email,role[,pct]' or key=value pairs like 'name=...,email=...,role=...,officer_title=cto,is_incorporator=true,address=street|city|state|zip' (repeatable)", (v: string, a: string[]) => [...a, v], [] as string[])
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
    // Map --entity-type and --legal-name to the internal keys expected by formCommand
    if (opts.entityType && !opts.type) opts.type = opts.entityType;
    if (opts.legalName && !opts.name) opts.name = opts.legalName;
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
  .action(async (opts) => {
    const { formCreateCommand } = await import("./commands/form.js");
    await formCreateCommand(opts);
  });
formCmd.command("add-founder <entity-id>")
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
  .action(async (entityId: string, opts) => {
    const { formAddFounderCommand } = await import("./commands/form.js");
    await formAddFounderCommand(entityId, opts);
  });
formCmd.command("finalize <entity-id>")
  .description("Finalize formation and generate documents + cap table (staged flow step 3)")
  .option("--authorized-shares <count>", "Authorized shares for corporations")
  .option("--par-value <value>", "Par value per share, e.g. 0.0001")
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
  .action(async (entityId: string, opts) => {
    const { formFinalizeCommand } = await import("./commands/form.js");
    await formFinalizeCommand(entityId, opts);
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
  .description("Seed a fully-populated demo corporation")
  .requiredOption("--name <name>", "Corporation name")
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

program.parse();
