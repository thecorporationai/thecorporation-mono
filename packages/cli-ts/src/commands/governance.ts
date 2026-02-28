import { requireConfig, resolveEntityId } from "../config.js";
import { CorpAPIClient } from "../api-client.js";
import {
  printGovernanceTable, printSeatsTable, printMeetingsTable,
  printResolutionsTable, printError, printSuccess, printJson,
} from "../output.js";

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

export async function governanceSeatsCommand(bodyId: string, opts: { json?: boolean }): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    const seats = await client.getGovernanceSeats(bodyId);
    if (opts.json) printJson(seats);
    else if (seats.length === 0) console.log("No seats found.");
    else printSeatsTable(seats);
  } catch (err) { printError(`Failed to fetch seats: ${err}`); process.exit(1); }
}

export async function governanceMeetingsCommand(bodyId: string, opts: { json?: boolean }): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    const meetings = await client.listMeetings(bodyId);
    if (opts.json) printJson(meetings);
    else if (meetings.length === 0) console.log("No meetings found.");
    else printMeetingsTable(meetings);
  } catch (err) { printError(`Failed to fetch meetings: ${err}`); process.exit(1); }
}

export async function governanceResolutionsCommand(meetingId: string, opts: { json?: boolean }): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    const resolutions = await client.getMeetingResolutions(meetingId);
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
