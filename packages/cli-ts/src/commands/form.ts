import { input, select, confirm, number } from "@inquirer/prompts";
import chalk from "chalk";
import Table from "cli-table3";
import { readFileSync, realpathSync } from "node:fs";
import { relative, resolve } from "node:path";
import { requireConfig, setActiveEntityId, saveConfig } from "../config.js";
import { CorpAPIClient } from "../api-client.js";
import { printDryRun, printError, printJson, printReferenceSummary, printSuccess } from "../output.js";
import { ReferenceResolver } from "../references.js";
import type { ApiRecord } from "../types.js";
import { EntityType, OfficerTitle } from "@thecorporation/corp-tools";
import { activateFormationEntity } from "../formation-automation.js";

// ── Types ──────────────────────────────────────────────────────

interface FounderInfo {
  name: string;
  email: string;
  role: string;
  address?: { street: string; street2?: string; city: string; state: string; zip: string };
  officer_title?: string;
  is_incorporator?: boolean;
  shares_purchased?: number;
  ownership_pct?: number;
  vesting?: { total_months: number; cliff_months: number; acceleration?: string };
  ip_description?: string;
}

interface FormOptions {
  type?: string;
  name?: string;
  jurisdiction?: string;
  member?: string[];
  memberJson?: string[];
  membersFile?: string;
  fiscalYearEnd?: string;
  sCorp?: boolean;
  transferRestrictions?: boolean;
  rofr?: boolean;
  address?: string;
  principalName?: string;
  json?: boolean;
  dryRun?: boolean;
  quiet?: boolean;
}

// ── Helpers ────────────────────────────────────────────────────

function isCorp(entityType: string): boolean {
  return entityType === "c_corp" || entityType === "s_corp" || entityType === "corporation";
}

function sectionHeader(title: string): void {
  console.log();
  console.log(chalk.blue("─".repeat(50)));
  console.log(chalk.blue.bold(`  ${title}`));
  console.log(chalk.blue("─".repeat(50)));
}

function officerTitleLabel(title: string): string {
  switch (title) {
    case "ceo":
      return "CEO";
    case "cfo":
      return "CFO";
    case "cto":
      return "CTO";
    case "coo":
      return "COO";
    case "vp":
      return "VP";
    default:
      return title.charAt(0).toUpperCase() + title.slice(1);
  }
}

function parseBoolean(value: unknown): boolean | undefined {
  if (typeof value === "boolean") return value;
  if (typeof value !== "string") return undefined;
  if (value === "true") return true;
  if (value === "false") return false;
  return undefined;
}

function parseAddressValue(raw: unknown): FounderInfo["address"] {
  if (!raw) return undefined;
  if (typeof raw === "string") {
    const parts = raw.split("|").map((part) => part.trim());
    if (parts.length === 4) {
      return { street: parts[0], city: parts[1], state: parts[2], zip: parts[3] };
    }
    throw new Error(`Invalid founder address: ${raw}. Expected street|city|state|zip.`);
  }
  if (typeof raw === "object" && raw !== null) {
    const value = raw as Record<string, unknown>;
    if (
      typeof value.street === "string" &&
      typeof value.city === "string" &&
      typeof value.state === "string" &&
      typeof value.zip === "string"
    ) {
      return {
        street: value.street,
        street2: typeof value.street2 === "string" ? value.street2 : undefined,
        city: value.city,
        state: value.state,
        zip: value.zip,
      };
    }
  }
  throw new Error("Founder address must be an object or street|city|state|zip string.");
}

function normalizeFounderInfo(input: Record<string, unknown>): FounderInfo {
  const name = typeof input.name === "string" ? input.name.trim() : "";
  const email = typeof input.email === "string" ? input.email.trim() : "";
  const role = typeof input.role === "string" ? input.role.trim() : "";
  if (!name || !email || !role) {
    throw new Error("Founder JSON requires non-empty name, email, and role.");
  }

  const founder: FounderInfo = { name, email, role };
  const ownershipPct = input.ownership_pct ?? input.membership_pct ?? input.pct;
  if (ownershipPct != null) founder.ownership_pct = Number(ownershipPct);
  const sharesPurchased = input.shares_purchased ?? input.shares;
  if (sharesPurchased != null) founder.shares_purchased = Number(sharesPurchased);
  if (typeof input.officer_title === "string") founder.officer_title = input.officer_title;
  const incorporator = parseBoolean(input.is_incorporator ?? input.incorporator);
  if (incorporator !== undefined) founder.is_incorporator = incorporator;
  if (input.address != null) founder.address = parseAddressValue(input.address);
  if (typeof input.ip_description === "string") founder.ip_description = input.ip_description;
  if (input.vesting && typeof input.vesting === "object") {
    const vesting = input.vesting as Record<string, unknown>;
    if (vesting.total_months != null && vesting.cliff_months != null) {
      founder.vesting = {
        total_months: Number(vesting.total_months),
        cliff_months: Number(vesting.cliff_months),
        acceleration:
          typeof vesting.acceleration === "string" ? vesting.acceleration : undefined,
      };
    }
  }
  return founder;
}

function parseLegacyMemberSpec(raw: string): FounderInfo {
  const parts = raw.split(",").map((p) => p.trim());
  if (parts.length < 3) {
    throw new Error(
      `Invalid member format: ${raw}. Expected: name,email,role[,pct[,street|city|state|zip[,officer_title[,is_incorporator]]]]`,
    );
  }
  const founder: FounderInfo = { name: parts[0], email: parts[1], role: parts[2] };
  if (parts.length >= 4) founder.ownership_pct = parseFloat(parts[3]);
  if (parts.length >= 5 && parts[4]) founder.address = parseAddressValue(parts[4]);
  if (parts.length >= 6 && parts[5]) founder.officer_title = parts[5];
  if (parts.length >= 7) {
    const incorporator = parseBoolean(parts[6]);
    if (incorporator !== undefined) founder.is_incorporator = incorporator;
  }
  return founder;
}

function parseKeyValueMemberSpec(raw: string): FounderInfo {
  const parsed: Record<string, unknown> = {};
  for (const segment of raw.split(",")) {
    const [key, ...rest] = segment.split("=");
    if (!key || rest.length === 0) {
      throw new Error(`Invalid member format: ${raw}. Expected key=value pairs.`);
    }
    parsed[key.trim()] = rest.join("=").trim();
  }
  return normalizeFounderInfo(parsed);
}

function readSafeJsonFile(filePath: string, label: string): string {
  if (process.env.CORP_ALLOW_UNSAFE_FILE_INPUT === "1") {
    return readFileSync(filePath, "utf8");
  }
  const resolvedFile = realpathSync(resolve(filePath));
  const workingTreeRoot = realpathSync(process.cwd());
  const rel = relative(workingTreeRoot, resolvedFile);
  if (rel === "" || (!rel.startsWith("..") && !rel.startsWith("/"))) {
    return readFileSync(resolvedFile, "utf8");
  }
  throw new Error(
    `--${label} must stay inside the current working directory unless CORP_ALLOW_UNSAFE_FILE_INPUT=1 is set.`,
  );
}

function validateFounders(founders: FounderInfo[]): void {
  // Check for duplicate emails
  const seenEmails = new Set<string>();
  for (const f of founders) {
    const email = f.email.toLowerCase().trim();
    if (seenEmails.has(email)) {
      throw new Error(`Duplicate founder email: ${email}. Each founder must have a unique email address.`);
    }
    seenEmails.add(email);
  }

  // Check total ownership doesn't exceed 100%
  let totalOwnership = 0;
  for (const f of founders) {
    if (f.ownership_pct != null) {
      if (f.ownership_pct <= 0 || f.ownership_pct > 100) {
        throw new Error(`Invalid ownership_pct ${f.ownership_pct} for ${f.name}. Must be between 0 and 100.`);
      }
      totalOwnership += f.ownership_pct;
    }
  }
  if (totalOwnership > 100.000001) {
    throw new Error(`Total ownership_pct is ${totalOwnership.toFixed(2)}%, which exceeds 100%. Reduce ownership percentages.`);
  }
}

function parseScriptedFounders(opts: FormOptions): FounderInfo[] {
  const founders: FounderInfo[] = [];
  for (const raw of opts.member ?? []) {
    founders.push(raw.includes("=") ? parseKeyValueMemberSpec(raw) : parseLegacyMemberSpec(raw));
  }
  for (const raw of opts.memberJson ?? []) {
    founders.push(normalizeFounderInfo(JSON.parse(raw) as Record<string, unknown>));
  }
  if (opts.membersFile) {
    const parsed = JSON.parse(readSafeJsonFile(opts.membersFile, "members-file")) as unknown;
    let entries: unknown[];
    if (Array.isArray(parsed)) {
      entries = parsed;
    } else if (
      typeof parsed === "object" &&
      parsed !== null &&
      "members" in parsed &&
      Array.isArray((parsed as { members?: unknown }).members)
    ) {
      entries = (parsed as { members: unknown[] }).members;
    } else {
      throw new Error("members file must be a JSON array or {\"members\": [...]}");
    }
    for (const entry of entries) {
      if (typeof entry !== "object" || entry === null || Array.isArray(entry)) {
        throw new Error("each founder entry must be a JSON object");
      }
      founders.push(normalizeFounderInfo(entry as Record<string, unknown>));
    }
  }
  validateFounders(founders);
  return founders;
}

async function promptAddress(): Promise<{ street: string; city: string; state: string; zip: string }> {
  const street = await input({ message: "    Street address" });
  const city = await input({ message: "    City" });
  const state = await input({ message: "    State (2-letter)", default: "DE" });
  const zip = await input({ message: "    ZIP code" });
  return { street, city, state, zip };
}

// ── Phase 1: Entity Details ────────────────────────────────────

async function phaseEntityDetails(opts: FormOptions, serverCfg: ApiRecord, scripted: boolean) {
  if (!scripted) sectionHeader("Phase 1: Entity Details");

  let entityType = opts.type;
  if (!entityType) {
    if (scripted) { entityType = "llc"; }
    else {
      entityType = await select({
        message: "Entity type",
        choices: [
          { value: "llc", name: "LLC" },
          { value: "c_corp", name: "C-Corp" },
        ],
      });
    }
  }

  let name = opts.name;
  if (!name) {
    if (scripted) { printError("--name is required in scripted mode"); process.exit(1); }
    name = await input({ message: "Legal name" });
  }

  let jurisdiction = opts.jurisdiction;
  if (!jurisdiction) {
    const defaultJ = entityType === "llc" ? "US-WY" : "US-DE";
    if (scripted) { jurisdiction = defaultJ; }
    else { jurisdiction = await input({ message: "Jurisdiction", default: defaultJ }); }
  }

  let companyAddress: { street: string; city: string; state: string; zip: string } | undefined;
  if (opts.address) {
    const parts = opts.address.split(",").map((p) => p.trim());
    if (parts.length === 4) {
      companyAddress = { street: parts[0], city: parts[1], state: parts[2], zip: parts[3] };
    }
  }
  if (!companyAddress && !scripted) {
    const wantAddress = await confirm({ message: "Add company address?", default: false });
    if (wantAddress) {
      companyAddress = await promptAddress();
    }
  }

  const fiscalYearEnd = opts.fiscalYearEnd ?? "12-31";

  let sCorpElection = opts.sCorp ?? false;
  if (!scripted && isCorp(entityType) && opts.sCorp === undefined) {
    sCorpElection = await confirm({ message: "S-Corp election?", default: false });
  }

  return { entityType, name, jurisdiction, companyAddress, fiscalYearEnd, sCorpElection };
}

// ── Phase 2: People ────────────────────────────────────────────

async function phasePeople(
  opts: FormOptions,
  entityType: string,
  scripted: boolean,
): Promise<FounderInfo[]> {
  if (!scripted) sectionHeader("Phase 2: Founders & Officers");

  const founders: FounderInfo[] = [];

  // CLI-provided members (scripted mode)
  if (scripted) {
    try {
      return parseScriptedFounders(opts);
    } catch (err) {
      printError(String(err));
      process.exit(1);
    }
  }

  // Interactive mode
  const founderCount = (await number({ message: "Number of founders (1-6)", default: 1 })) ?? 1;

  for (let i = 0; i < founderCount; i++) {
    console.log(chalk.dim(`\n  Founder ${i + 1} of ${founderCount}:`));
    const name = await input({ message: `  Name` });
    const email = await input({ message: `  Email` });

    let role = "member";
    if (isCorp(entityType)) {
      role = await select({
        message: "  Role",
        choices: [
          { value: "director", name: "Director" },
          { value: "officer", name: "Officer" },
          { value: "member", name: "Shareholder only" },
        ],
      });
    }

    const wantAddress = await confirm({ message: "  Add address?", default: false });
    const address = wantAddress ? await promptAddress() : undefined;

    let officerTitle: string | undefined;
    if (isCorp(entityType)) {
      const wantOfficer = role === "officer" || await confirm({ message: "  Assign officer title?", default: i === 0 });
      if (wantOfficer) {
        officerTitle = await select({
          message: "  Officer title",
          choices: OfficerTitle.map((t) => ({
            value: t,
            name: officerTitleLabel(t),
          })),
        });
      }
    }

    let isIncorporator = false;
    if (isCorp(entityType) && i === 0 && founderCount === 1) {
      isIncorporator = true;
    } else if (isCorp(entityType)) {
      isIncorporator = await confirm({ message: "  Designate as sole incorporator?", default: i === 0 });
    }

    founders.push({ name, email, role, address, officer_title: officerTitle, is_incorporator: isIncorporator });
  }

  return founders;
}

// ── Phase 3: Stock & Finalize ──────────────────────────────────

async function phaseStock(
  opts: FormOptions,
  entityType: string,
  founders: FounderInfo[],
  scripted: boolean,
): Promise<{ founders: FounderInfo[]; transferRestrictions: boolean; rofr: boolean }> {
  if (!scripted) sectionHeader("Phase 3: Equity & Finalize");

  const transferRestrictions = opts.transferRestrictions ?? (
    !scripted && isCorp(entityType)
      ? await confirm({ message: "Transfer restrictions on shares?", default: true })
      : isCorp(entityType)
  );

  const rofr = opts.rofr ?? (
    !scripted && isCorp(entityType)
      ? await confirm({ message: "Right of first refusal?", default: true })
      : isCorp(entityType)
  );

  if (!scripted) {
    for (const f of founders) {
      console.log(chalk.dim(`\n  Equity for ${f.name}:`));

      if (isCorp(entityType)) {
        const shares = await number({ message: `  Shares to purchase`, default: 0 });
        f.shares_purchased = shares ?? 0;
        if (f.shares_purchased === 0) {
          const pct = await number({ message: `  Ownership % (1-100)`, default: founders.length === 1 ? 100 : 0 });
          f.ownership_pct = pct ?? 0;
        }
      } else {
        const pct = await number({
          message: `  Ownership % (1-100)`,
          default: founders.length === 1 ? 100 : 0,
        });
        f.ownership_pct = pct ?? 0;
      }

      if (isCorp(entityType)) {
        const wantVesting = await confirm({ message: "  Add vesting schedule?", default: false });
        if (wantVesting) {
          const totalMonths = (await number({ message: "  Total vesting months", default: 48 })) ?? 48;
          const cliffMonths = (await number({ message: "  Cliff months", default: 12 })) ?? 12;
          const acceleration = await select({
            message: "  Acceleration",
            choices: [
              { value: "none", name: "None" },
              { value: "single_trigger", name: "Single trigger" },
              { value: "double_trigger", name: "Double trigger" },
            ],
          });
          f.vesting = {
            total_months: totalMonths,
            cliff_months: cliffMonths,
            acceleration: acceleration === "none" ? undefined : acceleration,
          };
        }
      }

      const wantIp = await confirm({ message: "  Contributing IP?", default: false });
      if (wantIp) {
        f.ip_description = await input({ message: "  Describe IP being contributed" });
      }
    }
  }

  return { founders, transferRestrictions, rofr };
}

// ── Summary Table ──────────────────────────────────────────────

function printSummary(
  entityType: string,
  name: string,
  jurisdiction: string,
  fiscalYearEnd: string,
  sCorpElection: boolean,
  founders: FounderInfo[],
  transferRestrictions: boolean,
  rofr: boolean,
): void {
  sectionHeader("Formation Summary");

  console.log(`  ${chalk.bold("Entity:")} ${name}`);
  console.log(`  ${chalk.bold("Type:")} ${entityType}`);
  console.log(`  ${chalk.bold("Jurisdiction:")} ${jurisdiction}`);
  console.log(`  ${chalk.bold("Fiscal Year End:")} ${fiscalYearEnd}`);
  if (isCorp(entityType)) {
    console.log(`  ${chalk.bold("S-Corp Election:")} ${sCorpElection ? "Yes" : "No"}`);
    console.log(`  ${chalk.bold("Transfer Restrictions:")} ${transferRestrictions ? "Yes" : "No"}`);
    console.log(`  ${chalk.bold("Right of First Refusal:")} ${rofr ? "Yes" : "No"}`);
  }

  const table = new Table({
    head: [chalk.dim("Name"), chalk.dim("Email"), chalk.dim("Role"), chalk.dim("Equity"), chalk.dim("Officer")],
  });
  for (const f of founders) {
    const equity = f.shares_purchased
      ? `${f.shares_purchased.toLocaleString()} shares`
      : f.ownership_pct
        ? `${f.ownership_pct}%`
        : "—";
    table.push([f.name, f.email, f.role, equity, f.officer_title ?? "—"]);
  }
  console.log(table.toString());
}

// ── Main Command ───────────────────────────────────────────────

const SUPPORTED_ENTITY_TYPES = ["llc", "c_corp", "s_corp", "corporation"];

export async function formCommand(opts: FormOptions): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);

  try {
    // Client-side type validation
    if (opts.type && !SUPPORTED_ENTITY_TYPES.includes(opts.type)) {
      printError(`Unsupported entity type '${opts.type}'. Supported types: ${SUPPORTED_ENTITY_TYPES.join(", ")}`);
      process.exit(1);
    }
    // Client-side name validation
    if (opts.name != null && !opts.name.trim()) {
      printError("--name cannot be empty or whitespace");
      process.exit(1);
    }

    let serverCfg: ApiRecord = {};
    try { serverCfg = await client.getConfig(); } catch { /* ignore */ }

    const hasMembers = Boolean(
      (opts.member && opts.member.length > 0) ||
      (opts.memberJson && opts.memberJson.length > 0) ||
      opts.membersFile,
    );
    const scripted = hasMembers || opts.json || opts.dryRun || !process.stdout.isTTY;

    // If non-TTY/json/dryRun and no members provided, error early instead of prompting
    if (scripted && !hasMembers) {
      printError("At least one --member, --member-json, or --members-file is required in non-interactive mode.");
      process.exit(1);
    }

    // Phase 1: Entity Details
    const { entityType, name, jurisdiction, companyAddress, fiscalYearEnd, sCorpElection } =
      await phaseEntityDetails(opts, serverCfg, scripted);

    // Phase 2: People
    const founders = await phasePeople(opts, entityType, scripted);

    // C-Corp upfront validation — check all requirements at once
    if (isCorp(entityType) && scripted) {
      const missing: string[] = [];
      const hasAddress = founders.some((f) => f.address);
      const hasOfficer = founders.some((f) => f.officer_title);
      const hasIncorporator = founders.some((f) => f.is_incorporator);
      if (!hasAddress) missing.push("At least one member with an address (for incorporator filing)");
      if (!hasOfficer) missing.push("At least one member with --officer-title (for initial board consent)");
      if (!hasIncorporator && founders.length === 1) {
        // Single founder auto-marked as incorporator, no error needed
      } else if (!hasIncorporator && founders.length > 1) {
        missing.push("At least one member marked as --incorporator");
      }
      if (missing.length > 0) {
        printError(
          "C-Corp formation requires:\n" +
          missing.map((m) => `  - ${m}`).join("\n") + "\n" +
          '  Example: --member "Name,email,director,100,street|city|state|zip,ceo,true"',
        );
        process.exit(1);
      }
    }

    // Phase 3: Stock & Finalize
    const { transferRestrictions, rofr } = await phaseStock(opts, entityType, founders, scripted);

    // Summary & Confirm (suppress for --json and --quiet to keep output clean)
    if (!opts.quiet && !opts.json) {
      printSummary(entityType, name, jurisdiction, fiscalYearEnd, sCorpElection, founders, transferRestrictions, rofr);
    }

    const shouldProceed = scripted
      ? true
      : await confirm({ message: "Proceed with formation?", default: true });

    if (!shouldProceed) {
      console.log(chalk.yellow("Formation cancelled."));
      return;
    }

    // Build members payload
    const members: ApiRecord[] = founders.map((f) => {
      const m: ApiRecord = {
        name: f.name,
        email: f.email,
        role: f.role,
        investor_type: "natural_person",
      };
      if (f.ownership_pct) m.ownership_pct = f.ownership_pct;
      if (f.shares_purchased) m.shares_purchased = f.shares_purchased;
      if (f.address) m.address = f.address;
      if (f.officer_title) m.officer_title = f.officer_title;
      if (f.is_incorporator) m.is_incorporator = true;
      if (f.vesting) m.vesting = f.vesting;
      if (f.ip_description) m.ip_description = f.ip_description;
      return m;
    });

    const payload: ApiRecord = {
      entity_type: entityType,
      legal_name: name,
      jurisdiction,
      members,
      workspace_id: cfg.workspace_id,
      fiscal_year_end: fiscalYearEnd,
      s_corp_election: sCorpElection,
      transfer_restrictions: transferRestrictions,
      right_of_first_refusal: rofr,
    };
    if (companyAddress) payload.company_address = companyAddress;
    // LLC: auto-set principal_name from first member if not explicitly provided
    if (entityType === "llc") {
      const principalName = opts.principalName ?? founders[0]?.name;
      if (principalName) payload.principal_name = principalName;
    }

    if (opts.dryRun) {
      printDryRun("formation.create_with_cap_table", payload);
      return;
    }

    const result = await client.createFormationWithCapTable(payload);
    await resolver.stabilizeRecord("entity", result);
    resolver.rememberFromRecord("entity", result);

    if (result.entity_id) {
      setActiveEntityId(cfg, String(result.entity_id));
      saveConfig(cfg);
    }

    if (opts.quiet) {
      const id = result.entity_id ?? result.formation_id;
      if (id) console.log(String(id));
      return;
    }

    if (opts.json) {
      printJson(result);
      return;
    }

    if (result.entity_id) {
      console.log(chalk.dim(`  Active entity set to ${result.entity_id}`));
    }

    // Output results
    printSuccess(`Formation created: ${result.formation_id ?? "OK"}`);
    if (result.entity_id) {
      printReferenceSummary("entity", result, { showReuseHint: true });
    }
    if (result.legal_entity_id) console.log(`  Legal Entity ID: ${result.legal_entity_id}`);
    if (result.instrument_id) console.log(`  Instrument ID: ${result.instrument_id}`);

    const docIds = (result.document_ids ?? []) as string[];
    if (docIds.length > 0) {
      console.log(`  Documents: ${docIds.length} generated`);
    }

    const holders = (result.holders ?? []) as ApiRecord[];
    if (holders.length > 0) {
      console.log();
      const table = new Table({
        head: [chalk.dim("Holder"), chalk.dim("Shares"), chalk.dim("Ownership %")],
      });
      for (const h of holders) {
        const pct = typeof h.ownership_pct === "number" ? `${h.ownership_pct.toFixed(1)}%` : "—";
        table.push([String(h.name ?? "?"), String(h.shares ?? 0), pct]);
      }
      console.log(chalk.bold("  Cap Table:"));
      console.log(table.toString());
    }

    if (result.next_action) {
      const entityRef = result.entity_id ? `${result.entity_id}` : "@last:entity";
      const actionHint: Record<string, string> = {
        sign_documents: `corp form activate ${entityRef}`,
        submit_state_filing: `corp form activate ${entityRef}`,
        confirm_state_filing: `corp form activate ${entityRef} --filing-id "..."`,
        apply_for_ein: `corp form activate ${entityRef}`,
        confirm_ein: `corp form activate ${entityRef} --ein "..."`,
      };
      const hint = actionHint[result.next_action as string];
      if (hint) {
        console.log(chalk.yellow(`\n  Next step: ${hint}`));
      } else {
        console.log(chalk.yellow(`\n  Next: ${result.next_action}`));
      }
    }
  } catch (err) {
    if (err instanceof Error && err.message.includes("exit")) throw err;
    const msg = String(err);
    if (msg.includes("officers_list") || msg.includes("officer")) {
      printError(
        `Formation failed: ${msg}\n` +
        "  Hint: C-Corp directors need an officer_title. Use --member with officer_title field, e.g.:\n" +
        "    --member 'name=Alice,email=a@co.com,role=director,officer_title=ceo,pct=100'\n" +
        "  Or use --member-json with {\"officer_title\": \"ceo\"}",
      );
    } else {
      printError(`Failed to create formation: ${err}`);
    }
    process.exit(1);
  }
}

// ── Staged Formation Subcommands ─────────────────────────────

interface FormCreateOptions {
  type: string;
  name: string;
  jurisdiction?: string;
  registeredAgentName?: string;
  registeredAgentAddress?: string;
  formationDate?: string;
  fiscalYearEnd?: string;
  sCorp?: boolean;
  transferRestrictions?: boolean;
  rofr?: boolean;
  companyAddress?: string;
  dryRun?: boolean;
  json?: boolean;
  quiet?: boolean;
}

function parseCsvAddress(raw?: string): { street: string; city: string; state: string; zip: string } | undefined {
  if (!raw) return undefined;
  const parts = raw.split(",").map((p) => p.trim()).filter(Boolean);
  if (parts.length !== 4) {
    throw new Error(`Invalid address format: ${raw}. Expected 'street,city,state,zip'`);
  }
  return { street: parts[0], city: parts[1], state: parts[2], zip: parts[3] };
}

function shouldResolveEntityRefForDryRun(entityRef: string): boolean {
  const trimmed = entityRef.trim().toLowerCase();
  return trimmed === "_" || trimmed === "@last" || trimmed.startsWith("@last:");
}

async function resolveEntityRefForFormCommand(
  resolver: ReferenceResolver,
  entityRef: string,
  dryRun?: boolean,
): Promise<string> {
  if (!dryRun || shouldResolveEntityRefForDryRun(entityRef)) {
    return resolver.resolveEntity(entityRef);
  }
  try {
    return await resolver.resolveEntity(entityRef);
  } catch (err) {
    if (String(err).includes("fetch failed")) {
      return entityRef;
    }
    throw err;
  }
}

export async function formCreateCommand(opts: FormCreateOptions): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);

  try {
    const entityType = opts.type === "corporation" ? "c_corp" : opts.type;
    const payload: ApiRecord = {
      entity_type: entityType,
      legal_name: opts.name,
    };
    if (opts.jurisdiction) payload.jurisdiction = opts.jurisdiction;
    if (opts.registeredAgentName) payload.registered_agent_name = opts.registeredAgentName;
    if (opts.registeredAgentAddress) payload.registered_agent_address = opts.registeredAgentAddress;
    if (opts.formationDate) payload.formation_date = opts.formationDate;
    if (opts.fiscalYearEnd) payload.fiscal_year_end = opts.fiscalYearEnd;
    if (opts.sCorp !== undefined) payload.s_corp_election = opts.sCorp;
    if (opts.transferRestrictions !== undefined) payload.transfer_restrictions = opts.transferRestrictions;
    if (opts.rofr !== undefined) payload.right_of_first_refusal = opts.rofr;
    const companyAddress = parseCsvAddress(opts.companyAddress);
    if (companyAddress) payload.company_address = companyAddress;

    if (opts.dryRun) {
      printDryRun("formation.create_pending", payload);
      return;
    }

    const result = await client.createPendingEntity(payload);
    await resolver.stabilizeRecord("entity", result);
    resolver.rememberFromRecord("entity", result);

    if (result.entity_id) {
      setActiveEntityId(cfg, String(result.entity_id));
      saveConfig(cfg);
    }

    if (opts.quiet) {
      const id = result.entity_id;
      if (id) console.log(String(id));
      return;
    }
    if (opts.json) {
      printJson(result);
      return;
    }
    printSuccess(`Pending entity created: ${result.entity_id}`);
    printReferenceSummary("entity", result, { showReuseHint: true });
    console.log(`  Name: ${result.legal_name}`);
    console.log(`  Type: ${result.entity_type}`);
    console.log(`  Jurisdiction: ${result.jurisdiction}`);
    console.log(`  Status: ${result.formation_status}`);
    console.log(chalk.yellow(`\n  Next: corp form add-founder @last:entity --name "..." --email "..." --role member --pct 50`));
  } catch (err) {
    printError(`Failed to create pending entity: ${err}`);
    process.exit(1);
  }
}

interface FormAddFounderOptions {
  name: string;
  email: string;
  role: string;
  pct: string;
  officerTitle?: string;
  incorporator?: boolean;
  address?: string;
  dryRun?: boolean;
  json?: boolean;
}

interface FormFinalizeOptions {
  authorizedShares?: string;
  parValue?: string;
  boardSize?: string;
  principalName?: string;
  registeredAgentName?: string;
  registeredAgentAddress?: string;
  formationDate?: string;
  fiscalYearEnd?: string;
  sCorp?: boolean;
  transferRestrictions?: boolean;
  rofr?: boolean;
  companyAddress?: string;
  incorporatorName?: string;
  incorporatorAddress?: string;
  dryRun?: boolean;
  json?: boolean;
}

interface FormActivateOptions {
  evidenceUri?: string;
  evidenceType?: string;
  filingId?: string;
  receiptReference?: string;
  ein?: string;
  dryRun?: boolean;
  json?: boolean;
}

export async function formAddFounderCommand(entityId: string, opts: FormAddFounderOptions): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);

  try {
    const resolvedEntityId = await resolveEntityRefForFormCommand(resolver, entityId, opts.dryRun);
    const payload: ApiRecord = {
      name: opts.name,
      email: opts.email,
      role: opts.role,
      ownership_pct: parseFloat(opts.pct),
    };
    if (opts.officerTitle) payload.officer_title = opts.officerTitle.toLowerCase();
    if (opts.incorporator) payload.is_incorporator = true;
    const address = parseCsvAddress(opts.address);
    if (address) payload.address = address;

    if (opts.dryRun) {
      printDryRun("formation.add_founder", { entity_id: resolvedEntityId, ...payload });
      return;
    }

    const result = await client.addFounder(resolvedEntityId, payload);
    if (opts.json) {
      printJson(result);
      return;
    }
    printSuccess(`Founder added (${result.member_count} total)`);
    const members = (result.members ?? []) as ApiRecord[];
    for (const m of members) {
      const pct = typeof m.ownership_pct === "number" ? ` (${m.ownership_pct}%)` : "";
      console.log(`  - ${m.name} <${m.email ?? "no email"}> [${m.role ?? "member"}]${pct}`);
    }
    console.log(chalk.yellow(`\n  Next: add more founders or run: corp form finalize @last:entity`));
  } catch (err) {
    printError(`Failed to add founder: ${err}`);
    process.exit(1);
  }
}

export async function formFinalizeCommand(entityId: string, opts: FormFinalizeOptions): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);

  try {
    const resolvedEntityId = await resolveEntityRefForFormCommand(resolver, entityId, opts.dryRun);
    const payload: ApiRecord = {};
    if (opts.authorizedShares) {
      const authorizedShares = parseInt(opts.authorizedShares, 10);
      if (!Number.isFinite(authorizedShares)) {
        throw new Error(`Invalid authorized shares: ${opts.authorizedShares}`);
      }
      payload.authorized_shares = authorizedShares;
    }
    if (opts.parValue) payload.par_value = opts.parValue;
    if (opts.boardSize) {
      const boardSize = parseInt(opts.boardSize, 10);
      if (!Number.isFinite(boardSize) || boardSize <= 0) {
        throw new Error(`Invalid board size: ${opts.boardSize}`);
      }
      payload.board_size = boardSize;
    }
    if (opts.principalName) payload.principal_name = opts.principalName;
    if (opts.registeredAgentName) payload.registered_agent_name = opts.registeredAgentName;
    if (opts.registeredAgentAddress) payload.registered_agent_address = opts.registeredAgentAddress;
    if (opts.formationDate) payload.formation_date = opts.formationDate;
    if (opts.fiscalYearEnd) payload.fiscal_year_end = opts.fiscalYearEnd;
    if (opts.sCorp !== undefined) payload.s_corp_election = opts.sCorp;
    if (opts.transferRestrictions !== undefined) payload.transfer_restrictions = opts.transferRestrictions;
    if (opts.rofr !== undefined) payload.right_of_first_refusal = opts.rofr;
    const companyAddress = parseCsvAddress(opts.companyAddress);
    if (companyAddress) payload.company_address = companyAddress;
    if (opts.incorporatorName) payload.incorporator_name = opts.incorporatorName;
    if (opts.incorporatorAddress) payload.incorporator_address = opts.incorporatorAddress;

    if (opts.dryRun) {
      printDryRun("formation.finalize", { entity_id: resolvedEntityId, ...payload });
      return;
    }

    const result = await client.finalizeFormation(resolvedEntityId, payload);
    await resolver.stabilizeRecord("entity", result);
    resolver.rememberFromRecord("entity", result);
    if (opts.json) {
      printJson(result);
      return;
    }
    printSuccess(`Formation finalized: ${result.entity_id}`);
    printReferenceSummary("entity", result, { showReuseHint: true });
    if (result.legal_entity_id) console.log(`  Legal Entity ID: ${result.legal_entity_id}`);
    if (result.instrument_id) console.log(`  Instrument ID: ${result.instrument_id}`);

    const docIds = (result.document_ids ?? []) as string[];
    if (docIds.length > 0) {
      console.log(`  Documents: ${docIds.length} generated`);
    }

    const holders = (result.holders ?? []) as ApiRecord[];
    if (holders.length > 0) {
      console.log();
      const table = new Table({
        head: [chalk.dim("Holder"), chalk.dim("Shares"), chalk.dim("Ownership %")],
      });
      for (const h of holders) {
        const pct = typeof h.ownership_pct === "number" ? `${h.ownership_pct.toFixed(1)}%` : "—";
        table.push([String(h.name ?? "?"), String(h.shares ?? 0), pct]);
      }
      console.log(chalk.bold("  Cap Table:"));
      console.log(table.toString());
    }

    if (result.next_action) {
      console.log(chalk.yellow(`\n  Next: ${result.next_action}`));
    }
  } catch (err) {
    const msg = String(err);
    if (msg.includes("officers_list") || msg.includes("officer")) {
      printError(
        `Finalization failed: ${msg}\n` +
        "  Hint: C-Corp entities require at least one founder with an officer_title.\n" +
        "  Add a founder with: corp form add-founder @last:entity --name '...' --email '...' --role director --pct 100 --officer-title ceo",
      );
    } else {
      printError(`Failed to finalize formation: ${err}`);
    }
    process.exit(1);
  }
}

export async function formActivateCommand(entityId: string, opts: FormActivateOptions): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);

  try {
    const resolvedEntityId = await resolveEntityRefForFormCommand(resolver, entityId, opts.dryRun);
    const payload: ApiRecord = { entity_id: resolvedEntityId };
    if (opts.evidenceUri) payload.evidence_uri = opts.evidenceUri;
    if (opts.evidenceType) payload.evidence_type = opts.evidenceType;
    if (opts.filingId) payload.filing_id = opts.filingId;
    if (opts.receiptReference) payload.receipt_reference = opts.receiptReference;
    if (opts.ein) payload.ein = opts.ein;

    if (opts.dryRun) {
      printDryRun("formation.activate", payload);
      return;
    }

    const result = await activateFormationEntity(client, resolver, resolvedEntityId, {
      evidenceUri: opts.evidenceUri,
      evidenceType: opts.evidenceType,
      filingId: opts.filingId,
      receiptReference: opts.receiptReference,
      ein: opts.ein,
    });
    const formation = await client.getFormation(resolvedEntityId);
    await resolver.stabilizeRecord("entity", formation as ApiRecord);
    resolver.rememberFromRecord("entity", formation as ApiRecord);

    if (opts.json) {
      printJson({
        ...result,
        formation,
      });
      return;
    }

    printSuccess(`Formation advanced to ${result.final_status}.`);
    printReferenceSummary("entity", formation as ApiRecord, { showReuseHint: true });
    if (result.steps.length > 0) {
      console.log("  Steps:");
      for (const step of result.steps) {
        console.log(`    - ${step}`);
      }
    }
    console.log(`  Signatures added: ${result.signatures_added}`);
    console.log(`  Documents updated: ${result.documents_signed}`);
  } catch (err) {
    printError(`Failed to activate formation: ${err}`);
    process.exit(1);
  }
}
