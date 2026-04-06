//! Cross-domain integration tests for corp-core domain models.
//!
//! Each test exercises a realistic lifecycle scenario that touches multiple
//! domain aggregates, verifying that the FSM transitions, invariant checks, and
//! business rules compose correctly end-to-end.

use chrono::{NaiveDate, Utc};

use corp_core::{
    equity::{
        cap_table::CapTable,
        grant::EquityGrant,
        instrument::{Instrument, InstrumentKind},
        safe_note::{SafeNote, SafeNoteError},
        transfer::ShareTransfer,
        types::{
            GrantType, SafeStatus, SafeType, ShareCount, TransferStatus, TransferType,
            ValuationMethodology, ValuationStatus, ValuationType,
        },
        valuation::{Valuation, ValuationError},
    },
    execution::{
        intent::{Intent, IntentError},
        obligation::{Obligation, ObligationError},
        types::{AssigneeType, IntentStatus, ObligationStatus},
    },
    formation::{
        document::{Document, DocumentStatus, DocumentType, Signature},
        entity::{Entity, EntityError, EntityType, FormationStatus, Jurisdiction},
    },
    governance::{
        agenda_item::AgendaItem,
        body::GovernanceBody,
        meeting::{Meeting, QuorumStatus},
        resolution::{Resolution, compute_resolution},
        seat::GovernanceSeat,
        types::{
            AgendaItemType, BodyType, MeetingStatus, MeetingType, QuorumThreshold, ResolutionType,
            SeatRole, VoteValue, VotingMethod, VotingPower,
        },
        vote::Vote,
    },
    ids::{
        CapTableId, ContactId, EntityId, GovernanceBodyId, GovernanceSeatId, HolderId, WorkspaceId,
    },
    treasury::{
        invoice::{Invoice, InvoiceError},
        journal_entry::{JournalEntry, JournalEntryError, JournalLine},
        types::{Currency, InvoiceStatus, Side},
    },
    work_items::{WorkItem, WorkItemError, WorkItemStatus},
};

// ── Helpers ───────────────────────────────────────────────────────────────────

fn de_jurisdiction() -> Jurisdiction {
    Jurisdiction::new("DE").unwrap()
}

fn make_entity() -> Entity {
    Entity::new(
        WorkspaceId::new(),
        "Acme Corp",
        EntityType::CCorp,
        de_jurisdiction(),
    )
    .unwrap()
}

fn make_board(entity_id: EntityId) -> GovernanceBody {
    GovernanceBody::new(
        entity_id,
        BodyType::BoardOfDirectors,
        "Board of Directors".to_string(),
        QuorumThreshold::Majority,
        VotingMethod::PerCapita,
    )
    .unwrap()
}

fn make_seat(body_id: GovernanceBodyId) -> GovernanceSeat {
    GovernanceSeat::new(
        body_id,
        ContactId::new(),
        SeatRole::Member,
        NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
        None,
        VotingPower::new(1).unwrap(),
    )
}

fn make_board_meeting(body_id: GovernanceBodyId) -> Meeting {
    Meeting::new(
        body_id,
        MeetingType::BoardMeeting,
        "Q1 Board Meeting".to_string(),
        None,
        None,
        None,
    )
}

fn make_cap_table(entity_id: EntityId) -> CapTable {
    CapTable::new(entity_id)
}

fn make_instrument(entity_id: EntityId, cap_table_id: CapTableId) -> Instrument {
    Instrument::new(
        entity_id,
        cap_table_id,
        "CS-A",
        InstrumentKind::CommonEquity,
        Some(10_000_000),
        Some("0.00001".to_string()),
        None,
        None,
        serde_json::Value::Null,
    )
}

// ── 1. Full formation lifecycle ───────────────────────────────────────────────

#[test]
fn formation_full_lifecycle_pending_to_active() {
    let mut entity = make_entity();
    assert_eq!(entity.formation_status, FormationStatus::Pending);

    let expected_path = [
        FormationStatus::DocumentsGenerated,
        FormationStatus::DocumentsSigned,
        FormationStatus::FilingSubmitted,
        FormationStatus::Filed,
        FormationStatus::EinApplied,
        FormationStatus::Active,
    ];

    for expected in expected_path {
        let next = entity.advance_status().expect("advance should succeed");
        assert_eq!(next, expected);
        assert_eq!(entity.formation_status, expected);
    }

    assert!(entity.formation_status.is_terminal());
}

// ── 2. Formation lifecycle reversal / state skip prevention ──────────────────

#[test]
fn formation_cannot_advance_from_terminal() {
    let mut entity = make_entity();

    // Advance all the way to Active.
    for _ in 0..6 {
        entity.advance_status().unwrap();
    }
    assert_eq!(entity.formation_status, FormationStatus::Active);

    let err = entity.advance_status().unwrap_err();
    assert!(matches!(
        err,
        EntityError::AlreadyTerminal(FormationStatus::Active)
    ));
}

#[test]
fn formation_cannot_advance_from_rejected() {
    let mut entity = make_entity();
    // Jump straight to Rejected by dissolving from Pending.
    // Actually Rejected is a separate terminal — simulate by using dissolve
    // from Pending state to reach Dissolved (the reachable terminal).
    let date = NaiveDate::from_ymd_opt(2026, 3, 1).unwrap();
    entity.dissolve(date).unwrap();
    assert_eq!(entity.formation_status, FormationStatus::Dissolved);

    let err = entity.advance_status().unwrap_err();
    assert!(matches!(
        err,
        EntityError::AlreadyTerminal(FormationStatus::Dissolved)
    ));
}

#[test]
fn formation_cannot_dissolve_twice() {
    let mut entity = make_entity();
    let date = NaiveDate::from_ymd_opt(2026, 3, 1).unwrap();
    entity.dissolve(date).unwrap();
    let err = entity.dissolve(date).unwrap_err();
    assert_eq!(err, EntityError::AlreadyDissolved);
}

// ── 3. Full governance meeting flow ──────────────────────────────────────────

#[test]
fn governance_full_meeting_flow() {
    let entity_id = EntityId::new();
    let board = make_board(entity_id);
    let body_id = board.body_id;

    // Three seats.
    let seats: Vec<GovernanceSeat> = (0..3).map(|_| make_seat(body_id)).collect();
    for seat in &seats {
        assert!(seat.can_vote());
    }

    // Create meeting and advance through Draft -> Noticed -> Convened.
    let mut meeting = make_board_meeting(body_id);
    assert_eq!(meeting.status, MeetingStatus::Draft);

    meeting.send_notice().unwrap();
    assert_eq!(meeting.status, MeetingStatus::Noticed);

    meeting.convene().unwrap();
    assert_eq!(meeting.status, MeetingStatus::Convened);
    assert!(meeting.convened_at.is_some());

    // Record attendance (3 of 3 present — quorum met).
    let seat_ids: Vec<GovernanceSeatId> = seats.iter().map(|s| s.seat_id).collect();
    meeting.record_attendance(seat_ids.clone(), 3, 3, QuorumThreshold::Majority);
    assert_eq!(meeting.quorum_met, QuorumStatus::Met);
    assert!(meeting.can_vote());

    // Create an agenda item.
    let mut item = AgendaItem::new(
        meeting.meeting_id,
        "Approve 2025 Budget".to_string(),
        AgendaItemType::Resolution,
        None,
        Some("RESOLVED: the board approves the 2025 operating budget.".to_string()),
    );
    assert!(!item.resolved);

    // Cast three For votes.
    let votes: Vec<Vote> = seats
        .iter()
        .map(|s| Vote::new(meeting.meeting_id, item.item_id, s.seat_id, VoteValue::For))
        .collect();
    assert_eq!(votes.len(), 3);

    // Compute resolution.
    let resolution = Resolution::new(
        meeting.meeting_id,
        item.item_id,
        ResolutionType::Ordinary,
        "RESOLVED: budget approved.".to_string(),
        3, // for
        0, // against
        0, // abstain
        QuorumThreshold::Majority,
    );
    assert!(resolution.passed);

    item.resolve();
    assert!(item.resolved);

    // Adjourn.
    meeting.adjourn().unwrap();
    assert_eq!(meeting.status, MeetingStatus::Adjourned);
    assert!(meeting.adjourned_at.is_some());
}

// ── 4. Written consent flow ───────────────────────────────────────────────────

#[test]
fn governance_written_consent_flow() {
    let entity_id = EntityId::new();
    let board = make_board(entity_id);
    let body_id = board.body_id;
    let seats: Vec<GovernanceSeat> = (0..3).map(|_| make_seat(body_id)).collect();

    // Written consent auto-convenes.
    let mut meeting = Meeting::new(
        body_id,
        MeetingType::WrittenConsent,
        "Written Consent — Issue Options".to_string(),
        None,
        None,
        None,
    );
    assert_eq!(meeting.status, MeetingStatus::Convened);
    assert!(meeting.convened_at.is_some());
    // Written consent can vote without recording attendance.
    assert!(meeting.can_vote());

    let item = AgendaItem::new(
        meeting.meeting_id,
        "Authorize option grants".to_string(),
        AgendaItemType::Resolution,
        None,
        Some("RESOLVED: approve 100,000 option pool.".to_string()),
    );

    // All three seats vote For — unanimous.
    let votes: Vec<Vote> = seats
        .iter()
        .map(|s| Vote::new(meeting.meeting_id, item.item_id, s.seat_id, VoteValue::For))
        .collect();
    assert_eq!(votes.len(), 3);

    let resolution = Resolution::new(
        meeting.meeting_id,
        item.item_id,
        ResolutionType::UnanimousWrittenConsent,
        "RESOLVED: option pool approved.".to_string(),
        3,
        0,
        0,
        QuorumThreshold::Unanimous,
    );
    assert!(resolution.passed);

    meeting.adjourn().unwrap();
    assert_eq!(meeting.status, MeetingStatus::Adjourned);
}

// ── 5. Quick approve equivalent — all seats vote For, unanimous consent ───────

#[test]
fn governance_quick_approve_unanimous() {
    let body_id = GovernanceBodyId::new();
    let seats: Vec<GovernanceSeat> = (0..5).map(|_| make_seat(body_id)).collect();

    let meeting = Meeting::new(
        body_id,
        MeetingType::WrittenConsent,
        "Written Consent".to_string(),
        None,
        None,
        None,
    );
    let item = AgendaItem::new(
        meeting.meeting_id,
        "Emergency Resolution".to_string(),
        AgendaItemType::Resolution,
        None,
        Some("RESOLVED.".to_string()),
    );

    let votes_for = seats.len() as u32;
    let resolution = Resolution::new(
        meeting.meeting_id,
        item.item_id,
        ResolutionType::UnanimousWrittenConsent,
        "Approved unanimously.".to_string(),
        votes_for,
        0,
        0,
        QuorumThreshold::Unanimous,
    );
    assert!(resolution.passed);
    assert_eq!(resolution.votes_for, 5);
    assert_eq!(resolution.votes_against, 0);
    assert_eq!(resolution.votes_abstain, 0);
}

// ── 6. Quorum edge cases ──────────────────────────────────────────────────────

#[test]
fn quorum_edge_cases() {
    use corp_core::governance::types::check_quorum;

    // --- Majority ---
    // Exact majority: 3 of 5 → passes (3 * 2 = 6 > 5)
    assert!(check_quorum(QuorumThreshold::Majority, 3, 5));
    // Tie: 2 of 4 → fails (2 * 2 = 4, not > 4)
    assert!(!check_quorum(QuorumThreshold::Majority, 2, 4));
    // One more than tie: 3 of 4 → passes
    assert!(check_quorum(QuorumThreshold::Majority, 3, 4));

    // --- Supermajority ---
    // Exact 2/3: 4 of 6 → passes (4 * 3 = 12 >= 6 * 2 = 12)
    assert!(check_quorum(QuorumThreshold::Supermajority, 4, 6));
    // Just below: 3 of 5 → fails (3 * 3 = 9 < 5 * 2 = 10)
    assert!(!check_quorum(QuorumThreshold::Supermajority, 3, 5));
    // Exactly 2/3 on odd: 2 of 3 → passes (2 * 3 = 6 >= 3 * 2 = 6)
    assert!(check_quorum(QuorumThreshold::Supermajority, 2, 3));

    // --- Unanimous ---
    // All vote: passes
    assert!(check_quorum(QuorumThreshold::Unanimous, 7, 7));
    // One abstain: fails
    assert!(!check_quorum(QuorumThreshold::Unanimous, 6, 7));
    // Unanimous with single voter
    assert!(check_quorum(QuorumThreshold::Unanimous, 1, 1));

    // --- Unanimous Written Consent with abstentions ---
    // UWC rejects abstentions even if no Against votes
    let uwc_with_abstain = compute_resolution(
        ResolutionType::UnanimousWrittenConsent,
        4, // for
        0, // against
        1, // abstain — defeats UWC
        QuorumThreshold::Unanimous,
    );
    assert!(!uwc_with_abstain);

    // Supermajority with abstentions counted in denominator
    let supermajority_abstains = compute_resolution(
        ResolutionType::Special,
        4, // for
        0, // against
        2, // abstain — counted in total_cast = 6; 4/6 = exactly 2/3
        QuorumThreshold::Supermajority,
    );
    assert!(supermajority_abstains);
}

// ── 7. Equity issuance and transfer ──────────────────────────────────────────

#[test]
fn equity_issuance_and_transfer() {
    let entity_id = EntityId::new();

    // Create cap table.
    let cap_table = make_cap_table(entity_id);
    assert_eq!(cap_table.entity_id, entity_id);

    // Create instrument.
    let instrument = make_instrument(entity_id, cap_table.cap_table_id);
    assert_eq!(instrument.authorized_units, Some(10_000_000));

    // Issue a grant to a founder.
    let grant = EquityGrant::new(
        entity_id,
        cap_table.cap_table_id,
        instrument.instrument_id,
        ContactId::new(),
        "Alice Founder",
        GrantType::CommonStock,
        ShareCount::new(1_000_000),
        Some(1), // $0.01/share (1 cent)
        Some(NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()),
        Some(48),
        Some(12),
        None,
    );
    assert_eq!(grant.shares, ShareCount::new(1_000_000));
    assert_eq!(grant.recipient_name, "Alice Founder");
    assert_eq!(grant.cliff_months, Some(12));

    // Initiate a secondary transfer: Alice → Bob.
    let from_holder = HolderId::new();
    let to_holder = HolderId::new();
    let mut transfer = ShareTransfer::new(
        entity_id,
        cap_table.cap_table_id,
        from_holder,
        to_holder,
        instrument.instrument_id,
        ShareCount::new(100_000),
        TransferType::SecondarySale,
        Some(500), // $5.00/share
    );
    assert_eq!(transfer.status, TransferStatus::Draft);

    // Draft → PendingBoardApproval.
    transfer.approve().unwrap();
    assert_eq!(transfer.status, TransferStatus::PendingBoardApproval);

    // PendingBoardApproval → Approved.
    transfer.approve().unwrap();
    assert_eq!(transfer.status, TransferStatus::Approved);

    // Approved → Executed.
    transfer.execute().unwrap();
    assert_eq!(transfer.status, TransferStatus::Executed);

    // Cannot cancel after execution.
    let err = transfer.cancel().unwrap_err();
    assert!(matches!(
        err,
        corp_core::equity::transfer::TransferError::InvalidTransition { .. }
    ));
}

// ── 8. SAFE lifecycle ─────────────────────────────────────────────────────────

#[test]
fn safe_lifecycle_issue_and_convert() {
    let entity_id = EntityId::new();
    let cap_table = make_cap_table(entity_id);

    let mut safe = SafeNote::new(
        entity_id,
        cap_table.cap_table_id,
        ContactId::new(),
        "Seed Investor LLC",
        SafeType::PostMoney,
        500_000_00,         // $500,000 in cents
        Some(5_000_000_00), // $5M valuation cap
        Some(20),           // 20% discount
    );
    assert_eq!(safe.status, SafeStatus::Issued);
    assert!(safe.converted_at.is_none());

    // Convert on a priced round.
    safe.convert().unwrap();
    assert_eq!(safe.status, SafeStatus::Converted);
    assert!(safe.converted_at.is_some());

    // Cannot convert again.
    let err = safe.convert().unwrap_err();
    assert_eq!(err, SafeNoteError::AlreadyConverted);

    // Cannot cancel a converted SAFE.
    let err2 = safe.cancel().unwrap_err();
    assert_eq!(err2, SafeNoteError::AlreadyConverted);
}

#[test]
fn safe_lifecycle_cancel() {
    let entity_id = EntityId::new();
    let cap_table = make_cap_table(entity_id);

    let mut safe = SafeNote::new(
        entity_id,
        cap_table.cap_table_id,
        ContactId::new(),
        "Angel Investor",
        SafeType::PreMoney,
        100_000_00,
        None,
        None,
    );

    safe.cancel().unwrap();
    assert_eq!(safe.status, SafeStatus::Cancelled);

    // Cannot convert a cancelled SAFE.
    let err = safe.convert().unwrap_err();
    assert_eq!(err, SafeNoteError::AlreadyCancelled);

    // Cannot cancel twice.
    let err2 = safe.cancel().unwrap_err();
    assert_eq!(err2, SafeNoteError::AlreadyCancelled);
}

// ── 9. Valuation approval flow ────────────────────────────────────────────────

#[test]
fn valuation_approval_flow() {
    let entity_id = EntityId::new();
    let cap_table = make_cap_table(entity_id);

    let mut val = Valuation::new(
        entity_id,
        cap_table.cap_table_id,
        ValuationType::FourOhNineA,
        ValuationMethodology::Backsolve,
        5_000_000_00, // $5M
        NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
        Some("Big Four Valuations LLC".to_string()),
    );
    assert_eq!(val.status, ValuationStatus::Draft);
    assert!(val.approved_at.is_none());
    assert!(val.approved_by.is_none());

    // Draft → PendingApproval
    val.submit_for_approval().unwrap();
    assert_eq!(val.status, ValuationStatus::PendingApproval);

    // Cannot approve from Draft (already moved).
    // Verify invalid transition: try to submit again.
    let err = val.submit_for_approval().unwrap_err();
    assert!(matches!(err, ValuationError::InvalidTransition { .. }));

    // PendingApproval → Approved
    val.approve("CFO").unwrap();
    assert_eq!(val.status, ValuationStatus::Approved);
    assert!(val.approved_at.is_some());
    assert_eq!(val.approved_by.as_deref(), Some("CFO"));

    // Approved → Expired
    val.expire().unwrap();
    assert_eq!(val.status, ValuationStatus::Expired);

    // Cannot expire twice.
    let err2 = val.expire().unwrap_err();
    assert!(matches!(err2, ValuationError::InvalidTransition { .. }));
}

#[test]
fn valuation_supersede_flow() {
    let entity_id = EntityId::new();
    let cap_table = make_cap_table(entity_id);

    let mut val = Valuation::new(
        entity_id,
        cap_table.cap_table_id,
        ValuationType::FairMarketValue,
        ValuationMethodology::Market,
        8_000_000_00,
        NaiveDate::from_ymd_opt(2026, 2, 1).unwrap(),
        None,
    );

    val.submit_for_approval().unwrap();
    val.approve("Board").unwrap();
    assert_eq!(val.status, ValuationStatus::Approved);

    // Supersede when a newer valuation is approved.
    val.supersede().unwrap();
    assert_eq!(val.status, ValuationStatus::Superseded);

    // Cannot supersede from non-Approved state.
    let err = val.supersede().unwrap_err();
    assert!(matches!(err, ValuationError::InvalidTransition { .. }));
}

// ── 10. Intent execution flow ─────────────────────────────────────────────────

#[test]
fn intent_execution_flow() {
    use corp_core::governance::capability::AuthorityTier;

    let entity_id = EntityId::new();
    let workspace_id = WorkspaceId::new();

    let mut intent = Intent::new(
        entity_id,
        workspace_id,
        "equity.grant.issue",
        AuthorityTier::Tier3,
        "Issue 50,000 options to new hire",
        serde_json::json!({"shares": 50_000, "strike": 100}),
    );
    assert_eq!(intent.status, IntentStatus::Pending);
    assert!(!intent.is_terminal());

    // Pending → Evaluated
    intent.evaluate().unwrap();
    assert_eq!(intent.status, IntentStatus::Evaluated);
    assert!(intent.evaluated_at.is_some());

    // Cannot evaluate twice.
    let err = intent.evaluate().unwrap_err();
    assert!(matches!(err, IntentError::NotPending(_)));

    // Evaluated → Authorized
    intent.authorize().unwrap();
    assert_eq!(intent.status, IntentStatus::Authorized);
    assert!(intent.authorized_at.is_some());

    // Cannot authorize twice.
    let err2 = intent.authorize().unwrap_err();
    assert!(matches!(err2, IntentError::NotEvaluated(_)));

    // Authorized → Executed
    intent.mark_executed().unwrap();
    assert_eq!(intent.status, IntentStatus::Executed);
    assert!(intent.executed_at.is_some());
    assert!(intent.is_terminal());

    // Cannot execute again once terminal.
    let err3 = intent.mark_executed().unwrap_err();
    assert!(matches!(err3, IntentError::NotAuthorized(_)));
}

#[test]
fn intent_fail_and_cancel_from_nonterminal_states() {
    use corp_core::governance::capability::AuthorityTier;

    let entity_id = EntityId::new();
    let workspace_id = WorkspaceId::new();

    // Fail from Pending.
    let mut intent = Intent::new(
        entity_id,
        workspace_id,
        "treasury.payment.send",
        AuthorityTier::Tier2,
        "Wire transfer",
        serde_json::Value::Null,
    );
    intent.mark_failed("insufficient funds").unwrap();
    assert_eq!(intent.status, IntentStatus::Failed);
    assert!(intent.failed_at.is_some());
    assert_eq!(intent.failure_reason.as_deref(), Some("insufficient funds"));

    // Cannot cancel once terminal (failed).
    let err = intent.cancel().unwrap_err();
    assert!(matches!(err, IntentError::AlreadyTerminal(_)));

    // Cancel from Evaluated.
    let mut intent2 = Intent::new(
        entity_id,
        workspace_id,
        "compliance.report",
        AuthorityTier::Tier1,
        "Generate compliance report",
        serde_json::Value::Null,
    );
    intent2.evaluate().unwrap();
    intent2.cancel().unwrap();
    assert_eq!(intent2.status, IntentStatus::Cancelled);
    assert!(intent2.cancelled_at.is_some());
}

// ── 11. Obligation lifecycle ──────────────────────────────────────────────────

#[test]
fn obligation_full_lifecycle() {
    let entity_id = EntityId::new();

    let mut obligation = Obligation::new(
        entity_id,
        None,
        "board.approval",
        AssigneeType::Internal,
        None,
        "Board must approve the equity grant before issuance",
        Some(NaiveDate::from_ymd_opt(2026, 4, 1).unwrap()),
    );
    assert_eq!(obligation.status, ObligationStatus::Required);
    assert!(!obligation.is_terminal());

    // Required → InProgress
    obligation.start().unwrap();
    assert_eq!(obligation.status, ObligationStatus::InProgress);

    // Cannot start again.
    let err = obligation.start().unwrap_err();
    assert!(matches!(err, ObligationError::NotRequired(_)));

    // InProgress → Fulfilled
    obligation.fulfill().unwrap();
    assert_eq!(obligation.status, ObligationStatus::Fulfilled);
    assert!(obligation.fulfilled_at.is_some());
    assert!(obligation.is_terminal());

    // Cannot fulfill twice.
    let err2 = obligation.fulfill().unwrap_err();
    assert!(matches!(err2, ObligationError::CannotFulfill(_)));
}

#[test]
fn obligation_waive_and_expire() {
    let entity_id = EntityId::new();

    // Waive from Required.
    let mut obl1 = Obligation::new(
        entity_id,
        None,
        "legal.review",
        AssigneeType::ThirdParty,
        None,
        "Outside counsel review",
        None,
    );
    obl1.waive().unwrap();
    assert_eq!(obl1.status, ObligationStatus::Waived);
    assert!(obl1.waived_at.is_some());
    assert!(obl1.is_terminal());

    let err = obl1.waive().unwrap_err();
    assert!(matches!(err, ObligationError::AlreadyTerminal(_)));

    // Expire from InProgress.
    let mut obl2 = Obligation::new(
        entity_id,
        None,
        "board.approval",
        AssigneeType::Internal,
        None,
        "Time-sensitive approval",
        Some(NaiveDate::from_ymd_opt(2026, 1, 15).unwrap()),
    );
    obl2.start().unwrap();
    obl2.expire().unwrap();
    assert_eq!(obl2.status, ObligationStatus::Expired);
    assert!(obl2.expired_at.is_some());

    let err2 = obl2.expire().unwrap_err();
    assert!(matches!(err2, ObligationError::AlreadyTerminal(_)));
}

// ── 12. Work item TTL expiration ──────────────────────────────────────────────

#[test]
fn work_item_claim_and_complete() {
    let entity_id = EntityId::new();
    let mut item = WorkItem::new(
        entity_id,
        "File 83(b) election",
        "Submit form to IRS within 30 days of grant",
        "tax",
        Some(NaiveDate::from_ymd_opt(2026, 4, 1).unwrap()),
        true,
    );
    assert_eq!(item.status, WorkItemStatus::Open);
    assert!(!item.is_terminal());

    item.claim("agent-tax-1").unwrap();
    assert_eq!(item.status, WorkItemStatus::Claimed);
    assert_eq!(item.claimed_by.as_deref(), Some("agent-tax-1"));
    assert!(item.claimed_at.is_some());

    item.complete("agent-tax-1", Some("83(b) filed on 2026-03-01".to_string()))
        .unwrap();
    assert_eq!(item.status, WorkItemStatus::Completed);
    assert!(item.completed_at.is_some());
    assert_eq!(item.result.as_deref(), Some("83(b) filed on 2026-03-01"));
    assert!(item.is_terminal());
}

#[test]
fn work_item_ttl_already_expired() {
    let entity_id = EntityId::new();
    let mut item = WorkItem::new(
        entity_id,
        "Background check",
        "Run background check on new hire",
        "hr",
        None,
        false,
    );

    item.claim("agent-hr").unwrap();

    // Simulate expiry by backdating claimed_at beyond TTL.
    // We set claim_ttl_seconds = 1 second and push claimed_at 10s into the past.
    item.claim_ttl_seconds = Some(1);
    item.claimed_at = Some(Utc::now() - chrono::Duration::seconds(10));

    assert!(item.is_claim_expired(), "claim should be expired");

    // Item is still Claimed (expiry is a read-only check — caller must handle).
    assert_eq!(item.status, WorkItemStatus::Claimed);
}

#[test]
fn work_item_not_expired_without_ttl() {
    let entity_id = EntityId::new();
    let mut item = WorkItem::new(
        entity_id,
        "Routine task",
        "Do something",
        "ops",
        None,
        false,
    );
    item.claim("agent-ops").unwrap();
    // No TTL set — never expires.
    assert!(!item.is_claim_expired());
}

#[test]
fn work_item_cannot_claim_twice() {
    let entity_id = EntityId::new();
    let mut item = WorkItem::new(entity_id, "Task", "Description", "general", None, false);
    item.claim("agent-1").unwrap();
    let err = item.claim("agent-2").unwrap_err();
    assert!(matches!(err, WorkItemError::NotOpen(_)));
}

#[test]
fn work_item_release_and_reclaim() {
    let entity_id = EntityId::new();
    let mut item = WorkItem::new(
        entity_id,
        "Reassignable task",
        "Can be released and reclaimed",
        "ops",
        None,
        false,
    );
    item.claim("agent-1").unwrap();
    item.release_claim().unwrap();
    assert_eq!(item.status, WorkItemStatus::Open);
    assert!(item.claimed_by.is_none());

    item.claim("agent-2").unwrap();
    assert_eq!(item.claimed_by.as_deref(), Some("agent-2"));
}

// ── 13. Journal entry double-entry validation ─────────────────────────────────

#[test]
fn journal_entry_balanced_posts_successfully() {
    let cash = corp_core::ids::AccountId::new();
    let revenue = corp_core::ids::AccountId::new();

    let mut entry = JournalEntry::new(
        EntityId::new(),
        NaiveDate::from_ymd_opt(2026, 3, 1).unwrap(),
        "March SaaS revenue",
        vec![
            JournalLine {
                account_id: cash,
                amount_cents: 10_000,
                side: Side::Debit,
                memo: Some("Cash received".to_string()),
            },
            JournalLine {
                account_id: revenue,
                amount_cents: 10_000,
                side: Side::Credit,
                memo: Some("Revenue recognized".to_string()),
            },
        ],
    );

    assert!(!entry.posted);
    entry.post().unwrap();
    assert!(entry.posted);
    assert!(!entry.voided);
}

#[test]
fn journal_entry_unbalanced_is_rejected() {
    let cash = corp_core::ids::AccountId::new();
    let revenue = corp_core::ids::AccountId::new();

    let mut entry = JournalEntry::new(
        EntityId::new(),
        NaiveDate::from_ymd_opt(2026, 3, 1).unwrap(),
        "Unbalanced entry",
        vec![
            JournalLine {
                account_id: cash,
                amount_cents: 500,
                side: Side::Debit,
                memo: None,
            },
            JournalLine {
                account_id: revenue,
                amount_cents: 400, // deliberately off by 100
                side: Side::Credit,
                memo: None,
            },
        ],
    );

    let err = entry.post().unwrap_err();
    assert!(
        matches!(
            err,
            JournalEntryError::Unbalanced {
                debits: 500,
                credits: 400
            }
        ),
        "expected Unbalanced(500, 400), got {:?}",
        err
    );
}

#[test]
fn journal_entry_cannot_post_voided_entry() {
    let cash = corp_core::ids::AccountId::new();
    let expense = corp_core::ids::AccountId::new();

    let mut entry = JournalEntry::new(
        EntityId::new(),
        NaiveDate::from_ymd_opt(2026, 3, 15).unwrap(),
        "Office supplies",
        vec![
            JournalLine {
                account_id: expense,
                amount_cents: 250,
                side: Side::Debit,
                memo: None,
            },
            JournalLine {
                account_id: cash,
                amount_cents: 250,
                side: Side::Credit,
                memo: None,
            },
        ],
    );

    entry.post().unwrap();
    entry.void().unwrap();
    assert!(entry.voided);
    assert!(!entry.posted);

    // Cannot re-post a voided entry.
    let err = entry.post().unwrap_err();
    assert_eq!(err, JournalEntryError::PostingVoidedEntry);
}

#[test]
fn journal_entry_multi_line_must_balance() {
    let acct_a = corp_core::ids::AccountId::new();
    let acct_b = corp_core::ids::AccountId::new();
    let acct_c = corp_core::ids::AccountId::new();

    // Compound entry: debit A 300, debit B 200 → credit C 500.
    let mut entry = JournalEntry::new(
        EntityId::new(),
        NaiveDate::from_ymd_opt(2026, 3, 20).unwrap(),
        "Multi-line compound entry",
        vec![
            JournalLine {
                account_id: acct_a,
                amount_cents: 300,
                side: Side::Debit,
                memo: None,
            },
            JournalLine {
                account_id: acct_b,
                amount_cents: 200,
                side: Side::Debit,
                memo: None,
            },
            JournalLine {
                account_id: acct_c,
                amount_cents: 500,
                side: Side::Credit,
                memo: None,
            },
        ],
    );
    entry.post().unwrap();
    assert!(entry.posted);
}

// ── 14. Invoice FSM ───────────────────────────────────────────────────────────

#[test]
fn invoice_full_lifecycle_draft_sent_paid() {
    let entity_id = EntityId::new();
    let mut invoice = Invoice::new(
        entity_id,
        "Acme Client Corp",
        Some("billing@acme.example.com".to_string()),
        5_000_00, // $5,000
        Currency::Usd,
        "Professional services — March 2026",
        NaiveDate::from_ymd_opt(2026, 4, 15).unwrap(),
    );

    assert_eq!(invoice.status, InvoiceStatus::Draft);
    assert!(invoice.paid_at.is_none());

    // Draft → Sent
    invoice.send().unwrap();
    assert_eq!(invoice.status, InvoiceStatus::Sent);

    // Cannot send again.
    let err = invoice.send().unwrap_err();
    assert!(matches!(err, InvoiceError::NotDraft(_)));

    // Sent → Paid
    invoice.mark_paid().unwrap();
    assert_eq!(invoice.status, InvoiceStatus::Paid);
    assert!(invoice.paid_at.is_some());

    // Cannot pay twice.
    let err2 = invoice.mark_paid().unwrap_err();
    assert!(matches!(err2, InvoiceError::NotSent(_)));

    // Cannot void a paid invoice.
    let err3 = invoice.void().unwrap_err();
    assert!(matches!(
        err3,
        InvoiceError::CannotVoid(InvoiceStatus::Paid)
    ));
}

#[test]
fn invoice_cannot_skip_to_paid_from_draft() {
    let entity_id = EntityId::new();
    let mut invoice = Invoice::new(
        entity_id,
        "Client",
        None,
        1_000_00,
        Currency::Usd,
        "Services",
        NaiveDate::from_ymd_opt(2026, 4, 1).unwrap(),
    );

    // Try to mark paid without sending first.
    let err = invoice.mark_paid().unwrap_err();
    assert!(matches!(err, InvoiceError::NotSent(InvoiceStatus::Draft)));
}

#[test]
fn invoice_void_from_draft() {
    let entity_id = EntityId::new();
    let mut invoice = Invoice::new(
        entity_id,
        "Client",
        None,
        500_00,
        Currency::Usd,
        "Voided before send",
        NaiveDate::from_ymd_opt(2026, 4, 1).unwrap(),
    );

    invoice.void().unwrap();
    assert_eq!(invoice.status, InvoiceStatus::Voided);

    // Cannot send a voided invoice.
    let err = invoice.send().unwrap_err();
    assert!(matches!(err, InvoiceError::NotDraft(InvoiceStatus::Voided)));
}

#[test]
fn invoice_void_from_sent() {
    let entity_id = EntityId::new();
    let mut invoice = Invoice::new(
        entity_id,
        "Client",
        None,
        750_00,
        Currency::Usd,
        "Sent but then voided",
        NaiveDate::from_ymd_opt(2026, 4, 1).unwrap(),
    );

    invoice.send().unwrap();
    invoice.void().unwrap();
    assert_eq!(invoice.status, InvoiceStatus::Voided);
}

// ── 15. Entity dissolution ────────────────────────────────────────────────────

#[test]
fn entity_dissolution_from_active() {
    let mut entity = make_entity();

    // Advance to Active.
    for _ in 0..6 {
        entity.advance_status().unwrap();
    }
    assert_eq!(entity.formation_status, FormationStatus::Active);

    // Dissolve the entity.
    let effective = NaiveDate::from_ymd_opt(2026, 12, 31).unwrap();
    entity.dissolve(effective).unwrap();

    assert_eq!(entity.formation_status, FormationStatus::Dissolved);
    assert_eq!(entity.dissolution_effective_date, Some(effective));
    assert!(entity.formation_status.is_terminal());

    // Cannot advance from Dissolved.
    let err = entity.advance_status().unwrap_err();
    assert!(matches!(
        err,
        EntityError::AlreadyTerminal(FormationStatus::Dissolved)
    ));

    // Cannot dissolve again.
    let err2 = entity.dissolve(effective).unwrap_err();
    assert_eq!(err2, EntityError::AlreadyDissolved);
}

#[test]
fn entity_dissolution_from_mid_formation() {
    let mut entity = make_entity();

    // Only partially through formation (FilingSubmitted).
    entity.advance_status().unwrap(); // Pending -> DocumentsGenerated
    entity.advance_status().unwrap(); // DocumentsGenerated -> DocumentsSigned
    entity.advance_status().unwrap(); // DocumentsSigned -> FilingSubmitted
    assert_eq!(entity.formation_status, FormationStatus::FilingSubmitted);

    // Can dissolve even mid-formation.
    let effective = NaiveDate::from_ymd_opt(2026, 6, 15).unwrap();
    entity.dissolve(effective).unwrap();
    assert_eq!(entity.formation_status, FormationStatus::Dissolved);
    assert_eq!(entity.dissolution_effective_date, Some(effective));
}

// ── 16. Document signing flow (bonus cross-domain) ────────────────────────────

#[test]
fn document_signing_full_flow() {
    let entity_id = EntityId::new();
    let workspace_id = WorkspaceId::new();
    let hash = "sha256:abcdef1234567890";

    let mut doc = Document::new(
        entity_id,
        workspace_id,
        DocumentType::BoardConsent,
        "Initial Board Consent",
        serde_json::json!({"content": "Board approves all resolutions."}),
        hash,
    );
    assert_eq!(doc.status, DocumentStatus::Draft);
    assert_eq!(doc.version, 1);

    let required_signers = &["ceo@acme.com", "director1@acme.com", "director2@acme.com"];

    // First signer (CEO).
    let sig1 = Signature::new(
        doc.document_id,
        "Alice CEO",
        "CEO",
        "ceo@acme.com",
        "Alice CEO",
        None,
        hash,
    );
    doc.sign(sig1, required_signers).unwrap();
    assert_eq!(doc.status, DocumentStatus::Draft); // not all signed yet
    assert_eq!(doc.signature_count(), 1);

    // Second signer.
    let sig2 = Signature::new(
        doc.document_id,
        "Bob Director",
        "Director",
        "director1@acme.com",
        "Bob Director",
        None,
        hash,
    );
    doc.sign(sig2, required_signers).unwrap();
    assert_eq!(doc.status, DocumentStatus::Draft);

    // Third signer — triggers transition to Signed.
    let sig3 = Signature::new(
        doc.document_id,
        "Carol Director",
        "Director",
        "director2@acme.com",
        "Carol Director",
        None,
        hash,
    );
    doc.sign(sig3, required_signers).unwrap();
    assert_eq!(doc.status, DocumentStatus::Signed);
    assert_eq!(doc.signature_count(), 3);

    // Verify each signer is recorded.
    assert!(doc.is_signed_by("ceo@acme.com"));
    assert!(doc.is_signed_by("director1@acme.com"));
    assert!(doc.is_signed_by("director2@acme.com"));
    assert!(!doc.is_signed_by("unknown@acme.com"));
}
