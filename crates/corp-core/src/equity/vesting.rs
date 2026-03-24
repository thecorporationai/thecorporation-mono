//! Vesting schedules and vesting events.

use chrono::{DateTime, Datelike, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use crate::ids::{EntityId, EquityGrantId, VestingEventId, VestingScheduleId};
use super::types::{ShareCount, VestingEventStatus, VestingEventType, VestingStatus};

// ── Error ─────────────────────────────────────────────────────────────────────

/// Errors produced by vesting FSM transitions.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EquityError {
    #[error("vesting event is already vested")]
    AlreadyVested,
    #[error("vesting event is already forfeited")]
    AlreadyForfeited,
    #[error("vesting event is already cancelled")]
    AlreadyCancelled,
    #[error("vesting event must be in Scheduled state to transition (current: {current})")]
    NotScheduled { current: &'static str },
    #[error("position quantity would overflow")]
    QuantityOverflow,
    #[error("position quantity must not be negative")]
    NegativeQuantity,
    #[error("position principal must not be negative")]
    NegativePrincipal,
}

// ── VestingSchedule ───────────────────────────────────────────────────────────

/// A vesting schedule attached to an equity grant.
///
/// Encodes the full parameters needed to derive a timeline of [`VestingEvent`]s
/// via [`materialize_vesting_events`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VestingSchedule {
    pub schedule_id: VestingScheduleId,
    pub grant_id: EquityGrantId,
    pub entity_id: EntityId,
    pub total_shares: ShareCount,
    pub vesting_start_date: NaiveDate,
    /// Human-readable template label, e.g. `"4yr/1yr cliff"`.
    pub template: String,
    /// Number of months before the cliff vests.
    pub cliff_months: u32,
    /// Total vesting duration in months.
    pub total_months: u32,
    pub acceleration_single_trigger: bool,
    pub acceleration_double_trigger: bool,
    pub early_exercise_allowed: bool,
    pub status: VestingStatus,
    pub terminated_at: Option<NaiveDate>,
    pub created_at: DateTime<Utc>,
}

impl VestingSchedule {
    /// Create a new vesting schedule in the `Active` state.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        grant_id: EquityGrantId,
        entity_id: EntityId,
        total_shares: ShareCount,
        vesting_start_date: NaiveDate,
        template: impl Into<String>,
        cliff_months: u32,
        total_months: u32,
        acceleration_single_trigger: bool,
        acceleration_double_trigger: bool,
        early_exercise_allowed: bool,
    ) -> Self {
        Self {
            schedule_id: VestingScheduleId::new(),
            grant_id,
            entity_id,
            total_shares,
            vesting_start_date,
            template: template.into(),
            cliff_months,
            total_months,
            acceleration_single_trigger,
            acceleration_double_trigger,
            early_exercise_allowed,
            status: VestingStatus::Active,
            terminated_at: None,
            created_at: Utc::now(),
        }
    }

    /// Mark this schedule as terminated as of `date`.
    pub fn terminate(&mut self, date: NaiveDate) {
        self.status = VestingStatus::Terminated;
        self.terminated_at = Some(date);
    }
}

// ── VestingEvent ──────────────────────────────────────────────────────────────

/// A single scheduled or realised vesting event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VestingEvent {
    pub event_id: VestingEventId,
    pub schedule_id: VestingScheduleId,
    pub grant_id: EquityGrantId,
    pub entity_id: EntityId,
    pub vest_date: NaiveDate,
    pub share_count: ShareCount,
    pub event_type: VestingEventType,
    pub status: VestingEventStatus,
    pub note: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl VestingEvent {
    /// Create a new vesting event in the `Scheduled` state.
    pub fn new(
        schedule_id: VestingScheduleId,
        grant_id: EquityGrantId,
        entity_id: EntityId,
        vest_date: NaiveDate,
        share_count: ShareCount,
        event_type: VestingEventType,
        note: Option<String>,
    ) -> Self {
        Self {
            event_id: VestingEventId::new(),
            schedule_id,
            grant_id,
            entity_id,
            vest_date,
            share_count,
            event_type,
            status: VestingEventStatus::Scheduled,
            note,
            created_at: Utc::now(),
        }
    }

    /// Transition `Scheduled` → `Vested`.
    pub fn vest(&mut self) -> Result<(), EquityError> {
        match self.status {
            VestingEventStatus::Scheduled => {
                self.status = VestingEventStatus::Vested;
                Ok(())
            }
            VestingEventStatus::Vested => Err(EquityError::AlreadyVested),
            VestingEventStatus::Forfeited => Err(EquityError::NotScheduled { current: "forfeited" }),
            VestingEventStatus::Cancelled => Err(EquityError::NotScheduled { current: "cancelled" }),
        }
    }

    /// Transition `Scheduled` → `Forfeited`.
    pub fn forfeit(&mut self) -> Result<(), EquityError> {
        match self.status {
            VestingEventStatus::Scheduled => {
                self.status = VestingEventStatus::Forfeited;
                Ok(())
            }
            VestingEventStatus::Vested => Err(EquityError::NotScheduled { current: "vested" }),
            VestingEventStatus::Forfeited => Err(EquityError::AlreadyForfeited),
            VestingEventStatus::Cancelled => Err(EquityError::NotScheduled { current: "cancelled" }),
        }
    }

    /// Transition `Scheduled` → `Cancelled`.
    pub fn cancel(&mut self) -> Result<(), EquityError> {
        match self.status {
            VestingEventStatus::Scheduled => {
                self.status = VestingEventStatus::Cancelled;
                Ok(())
            }
            VestingEventStatus::Vested => Err(EquityError::NotScheduled { current: "vested" }),
            VestingEventStatus::Forfeited => Err(EquityError::NotScheduled { current: "forfeited" }),
            VestingEventStatus::Cancelled => Err(EquityError::AlreadyCancelled),
        }
    }
}

// ── materialize_vesting_events ────────────────────────────────────────────────

/// Add `months` months to a `NaiveDate`, clamping to the last day of the
/// resulting month if necessary (e.g. Jan 31 + 1 month → Feb 28/29).
fn add_months(date: NaiveDate, months: u32) -> NaiveDate {
    let total_months = date.year() as u32 * 12 + date.month() - 1 + months;
    let year = (total_months / 12) as i32;
    let month = (total_months % 12) + 1;
    // Clamp the day to the last valid day in the target month.
    let days_in_month = days_in_month(year, month);
    let day = date.day().min(days_in_month);
    NaiveDate::from_ymd_opt(year, month, day)
        .expect("clamped date must be valid")
}

fn days_in_month(year: i32, month: u32) -> u32 {
    let next_month_year = if month == 12 { year + 1 } else { year };
    let next_month = if month == 12 { 1 } else { month + 1 };
    NaiveDate::from_ymd_opt(next_month_year, next_month, 1)
        .unwrap()
        .signed_duration_since(NaiveDate::from_ymd_opt(year, month, 1).unwrap())
        .num_days() as u32
}

/// Generate all vesting events for a schedule.
///
/// Algorithm:
/// 1. If `cliff_months > 0` and `total_months > 0`: emit one cliff event at
///    `start + cliff_months` covering the cliff tranche.
/// 2. Monthly events from `cliff_months + 1` through `total_months`.
/// 3. The last event absorbs any remainder so that `sum(shares) == total_shares`.
///
/// If `total_months == 0` a single immediate-vest event is emitted with
/// `total_shares`.
pub fn materialize_vesting_events(schedule: &VestingSchedule) -> Vec<VestingEvent> {
    let total = schedule.total_shares.raw();
    let start = schedule.vesting_start_date;
    let sid = schedule.schedule_id;
    let gid = schedule.grant_id;
    let eid = schedule.entity_id;

    // Edge case: immediate vest
    if schedule.total_months == 0 {
        return vec![VestingEvent::new(
            sid,
            gid,
            eid,
            start,
            ShareCount::new(total),
            VestingEventType::Cliff,
            Some("Immediate vest".to_string()),
        )];
    }

    let cliff = schedule.cliff_months;
    let total_months = schedule.total_months;

    // Number of monthly events (excluding the cliff event itself)
    let monthly_count: u32 = if cliff < total_months {
        total_months - cliff
    } else {
        0
    };

    // Total events: cliff_event (if cliff > 0) + monthly_count events
    let event_count = (if cliff > 0 { 1u32 } else { 0u32 }) + monthly_count;

    if event_count == 0 {
        return vec![];
    }

    // Cliff shares: proportional to cliff_months / total_months
    let cliff_shares = if cliff > 0 {
        (total as i128 * cliff as i128 / total_months as i128) as i64
    } else {
        0
    };

    // Remaining shares split across monthly_count events
    let remaining = total - cliff_shares;
    let per_month = if monthly_count > 0 {
        remaining / monthly_count as i64
    } else {
        0
    };

    let mut events: Vec<VestingEvent> = Vec::with_capacity(event_count as usize);
    let mut allocated: i64 = 0;

    // Cliff event
    if cliff > 0 {
        let cliff_date = add_months(start, cliff);
        events.push(VestingEvent::new(
            sid,
            gid,
            eid,
            cliff_date,
            ShareCount::new(cliff_shares),
            VestingEventType::Cliff,
            None,
        ));
        allocated += cliff_shares;
    }

    // Monthly events: from cliff+1 through total_months
    for m in (cliff + 1)..=total_months {
        let vest_date = add_months(start, m);
        let month_shares = if m == total_months {
            // Last event gets all remaining shares to absorb rounding
            total - allocated
        } else {
            per_month
        };
        events.push(VestingEvent::new(
            sid,
            gid,
            eid,
            vest_date,
            ShareCount::new(month_shares),
            VestingEventType::Monthly,
            None,
        ));
        allocated += month_shares;
    }

    events
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_schedule(
        total_shares: i64,
        cliff_months: u32,
        total_months: u32,
    ) -> VestingSchedule {
        VestingSchedule::new(
            EquityGrantId::new(),
            EntityId::new(),
            ShareCount::new(total_shares),
            NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            "test",
            cliff_months,
            total_months,
            false,
            false,
            false,
        )
    }

    // ── materialize_vesting_events ────────────────────────────────────────────

    #[test]
    fn standard_4yr_1yr_cliff() {
        let schedule = make_schedule(4_800_000, 12, 48);
        let events = materialize_vesting_events(&schedule);

        // 1 cliff + 36 monthly = 37 events
        assert_eq!(events.len(), 37);

        // Cliff event
        assert_eq!(events[0].event_type, VestingEventType::Cliff);
        // Cliff = 12/48 = 25% of 4_800_000 = 1_200_000
        assert_eq!(events[0].share_count.raw(), 1_200_000);
        assert_eq!(events[0].vest_date, NaiveDate::from_ymd_opt(2027, 1, 1).unwrap());

        // Monthly events
        for e in &events[1..] {
            assert_eq!(e.event_type, VestingEventType::Monthly);
        }

        // Total shares must equal sum of all events
        let total: i64 = events.iter().map(|e| e.share_count.raw()).sum();
        assert_eq!(total, 4_800_000);
    }

    #[test]
    fn standard_4yr_1yr_cliff_event_count() {
        // 1_000_000 shares, 4yr/1yr cliff → 1 cliff + 36 monthly = 37
        let schedule = make_schedule(1_000_000, 12, 48);
        let events = materialize_vesting_events(&schedule);
        assert_eq!(events.len(), 37);
    }

    #[test]
    fn immediate_vest_zero_months() {
        let schedule = make_schedule(1_000_000, 0, 0);
        let events = materialize_vesting_events(&schedule);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].share_count.raw(), 1_000_000);
        assert_eq!(events[0].event_type, VestingEventType::Cliff);
    }

    #[test]
    fn no_cliff_monthly_only() {
        let schedule = make_schedule(1_200, 0, 12);
        let events = materialize_vesting_events(&schedule);
        // No cliff event, 12 monthly events
        assert_eq!(events.len(), 12);
        for e in &events {
            assert_eq!(e.event_type, VestingEventType::Monthly);
        }
        let total: i64 = events.iter().map(|e| e.share_count.raw()).sum();
        assert_eq!(total, 1_200);
    }

    #[test]
    fn uneven_shares_remainder_last_event_gets_leftover() {
        // 10 shares over 3 months — 3 shares per month, last gets remainder
        let schedule = make_schedule(10, 0, 3);
        let events = materialize_vesting_events(&schedule);
        assert_eq!(events.len(), 3);
        // First two: 3 shares each (10/3 = 3)
        assert_eq!(events[0].share_count.raw(), 3);
        assert_eq!(events[1].share_count.raw(), 3);
        // Last: 4 shares (10 - 3 - 3)
        assert_eq!(events[2].share_count.raw(), 4);
        let total: i64 = events.iter().map(|e| e.share_count.raw()).sum();
        assert_eq!(total, 10);
    }

    #[test]
    fn uneven_cliff_remainder_last_event_gets_leftover() {
        // 100 shares, 1 month cliff, 4 months total
        // cliff = 1/4 * 100 = 25; remaining 75 over 3 months = 25 each
        let schedule = make_schedule(100, 1, 4);
        let events = materialize_vesting_events(&schedule);
        assert_eq!(events.len(), 4);
        let total: i64 = events.iter().map(|e| e.share_count.raw()).sum();
        assert_eq!(total, 100);
    }

    // ── terminate_schedule ────────────────────────────────────────────────────

    #[test]
    fn terminate_schedule_sets_status_and_date() {
        let mut schedule = make_schedule(1_000_000, 12, 48);
        assert_eq!(schedule.status, VestingStatus::Active);
        let date = NaiveDate::from_ymd_opt(2027, 6, 15).unwrap();
        schedule.terminate(date);
        assert_eq!(schedule.status, VestingStatus::Terminated);
        assert_eq!(schedule.terminated_at, Some(date));
    }

    #[test]
    fn new_schedule_has_no_terminated_at() {
        let schedule = make_schedule(1_000_000, 12, 48);
        assert!(schedule.terminated_at.is_none());
    }

    // ── VestingEvent FSM ──────────────────────────────────────────────────────

    fn make_event() -> VestingEvent {
        let schedule = make_schedule(1_000, 12, 48);
        VestingEvent::new(
            schedule.schedule_id,
            schedule.grant_id,
            schedule.entity_id,
            NaiveDate::from_ymd_opt(2027, 1, 1).unwrap(),
            ShareCount::new(250),
            VestingEventType::Cliff,
            None,
        )
    }

    #[test]
    fn new_event_status_is_scheduled() {
        let e = make_event();
        assert_eq!(e.status, VestingEventStatus::Scheduled);
    }

    #[test]
    fn vest_transitions_scheduled_to_vested() {
        let mut e = make_event();
        e.vest().unwrap();
        assert_eq!(e.status, VestingEventStatus::Vested);
    }

    #[test]
    fn forfeit_transitions_scheduled_to_forfeited() {
        let mut e = make_event();
        e.forfeit().unwrap();
        assert_eq!(e.status, VestingEventStatus::Forfeited);
    }

    #[test]
    fn cancel_transitions_scheduled_to_cancelled() {
        let mut e = make_event();
        e.cancel().unwrap();
        assert_eq!(e.status, VestingEventStatus::Cancelled);
    }

    // Double-transition rejections

    #[test]
    fn vest_already_vested_returns_error() {
        let mut e = make_event();
        e.vest().unwrap();
        assert_eq!(e.vest().unwrap_err(), EquityError::AlreadyVested);
    }

    #[test]
    fn vest_forfeited_returns_error() {
        let mut e = make_event();
        e.forfeit().unwrap();
        assert!(matches!(e.vest().unwrap_err(), EquityError::NotScheduled { .. }));
    }

    #[test]
    fn vest_cancelled_returns_error() {
        let mut e = make_event();
        e.cancel().unwrap();
        assert!(matches!(e.vest().unwrap_err(), EquityError::NotScheduled { .. }));
    }

    #[test]
    fn forfeit_already_forfeited_returns_error() {
        let mut e = make_event();
        e.forfeit().unwrap();
        assert_eq!(e.forfeit().unwrap_err(), EquityError::AlreadyForfeited);
    }

    #[test]
    fn forfeit_vested_returns_error() {
        let mut e = make_event();
        e.vest().unwrap();
        assert!(matches!(e.forfeit().unwrap_err(), EquityError::NotScheduled { .. }));
    }

    #[test]
    fn cancel_already_cancelled_returns_error() {
        let mut e = make_event();
        e.cancel().unwrap();
        assert_eq!(e.cancel().unwrap_err(), EquityError::AlreadyCancelled);
    }

    #[test]
    fn cancel_vested_returns_error() {
        let mut e = make_event();
        e.vest().unwrap();
        assert!(matches!(e.cancel().unwrap_err(), EquityError::NotScheduled { .. }));
    }

    // ── serde roundtrips ──────────────────────────────────────────────────────

    #[test]
    fn vesting_schedule_serde_roundtrip() {
        let s = make_schedule(1_000_000, 12, 48);
        let json = serde_json::to_string(&s).unwrap();
        let de: VestingSchedule = serde_json::from_str(&json).unwrap();
        assert_eq!(de.schedule_id, s.schedule_id);
        assert_eq!(de.total_shares, s.total_shares);
    }

    #[test]
    fn vesting_event_serde_roundtrip() {
        let e = make_event();
        let json = serde_json::to_string(&e).unwrap();
        let de: VestingEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(de.event_id, e.event_id);
        assert_eq!(de.status, VestingEventStatus::Scheduled);
    }

    // ── date arithmetic ───────────────────────────────────────────────────────

    #[test]
    fn add_months_no_overflow() {
        let d = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        assert_eq!(add_months(d, 1), NaiveDate::from_ymd_opt(2026, 2, 1).unwrap());
        assert_eq!(add_months(d, 12), NaiveDate::from_ymd_opt(2027, 1, 1).unwrap());
    }

    #[test]
    fn add_months_clamps_day_for_short_months() {
        // Jan 31 + 1 month → Feb 28 (non-leap)
        let d = NaiveDate::from_ymd_opt(2026, 1, 31).unwrap();
        assert_eq!(add_months(d, 1), NaiveDate::from_ymd_opt(2026, 2, 28).unwrap());
    }

    #[test]
    fn total_shares_equals_sum_of_events_various_schedules() {
        for (total, cliff, months) in [(1_000_000i64, 12u32, 48u32), (999, 0, 36), (7, 3, 12)] {
            let s = make_schedule(total, cliff, months);
            let events = materialize_vesting_events(&s);
            let sum: i64 = events.iter().map(|e| e.share_count.raw()).sum();
            assert_eq!(sum, total, "mismatch for total={total} cliff={cliff} months={months}");
        }
    }
}
