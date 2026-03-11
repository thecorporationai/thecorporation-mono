import { requireConfig } from "../config.js";
import { CorpAPIClient } from "../api-client.js";
import {
  printAgendaItemsTable,
  printGovernanceTable, printSeatsTable, printMeetingsTable,
  printResolutionsTable, printDryRun, printError, printReferenceSummary, printSuccess, printJson,
} from "../output.js";
import { ReferenceResolver } from "../references.js";
import chalk from "chalk";

export async function governanceCreateBodyCommand(opts: {
  entityId?: string; name: string; bodyType: string; quorum: string; voting: string;
  json?: boolean; dryRun?: boolean;
}): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const eid = await resolver.resolveEntity(opts.entityId);
    const payload = {
      entity_id: eid,
      body_type: opts.bodyType,
      name: opts.name,
      quorum_rule: opts.quorum,
      voting_method: opts.voting,
    };
    if (opts.dryRun) {
      printDryRun("governance.create_body", payload);
      return;
    }
    const result = await client.createGovernanceBody(payload);
    await resolver.stabilizeRecord("body", result, eid);
    resolver.rememberFromRecord("body", result, eid);
    const bodyId = result.body_id ?? "OK";
    if (opts.json) {
      printJson(result);
      return;
    }
    printSuccess(`Governance body created: ${bodyId}`);
    printReferenceSummary("body", result, { showReuseHint: true });
    printJson(result);
    console.log(chalk.dim("\n  Next steps:"));
    console.log(chalk.dim(`    corp governance add-seat @last:body --holder <contact-ref>`));
    console.log(chalk.dim(`    corp governance seats @last:body`));
  } catch (err) { printError(`Failed to create governance body: ${err}`); process.exit(1); }
}

export async function governanceAddSeatCommand(bodyId: string, opts: {
  holder: string; role?: string; entityId?: string; json?: boolean; dryRun?: boolean;
}): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const eid = await resolver.resolveEntity(opts.entityId);
    const resolvedBodyId = await resolver.resolveBody(eid, bodyId);
    const resolvedHolderId = await resolver.resolveContact(eid, opts.holder);
    const data: Record<string, unknown> = { holder_id: resolvedHolderId, role: opts.role ?? "member" };
    if (opts.dryRun) {
      printDryRun("governance.add_seat", { entity_id: eid, body_id: resolvedBodyId, ...data });
      return;
    }
    const result = await client.createGovernanceSeat(resolvedBodyId, eid, data);
    await resolver.stabilizeRecord("seat", result, eid);
    resolver.rememberFromRecord("seat", result, eid);
    if (opts.json) {
      printJson(result);
      return;
    }
    printSuccess(`Seat added: ${result.seat_id ?? "OK"}`);
    printReferenceSummary("seat", result, { showReuseHint: true });
    printJson(result);
  } catch (err) { printError(`Failed to add seat: ${err}`); process.exit(1); }
}

export async function governanceListCommand(opts: { entityId?: string; json?: boolean }): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const eid = await resolver.resolveEntity(opts.entityId);
    const bodies = await client.listGovernanceBodies(eid);
    await resolver.stabilizeRecords("body", bodies, eid);
    if (opts.json) printJson(bodies);
    else if (bodies.length === 0) console.log("No governance bodies found.");
    else printGovernanceTable(bodies);
  } catch (err) { printError(`Failed to fetch governance bodies: ${err}`); process.exit(1); }
}

export async function governanceSeatsCommand(bodyId: string, opts: { entityId?: string; json?: boolean }): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const eid = await resolver.resolveEntity(opts.entityId);
    const resolvedBodyId = await resolver.resolveBody(eid, bodyId);
    const seats = await client.getGovernanceSeats(resolvedBodyId, eid);
    await resolver.stabilizeRecords("seat", seats, eid);
    if (opts.json) printJson(seats);
    else if (seats.length === 0) console.log("No seats found.");
    else printSeatsTable(seats);
  } catch (err) { printError(`Failed to fetch seats: ${err}`); process.exit(1); }
}

export async function governanceMeetingsCommand(bodyId: string, opts: { entityId?: string; json?: boolean }): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const eid = await resolver.resolveEntity(opts.entityId);
    const resolvedBodyId = await resolver.resolveBody(eid, bodyId);
    const meetings = await client.listMeetings(resolvedBodyId, eid);
    await resolver.stabilizeRecords("meeting", meetings, eid);
    if (opts.json) printJson(meetings);
    else if (meetings.length === 0) console.log("No meetings found.");
    else printMeetingsTable(meetings);
  } catch (err) { printError(`Failed to fetch meetings: ${err}`); process.exit(1); }
}

export async function governanceResolutionsCommand(meetingId: string, opts: { entityId?: string; json?: boolean }): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const eid = await resolver.resolveEntity(opts.entityId);
    const resolvedMeetingId = await resolver.resolveMeeting(eid, meetingId);
    const resolutions = await client.getMeetingResolutions(resolvedMeetingId, eid);
    await resolver.stabilizeRecords("resolution", resolutions, eid);
    if (opts.json) printJson(resolutions);
    else if (resolutions.length === 0) console.log("No resolutions found.");
    else printResolutionsTable(resolutions);
  } catch (err) { printError(`Failed to fetch resolutions: ${err}`); process.exit(1); }
}

export async function governanceConveneCommand(opts: {
  entityId?: string; body: string; meetingType: string; title: string; date?: string; agenda: string[];
  json?: boolean; dryRun?: boolean;
}): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const eid = await resolver.resolveEntity(opts.entityId);
    const resolvedBodyId = await resolver.resolveBody(eid, opts.body);
    const payload: Record<string, unknown> = {
      entity_id: eid, body_id: resolvedBodyId, meeting_type: opts.meetingType,
      title: opts.title,
      agenda_item_titles: opts.agenda,
    };
    if (opts.date) payload.scheduled_date = opts.date;
    if (opts.dryRun) {
      printDryRun("governance.schedule_meeting", payload);
      return;
    }
    const result = await client.scheduleMeeting(payload);
    await resolver.stabilizeRecord("meeting", result, eid);
    resolver.rememberFromRecord("meeting", result, eid);
    const meetingId = result.meeting_id ?? "OK";
    if (opts.json) {
      printJson(result);
      return;
    }
    printSuccess(`Meeting scheduled: ${meetingId}`);
    printReferenceSummary("meeting", result, { showReuseHint: true });
    printJson(result);
    console.log(chalk.dim("\n  Next steps:"));
    console.log(chalk.dim(`    corp governance notice @last:meeting`));
    console.log(chalk.dim(`    corp governance open @last:meeting --present-seat <seat-ref>`));
    console.log(chalk.dim(`    corp governance agenda-items @last:meeting`));
  } catch (err) { printError(`Failed to schedule meeting: ${err}`); process.exit(1); }
}

export async function governanceOpenMeetingCommand(
  meetingId: string,
  opts: { entityId?: string; presentSeat: string[]; json?: boolean; dryRun?: boolean },
): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const eid = await resolver.resolveEntity(opts.entityId);
    const resolvedMeetingId = await resolver.resolveMeeting(eid, meetingId);
    const resolvedSeats = await Promise.all(
      opts.presentSeat.map((seatRef) => resolver.resolveSeat(eid, seatRef)),
    );
    const payload = { present_seat_ids: resolvedSeats };
    if (opts.dryRun) {
      printDryRun("governance.open_meeting", { entity_id: eid, meeting_id: resolvedMeetingId, ...payload });
      return;
    }
    const result = await client.conveneMeeting(resolvedMeetingId, eid, payload);
    if (opts.json) {
      printJson(result);
      return;
    }
    printSuccess(`Meeting opened: ${resolvedMeetingId}`);
    printJson(result);
  } catch (err) { printError(`Failed to open meeting: ${err}`); process.exit(1); }
}

export async function governanceVoteCommand(
  meetingId: string,
  itemId: string,
  opts: { voter: string; vote: string; entityId?: string; json?: boolean; dryRun?: boolean }
): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const eid = await resolver.resolveEntity(opts.entityId);
    const resolvedMeetingId = await resolver.resolveMeeting(eid, meetingId);
    const resolvedItemId = await resolver.resolveAgendaItem(eid, resolvedMeetingId, itemId);
    const resolvedVoterId = await resolver.resolveContact(eid, opts.voter);
    const payload = {
      voter_id: resolvedVoterId, vote_value: opts.vote,
    };
    if (opts.dryRun) {
      printDryRun("governance.cast_vote", { entity_id: eid, meeting_id: resolvedMeetingId, agenda_item_id: resolvedItemId, ...payload });
      return;
    }
    const result = await client.castVote(eid, resolvedMeetingId, resolvedItemId, payload);
    resolver.rememberFromRecord("agenda_item", { agenda_item_id: resolvedItemId, title: itemId }, eid);
    if (opts.json) {
      printJson(result);
      return;
    }
    printSuccess(`Vote cast: ${result.vote_id ?? "OK"}`);
    printJson(result);
  } catch (err) {
    const message = String(err);
    if (message.includes("voting session is not open")) {
      printError(
        `Failed to cast vote: ${err}\n` +
        `  Open the meeting first: corp governance open ${meetingId} --present-seat <seat-ref>`,
      );
    } else {
      printError(`Failed to cast vote: ${err}`);
    }
    process.exit(1);
  }
}

export async function sendNoticeCommand(
  meetingId: string,
  opts: { entityId?: string; json?: boolean; dryRun?: boolean },
): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const eid = await resolver.resolveEntity(opts.entityId);
    const resolvedMeetingId = await resolver.resolveMeeting(eid, meetingId);
    if (opts.dryRun) {
      printDryRun("governance.send_notice", { entity_id: eid, meeting_id: resolvedMeetingId });
      return;
    }
    const result = await client.sendNotice(resolvedMeetingId, eid);
    if (opts.json) {
      printJson(result);
      return;
    }
    printSuccess(`Notice sent for meeting ${resolvedMeetingId}`);
    printJson(result);
  } catch (err) { printError(`Failed to send notice: ${err}`); process.exit(1); }
}

export async function adjournMeetingCommand(
  meetingId: string,
  opts: { entityId?: string; json?: boolean; dryRun?: boolean },
): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const eid = await resolver.resolveEntity(opts.entityId);
    const resolvedMeetingId = await resolver.resolveMeeting(eid, meetingId);
    if (opts.dryRun) {
      printDryRun("governance.adjourn_meeting", { entity_id: eid, meeting_id: resolvedMeetingId });
      return;
    }
    const result = await client.adjournMeeting(resolvedMeetingId, eid);
    if (opts.json) {
      printJson(result);
      return;
    }
    printSuccess(`Meeting ${resolvedMeetingId} adjourned`);
    printJson(result);
  } catch (err) { printError(`Failed to adjourn meeting: ${err}`); process.exit(1); }
}

export async function cancelMeetingCommand(
  meetingId: string,
  opts: { entityId?: string; json?: boolean; dryRun?: boolean },
): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const eid = await resolver.resolveEntity(opts.entityId);
    const resolvedMeetingId = await resolver.resolveMeeting(eid, meetingId);
    if (opts.dryRun) {
      printDryRun("governance.cancel_meeting", { entity_id: eid, meeting_id: resolvedMeetingId });
      return;
    }
    const result = await client.cancelMeeting(resolvedMeetingId, eid);
    if (opts.json) {
      printJson(result);
      return;
    }
    printSuccess(`Meeting ${resolvedMeetingId} cancelled`);
    printJson(result);
  } catch (err) { printError(`Failed to cancel meeting: ${err}`); process.exit(1); }
}

export async function reopenMeetingCommand(
  meetingId: string,
  opts: { entityId?: string; json?: boolean; dryRun?: boolean },
): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const eid = await resolver.resolveEntity(opts.entityId);
    const resolvedMeetingId = await resolver.resolveMeeting(eid, meetingId);
    if (opts.dryRun) {
      printDryRun("governance.reopen_meeting", { entity_id: eid, meeting_id: resolvedMeetingId });
      return;
    }
    const result = await client.reopenMeeting(resolvedMeetingId, eid);
    if (opts.json) {
      printJson(result);
      return;
    }
    printSuccess(`Meeting ${resolvedMeetingId} re-opened`);
    printJson(result);
  } catch (err) { printError(`Failed to re-open meeting: ${err}`); process.exit(1); }
}

export async function finalizeAgendaItemCommand(
  meetingId: string,
  itemId: string,
  opts: { status: string; entityId?: string; json?: boolean; dryRun?: boolean },
): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const eid = await resolver.resolveEntity(opts.entityId);
    const resolvedMeetingId = await resolver.resolveMeeting(eid, meetingId);
    const resolvedItemId = await resolver.resolveAgendaItem(eid, resolvedMeetingId, itemId);
    const payload = {
      entity_id: eid, status: opts.status,
    };
    if (opts.dryRun) {
      printDryRun("governance.finalize_agenda_item", { meeting_id: resolvedMeetingId, agenda_item_id: resolvedItemId, ...payload });
      return;
    }
    const result = await client.finalizeAgendaItem(resolvedMeetingId, resolvedItemId, payload);
    if (opts.json) {
      printJson(result);
      return;
    }
    printSuccess(`Agenda item ${resolvedItemId} finalized as ${opts.status}`);
    printJson(result);
  } catch (err) { printError(`Failed to finalize agenda item: ${err}`); process.exit(1); }
}

export async function computeResolutionCommand(
  meetingId: string,
  itemId: string,
  opts: { text: string; entityId?: string; json?: boolean; dryRun?: boolean },
): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const eid = await resolver.resolveEntity(opts.entityId);
    const resolvedMeetingId = await resolver.resolveMeeting(eid, meetingId);
    const resolvedItemId = await resolver.resolveAgendaItem(eid, resolvedMeetingId, itemId);
    const payload = {
      resolution_text: opts.text,
    };
    if (opts.dryRun) {
      printDryRun("governance.compute_resolution", {
        entity_id: eid,
        meeting_id: resolvedMeetingId,
        agenda_item_id: resolvedItemId,
        ...payload,
      });
      return;
    }
    const result = await client.computeResolution(resolvedMeetingId, resolvedItemId, eid, payload);
    await resolver.stabilizeRecord("resolution", result, eid);
    resolver.rememberFromRecord("resolution", result, eid);
    if (opts.json) {
      printJson(result);
      return;
    }
    printSuccess(`Resolution computed for agenda item ${itemId}`);
    printReferenceSummary("resolution", result, { showReuseHint: true });
    printJson(result);
  } catch (err) { printError(`Failed to compute resolution: ${err}`); process.exit(1); }
}

export async function writtenConsentCommand(opts: {
  body: string; title: string; description: string; entityId?: string; json?: boolean; dryRun?: boolean;
}): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const eid = await resolver.resolveEntity(opts.entityId);
    const resolvedBodyId = await resolver.resolveBody(eid, opts.body);
    const payload = {
      entity_id: eid, body_id: resolvedBodyId, title: opts.title, description: opts.description,
    };
    if (opts.dryRun) {
      printDryRun("governance.written_consent", payload);
      return;
    }
    const result = await client.writtenConsent(payload);
    await resolver.stabilizeRecord("meeting", result, eid);
    resolver.rememberFromRecord("meeting", result, eid);
    const meetingId = result.meeting_id ?? "OK";
    if (opts.json) {
      printJson(result);
      return;
    }
    printSuccess(`Written consent created: ${meetingId}`);
    printReferenceSummary("meeting", result, { showReuseHint: true });
    printJson(result);
    console.log(chalk.dim("\n  Next steps:"));
    console.log(chalk.dim(`    corp governance agenda-items @last:meeting`));
    console.log(chalk.dim(`    corp governance vote @last:meeting <item-ref> --voter <contact-ref> --vote for`));
  } catch (err) { printError(`Failed to create written consent: ${err}`); process.exit(1); }
}

export async function listAgendaItemsCommand(meetingId: string, opts: { entityId?: string; json?: boolean }): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  try {
    const eid = await resolver.resolveEntity(opts.entityId);
    const resolvedMeetingId = await resolver.resolveMeeting(eid, meetingId);
    const items = await client.listAgendaItems(resolvedMeetingId, eid);
    await resolver.stabilizeRecords("agenda_item", items, eid);
    if (opts.json) printJson(items);
    else if (items.length === 0) console.log("No agenda items found.");
    else printAgendaItemsTable(items);
  } catch (err) { printError(`Failed to list agenda items: ${err}`); process.exit(1); }
}
