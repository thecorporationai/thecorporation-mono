import chalk from "chalk";
import type { CommandDef, CommandContext } from "./types.js";
import type { ApiRecord } from "../types.js";
import {
  printGovernanceTable,
  printSeatsTable,
  printMeetingsTable,
  printResolutionsTable,
  printAgendaItemsTable,
  printReferenceSummary,
} from "../output.js";
import { confirm } from "@inquirer/prompts";
import { writtenConsent as writtenConsentWorkflow } from "@thecorporation/corp-tools";

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const FINALIZE_ITEM_STATUS_CHOICES = [
  "discussed",
  "voted",
  "tabled",
  "withdrawn",
] as const;

// ---------------------------------------------------------------------------
// Governance registry entries
// ---------------------------------------------------------------------------

export const governanceCommands: CommandDef[] = [
  // --- governance (list bodies) ---
  {
    name: "governance",
    description: "Governance bodies, seats, meetings, resolutions",
    route: { method: "GET", path: "/v1/entities/{eid}/governance-bodies" },
    entity: true,
    display: {
      title: "Governance Bodies",
      cols: ["name>Body", "body_type>Type", "seat_count|seats>Seats", "meeting_count|meetings>Meetings", "#body_id>ID"],
    },
    options: [
      { flags: "--body-id <ref>", description: "Governance body reference" },
    ],
    handler: async (ctx) => {
      const eid = await ctx.resolver.resolveEntity(ctx.opts.entityId as string | undefined);
      const bodies = await ctx.client.listGovernanceBodies(eid);
      await ctx.resolver.stabilizeRecords("body", bodies, eid);
      if (ctx.opts.json) { ctx.writer.json(bodies); return; }
      if (bodies.length === 0) { ctx.writer.writeln("No governance bodies found."); return; }
      printGovernanceTable(bodies);
    },
    examples: [
      "corp governance",
      'corp governance create-body --name "Board of Directors" --body-type board_of_directors',
      'corp governance add-seat @last:body --holder "alice"',
      'corp governance convene --body board --type board_meeting --title "Q1 Review" --agenda "Approve budget"',
      "corp governance open @last:meeting --present-seat alice-seat",
      "corp governance vote @last:meeting <item-ref> --voter alice --vote for",
      'corp governance written-consent --body board --title "Approve Option Plan" --description "Board approves 2026 option plan"',
      "corp governance mode",
      "corp governance mode --set board",
      "corp governance resign <seat-ref>",
      "corp governance incidents",
      "corp governance profile",
    ],
  },

  // --- governance seats <body-ref> ---
  {
    name: "governance seats",
    description: "Seats for a governance body",
    route: { method: "GET", path: "/v1/governance-bodies/{pos}/seats" },
    entity: "query",
    args: [{ name: "body-ref", required: true, description: "Governance body reference" }],
    display: {
      title: "Seats",
      cols: ["holder_name|holder>Holder", "role>Role", "status>Status", "#seat_id>ID"],
    },
    handler: async (ctx) => {
      const bodyRef = ctx.positional[0];
      if (!bodyRef) {
        throw new Error("Missing required argument <body-ref>.\n  List bodies first: corp governance\n  Then: corp governance seats <body-id>");
      }
      const eid = await ctx.resolver.resolveEntity(ctx.opts.entityId as string | undefined);
      const resolvedBodyId = await ctx.resolver.resolveBody(eid, bodyRef);
      const seats = await ctx.client.getGovernanceSeats(resolvedBodyId, eid);
      await ctx.resolver.stabilizeRecords("seat", seats, eid);
      if (ctx.opts.json) { ctx.writer.json(seats); return; }
      if (seats.length === 0) { ctx.writer.writeln("No seats found."); return; }
      printSeatsTable(seats);
    },
    examples: ["corp governance seats <body-ref>", "corp governance seats <body-ref> --json"],
  },

  // --- governance meetings <body-ref> ---
  {
    name: "governance meetings",
    description: "Meetings for a governance body",
    route: { method: "GET", path: "/v1/governance-bodies/{pos}/meetings" },
    entity: "query",
    args: [{ name: "body-ref", required: true, description: "Governance body reference" }],
    display: {
      title: "Meetings",
      cols: ["title>Title", "@scheduled_date>Date", "status>Status", "resolution_count|resolutions>Resolutions", "#meeting_id>ID"],
    },
    handler: async (ctx) => {
      const bodyRef = ctx.positional[0];
      if (!bodyRef) {
        throw new Error("Missing required argument <body-ref>.\n  List bodies first: corp governance\n  Then: corp governance meetings <body-id>");
      }
      const eid = await ctx.resolver.resolveEntity(ctx.opts.entityId as string | undefined);
      const resolvedBodyId = await ctx.resolver.resolveBody(eid, bodyRef);
      const meetings = await ctx.client.listMeetings(resolvedBodyId, eid);
      await ctx.resolver.stabilizeRecords("meeting", meetings, eid);
      if (ctx.opts.json) { ctx.writer.json(meetings); return; }
      if (meetings.length === 0) { ctx.writer.writeln("No meetings found."); return; }
      printMeetingsTable(meetings);
    },
    examples: ["corp governance meetings <body-ref>", "corp governance meetings <body-ref> --json"],
  },

  // --- governance resolutions <meeting-ref> ---
  {
    name: "governance resolutions",
    description: "Resolutions for a meeting",
    route: { method: "GET", path: "/v1/meetings/{pos}/resolutions" },
    entity: "query",
    args: [{ name: "meeting-ref", required: true, description: "Meeting reference" }],
    display: {
      title: "Resolutions",
      cols: ["title>Title", "resolution_type>Type", "status>Status", "votes_for>For", "votes_against>Against", "#resolution_id>ID"],
    },
    handler: async (ctx) => {
      const meetingRef = ctx.positional[0];
      if (!meetingRef) {
        throw new Error("Missing required argument <meeting-ref>.\n  List meetings first: corp governance meetings <body-ref>\n  Then: corp governance resolutions <meeting-id>");
      }
      const eid = await ctx.resolver.resolveEntity(ctx.opts.entityId as string | undefined);
      const resolvedMeetingId = await ctx.resolver.resolveMeeting(eid, meetingRef);
      const resolutions = await ctx.client.getMeetingResolutions(resolvedMeetingId, eid);
      await ctx.resolver.stabilizeRecords("resolution", resolutions, eid);
      if (ctx.opts.json) { ctx.writer.json(resolutions); return; }
      if (resolutions.length === 0) { ctx.writer.writeln("No resolutions found."); return; }
      printResolutionsTable(resolutions);
    },
    examples: ["corp governance resolutions <meeting-ref>", "corp governance resolutions <meeting-ref> --json"],
  },

  // --- governance agenda-items <meeting-ref> ---
  {
    name: "governance agenda-items",
    description: "List agenda items for a meeting",
    route: { method: "GET", path: "/v1/meetings/{pos}/agenda-items" },
    entity: "query",
    args: [{ name: "meeting-ref", required: true, description: "Meeting reference" }],
    display: {
      title: "Agenda Items",
      cols: ["title>Title", "status>Status", "item_type>Type", "#agenda_item_id>ID"],
    },
    handler: async (ctx) => {
      const meetingRef = ctx.positional[0];
      if (!meetingRef) {
        throw new Error("Missing required argument <meeting-ref>.\n  List meetings first: corp governance meetings <body-ref>\n  Then: corp governance agenda-items <meeting-id>");
      }
      const eid = await ctx.resolver.resolveEntity(ctx.opts.entityId as string | undefined);
      const resolvedMeetingId = await ctx.resolver.resolveMeeting(eid, meetingRef);
      const items = await ctx.client.listAgendaItems(resolvedMeetingId, eid);
      await ctx.resolver.stabilizeRecords("agenda_item", items, eid);
      if (ctx.opts.json) { ctx.writer.json(items); return; }
      if (items.length === 0) { ctx.writer.writeln("No agenda items found."); return; }
      printAgendaItemsTable(items);
    },
    examples: ["corp governance agenda-items <meeting-ref>", "corp governance agenda-items <meeting-ref> --json"],
  },

  // --- governance incidents ---
  {
    name: "governance incidents",
    description: "Report a governance incident",
    route: { method: "GET", path: "/v1/entities/{eid}/governance/incidents" },
    entity: true,
    display: { title: "Governance Incidents" },
    handler: async (ctx) => {
      const eid = await ctx.resolver.resolveEntity(ctx.opts.entityId as string | undefined);
      const incidents = await ctx.client.listGovernanceIncidents(eid);
      if (ctx.opts.json) { ctx.writer.json(incidents); return; }
      if (incidents.length === 0) { ctx.writer.writeln("No governance incidents found."); return; }
      for (const inc of incidents) {
        const status = String(inc.status ?? "open");
        const colored = status === "resolved" ? chalk.green(status) : chalk.red(status);
        console.log(`  [${colored}] ${inc.incident_type ?? "unknown"}: ${inc.description ?? inc.id}`);
      }
    },
    examples: ["corp governance incidents", "corp governance incidents --json"],
  },

  // --- governance profile ---
  {
    name: "governance profile",
    description: "View governance profile and configuration",
    route: { method: "GET", path: "/v1/entities/{eid}/governance/profile" },
    entity: true,
    display: { title: "Governance Profile" },
    handler: async (ctx) => {
      const eid = await ctx.resolver.resolveEntity(ctx.opts.entityId as string | undefined);
      const profile = await ctx.client.getGovernanceProfile(eid);
      if (ctx.opts.json) { ctx.writer.json(profile); return; }
      console.log(chalk.blue("\u2500".repeat(40)));
      console.log(chalk.blue.bold("  Governance Profile"));
      console.log(chalk.blue("\u2500".repeat(40)));
      for (const [key, value] of Object.entries(profile)) {
        if (typeof value === "string" || typeof value === "number" || typeof value === "boolean") {
          console.log(`  ${chalk.bold(key.replaceAll("_", " ") + ":")} ${value}`);
        }
      }
      console.log(chalk.blue("\u2500".repeat(40)));
    },
    examples: ["corp governance profile", "corp governance profile --json"],
  },

  // --- governance mode ---
  {
    name: "governance mode",
    description: "View or set governance mode",
    route: { method: "GET", path: "/v1/governance/mode" },
    entity: true,
    display: { title: "Governance Mode" },
    options: [
      {
        flags: "--set <mode>",
        description: "Set governance mode",
        choices: ["founder", "board", "executive", "normal", "incident_lockdown"],
      },
    ],
    handler: async (ctx) => {
      const eid = await ctx.resolver.resolveEntity(ctx.opts.entityId as string | undefined);
      const setMode = ctx.opts.set as string | undefined;
      if (setMode) {
        const result = await ctx.client.setGovernanceMode({ entity_id: eid, mode: setMode });
        if (ctx.opts.json) { ctx.writer.json(result); return; }
        ctx.writer.success(`Governance mode set to: ${setMode}`);
      } else {
        const result = await ctx.client.getGovernanceMode(eid);
        if (ctx.opts.json) { ctx.writer.json(result); return; }
        console.log(`  ${chalk.bold("Governance Mode:")} ${result.mode ?? "N/A"}`);
        if (result.reason) console.log(`  ${chalk.bold("Reason:")} ${result.reason}`);
      }
    },
    examples: ["corp governance mode", "corp governance mode --json"],
  },

  // --- governance create-body ---
  {
    name: "governance create-body",
    description: "Create a governance body",
    route: { method: "POST", path: "/v1/entities/{eid}/governance-bodies" },
    entity: true,
    dryRun: true,
    options: [
      { flags: "--name <name>", description: "Body name (e.g. 'Board of Directors')", required: true },
      { flags: "--body-type <type>", description: "Body type", required: true, choices: ["board_of_directors", "llc_member_vote"] },
      { flags: "--quorum <rule>", description: "Quorum rule", default: "majority", choices: ["majority", "supermajority", "unanimous"] },
      { flags: "--voting <method>", description: "Voting method", default: "per_capita", choices: ["per_capita", "per_unit"] },
    ],
    handler: async (ctx) => {
      const eid = await ctx.resolver.resolveEntity(ctx.opts.entityId as string | undefined);
      const payload = {
        entity_id: eid,
        body_type: ctx.opts.bodyType as string,
        name: ctx.opts.name as string,
        quorum_rule: ctx.opts.quorum as string,
        voting_method: ctx.opts.voting as string,
      };
      if (ctx.dryRun) {
        ctx.writer.dryRun("governance.create_body", payload);
        return;
      }
      const result = await ctx.client.createGovernanceBody(payload);
      await ctx.resolver.stabilizeRecord("body", result, eid);
      ctx.resolver.rememberFromRecord("body", result, eid);
      const bodyId = result.body_id ?? "OK";
      if (ctx.opts.json) { ctx.writer.json(result); return; }
      ctx.writer.success(`Governance body created: ${bodyId}`);
      printReferenceSummary("body", result, { showReuseHint: true });
      console.log(chalk.dim("\n  Next steps:"));
      console.log(chalk.dim(`    corp governance add-seat @last:body --holder <contact-ref>`));
      console.log(chalk.dim(`    corp governance seats @last:body`));
    },
    produces: { kind: "body" },
    successTemplate: "Governance body created: {name}",
    examples: ["corp governance create-body --name 'name' --body-type 'type'", "corp governance create-body --json"],
  },

  // --- governance add-seat <body-ref> ---
  {
    name: "governance add-seat",
    description: "Add a seat to a governance body",
    route: { method: "POST", path: "/v1/governance-bodies/{pos}/seats" },
    entity: true,
    dryRun: true,
    args: [{ name: "body-ref", required: true, description: "Governance body reference" }],
    options: [
      { flags: "--holder <contact-ref>", description: "Contact reference for the seat holder", required: true },
      { flags: "--role <role>", description: "Seat role (chair, member, officer, observer)", default: "member" },
    ],
    handler: async (ctx) => {
      const bodyRef = ctx.positional[0];
      const eid = await ctx.resolver.resolveEntity(ctx.opts.entityId as string | undefined);
      const resolvedBodyId = await ctx.resolver.resolveBody(eid, bodyRef);
      const resolvedHolderId = await ctx.resolver.resolveContact(eid, ctx.opts.holder as string);
      const data: Record<string, unknown> = { holder_id: resolvedHolderId, role: ctx.opts.role ?? "member" };
      if (ctx.dryRun) {
        ctx.writer.dryRun("governance.add_seat", { entity_id: eid, body_id: resolvedBodyId, ...data });
        return;
      }
      const result = await ctx.client.createGovernanceSeat(resolvedBodyId, eid, data);
      await ctx.resolver.stabilizeRecord("seat", result, eid);
      ctx.resolver.rememberFromRecord("seat", result, eid);
      if (ctx.opts.json) { ctx.writer.json(result); return; }
      ctx.writer.success(`Seat added: ${result.seat_id ?? "OK"}`);
      printReferenceSummary("seat", result, { showReuseHint: true });
    },
    produces: { kind: "seat" },
    successTemplate: "Seat added to {body_id}",
    examples: ["corp governance add-seat <body-ref> --holder 'contact-ref'", "corp governance add-seat --json"],
  },

  // --- governance convene ---
  {
    name: "governance convene",
    description: "Convene a governance meeting",
    route: { method: "POST", path: "/v1/meetings" },
    entity: true,
    dryRun: true,
    options: [
      { flags: "--body <ref>", description: "Governance body reference", required: true },
      { flags: "--type <type>", description: "Meeting type (board_meeting, shareholder_meeting, member_meeting, written_consent)", required: true },
      { flags: "--title <title>", description: "Meeting title", required: true },
      { flags: "--date <date>", description: "Meeting date (ISO 8601)" },
      { flags: "--agenda <item>", description: "Agenda item (repeatable)", type: "array" },
    ],
    handler: async (ctx) => {
      const eid = await ctx.resolver.resolveEntity(ctx.opts.entityId as string | undefined);
      const resolvedBodyId = await ctx.resolver.resolveBody(eid, ctx.opts.body as string);
      const payload: Record<string, unknown> = {
        entity_id: eid,
        body_id: resolvedBodyId,
        meeting_type: ctx.opts.type as string,
        title: ctx.opts.title as string,
        agenda_item_titles: (ctx.opts.agenda as string[]) ?? [],
      };
      if (ctx.opts.date) payload.scheduled_date = ctx.opts.date as string;
      if (ctx.dryRun) {
        ctx.writer.dryRun("governance.schedule_meeting", payload);
        return;
      }
      const result = await ctx.client.scheduleMeeting(payload);
      await ctx.resolver.stabilizeRecord("meeting", result, eid);
      ctx.resolver.rememberFromRecord("meeting", result, eid);
      const meetingId = result.meeting_id ?? "OK";
      if (ctx.opts.json) { ctx.writer.json(result); return; }
      ctx.writer.success(`Meeting scheduled: ${meetingId}`);
      printReferenceSummary("meeting", result, { showReuseHint: true });
      console.log(chalk.dim("\n  Next steps:"));
      console.log(chalk.dim(`    corp governance notice @last:meeting`));
      console.log(chalk.dim(`    corp governance open @last:meeting --present-seat <seat-ref>`));
      console.log(chalk.dim(`    corp governance agenda-items @last:meeting`));
    },
    produces: { kind: "meeting" },
    successTemplate: "Meeting scheduled: {title}",
    examples: ["corp governance convene --body 'ref' --type 'type' --title 'title'", "corp governance convene --json"],
  },

  // --- governance open <meeting-ref> ---
  {
    name: "governance open",
    description: "Open a scheduled meeting for voting",
    route: { method: "POST", path: "/v1/meetings/{pos}/open" },
    entity: true,
    dryRun: true,
    args: [{ name: "meeting-ref", required: true, description: "Meeting reference" }],
    options: [
      { flags: "--present-seat <ref>", description: "Seat reference present at the meeting (repeatable)", required: true, type: "array" },
    ],
    handler: async (ctx) => {
      const meetingRef = ctx.positional[0];
      const eid = await ctx.resolver.resolveEntity(ctx.opts.entityId as string | undefined);
      const resolvedMeetingId = await ctx.resolver.resolveMeeting(eid, meetingRef);
      const presentSeats = ctx.opts.presentSeat as string[];
      let resolvedSeats: string[];
      try {
        resolvedSeats = await Promise.all(
          presentSeats.map((seatRef) => ctx.resolver.resolveSeat(eid, seatRef)),
        );
      } catch (err) {
        throw new Error(
          `Failed to resolve seat reference: ${err}\n` +
          "  --present-seat expects seat IDs, not contact IDs.\n" +
          "  Find seat IDs with: corp governance seats <body-ref>",
        );
      }
      const payload = { present_seat_ids: resolvedSeats };
      if (ctx.dryRun) {
        ctx.writer.dryRun("governance.open_meeting", { entity_id: eid, meeting_id: resolvedMeetingId, ...payload });
        return;
      }
      const result = await ctx.client.conveneMeeting(resolvedMeetingId, eid, payload);
      if (ctx.opts.json) { ctx.writer.json(result); return; }
      ctx.writer.success(`Meeting opened: ${resolvedMeetingId}`);
    },
    examples: ["corp governance open <meeting-ref> --present-seat 'ref'"],
  },

  // --- governance vote <meeting-ref> <item-ref> ---
  {
    name: "governance vote",
    description: "Cast a vote on an agenda item",
    route: { method: "POST", path: "/v1/meetings/{pos}/agenda-items/{pos}/votes" },
    entity: true,
    dryRun: true,
    args: [
      { name: "meeting-ref", required: true, description: "Meeting reference" },
      { name: "item-ref", required: true, description: "Agenda item reference" },
    ],
    options: [
      { flags: "--voter <ref>", description: "Voter contact reference", required: true },
      { flags: "--vote <value>", description: "Vote (for, against, abstain, recusal)", required: true, choices: ["for", "against", "abstain", "recusal"] },
    ],
    handler: async (ctx) => {
      const meetingRef = ctx.positional[0];
      const itemRef = ctx.positional[1];
      const eid = await ctx.resolver.resolveEntity(ctx.opts.entityId as string | undefined);
      const resolvedMeetingId = await ctx.resolver.resolveMeeting(eid, meetingRef);
      const resolvedItemId = await ctx.resolver.resolveAgendaItem(eid, resolvedMeetingId, itemRef);
      const resolvedVoterId = await ctx.resolver.resolveContact(eid, ctx.opts.voter as string);
      const payload = { voter_id: resolvedVoterId, vote_value: ctx.opts.vote as string };
      if (ctx.dryRun) {
        ctx.writer.dryRun("governance.cast_vote", {
          entity_id: eid, meeting_id: resolvedMeetingId, agenda_item_id: resolvedItemId, ...payload,
        });
        return;
      }
      try {
        const result = await ctx.client.castVote(eid, resolvedMeetingId, resolvedItemId, payload);
        ctx.resolver.rememberFromRecord("agenda_item", { agenda_item_id: resolvedItemId, title: itemRef }, eid);
        if (ctx.opts.json) { ctx.writer.json(result); return; }
        ctx.writer.success(`Vote cast: ${result.vote_id ?? "OK"}`);
      } catch (err) {
        const message = String(err);
        if (message.includes("voting session is not open")) {
          ctx.writer.error(
            `Failed to cast vote: ${err}\n` +
            `  Open the meeting first: corp governance open ${meetingRef} --present-seat <seat-ref>`,
          );
        } else {
          throw err;
        }
      }
    },
    examples: ["corp governance vote <meeting-ref> <item-ref> --voter for --vote for"],
  },

  // --- governance notice <meeting-ref> ---
  {
    name: "governance notice",
    description: "Send meeting notice",
    route: { method: "POST", path: "/v1/meetings/{pos}/notice" },
    entity: true,
    dryRun: true,
    args: [{ name: "meeting-ref", required: true, description: "Meeting reference" }],
    handler: async (ctx) => {
      const meetingRef = ctx.positional[0];
      const eid = await ctx.resolver.resolveEntity(ctx.opts.entityId as string | undefined);
      const resolvedMeetingId = await ctx.resolver.resolveMeeting(eid, meetingRef);
      if (ctx.dryRun) {
        ctx.writer.dryRun("governance.send_notice", { entity_id: eid, meeting_id: resolvedMeetingId });
        return;
      }
      const result = await ctx.client.sendNotice(resolvedMeetingId, eid);
      if (ctx.opts.json) { ctx.writer.json(result); return; }
      ctx.writer.success(`Notice sent for meeting ${resolvedMeetingId}`);
    },
    examples: ["corp governance notice <meeting-ref>"],
  },

  // --- governance adjourn <meeting-ref> ---
  {
    name: "governance adjourn",
    description: "Adjourn a meeting",
    route: { method: "POST", path: "/v1/meetings/{pos}/adjourn" },
    entity: true,
    dryRun: true,
    args: [{ name: "meeting-ref", required: true, description: "Meeting reference" }],
    handler: async (ctx) => {
      const meetingRef = ctx.positional[0];
      const eid = await ctx.resolver.resolveEntity(ctx.opts.entityId as string | undefined);
      const resolvedMeetingId = await ctx.resolver.resolveMeeting(eid, meetingRef);
      if (ctx.dryRun) {
        ctx.writer.dryRun("governance.adjourn_meeting", { entity_id: eid, meeting_id: resolvedMeetingId });
        return;
      }
      const result = await ctx.client.adjournMeeting(resolvedMeetingId, eid);
      if (ctx.opts.json) { ctx.writer.json(result); return; }
      ctx.writer.success(`Meeting ${resolvedMeetingId} adjourned`);
    },
    examples: ["corp governance adjourn <meeting-ref>"],
  },

  // --- governance reopen <meeting-ref> ---
  {
    name: "governance reopen",
    description: "Re-open an adjourned meeting",
    route: { method: "POST", path: "/v1/meetings/{pos}/reopen" },
    entity: true,
    dryRun: true,
    args: [{ name: "meeting-ref", required: true, description: "Meeting reference" }],
    handler: async (ctx) => {
      const meetingRef = ctx.positional[0];
      const eid = await ctx.resolver.resolveEntity(ctx.opts.entityId as string | undefined);
      const resolvedMeetingId = await ctx.resolver.resolveMeeting(eid, meetingRef);
      if (ctx.dryRun) {
        ctx.writer.dryRun("governance.reopen_meeting", { entity_id: eid, meeting_id: resolvedMeetingId });
        return;
      }
      const result = await ctx.client.reopenMeeting(resolvedMeetingId, eid);
      if (ctx.opts.json) { ctx.writer.json(result); return; }
      ctx.writer.success(`Meeting ${resolvedMeetingId} re-opened`);
    },
    examples: ["corp governance reopen <meeting-ref>"],
  },

  // --- governance cancel <meeting-ref> ---
  {
    name: "governance cancel",
    description: "Cancel a meeting",
    route: { method: "POST", path: "/v1/meetings/{pos}/cancel" },
    entity: true,
    dryRun: true,
    args: [{ name: "meeting-ref", required: true, description: "Meeting reference" }],
    options: [
      { flags: "--yes, -y", description: "Skip confirmation prompt" },
    ],
    handler: async (ctx) => {
      const meetingRef = ctx.positional[0];
      const eid = await ctx.resolver.resolveEntity(ctx.opts.entityId as string | undefined);
      const resolvedMeetingId = await ctx.resolver.resolveMeeting(eid, meetingRef);
      if (ctx.dryRun) {
        ctx.writer.dryRun("governance.cancel_meeting", { entity_id: eid, meeting_id: resolvedMeetingId });
        return;
      }
      if (!ctx.opts.yes) {
        const ok = await confirm({
          message: `Cancel meeting ${resolvedMeetingId}?`,
          default: false,
        });
        if (!ok) {
          ctx.writer.writeln("Cancelled.");
          return;
        }
      }
      const result = await ctx.client.cancelMeeting(resolvedMeetingId, eid);
      if (ctx.opts.json) { ctx.writer.json(result); return; }
      ctx.writer.success(`Meeting ${resolvedMeetingId} cancelled`);
    },
    examples: ["corp governance cancel <meeting-ref>", "corp governance cancel --json"],
  },

  // --- governance finalize-item <meeting-ref> <item-ref> ---
  {
    name: "governance finalize-item",
    description: "Finalize an agenda item",
    route: { method: "POST", path: "/v1/meetings/{pos}/agenda-items/{pos}/finalize" },
    entity: true,
    dryRun: true,
    args: [
      { name: "meeting-ref", required: true, description: "Meeting reference" },
      { name: "item-ref", required: true, description: "Agenda item reference" },
    ],
    options: [
      {
        flags: "--status <status>",
        description: `Status (${[...FINALIZE_ITEM_STATUS_CHOICES].join(", ")})`,
        required: true,
        choices: [...FINALIZE_ITEM_STATUS_CHOICES],
      },
    ],
    handler: async (ctx) => {
      const meetingRef = ctx.positional[0];
      const itemRef = ctx.positional[1];
      const eid = await ctx.resolver.resolveEntity(ctx.opts.entityId as string | undefined);
      const resolvedMeetingId = await ctx.resolver.resolveMeeting(eid, meetingRef);
      const resolvedItemId = await ctx.resolver.resolveAgendaItem(eid, resolvedMeetingId, itemRef);
      const payload = { entity_id: eid, status: ctx.opts.status as string };
      if (ctx.dryRun) {
        ctx.writer.dryRun("governance.finalize_agenda_item", { meeting_id: resolvedMeetingId, agenda_item_id: resolvedItemId, ...payload });
        return;
      }
      const result = await ctx.client.finalizeAgendaItem(resolvedMeetingId, resolvedItemId, payload);
      if (ctx.opts.json) { ctx.writer.json(result); return; }
      ctx.writer.success(`Agenda item ${resolvedItemId} finalized as ${ctx.opts.status}`);
    },
    examples: ["corp governance finalize-item <meeting-ref> <item-ref>"],
  },

  // --- governance resolve <meeting-ref> <item-ref> ---
  {
    name: "governance resolve",
    description: "Compute a resolution for an agenda item",
    route: { method: "POST", path: "/v1/meetings/{pos}/agenda-items/{pos}/resolution" },
    entity: true,
    dryRun: true,
    args: [
      { name: "meeting-ref", required: true, description: "Meeting reference" },
      { name: "item-ref", required: true, description: "Agenda item reference" },
    ],
    options: [
      { flags: "--text <resolution_text>", description: "Resolution text", required: true },
    ],
    handler: async (ctx) => {
      const meetingRef = ctx.positional[0];
      const itemRef = ctx.positional[1];
      const eid = await ctx.resolver.resolveEntity(ctx.opts.entityId as string | undefined);
      const resolvedMeetingId = await ctx.resolver.resolveMeeting(eid, meetingRef);
      const resolvedItemId = await ctx.resolver.resolveAgendaItem(eid, resolvedMeetingId, itemRef);
      const payload = { resolution_text: ctx.opts.text as string };
      if (ctx.dryRun) {
        ctx.writer.dryRun("governance.compute_resolution", {
          entity_id: eid, meeting_id: resolvedMeetingId, agenda_item_id: resolvedItemId, ...payload,
        });
        return;
      }
      const result = await ctx.client.computeResolution(resolvedMeetingId, resolvedItemId, eid, payload);
      await ctx.resolver.stabilizeRecord("resolution", result, eid);
      ctx.resolver.rememberFromRecord("resolution", result, eid);
      if (ctx.opts.json) { ctx.writer.json(result); return; }
      ctx.writer.success(`Resolution computed for agenda item ${itemRef}`);
      printReferenceSummary("resolution", result, { showReuseHint: true });
    },
    produces: { kind: "resolution" },
    successTemplate: "Resolution computed",
    examples: ["corp governance resolve <meeting-ref> <item-ref> --text 'resolution_text'"],
  },

  // --- governance written-consent ---
  {
    name: "governance written-consent",
    description: "Create a written consent action",
    route: { method: "POST", path: "/v1/governance/written-consent" },
    entity: true,
    dryRun: true,
    options: [
      { flags: "--body <ref>", description: "Governance body reference", required: true },
      { flags: "--title <title>", description: "Title", required: true },
      { flags: "--description <desc>", description: "Description text", required: true },
    ],
    handler: async (ctx) => {
      const eid = await ctx.resolver.resolveEntity(ctx.opts.entityId as string | undefined);
      const resolvedBodyId = await ctx.resolver.resolveBody(eid, ctx.opts.body as string);
      const payload = {
        entity_id: eid, body_id: resolvedBodyId, title: ctx.opts.title as string, description: ctx.opts.description as string,
      };
      if (ctx.dryRun) {
        ctx.writer.dryRun("governance.written_consent", payload);
        return;
      }

      const result = await writtenConsentWorkflow(ctx.client, {
        entityId: eid,
        bodyId: resolvedBodyId,
        title: ctx.opts.title as string,
        description: ctx.opts.description as string,
      });

      if (!result.success) {
        ctx.writer.error(result.error!);
        return;
      }

      await ctx.resolver.stabilizeRecord("meeting", result.data!, eid);
      ctx.resolver.rememberFromRecord("meeting", result.data!, eid);
      const meetingId = String(result.data?.meeting_id ?? "");

      if (ctx.opts.json) { ctx.writer.json(result.data); return; }
      ctx.writer.success(`Written consent created: ${meetingId || "OK"}`);
      printReferenceSummary("meeting", result.data!, { showReuseHint: true });
      console.log(chalk.dim("\n  Next steps:"));
      console.log(chalk.dim(`    corp governance agenda-items @last:meeting`));
      console.log(chalk.dim(`    corp governance vote @last:meeting <item-ref> --voter <contact-ref> --vote for`));
    },
    produces: { kind: "meeting" },
    successTemplate: "Written consent created: {title}",
    examples: ["corp governance written-consent --body 'ref' --title 'title' --description 'desc'"],
  },

  // --- governance quick-approve ---
  {
    name: "governance quick-approve",
    description: "One-step board approval: create written consent, auto-vote, return meeting + resolution IDs",
    entity: true,
    dryRun: true,
    options: [
      { flags: "--body <ref>", description: "Governance body reference (auto-detected if only one exists)" },
      { flags: "--text <resolution_text>", description: "Resolution text (e.g. 'RESOLVED: authorize SAFE issuance')", required: true },
    ],
    handler: async (ctx) => {
      const eid = await ctx.resolver.resolveEntity(ctx.opts.entityId as string | undefined);

      // Auto-detect body if not specified
      let resolvedBodyId: string;
      if (ctx.opts.body) {
        resolvedBodyId = await ctx.resolver.resolveBody(eid, ctx.opts.body as string);
      } else {
        const bodies = await ctx.client.listGovernanceBodies(eid);
        const active = bodies.filter((b: ApiRecord) => b.status === "active");
        if (active.length === 1) {
          resolvedBodyId = String(active[0].body_id);
        } else if (active.length === 0) {
          throw new Error("No active governance bodies found. Create one first: corp governance create-body");
        } else {
          throw new Error(`Multiple governance bodies found (${active.length}). Specify --body <ref>.`);
        }
      }

      const resolutionText = ctx.opts.text as string;
      const title = `Board Approval: ${resolutionText.slice(0, 60)}`;

      if (ctx.dryRun) {
        ctx.writer.dryRun("governance.quick_approve", {
          entity_id: eid, body_id: resolvedBodyId, title, resolution_text: resolutionText,
        });
        return;
      }

      // Step 1: Create written consent
      const consentResult = await writtenConsentWorkflow(ctx.client, {
        entityId: eid,
        bodyId: resolvedBodyId,
        title,
        description: resolutionText,
      });
      if (!consentResult.success) {
        throw new Error(`Written consent failed: ${consentResult.error}`);
      }
      const meetingId = String(consentResult.data?.meeting_id);
      ctx.resolver.remember("meeting", meetingId, eid);

      // Step 2: Get agenda items
      const agendaItems = await ctx.client.listAgendaItems(meetingId, eid);
      if (agendaItems.length === 0) {
        throw new Error("Written consent created but no agenda items found.");
      }
      const itemId = String((agendaItems[0] as ApiRecord).agenda_item_id);

      // Step 3: Convene (open) the meeting if not already convened
      const seats = await ctx.client.getGovernanceSeats(resolvedBodyId, eid);
      const filledSeats = seats.filter((s: ApiRecord) => s.status === "filled" || s.status === "active");
      const seatIds = filledSeats.map((s: ApiRecord) => String(s.seat_id));
      if (seatIds.length === 0) {
        throw new Error("No filled seats found on this governance body. Add seats first: corp governance add-seat <body-ref>");
      }
      // Written consent creates meetings already in "convened" state — skip if so
      const meetingStatus = consentResult.data?.status ?? consentResult.data?.meeting_status;
      if (meetingStatus !== "convened") {
        await ctx.client.conveneMeeting(meetingId, eid, { present_seat_ids: seatIds });
      }

      // Step 4: Cast votes — all seated members vote "for"
      for (const seatId of seatIds) {
        try {
          await ctx.client.castVote(eid, meetingId, itemId, {
            seat_id: seatId,
            vote: "for",
          });
        } catch {
          // Seat may have already voted or be ineligible — continue
        }
      }

      // Step 5: Compute resolution (tallies votes and determines outcome)
      const resolution = await ctx.client.computeResolution(meetingId, itemId, eid, {
        resolution_text: resolutionText,
      });
      const resolutionId = String(resolution.resolution_id);
      ctx.resolver.remember("resolution", resolutionId, eid);

      const passed = resolution.passed === true;

      if (ctx.opts.json) {
        ctx.writer.json({ meeting_id: meetingId, resolution_id: resolutionId, passed, votes: filledSeats.length });
        return;
      }
      ctx.writer.success(passed ? "Board approval completed" : "Board vote completed (resolution did not pass)");
      console.log(`  Meeting:    ${meetingId}`);
      console.log(`  Resolution: ${resolutionId}`);
      console.log(chalk.dim("\n  Use with:"));
      console.log(chalk.dim(`    --meeting-id ${meetingId} --resolution-id ${resolutionId}`));
      console.log(chalk.dim(`    or: --meeting-id @last:meeting --resolution-id @last:resolution`));
    },
    produces: { kind: "resolution" },
    successTemplate: "Board approval completed",
    examples: [
      'corp governance quick-approve --text "RESOLVED: authorize SAFE issuance to Seed Fund"',
      'corp governance quick-approve --body @last:body --text "RESOLVED: issue Series A equity round"',
    ],
  },

  // --- governance resign <seat-ref> ---
  {
    name: "governance resign",
    description: "Resign from a governance seat",
    route: { method: "POST", path: "/v1/seats/{pos}/resign" },
    entity: true,
    dryRun: true,
    args: [{ name: "seat-ref", required: true, description: "Seat reference" }],
    options: [
      { flags: "--body-id <ref>", description: "Governance body reference" },
    ],
    handler: async (ctx) => {
      const seatRef = ctx.positional[0];
      const eid = await ctx.resolver.resolveEntity(ctx.opts.entityId as string | undefined);
      const seatId = await ctx.resolver.resolveSeat(eid, seatRef, ctx.opts.bodyId as string | undefined);
      const result = await ctx.client.resignSeat(seatId, eid);
      if (ctx.opts.json) { ctx.writer.json(result); return; }
      ctx.writer.success(`Seat ${seatId} resigned.`);
    },
    examples: ["corp governance resign <seat-ref>", "corp governance resign --json"],
  },

  // ── Auto-generated from OpenAPI ──────────────────────────────
  {
    name: "entities governance-audit-checkpoints",
    description: "List governance audit checkpoints",
    route: { method: "GET", path: "/v1/entities/{eid}/governance/audit/checkpoints" },
    entity: true,
    display: { title: "Entities Governance Audit Checkpoints", cols: ["#checkpoint_id>ID", "@created_at>Created At", "#entity_id>ID", "latest_entry_hash>Latest Entry Hash", "#latest_entry_id>ID", "total_entries>Total Entries"] },
    examples: ["corp entities governance-audit-checkpoints", "corp entities governance-audit-checkpoints --json"],
  },
  {
    name: "entities governance-audit-entries",
    description: "List governance audit log entries",
    route: { method: "GET", path: "/v1/entities/{eid}/governance/audit/entries" },
    entity: true,
    display: { title: "Entities Governance Audit Entries", cols: ["action>Action", "details>Details", "entry_hash>Entry Hash", "event_type>Event Type", "@created_at>Created At", "#audit_entry_id>ID"] },
    examples: ["corp entities governance-audit-entries", "corp entities governance-audit-entries --json"],
  },
  {
    name: "entities governance-audit-verifications",
    description: "List governance audit verifications",
    route: { method: "GET", path: "/v1/entities/{eid}/governance/audit/verifications" },
    entity: true,
    display: { title: "Entities Governance Audit Verifications", cols: ["anomalies>Anomalies", "latest_entry_hash>Latest Entry Hash", "ok>Ok", "total_entries>Total Entries", "@created_at>Created At", "#entity_id>ID"] },
    examples: ["corp entities governance-audit-verifications", "corp entities governance-audit-verifications --json"],
  },
  {
    name: "entities governance-doc-bundles",
    description: "List governance document bundles",
    route: { method: "GET", path: "/v1/entities/{eid}/governance/doc-bundles" },
    entity: true,
    display: { title: "Entities Governance Doc Bundles", cols: ["document_count>Document Count", "entity_type>Entity Type", "generated_at>Generated At", "profile_version>Profile Version", "#bundle_id>ID"] },
    examples: ["corp entities governance-doc-bundles", "corp entities governance-doc-bundles --json"],
  },
  {
    name: "entities governance-doc-bundles-current",
    description: "View the current governance document bundle",
    route: { method: "GET", path: "/v1/entities/{eid}/governance/doc-bundles/current" },
    entity: true,
    display: { title: "Entities Governance Doc Bundles Current", cols: ["#bundle_id>ID", "#entity_id>ID", "generated_at>Generated At", "manifest_path>Manifest Path", "template_version>Template Version"] },
    examples: ["corp entities governance-doc-bundles-current", "corp entities governance-doc-bundles-current --json"],
  },
  {
    name: "entities governance-doc-bundles-generate",
    description: "Generate a new governance document bundle",
    route: { method: "POST", path: "/v1/entities/{eid}/governance/doc-bundles/generate" },
    entity: true,
    options: [
      { flags: "--template-version <template-version>", description: "Template Version" },
    ],
    examples: ["corp entities governance-doc-bundles-generate", "corp entities governance-doc-bundles-generate --json"],
    successTemplate: "Governance Doc Bundles Generate created",
  },
  {
    name: "entities governance-doc-bundle",
    description: "List governance document bundles",
    route: { method: "GET", path: "/v1/entities/{eid}/governance/doc-bundles/{pos}" },
    entity: true,
    args: [{ name: "bundle-id", required: true, description: "Document bundle ID" }],
    display: { title: "Entities Governance Doc Bundles", cols: ["documents>Documents", "entity_type>Entity Type", "generated_at>Generated At", "profile_version>Profile Version", "#bundle_id>ID"] },
    examples: ["corp entities governance-doc-bundles", "corp entities governance-doc-bundles --json"],
  },
  {
    name: "entities governance-mode-history",
    description: "View governance mode change history",
    route: { method: "GET", path: "/v1/entities/{eid}/governance/mode-history" },
    entity: true,
    display: { title: "Entities Governance Mode History", cols: ["evidence_refs>Evidence Refs", "from_mode>From Mode", "incident_ids>Incident Ids", "reason>Reason", "@created_at>Created At", "#entity_id>ID"] },
    examples: ["corp entities governance-mode-history", "corp entities governance-mode-history --json"],
  },
  {
    name: "entities governance-triggers",
    description: "List governance triggers for an entity",
    route: { method: "GET", path: "/v1/entities/{eid}/governance/triggers" },
    entity: true,
    display: { title: "Entities Governance Triggers", cols: ["description>Description", "evidence_refs>Evidence Refs", "idempotency_key_hash>Idempotency Key Hash", "@created_at>Created At", "#entity_id>ID"] },
    examples: ["corp entities governance-triggers", "corp entities governance-triggers --json"],
  },
  {
    name: "governance-bodies",
    description: "List all governance bodies",
    route: { method: "GET", path: "/v1/governance-bodies" },
    entity: true,
    display: { title: "Governance Bodies", cols: ["body_type>Body Type", "name>Name", "quorum_rule>Quorum Rule", "status>Status", "@created_at>Created At", "#body_id>ID"] },
    examples: ["corp governance-bodies", "corp governance-bodies --json"],
  },
  {
    name: "governance-bodies create",
    description: "List all governance bodies",
    route: { method: "POST", path: "/v1/governance-bodies" },
    options: [
      { flags: "--body-type <body-type>", description: "The type of governance body.", required: true, choices: ["board_of_directors", "llc_member_vote"] },
      { flags: "--name <name>", description: "Display name", required: true },
      { flags: "--quorum-rule <quorum-rule>", description: "The threshold required for a vote to pass.", required: true, choices: ["majority", "supermajority", "unanimous"] },
      { flags: "--voting-method <voting-method>", description: "How votes are counted.", required: true, choices: ["per_capita", "per_unit"] },
    ],
    examples: ["corp governance-bodies --body-type board_of_directors --name majority --quorum-rule majority --voting-method per_capita"],
    successTemplate: "Governance Bodies created",
  },
  {
    name: "governance-seats scan-expired",
    description: "Scan for and flag expired governance seats",
    route: { method: "POST", path: "/v1/governance-seats/scan-expired" },
    entity: true,
    examples: ["corp governance-seats scan-expired"],
    successTemplate: "Scan Expired created",
  },
  {
    name: "governance-seats resign",
    description: "Resign from a governance seat",
    route: { method: "POST", path: "/v1/governance-seats/{pos}/resign" },
    entity: true,
    args: [{ name: "seat-id", required: true, description: "Governance seat ID" }],
    examples: ["corp governance-seats resign <seat-id>"],
    successTemplate: "Resign created",
  },
  {
    name: "governance audit-checkpoints",
    description: "Create a governance audit checkpoint",
    route: { method: "POST", path: "/v1/governance/audit/checkpoints" },
    entity: true,
    examples: ["corp governance audit-checkpoints"],
    successTemplate: "Audit Checkpoints created",
  },
  {
    name: "governance audit-events",
    description: "Record a governance audit event",
    route: { method: "POST", path: "/v1/governance/audit/events" },
    entity: true,
    options: [
      { flags: "--action <action>", description: "Action", required: true },
      { flags: "--details <details>", description: "Details" },
      { flags: "--event-type <event-type>", description: "Event Type", required: true, choices: ["mode_changed", "lockdown_trigger_applied", "manual_event", "checkpoint_written", "chain_verified", "chain_verification_failed"] },
      { flags: "--evidence-refs <evidence-refs>", description: "Evidence Refs", type: "array" },
      { flags: "--linked-incident-id <linked-incident-id>", description: "Linked Incident Id" },
      { flags: "--linked-intent-id <linked-intent-id>", description: "Linked Intent Id" },
      { flags: "--linked-mode-event-id <linked-mode-event-id>", description: "Linked Mode Event Id" },
      { flags: "--linked-trigger-id <linked-trigger-id>", description: "Linked Trigger Id" },
    ],
    examples: ["corp governance audit-events --action 'action' --event-type mode_changed", "corp governance audit-events --json"],
    successTemplate: "Audit Events created",
  },
  {
    name: "governance audit-verify",
    description: "Verify governance state integrity",
    route: { method: "POST", path: "/v1/governance/audit/verify" },
    entity: true,
    examples: ["corp governance audit-verify"],
    successTemplate: "Audit Verify created",
  },
  {
    name: "governance delegation-schedule",
    description: "View the current delegation schedule",
    route: { method: "GET", path: "/v1/governance/delegation-schedule" },
    entity: true,
    display: { title: "Governance Delegation Schedule", cols: ["allowed_tier1_intent_types>Allowed Tier1 Intent Types", "last_reauthorized_at>Last Reauthorized At", "next_mandatory_review_at>Next Mandatory Review At", "reauth_full_suspension_at_days>Reauth Full Suspension At Days", "@created_at>Created At", "#adopted_resolution_id>ID"] },
    examples: ["corp governance delegation-schedule", "corp governance delegation-schedule --json"],
  },
  {
    name: "governance delegation-schedule-amend",
    description: "Amend the delegation schedule",
    route: { method: "POST", path: "/v1/governance/delegation-schedule/amend" },
    entity: true,
    options: [
      { flags: "--adopted-resolution-id <adopted-resolution-id>", description: "Adopted Resolution Id" },
      { flags: "--allowed-tier1-intent-types <allowed-tier1-intent-types>", description: "Allowed Tier1 Intent Types" },
      { flags: "--meeting-id <meeting-id>", description: "Meeting ID" },
      { flags: "--next-mandatory-review-at <next-mandatory-review-at>", description: "Next Mandatory Review At" },
      { flags: "--rationale <rationale>", description: "Rationale" },
      { flags: "--tier1-max-amount-cents <tier1-max-amount-cents>", description: "Tier1 Max Amount Cents" },
    ],
    examples: ["corp governance delegation-schedule-amend", "corp governance delegation-schedule-amend --json"],
    successTemplate: "Delegation Schedule Amend created",
  },
  {
    name: "governance delegation-schedule-history",
    description: "View delegation schedule change history",
    route: { method: "GET", path: "/v1/governance/delegation-schedule/history" },
    entity: true,
    display: { title: "Governance Delegation Schedule History", cols: ["added_tier1_intent_types>Added Tier1 Intent Types", "authority_expansion>Authority Expansion", "from_version>From Version", "new_tier1_max_amount_cents>New Tier1 Max Amount Cents", "@created_at>Created At", "#adopted_resolution_id>ID"] },
    examples: ["corp governance delegation-schedule-history", "corp governance delegation-schedule-history --json"],
  },
  {
    name: "governance delegation-schedule-reauthorize",
    description: "Reauthorize the delegation schedule",
    route: { method: "POST", path: "/v1/governance/delegation-schedule/reauthorize" },
    entity: true,
    options: [
      { flags: "--adopted-resolution-id <adopted-resolution-id>", description: "Adopted Resolution Id", required: true },
      { flags: "--meeting-id <meeting-id>", description: "Meeting ID", required: true },
      { flags: "--rationale <rationale>", description: "Rationale" },
    ],
    examples: ["corp governance delegation-schedule-reauthorize --adopted-resolution-id 'adopted-resolution-id' --meeting-id 'meeting-id'", "corp governance delegation-schedule-reauthorize --json"],
    successTemplate: "Delegation Schedule Reauthorize created",
  },
  {
    name: "governance evaluate",
    description: "Evaluate governance compliance for an action",
    route: { method: "POST", path: "/v1/governance/evaluate" },
    entity: true,
    options: [
      { flags: "--intent-type <intent-type>", description: "Type of intent", required: true },
      { flags: "--metadata <metadata>", description: "Additional metadata (JSON)" },
    ],
    examples: ["corp governance evaluate --intent-type 'intent-type'", "corp governance evaluate --json"],
    successTemplate: "Evaluate created",
  },
  {
    name: "governance report-incident",
    description: "Report a governance incident",
    route: { method: "POST", path: "/v1/governance/incidents" },
    entity: true,
    options: [
      { flags: "--description <description>", description: "Description text", required: true },
      { flags: "--severity <severity>", description: "Severity level", required: true, choices: ["low", "medium", "high", "critical"] },
      { flags: "--title <title>", description: "Title", required: true },
    ],
    examples: ["corp governance incidents --description low --severity low --title 'title'"],
    successTemplate: "Incidents created",
  },
  {
    name: "governance incidents-resolve",
    description: "Resolve a governance incident",
    route: { method: "POST", path: "/v1/governance/incidents/{pos}/resolve" },
    entity: true,
    args: [{ name: "incident-id", required: true, description: "Incident ID" }],
    examples: ["corp governance incidents-resolve <incident-id>"],
    successTemplate: "Incidents Resolve created",
  },
  {
    name: "meetings written-consent",
    description: "Create a written consent resolution (no meeting)",
    route: { method: "POST", path: "/v1/meetings/written-consent" },
    options: [
      { flags: "--body-id <body-id>", description: "Governance body ID", required: true },
      { flags: "--description <description>", description: "Description text", required: true },
      { flags: "--title <title>", description: "Title", required: true },
    ],
    examples: ["corp meetings written-consent --body-id 'body-id' --description 'description' --title 'title'"],
    successTemplate: "Written Consent created",
  },
  {
    name: "meetings agenda-items-vote",
    description: "Cast a vote on a meeting agenda item",
    route: { method: "POST", path: "/v1/meetings/{pos}/agenda-items/{pos2}/vote" },
    entity: true,
    args: [{ name: "meeting-id", required: true, description: "Meeting ID" }, { name: "item-id", required: true, description: "Agenda item ID" }],
    options: [
      { flags: "--vote-value <vote-value>", description: "How a participant voted.", required: true, choices: ["for", "against", "abstain", "recusal"] },
      { flags: "--voter-id <voter-id>", description: "Voter Id", required: true },
    ],
    examples: ["corp meetings agenda-items-vote <meeting-id> <item-id> --vote-value for --voter-id 'voter-id'"],
    successTemplate: "Agenda Items Vote created",
  },
  {
    name: "meetings convene",
    description: "Convene a scheduled meeting (start it)",
    route: { method: "POST", path: "/v1/meetings/{pos}/convene" },
    entity: true,
    args: [{ name: "meeting-id", required: true, description: "Meeting ID" }],
    options: [
      { flags: "--present-seat-ids <present-seat-ids>", description: "Present Seat Ids", required: true, type: "array" },
    ],
    examples: ["corp meetings convene <meeting-id> --present-seat-ids 'present-seat-ids'"],
    successTemplate: "Convene created",
  },
  {
    name: "meetings resolutions-attach-document",
    description: "Attach a document to a meeting resolution",
    route: { method: "POST", path: "/v1/meetings/{pos}/resolutions/{pos2}/attach-document" },
    args: [{ name: "meeting-id", required: true, description: "Meeting ID" }, { name: "resolution-id", required: true, description: "Resolution ID" }],
    options: [
      { flags: "--document-id <document-id>", description: "Document ID", required: true },
    ],
    examples: ["corp meetings resolutions-attach-document <meeting-id> <resolution-id> --document-id 'document-id'"],
    successTemplate: "Resolutions Attach Document created",
  },

];