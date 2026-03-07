import { requireConfig, resolveEntityId } from "../config.js";
import { CorpAPIClient } from "../api-client.js";
import {
  printGovernanceTable, printSeatsTable, printMeetingsTable,
  printResolutionsTable, printError, printSuccess, printJson,
} from "../output.js";

export async function governanceCreateBodyCommand(opts: {
  entityId?: string; name: string; bodyType: string; quorum: string; voting: string;
}): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const eid = resolveEntityId(cfg, opts.entityId);
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    const result = await client.createGovernanceBody({
      entity_id: eid,
      body_type: opts.bodyType,
      name: opts.name,
      quorum_rule: opts.quorum,
      voting_method: opts.voting,
    });
    printSuccess(`Governance body created: ${result.body_id ?? "OK"}`);
    printJson(result);
  } catch (err) { printError(`Failed to create governance body: ${err}`); process.exit(1); }
}

export async function governanceAddSeatCommand(bodyId: string, opts: {
  holder: string; role?: string;
}): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    const data: Record<string, unknown> = { holder_id: opts.holder, role: opts.role ?? "member" };
    const result = await client.createGovernanceSeat(bodyId, data);
    printSuccess(`Seat added: ${result.seat_id ?? "OK"}`);
    printJson(result);
  } catch (err) { printError(`Failed to add seat: ${err}`); process.exit(1); }
}

export async function governanceListCommand(opts: { entityId?: string; json?: boolean }): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const eid = resolveEntityId(cfg, opts.entityId);
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    const bodies = await client.listGovernanceBodies(eid);
    if (opts.json) printJson(bodies);
    else if (bodies.length === 0) console.log("No governance bodies found.");
    else printGovernanceTable(bodies);
  } catch (err) { printError(`Failed to fetch governance bodies: ${err}`); process.exit(1); }
}

export async function governanceSeatsCommand(bodyId: string, opts: { entityId?: string; json?: boolean }): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const eid = resolveEntityId(cfg, opts.entityId);
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    const seats = await client.getGovernanceSeats(bodyId, eid);
    if (opts.json) printJson(seats);
    else if (seats.length === 0) console.log("No seats found.");
    else printSeatsTable(seats);
  } catch (err) { printError(`Failed to fetch seats: ${err}`); process.exit(1); }
}

export async function governanceMeetingsCommand(bodyId: string, opts: { entityId?: string; json?: boolean }): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const eid = resolveEntityId(cfg, opts.entityId);
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    const meetings = await client.listMeetings(bodyId, eid);
    if (opts.json) printJson(meetings);
    else if (meetings.length === 0) console.log("No meetings found.");
    else printMeetingsTable(meetings);
  } catch (err) { printError(`Failed to fetch meetings: ${err}`); process.exit(1); }
}

export async function governanceResolutionsCommand(meetingId: string, opts: { entityId?: string; json?: boolean }): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const eid = resolveEntityId(cfg, opts.entityId);
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    const resolutions = await client.getMeetingResolutions(meetingId, eid);
    if (opts.json) printJson(resolutions);
    else if (resolutions.length === 0) console.log("No resolutions found.");
    else printResolutionsTable(resolutions);
  } catch (err) { printError(`Failed to fetch resolutions: ${err}`); process.exit(1); }
}

export async function governanceConveneCommand(opts: {
  entityId?: string; body: string; meetingType: string; title: string; date: string; agenda: string[];
}): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const eid = resolveEntityId(cfg, opts.entityId);
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    const result = await client.scheduleMeeting({
      entity_id: eid, body_id: opts.body, meeting_type: opts.meetingType,
      title: opts.title, scheduled_date: opts.date,
      agenda_item_titles: opts.agenda,
    });
    printSuccess(`Meeting scheduled: ${result.meeting_id ?? "OK"}`);
    printJson(result);
  } catch (err) { printError(`Failed to schedule meeting: ${err}`); process.exit(1); }
}

export async function governanceVoteCommand(
  meetingId: string,
  itemId: string,
  opts: { voter: string; vote: string }
): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const eid = resolveEntityId(cfg, undefined);
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    const result = await client.castVote(eid, meetingId, itemId, {
      voter_id: opts.voter, vote_value: opts.vote,
    });
    printSuccess(`Vote cast: ${result.vote_id ?? "OK"}`);
    printJson(result);
  } catch (err) { printError(`Failed to cast vote: ${err}`); process.exit(1); }
}

export async function sendNoticeCommand(meetingId: string, opts: { entityId?: string }): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const eid = resolveEntityId(cfg, opts.entityId);
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    const result = await client.sendNotice(meetingId, eid);
    printSuccess(`Notice sent for meeting ${meetingId}`);
    printJson(result);
  } catch (err) { printError(`Failed to send notice: ${err}`); process.exit(1); }
}

export async function adjournMeetingCommand(meetingId: string, opts: { entityId?: string }): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const eid = resolveEntityId(cfg, opts.entityId);
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    const result = await client.adjournMeeting(meetingId, eid);
    printSuccess(`Meeting ${meetingId} adjourned`);
    printJson(result);
  } catch (err) { printError(`Failed to adjourn meeting: ${err}`); process.exit(1); }
}

export async function cancelMeetingCommand(meetingId: string, opts: { entityId?: string }): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const eid = resolveEntityId(cfg, opts.entityId);
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    const result = await client.cancelMeeting(meetingId, eid);
    printSuccess(`Meeting ${meetingId} cancelled`);
    printJson(result);
  } catch (err) { printError(`Failed to cancel meeting: ${err}`); process.exit(1); }
}

export async function finalizeAgendaItemCommand(
  meetingId: string, itemId: string, opts: { status: string; entityId?: string }
): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const eid = resolveEntityId(cfg, opts.entityId);
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    const result = await client.finalizeAgendaItem(meetingId, itemId, {
      entity_id: eid, status: opts.status,
    });
    printSuccess(`Agenda item ${itemId} finalized as ${opts.status}`);
    printJson(result);
  } catch (err) { printError(`Failed to finalize agenda item: ${err}`); process.exit(1); }
}

export async function computeResolutionCommand(
  meetingId: string, itemId: string, opts: { text: string; entityId?: string }
): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const eid = resolveEntityId(cfg, opts.entityId);
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    const result = await client.computeResolution(meetingId, itemId, eid, {
      resolution_text: opts.text,
    });
    printSuccess(`Resolution computed for agenda item ${itemId}`);
    printJson(result);
  } catch (err) { printError(`Failed to compute resolution: ${err}`); process.exit(1); }
}

export async function writtenConsentCommand(opts: {
  body: string; title: string; description: string; entityId?: string;
}): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const eid = resolveEntityId(cfg, opts.entityId);
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    const result = await client.writtenConsent({
      entity_id: eid, body_id: opts.body, title: opts.title, description: opts.description,
    });
    printSuccess(`Written consent created: ${result.meeting_id ?? "OK"}`);
    printJson(result);
  } catch (err) { printError(`Failed to create written consent: ${err}`); process.exit(1); }
}

export async function listAgendaItemsCommand(meetingId: string, opts: { entityId?: string; json?: boolean }): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const eid = resolveEntityId(cfg, opts.entityId);
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    const items = await client.listAgendaItems(meetingId, eid);
    if (opts.json) printJson(items);
    else if (items.length === 0) console.log("No agenda items found.");
    else printJson(items);
  } catch (err) { printError(`Failed to list agenda items: ${err}`); process.exit(1); }
}
