import { input, select, confirm, number } from "@inquirer/prompts";
import chalk from "chalk";
import Table from "cli-table3";
import { requireConfig } from "../config.js";
import { CorpAPIClient } from "../api-client.js";
import { printError, printSuccess } from "../output.js";
import type { ApiRecord } from "../types.js";
import { EntityType, OfficerTitle } from "@thecorporation/corp-tools";

// ── Types ──────────────────────────────────────────────────────

interface FounderInfo {
  name: string;
  email: string;
  role: string;
  address?: { street: string; city: string; state: string; zip: string };
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
  fiscalYearEnd?: string;
  sCorp?: boolean;
  transferRestrictions?: boolean;
  rofr?: boolean;
  address?: string;
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
    for (const m of opts.member!) {
      const parts = m.split(",").map((p) => p.trim());
      if (parts.length < 3) {
        printError(`Invalid member format: ${m}. Expected: name,email,role[,pct]`);
        process.exit(1);
      }
      const f: FounderInfo = { name: parts[0], email: parts[1], role: parts[2] };
      if (parts.length >= 4) f.ownership_pct = parseFloat(parts[3]);
      founders.push(f);
    }
    return founders;
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
            name: t === "ceo" ? "CEO" : t === "cfo" ? "CFO" : t === "vp" ? "VP" : t.charAt(0).toUpperCase() + t.slice(1),
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

export async function formCommand(opts: FormOptions): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);

  try {
    let serverCfg: ApiRecord = {};
    try { serverCfg = await client.getConfig(); } catch { /* ignore */ }

    const scripted = !!(opts.member && opts.member.length > 0);

    // Phase 1: Entity Details
    const { entityType, name, jurisdiction, companyAddress, fiscalYearEnd, sCorpElection } =
      await phaseEntityDetails(opts, serverCfg, scripted);

    // Phase 2: People
    const founders = await phasePeople(opts, entityType, scripted);

    // Phase 3: Stock & Finalize
    const { transferRestrictions, rofr } = await phaseStock(opts, entityType, founders, scripted);

    // Summary & Confirm
    printSummary(entityType, name, jurisdiction, fiscalYearEnd, sCorpElection, founders, transferRestrictions, rofr);

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

    const result = await client.createFormationWithCapTable(payload);

    // Output results
    printSuccess(`Formation created: ${result.formation_id ?? "OK"}`);
    if (result.entity_id) console.log(`  Entity ID: ${result.entity_id}`);
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
    if (err instanceof Error && err.message.includes("exit")) throw err;
    printError(`Failed to create formation: ${err}`);
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
}

function parseCsvAddress(raw?: string): { street: string; city: string; state: string; zip: string } | undefined {
  if (!raw) return undefined;
  const parts = raw.split(",").map((p) => p.trim()).filter(Boolean);
  if (parts.length !== 4) {
    throw new Error(`Invalid address format: ${raw}. Expected 'street,city,state,zip'`);
  }
  return { street: parts[0], city: parts[1], state: parts[2], zip: parts[3] };
}

export async function formCreateCommand(opts: FormCreateOptions): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);

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

    const result = await client.createPendingEntity(payload);
    printSuccess(`Pending entity created: ${result.entity_id}`);
    console.log(`  Name: ${result.legal_name}`);
    console.log(`  Type: ${result.entity_type}`);
    console.log(`  Jurisdiction: ${result.jurisdiction}`);
    console.log(`  Status: ${result.formation_status}`);
    console.log(chalk.yellow(`\n  Next: corp form add-founder ${result.entity_id} --name "..." --email "..." --role member --pct 50`));
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
}

interface FormFinalizeOptions {
  authorizedShares?: string;
  parValue?: string;
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
}

export async function formAddFounderCommand(entityId: string, opts: FormAddFounderOptions): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);

  try {
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

    const result = await client.addFounder(entityId, payload);
    printSuccess(`Founder added (${result.member_count} total)`);
    const members = (result.members ?? []) as ApiRecord[];
    for (const m of members) {
      const pct = typeof m.ownership_pct === "number" ? ` (${m.ownership_pct}%)` : "";
      console.log(`  - ${m.name} <${m.email ?? "no email"}> [${m.role ?? "member"}]${pct}`);
    }
    console.log(chalk.yellow(`\n  Next: add more founders or run: corp form finalize ${entityId}`));
  } catch (err) {
    printError(`Failed to add founder: ${err}`);
    process.exit(1);
  }
}

export async function formFinalizeCommand(entityId: string, opts: FormFinalizeOptions): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);

  try {
    const payload: ApiRecord = {};
    if (opts.authorizedShares) {
      const authorizedShares = parseInt(opts.authorizedShares, 10);
      if (!Number.isFinite(authorizedShares)) {
        throw new Error(`Invalid authorized shares: ${opts.authorizedShares}`);
      }
      payload.authorized_shares = authorizedShares;
    }
    if (opts.parValue) payload.par_value = opts.parValue;
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

    const result = await client.finalizeFormation(entityId, payload);
    printSuccess(`Formation finalized: ${result.entity_id}`);
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
    printError(`Failed to finalize formation: ${err}`);
    process.exit(1);
  }
}
