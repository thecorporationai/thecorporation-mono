import chalk from "chalk";
import type { CommandDef, CommandContext } from "./types.js";
import {
  printGovernanceTable,
  printSeatsTable,
  printMeetingsTable,
  printResolutionsTable,
  printAgendaItemsTable,
  printReferenceSummary,
} from "../output.js";
import { confirm } from "@inquirer/prompts";

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
      const eid = await ctx.resolver.resolveEntity(ctx.opts.entityId as string | undefined);
      const resolvedBodyId = await ctx.resolver.resolveBody(eid, bodyRef);
      const seats = await ctx.client.getGovernanceSeats(resolvedBodyId, eid);
      await ctx.resolver.stabilizeRecords("seat", seats, eid);
      if (ctx.opts.json) { ctx.writer.json(seats); return; }
      if (seats.length === 0) { ctx.writer.writeln("No seats found."); return; }
      printSeatsTable(seats);
    },
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
      const eid = await ctx.resolver.resolveEntity(ctx.opts.entityId as string | undefined);
      const resolvedBodyId = await ctx.resolver.resolveBody(eid, bodyRef);
      const meetings = await ctx.client.listMeetings(resolvedBodyId, eid);
      await ctx.resolver.stabilizeRecords("meeting", meetings, eid);
      if (ctx.opts.json) { ctx.writer.json(meetings); return; }
      if (meetings.length === 0) { ctx.writer.writeln("No meetings found."); return; }
      printMeetingsTable(meetings);
    },
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
      const eid = await ctx.resolver.resolveEntity(ctx.opts.entityId as string | undefined);
      const resolvedMeetingId = await ctx.resolver.resolveMeeting(eid, meetingRef);
      const resolutions = await ctx.client.getMeetingResolutions(resolvedMeetingId, eid);
      await ctx.resolver.stabilizeRecords("resolution", resolutions, eid);
      if (ctx.opts.json) { ctx.writer.json(resolutions); return; }
      if (resolutions.length === 0) { ctx.writer.writeln("No resolutions found."); return; }
      printResolutionsTable(resolutions);
    },
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
      const eid = await ctx.resolver.resolveEntity(ctx.opts.entityId as string | undefined);
      const resolvedMeetingId = await ctx.resolver.resolveMeeting(eid, meetingRef);
      const items = await ctx.client.listAgendaItems(resolvedMeetingId, eid);
      await ctx.resolver.stabilizeRecords("agenda_item", items, eid);
      if (ctx.opts.json) { ctx.writer.json(items); return; }
      if (items.length === 0) { ctx.writer.writeln("No agenda items found."); return; }
      printAgendaItemsTable(items);
    },
  },

  // --- governance incidents ---
  {
    name: "governance incidents",
    description: "List governance incidents",
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
      { flags: "--body-type <type>", description: "Body type (board_of_directors, llc_member_vote)", required: true },
      { flags: "--quorum <rule>", description: "Quorum rule (majority, supermajority, unanimous)", default: "majority" },
      { flags: "--voting <method>", description: "Voting method (per_capita, per_unit)", default: "per_capita" },
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
      const resolvedSeats = await Promise.all(
        presentSeats.map((seatRef) => ctx.resolver.resolveSeat(eid, seatRef)),
      );
      const payload = { present_seat_ids: resolvedSeats };
      if (ctx.dryRun) {
        ctx.writer.dryRun("governance.open_meeting", { entity_id: eid, meeting_id: resolvedMeetingId, ...payload });
        return;
      }
      const result = await ctx.client.conveneMeeting(resolvedMeetingId, eid, payload);
      if (ctx.opts.json) { ctx.writer.json(result); return; }
      ctx.writer.success(`Meeting opened: ${resolvedMeetingId}`);
    },
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
      { flags: "--description <desc>", description: "Description", required: true },
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
      const result = await ctx.client.writtenConsent(payload);
      await ctx.resolver.stabilizeRecord("meeting", result, eid);
      ctx.resolver.rememberFromRecord("meeting", result, eid);
      const meetingId = String(result.meeting_id ?? "");

      // Auto-open written consent with all body seats present for quorum evaluation
      if (meetingId) {
        try {
          const seats = await ctx.client.getGovernanceSeats(resolvedBodyId, eid);
          const seatIds = seats
            .map((s) => String(s.seat_id ?? (s as Record<string, unknown>).id ?? ""))
            .filter((id) => id.length > 0);
          if (seatIds.length > 0) {
            await ctx.client.conveneMeeting(meetingId, eid, { present_seat_ids: seatIds });
          }
        } catch {
          // Non-fatal: written consent can still proceed without open step
        }
      }

      if (ctx.opts.json) { ctx.writer.json(result); return; }
      ctx.writer.success(`Written consent created: ${meetingId || "OK"}`);
      printReferenceSummary("meeting", result, { showReuseHint: true });
      console.log(chalk.dim("\n  Next steps:"));
      console.log(chalk.dim(`    corp governance agenda-items @last:meeting`));
      console.log(chalk.dim(`    corp governance vote @last:meeting <item-ref> --voter <contact-ref> --vote for`));
    },
    produces: { kind: "meeting" },
    successTemplate: "Written consent created: {title}",
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
  },
];
