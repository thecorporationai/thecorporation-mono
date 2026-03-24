//! `corp governance` — governance bodies, seats, meetings, and votes.

use serde_json::json;

use crate::output;
use super::Context;

// ── GovernanceCommand ─────────────────────────────────────────────────────────

/// Manage governance: bodies (boards), seats, meetings, agenda items, votes, and resolutions.
///
/// Meeting lifecycle: draft → noticed → convened → adjourned
///
/// Shortcut: `corp governance quick-approve` runs the full meeting flow in one command.
#[derive(clap::Subcommand)]
pub enum GovernanceCommand {
    /// List governance bodies
    Bodies,

    /// Show a governance body
    ShowBody {
        /// Governance body ID (from `corp governance bodies`)
        body_id: String,
    },

    /// Create a governance body
    CreateBody {
        /// Body name
        #[arg(long)]
        name: String,

        /// Body type [possible values: board_of_directors, llc_member_vote]
        #[arg(long, default_value = "board_of_directors")]
        body_type: String,

        /// Quorum rule: majority (>50%), supermajority (≥2/3), or unanimous
        #[arg(long, default_value = "majority")]
        quorum: String,

        /// Voting method: per_capita = one vote per seat; per_unit = weighted by voting_power
        #[arg(long, default_value = "per_capita")]
        voting_method: String,
    },

    /// Deactivate a governance body
    DeactivateBody {
        /// Governance body ID (from `corp governance bodies`)
        body_id: String,
    },

    /// List all seats
    Seats,

    /// Show a seat
    ShowSeat {
        /// Seat ID (from `corp governance seats`)
        seat_id: String,
    },

    /// Add a seat to a governance body
    AddSeat {
        /// Governance body ID (from `corp governance bodies`)
        #[arg(long)]
        body_id: String,

        /// Contact ID of the seat holder (from `corp contacts list`)
        #[arg(long)]
        holder_id: String,

        /// Role: chair, member, officer, or observer (observers cannot vote)
        #[arg(long, default_value = "member")]
        role: String,

        /// Appointment date (YYYY-MM-DD)
        #[arg(long)]
        appointed_date: String,

        /// Term expiration date (YYYY-MM-DD)
        #[arg(long)]
        term_expiration: Option<String>,

        /// Voting power (default: 1)
        #[arg(long, default_value = "1")]
        voting_power: u32,
    },

    /// Resign a seat
    ResignSeat {
        /// Seat ID (from `corp governance seats`)
        seat_id: String,
    },

    /// List meetings
    Meetings,

    /// Show a meeting
    ShowMeeting {
        /// Meeting ID (from `corp governance meetings`)
        meeting_id: String,
    },

    /// Create a meeting
    CreateMeeting {
        /// Governance body ID (from `corp governance bodies`)
        #[arg(long)]
        body_id: String,

        /// Meeting title
        #[arg(long)]
        title: String,

        /// Meeting type [possible values: board_meeting, shareholder_meeting, written_consent, member_meeting]
        #[arg(long, default_value = "board_meeting")]
        meeting_type: String,
    },

    /// Record that meeting notice was sent (draft → noticed)
    SendNotice {
        /// Meeting ID (from `corp governance meetings`)
        meeting_id: String,
    },

    /// Convene a meeting (draft|noticed → convened)
    Convene {
        /// Meeting ID (from `corp governance meetings`)
        meeting_id: String,
    },

    /// Adjourn a meeting (convened → adjourned)
    Adjourn {
        /// Meeting ID (from `corp governance meetings`)
        meeting_id: String,
    },

    /// Cancel a meeting (from draft or noticed state)
    CancelMeeting {
        /// Meeting ID (from `corp governance meetings`)
        meeting_id: String,
    },

    /// Reopen an adjourned meeting (adjourned → convened)
    Reopen {
        /// Meeting ID (from `corp governance meetings`)
        meeting_id: String,
    },

    /// Record attendance for a meeting
    RecordAttendance {
        /// Meeting ID (from `corp governance meetings`)
        meeting_id: String,

        /// Comma-separated seat IDs of members present
        #[arg(long)]
        seat_ids: String,
    },

    /// List agenda items for a meeting
    Items {
        /// Meeting ID (from `corp governance meetings`)
        meeting_id: String,
    },

    /// Add an agenda item to a meeting
    AddItem {
        /// Meeting ID (from `corp governance meetings`)
        meeting_id: String,

        /// Item title
        #[arg(long)]
        title: String,

        /// Agenda item type: resolution, discussion, report, or election
        #[arg(long, default_value = "resolution")]
        item_type: String,

        /// Description of the agenda item (optional)
        #[arg(long)]
        description: Option<String>,

        /// Resolution text (optional; required for resolution items)
        #[arg(long)]
        resolution_text: Option<String>,
    },

    /// List votes for a meeting
    Votes {
        /// Meeting ID (from `corp governance meetings`)
        meeting_id: String,
    },

    /// Cast a vote on an agenda item
    ///
    /// Examples:
    ///   corp governance vote <meeting_id> --item-id <ID> --seat-id <ID> --value for
    ///   corp governance vote <meeting_id> --item-id <ID> --seat-id <ID> --value abstain
    Vote {
        /// Meeting ID (from `corp governance meetings`)
        meeting_id: String,

        /// Agenda item ID (from `corp governance items`)
        #[arg(long)]
        item_id: String,

        /// Seat ID casting the vote (from `corp governance seats`)
        #[arg(long)]
        seat_id: String,

        /// Vote value: for, against, abstain, or recusal
        #[arg(long)]
        value: String,
    },

    /// Resolve an agenda item
    ///
    /// Examples:
    ///   corp governance resolve-item <meeting_id> --item-id <ID> \
    ///     --resolution-type ordinary --resolution-text "The board approves the budget."
    ResolveItem {
        /// Meeting ID (from `corp governance meetings`)
        meeting_id: String,

        /// Agenda item ID (from `corp governance items`)
        #[arg(long)]
        item_id: String,

        /// Resolution outcome: ordinary, special, or unanimous_written_consent
        #[arg(long)]
        resolution_type: String,

        /// Full text of the resolution
        #[arg(long)]
        resolution_text: String,
    },

    /// Create a written consent meeting with a single resolution item, ready for votes
    ///
    /// Creates a meeting of type 'written_consent' (auto-convened).
    /// Each seat holder must vote individually via `corp governance vote`.
    WrittenConsent {
        /// Governance body ID (from `corp governance bodies`)
        #[arg(long)]
        body_id: String,

        /// Resolution title
        #[arg(long)]
        title: String,

        /// Full text of the resolution
        #[arg(long)]
        resolution_text: String,
    },

    /// One-step approval: creates meeting, adds resolution, records unanimous votes, resolves, and adjourns
    ///
    /// Shortcut that creates a written consent, casts unanimous For votes
    /// from all active voting seats, resolves the item, and adjourns.
    /// Returns all created IDs. Use for routine approvals.
    QuickApprove {
        /// Governance body ID (from `corp governance bodies`)
        #[arg(long)]
        body_id: String,

        /// Resolution title
        #[arg(long)]
        title: String,

        /// Full text of the resolution
        #[arg(long)]
        resolution_text: String,
    },

    /// Show the governance profile for the active entity
    Profile,

    /// Update the governance profile (full replacement, not partial patch — all fields required)
    UpdateProfile {
        /// Entity type (c_corp or llc)
        #[arg(long)]
        entity_type: String,

        /// Legal name
        #[arg(long)]
        legal_name: String,

        /// Jurisdiction (e.g. DE)
        #[arg(long)]
        jurisdiction: String,

        /// Effective date (YYYY-MM-DD)
        #[arg(long)]
        effective_date: String,
    },
}

// ── run ───────────────────────────────────────────────────────────────────────

pub async fn run(cmd: GovernanceCommand, ctx: &Context) -> anyhow::Result<()> {
    let mode = ctx.mode();
    let entity_id = ctx.require_entity()?;

    match cmd {
        GovernanceCommand::Bodies => {
            let path = format!("/v1/entities/{entity_id}/governance/bodies");
            let value = ctx.get(&path).await?;
            output::print_value(&value, mode);
        }

        GovernanceCommand::ShowBody { body_id } => {
            let path = format!("/v1/entities/{entity_id}/governance/bodies/{body_id}");
            let value = ctx.get(&path).await?;
            output::print_value(&value, mode);
        }

        GovernanceCommand::CreateBody { name, body_type, quorum, voting_method } => {
            let path = format!("/v1/entities/{entity_id}/governance/bodies");
            let body = json!({
                "name": name,
                "body_type": body_type,
                "quorum_rule": quorum,
                "voting_method": voting_method,
            });
            let value = ctx.post(&path, &body).await?;
            output::print_value(&value, mode);
            output::print_success("Governance body created.", mode);
        }

        GovernanceCommand::DeactivateBody { body_id } => {
            let path = format!("/v1/entities/{entity_id}/governance/bodies/{body_id}/deactivate");
            let value = ctx.post(&path, &json!({})).await?;
            output::print_value(&value, mode);
            output::print_success("Body deactivated.", mode);
        }

        GovernanceCommand::Seats => {
            let path = format!("/v1/entities/{entity_id}/governance/seats");
            let value = ctx.get(&path).await?;
            output::print_value(&value, mode);
        }

        GovernanceCommand::ShowSeat { seat_id } => {
            let path = format!("/v1/entities/{entity_id}/governance/seats/{seat_id}");
            let value = ctx.get(&path).await?;
            output::print_value(&value, mode);
        }

        GovernanceCommand::AddSeat {
            body_id,
            holder_id,
            role,
            appointed_date,
            term_expiration,
            voting_power,
        } => {
            let path = format!("/v1/entities/{entity_id}/governance/seats");
            let req_body = json!({
                "body_id": body_id,
                "holder_id": holder_id,
                "role": role,
                "appointed_date": appointed_date,
                "term_expiration": term_expiration,
                "voting_power": voting_power,
            });
            let value = ctx.post(&path, &req_body).await?;
            output::print_value(&value, mode);
            output::print_success("Seat added.", mode);
        }

        GovernanceCommand::ResignSeat { seat_id } => {
            let path = format!("/v1/entities/{entity_id}/governance/seats/{seat_id}/resign");
            let value = ctx.post(&path, &json!({})).await?;
            output::print_value(&value, mode);
            output::print_success("Seat resigned.", mode);
        }

        GovernanceCommand::Meetings => {
            let path = format!("/v1/entities/{entity_id}/governance/meetings");
            let value = ctx.get(&path).await?;
            output::print_value(&value, mode);
        }

        GovernanceCommand::ShowMeeting { meeting_id } => {
            let path = format!("/v1/entities/{entity_id}/governance/meetings/{meeting_id}");
            let value = ctx.get(&path).await?;
            output::print_value(&value, mode);
        }

        GovernanceCommand::CreateMeeting { body_id, title, meeting_type } => {
            let path = format!("/v1/entities/{entity_id}/governance/meetings");
            let req_body = json!({
                "body_id": body_id,
                "title": title,
                "meeting_type": meeting_type,
            });
            let value = ctx.post(&path, &req_body).await?;
            output::print_value(&value, mode);
            output::print_success("Meeting created.", mode);
        }

        GovernanceCommand::SendNotice { meeting_id } => {
            let path = format!(
                "/v1/entities/{entity_id}/governance/meetings/{meeting_id}/notice"
            );
            let value = ctx.post(&path, &json!({})).await?;
            output::print_value(&value, mode);
            output::print_success("Notice sent.", mode);
        }

        GovernanceCommand::Convene { meeting_id } => {
            let path = format!(
                "/v1/entities/{entity_id}/governance/meetings/{meeting_id}/convene"
            );
            let value = ctx.post(&path, &json!({})).await?;
            output::print_value(&value, mode);
            output::print_success("Meeting convened.", mode);
        }

        GovernanceCommand::Adjourn { meeting_id } => {
            let path = format!(
                "/v1/entities/{entity_id}/governance/meetings/{meeting_id}/adjourn"
            );
            let value = ctx.post(&path, &json!({})).await?;
            output::print_value(&value, mode);
            output::print_success("Meeting adjourned.", mode);
        }

        GovernanceCommand::CancelMeeting { meeting_id } => {
            let path = format!(
                "/v1/entities/{entity_id}/governance/meetings/{meeting_id}/cancel"
            );
            let value = ctx.post(&path, &json!({})).await?;
            output::print_value(&value, mode);
            output::print_success("Meeting cancelled.", mode);
        }

        GovernanceCommand::Reopen { meeting_id } => {
            let path = format!(
                "/v1/entities/{entity_id}/governance/meetings/{meeting_id}/reopen"
            );
            let value = ctx.post(&path, &json!({})).await?;
            output::print_value(&value, mode);
            output::print_success("Meeting reopened.", mode);
        }

        GovernanceCommand::RecordAttendance { meeting_id, seat_ids } => {
            let path = format!(
                "/v1/entities/{entity_id}/governance/meetings/{meeting_id}/attendance"
            );
            let ids: Vec<&str> = seat_ids.split(',').map(str::trim).collect();
            let req_body = json!({ "seat_ids": ids });
            let value = ctx.post(&path, &req_body).await?;
            output::print_value(&value, mode);
            output::print_success("Attendance recorded.", mode);
        }

        GovernanceCommand::Items { meeting_id } => {
            let path = format!(
                "/v1/entities/{entity_id}/governance/meetings/{meeting_id}/items"
            );
            let value = ctx.get(&path).await?;
            output::print_value(&value, mode);
        }

        GovernanceCommand::AddItem {
            meeting_id,
            title,
            item_type,
            description,
            resolution_text,
        } => {
            let path = format!(
                "/v1/entities/{entity_id}/governance/meetings/{meeting_id}/items"
            );
            let req_body = json!({
                "title": title,
                "item_type": item_type,
                "description": description,
                "resolution_text": resolution_text,
            });
            let value = ctx.post(&path, &req_body).await?;
            output::print_value(&value, mode);
            output::print_success("Agenda item added.", mode);
        }

        GovernanceCommand::Votes { meeting_id } => {
            let path = format!(
                "/v1/entities/{entity_id}/governance/meetings/{meeting_id}/votes"
            );
            let value = ctx.get(&path).await?;
            output::print_value(&value, mode);
        }

        GovernanceCommand::Vote { meeting_id, item_id, seat_id, value: vote_value } => {
            let path = format!(
                "/v1/entities/{entity_id}/governance/meetings/{meeting_id}/votes"
            );
            let req_body = json!({
                "agenda_item_id": item_id,
                "seat_id": seat_id,
                "value": vote_value,
            });
            let value = ctx.post(&path, &req_body).await?;
            output::print_value(&value, mode);
            output::print_success("Vote recorded.", mode);
        }

        GovernanceCommand::ResolveItem { meeting_id, item_id, resolution_type, resolution_text } => {
            let path = format!(
                "/v1/entities/{entity_id}/governance/meetings/{meeting_id}/items/{item_id}/resolve"
            );
            let req_body = json!({
                "resolution_type": resolution_type,
                "resolution_text": resolution_text,
            });
            let value = ctx.post(&path, &req_body).await?;
            output::print_value(&value, mode);
            output::print_success("Item resolved.", mode);
        }

        GovernanceCommand::WrittenConsent { body_id, title, resolution_text } => {
            let path = format!(
                "/v1/entities/{entity_id}/governance/written-consent"
            );
            let req_body = json!({
                "body_id": body_id,
                "title": title,
                "resolution_text": resolution_text,
            });
            let value = ctx.post(&path, &req_body).await?;
            output::print_value(&value, mode);
            output::print_success("Written consent created.", mode);
        }

        GovernanceCommand::QuickApprove { body_id, title, resolution_text } => {
            let path = format!(
                "/v1/entities/{entity_id}/governance/quick-approve"
            );
            let req_body = json!({
                "body_id": body_id,
                "title": title,
                "resolution_text": resolution_text,
            });
            let value = ctx.post(&path, &req_body).await?;
            output::print_value(&value, mode);
            output::print_success("Resolution approved.", mode);
        }

        GovernanceCommand::Profile => {
            let path = format!("/v1/entities/{entity_id}/governance/profile");
            let value = ctx.get(&path).await?;
            output::print_value(&value, mode);
        }

        GovernanceCommand::UpdateProfile {
            entity_type,
            legal_name,
            jurisdiction,
            effective_date,
        } => {
            let path = format!("/v1/entities/{entity_id}/governance/profile");
            let req_body = json!({
                "entity_type": entity_type,
                "legal_name": legal_name,
                "jurisdiction": jurisdiction,
                "effective_date": effective_date,
                "founders": [],
                "directors": [],
                "officers": [],
            });
            let value = ctx.client.put(&path, &req_body).await?;
            output::print_value(&value, mode);
            output::print_success("Governance profile updated.", mode);
        }
    }

    Ok(())
}
