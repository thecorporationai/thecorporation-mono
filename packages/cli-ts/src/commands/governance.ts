import { requireConfig, resolveEntityId } from "../config.js";
import { CorpAPIClient } from "../api-client.js";
import {
  printGovernanceTable, printSeatsTable, printMeetingsTable,
  printResolutionsTable, printDryRun, printError, printSuccess, printJson,
} from "../output.js";
import chalk from "chalk";

export async function governanceCreateBodyCommand(opts: {
  entityId?: string; name: string; bodyType: string; quorum: string; voting: string;
  json?: boolean; dryRun?: boolean;
}): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const eid = resolveEntityId(cfg, opts.entityId);
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
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
    const bodyId = result.body_id ?? "OK";
    if (opts.json) {
      printJson(result);
      return;
    }
    printSuccess(`Governance body created: ${bodyId}`);
    printJson(result);
    console.log(chalk.dim("\n  Next steps:"));
    console.log(chalk.dim(`    corp governance add-seat ${bodyId} --holder <contact-id>`));
    console.log(chalk.dim(`    corp governance seats ${bodyId}`));
  } catch (err) { printError(`Failed to create governance body: ${err}`); process.exit(1); }
}

export async function governanceAddSeatCommand(bodyId: string, opts: {
  holder: string; role?: string; entityId?: string; json?: boolean; dryRun?: boolean;
}): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const eid = resolveEntityId(cfg, opts.entityId);
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    const data: Record<string, unknown> = { holder_id: opts.holder, role: opts.role ?? "member" };
    if (opts.dryRun) {
      printDryRun("governance.add_seat", { entity_id: eid, body_id: bodyId, ...data });
      return;
    }
    const result = await client.createGovernanceSeat(bodyId, eid, data);
    if (opts.json) {
      printJson(result);
      return;
    }
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
  json?: boolean; dryRun?: boolean;
}): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const eid = resolveEntityId(cfg, opts.entityId);
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    const payload = {
      entity_id: eid, body_id: opts.body, meeting_type: opts.meetingType,
      title: opts.title, scheduled_date: opts.date,
      agenda_item_titles: opts.agenda,
    };
    if (opts.dryRun) {
      printDryRun("governance.schedule_meeting", payload);
      return;
    }
    const result = await client.scheduleMeeting(payload);
    const meetingId = result.meeting_id ?? "OK";
    if (opts.json) {
      printJson(result);
      return;
    }
    printSuccess(`Meeting scheduled: ${meetingId}`);
    printJson(result);
    console.log(chalk.dim("\n  Next steps:"));
    console.log(chalk.dim(`    corp governance notice ${meetingId}`));
    console.log(chalk.dim(`    corp governance open ${meetingId} --present-seat <seat-id>`));
    console.log(chalk.dim(`    corp governance agenda-items ${meetingId}`));
  } catch (err) { printError(`Failed to schedule meeting: ${err}`); process.exit(1); }
}

export async function governanceOpenMeetingCommand(
  meetingId: string,
  opts: { entityId?: string; presentSeat: string[]; json?: boolean; dryRun?: boolean },
): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const eid = resolveEntityId(cfg, opts.entityId);
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    const payload = { present_seat_ids: opts.presentSeat };
    if (opts.dryRun) {
      printDryRun("governance.open_meeting", { entity_id: eid, meeting_id: meetingId, ...payload });
      return;
    }
    const result = await client.conveneMeeting(meetingId, eid, payload);
    if (opts.json) {
      printJson(result);
      return;
    }
    printSuccess(`Meeting opened: ${meetingId}`);
    printJson(result);
  } catch (err) { printError(`Failed to open meeting: ${err}`); process.exit(1); }
}

export async function governanceVoteCommand(
  meetingId: string,
  itemId: string,
  opts: { voter: string; vote: string; entityId?: string; json?: boolean; dryRun?: boolean }
): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const eid = resolveEntityId(cfg, opts.entityId);
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    const payload = {
      voter_id: opts.voter, vote_value: opts.vote,
    };
    if (opts.dryRun) {
      printDryRun("governance.cast_vote", { entity_id: eid, meeting_id: meetingId, agenda_item_id: itemId, ...payload });
      return;
    }
    const result = await client.castVote(eid, meetingId, itemId, payload);
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
        `  Open the meeting first: corp governance open ${meetingId} --present-seat <seat-id>`,
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
  const eid = resolveEntityId(cfg, opts.entityId);
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    if (opts.dryRun) {
      printDryRun("governance.send_notice", { entity_id: eid, meeting_id: meetingId });
      return;
    }
    const result = await client.sendNotice(meetingId, eid);
    if (opts.json) {
      printJson(result);
      return;
    }
    printSuccess(`Notice sent for meeting ${meetingId}`);
    printJson(result);
  } catch (err) { printError(`Failed to send notice: ${err}`); process.exit(1); }
}

export async function adjournMeetingCommand(
  meetingId: string,
  opts: { entityId?: string; json?: boolean; dryRun?: boolean },
): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const eid = resolveEntityId(cfg, opts.entityId);
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    if (opts.dryRun) {
      printDryRun("governance.adjourn_meeting", { entity_id: eid, meeting_id: meetingId });
      return;
    }
    const result = await client.adjournMeeting(meetingId, eid);
    if (opts.json) {
      printJson(result);
      return;
    }
    printSuccess(`Meeting ${meetingId} adjourned`);
    printJson(result);
  } catch (err) { printError(`Failed to adjourn meeting: ${err}`); process.exit(1); }
}

export async function cancelMeetingCommand(
  meetingId: string,
  opts: { entityId?: string; json?: boolean; dryRun?: boolean },
): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const eid = resolveEntityId(cfg, opts.entityId);
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    if (opts.dryRun) {
      printDryRun("governance.cancel_meeting", { entity_id: eid, meeting_id: meetingId });
      return;
    }
    const result = await client.cancelMeeting(meetingId, eid);
    if (opts.json) {
      printJson(result);
      return;
    }
    printSuccess(`Meeting ${meetingId} cancelled`);
    printJson(result);
  } catch (err) { printError(`Failed to cancel meeting: ${err}`); process.exit(1); }
}

export async function finalizeAgendaItemCommand(
  meetingId: string,
  itemId: string,
  opts: { status: string; entityId?: string; json?: boolean; dryRun?: boolean },
): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const eid = resolveEntityId(cfg, opts.entityId);
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    const payload = {
      entity_id: eid, status: opts.status,
    };
    if (opts.dryRun) {
      printDryRun("governance.finalize_agenda_item", { meeting_id: meetingId, agenda_item_id: itemId, ...payload });
      return;
    }
    const result = await client.finalizeAgendaItem(meetingId, itemId, payload);
    if (opts.json) {
      printJson(result);
      return;
    }
    printSuccess(`Agenda item ${itemId} finalized as ${opts.status}`);
    printJson(result);
  } catch (err) { printError(`Failed to finalize agenda item: ${err}`); process.exit(1); }
}

export async function computeResolutionCommand(
  meetingId: string,
  itemId: string,
  opts: { text: string; entityId?: string; json?: boolean; dryRun?: boolean },
): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const eid = resolveEntityId(cfg, opts.entityId);
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    const payload = {
      resolution_text: opts.text,
    };
    if (opts.dryRun) {
      printDryRun("governance.compute_resolution", {
        entity_id: eid,
        meeting_id: meetingId,
        agenda_item_id: itemId,
        ...payload,
      });
      return;
    }
    const result = await client.computeResolution(meetingId, itemId, eid, payload);
    if (opts.json) {
      printJson(result);
      return;
    }
    printSuccess(`Resolution computed for agenda item ${itemId}`);
    printJson(result);
  } catch (err) { printError(`Failed to compute resolution: ${err}`); process.exit(1); }
}

export async function writtenConsentCommand(opts: {
  body: string; title: string; description: string; entityId?: string; json?: boolean; dryRun?: boolean;
}): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const eid = resolveEntityId(cfg, opts.entityId);
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    const payload = {
      entity_id: eid, body_id: opts.body, title: opts.title, description: opts.description,
    };
    if (opts.dryRun) {
      printDryRun("governance.written_consent", payload);
      return;
    }
    const result = await client.writtenConsent(payload);
    const meetingId = result.meeting_id ?? "OK";
    if (opts.json) {
      printJson(result);
      return;
    }
    printSuccess(`Written consent created: ${meetingId}`);
    printJson(result);
    console.log(chalk.dim("\n  Next steps:"));
    console.log(chalk.dim(`    corp governance agenda-items ${meetingId}`));
    console.log(chalk.dim(`    corp governance vote ${meetingId} <item-id> --voter <contact-uuid> --vote for`));
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
    else {
      const Table = (await import("cli-table3")).default;
      const chalk = (await import("chalk")).default;
      console.log(`\n${chalk.bold("Agenda Items")}`);
      const table = new Table({ head: [chalk.dim("ID"), chalk.dim("Title"), chalk.dim("Status"), chalk.dim("Type")] });
      for (const item of items) {
        table.push([
          String(item.item_id ?? item.agenda_item_id ?? item.id ?? "").slice(0, 12),
          String(item.title ?? ""),
          String(item.status ?? ""),
          String(item.item_type ?? item.type ?? ""),
        ]);
      }
      console.log(table.toString());
    }
  } catch (err) { printError(`Failed to list agenda items: ${err}`); process.exit(1); }
}
