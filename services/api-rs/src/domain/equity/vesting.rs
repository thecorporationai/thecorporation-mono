//! Vesting schedule and event records.

use chrono::{DateTime, Datelike, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use super::error::EquityError;
use super::types::{ShareCount, VestingEventStatus, VestingEventType, VestingStatus};
use crate::domain::ids::{EntityId, EquityGrantId, VestingEventId, VestingScheduleId};

/// A vesting schedule attached to an equity grant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VestingSchedule {
    schedule_id: VestingScheduleId,
    grant_id: EquityGrantId,
    entity_id: EntityId,
    total_shares: ShareCount,
    vesting_start_date: NaiveDate,
    template: String,
    cliff_months: u32,
    total_months: u32,
    acceleration_single_trigger: bool,
    acceleration_double_trigger: bool,
    early_exercise_allowed: bool,
    status: VestingStatus,
    terminated_at: Option<NaiveDate>,
    created_at: DateTime<Utc>,
}

impl VestingSchedule {
    /// Create a new vesting schedule.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        schedule_id: VestingScheduleId,
        grant_id: EquityGrantId,
        entity_id: EntityId,
        total_shares: ShareCount,
        vesting_start_date: NaiveDate,
        template: String,
        cliff_months: u32,
        total_months: u32,
        acceleration_single_trigger: bool,
        acceleration_double_trigger: bool,
        early_exercise_allowed: bool,
    ) -> Self {
        Self {
            schedule_id,
            grant_id,
            entity_id,
            total_shares,
            vesting_start_date,
            template,
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

    /// Terminate the vesting schedule.
    pub fn terminate(&mut self, date: NaiveDate) {
        self.status = VestingStatus::Terminated;
        self.terminated_at = Some(date);
    }

    pub fn schedule_id(&self) -> VestingScheduleId {
        self.schedule_id
    }

    pub fn grant_id(&self) -> EquityGrantId {
        self.grant_id
    }

    pub fn entity_id(&self) -> EntityId {
        self.entity_id
    }

    pub fn total_shares(&self) -> ShareCount {
        self.total_shares
    }

    pub fn vesting_start_date(&self) -> NaiveDate {
        self.vesting_start_date
    }

    pub fn template(&self) -> &str {
        &self.template
    }

    pub fn cliff_months(&self) -> u32 {
        self.cliff_months
    }

    pub fn total_months(&self) -> u32 {
        self.total_months
    }

    pub fn acceleration_single_trigger(&self) -> bool {
        self.acceleration_single_trigger
    }

    pub fn acceleration_double_trigger(&self) -> bool {
        self.acceleration_double_trigger
    }

    pub fn early_exercise_allowed(&self) -> bool {
        self.early_exercise_allowed
    }

    pub fn status(&self) -> VestingStatus {
        self.status
    }

    pub fn terminated_at(&self) -> Option<NaiveDate> {
        self.terminated_at
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}

/// A single vesting event within a schedule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VestingEvent {
    event_id: VestingEventId,
    schedule_id: VestingScheduleId,
    grant_id: EquityGrantId,
    entity_id: EntityId,
    vest_date: NaiveDate,
    share_count: ShareCount,
    event_type: VestingEventType,
    status: VestingEventStatus,
    note: Option<String>,
    created_at: DateTime<Utc>,
}

impl VestingEvent {
    /// Create a new vesting event.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        event_id: VestingEventId,
        schedule_id: VestingScheduleId,
        grant_id: EquityGrantId,
        entity_id: EntityId,
        vest_date: NaiveDate,
        share_count: ShareCount,
        event_type: VestingEventType,
    ) -> Self {
        Self {
            event_id,
            schedule_id,
            grant_id,
            entity_id,
            vest_date,
            share_count,
            event_type,
            status: VestingEventStatus::Scheduled,
            note: None,
            created_at: Utc::now(),
        }
    }

    /// Mark this event as vested. Only valid from `Scheduled`.
    pub fn vest(&mut self) -> Result<(), EquityError> {
        if self.status != VestingEventStatus::Scheduled {
            return Err(EquityError::InvalidVestingEventTransition {
                from: self.status,
                to: VestingEventStatus::Vested,
            });
        }
        self.status = VestingEventStatus::Vested;
        Ok(())
    }

    /// Mark this event as forfeited. Only valid from `Scheduled`.
    pub fn forfeit(&mut self) -> Result<(), EquityError> {
        if self.status != VestingEventStatus::Scheduled {
            return Err(EquityError::InvalidVestingEventTransition {
                from: self.status,
                to: VestingEventStatus::Forfeited,
            });
        }
        self.status = VestingEventStatus::Forfeited;
        Ok(())
    }

    /// Cancel this event. Only valid from `Scheduled`.
    pub fn cancel(&mut self) -> Result<(), EquityError> {
        if self.status != VestingEventStatus::Scheduled {
            return Err(EquityError::InvalidVestingEventTransition {
                from: self.status,
                to: VestingEventStatus::Cancelled,
            });
        }
        self.status = VestingEventStatus::Cancelled;
        Ok(())
    }

    pub fn event_id(&self) -> VestingEventId {
        self.event_id
    }

    pub fn schedule_id(&self) -> VestingScheduleId {
        self.schedule_id
    }

    pub fn grant_id(&self) -> EquityGrantId {
        self.grant_id
    }

    pub fn entity_id(&self) -> EntityId {
        self.entity_id
    }

    pub fn vest_date(&self) -> NaiveDate {
        self.vest_date
    }

    pub fn share_count(&self) -> ShareCount {
        self.share_count
    }

    pub fn event_type(&self) -> VestingEventType {
        self.event_type
    }

    pub fn status(&self) -> VestingEventStatus {
        self.status
    }

    pub fn note(&self) -> Option<&str> {
        self.note.as_deref()
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}

/// Add months to a NaiveDate, clamping to valid day of month.
///
/// Returns `None` only if the resulting year overflows chrono's range, which
/// cannot happen for any realistic vesting schedule (< 100 years).
fn add_months(date: NaiveDate, months: u32) -> NaiveDate {
    let total_months = date.month0() + months;
    // Safe: u32 / 12 always fits in i32 for any realistic vesting schedule.
    let year = date.year() + (total_months / 12) as i32;
    let month = (total_months % 12) + 1;
    // Find the last valid day of the target month by computing the first day
    // of the *next* month and subtracting one day.
    let next_month_start = if month == 12 {
        // SAFETY: Jan 1 of year+1 is always valid when year is valid.
        NaiveDate::from_ymd_opt(year + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1)
    };
    // If the date is somehow out of range, fall back to the original date
    // (defensive — this should never happen for realistic vesting schedules).
    let Some(nms) = next_month_start else {
        return date;
    };
    let Some(prev) = nms.pred_opt() else {
        return date;
    };
    let max_day = prev.day();
    let day = date.day().min(max_day);
    NaiveDate::from_ymd_opt(year, month, day).unwrap_or(date)
}

/// Materialize all vesting events for a schedule.
///
/// - Immediate vest if total_months == 0.
/// - Cliff event at start + cliff_months: floor(total * cliff_months / total_months) shares.
/// - Monthly events for remaining months, with leftover on the last event.
pub fn materialize_vesting_events(schedule: &VestingSchedule) -> Vec<VestingEvent> {
    let total = schedule.total_shares().raw();
    let total_months = schedule.total_months();

    // Immediate vest
    if total_months == 0 {
        return vec![VestingEvent::new(
            VestingEventId::new(),
            schedule.schedule_id(),
            schedule.grant_id(),
            schedule.entity_id(),
            schedule.vesting_start_date(),
            schedule.total_shares(),
            VestingEventType::Cliff,
        )];
    }

    let mut events = Vec::new();
    let cliff_months = schedule.cliff_months();

    // Cliff event — division is safe because total_months > 0 (guarded above).
    // Cast is safe because u32 always fits in i64.
    let cliff_shares = total * i64::from(cliff_months) / i64::from(total_months);
    if cliff_months > 0 {
        let cliff_date = add_months(schedule.vesting_start_date(), cliff_months);
        events.push(VestingEvent::new(
            VestingEventId::new(),
            schedule.schedule_id(),
            schedule.grant_id(),
            schedule.entity_id(),
            cliff_date,
            ShareCount::new(cliff_shares),
            VestingEventType::Cliff,
        ));
    }

    // Monthly events after the cliff
    let remaining_shares = total - cliff_shares;
    let remaining_months = total_months - cliff_months;
    if remaining_months > 0 && remaining_shares > 0 {
        let per_month = remaining_shares / i64::from(remaining_months);
        let mut allocated = 0i64;

        for i in 1..=remaining_months {
            let month_offset = cliff_months + i;
            let vest_date = add_months(schedule.vesting_start_date(), month_offset);
            let shares = if i == remaining_months {
                remaining_shares - allocated
            } else {
                per_month
            };
            allocated += shares;

            events.push(VestingEvent::new(
                VestingEventId::new(),
                schedule.schedule_id(),
                schedule.grant_id(),
                schedule.entity_id(),
                vest_date,
                ShareCount::new(shares),
                VestingEventType::Monthly,
            ));
        }
    }

    events
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_schedule(total: i64, cliff: u32, months: u32) -> VestingSchedule {
        VestingSchedule::new(
            VestingScheduleId::new(),
            EquityGrantId::new(),
            EntityId::new(),
            ShareCount::new(total),
            NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            "standard".to_string(),
            cliff,
            months,
            false,
            false,
            false,
        )
    }

    #[test]
    fn standard_4yr_1yr_cliff() {
        let schedule = make_schedule(48_000, 12, 48);
        let events = materialize_vesting_events(&schedule);

        // 1 cliff + 36 monthly = 37 events
        assert_eq!(events.len(), 37);

        // Cliff: 48000 * 12 / 48 = 12000
        assert_eq!(events[0].share_count().raw(), 12_000);
        assert_eq!(events[0].event_type(), VestingEventType::Cliff);
        assert_eq!(
            events[0].vest_date(),
            NaiveDate::from_ymd_opt(2027, 1, 1).unwrap()
        );

        // Monthly: remaining 36000 / 36 = 1000 each
        for event in &events[1..] {
            assert_eq!(event.event_type(), VestingEventType::Monthly);
            assert_eq!(event.share_count().raw(), 1000);
        }

        // Total shares
        let total: i64 = events.iter().map(|e| e.share_count().raw()).sum();
        assert_eq!(total, 48_000);
    }

    #[test]
    fn immediate_vest() {
        let schedule = make_schedule(10_000, 0, 0);
        let events = materialize_vesting_events(&schedule);

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].share_count().raw(), 10_000);
        assert_eq!(events[0].event_type(), VestingEventType::Cliff);
        assert_eq!(
            events[0].vest_date(),
            NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()
        );
    }

    #[test]
    fn no_cliff_monthly_only() {
        let schedule = make_schedule(12_000, 0, 12);
        let events = materialize_vesting_events(&schedule);

        // 12 monthly events, no cliff event
        assert_eq!(events.len(), 12);
        for event in &events {
            assert_eq!(event.event_type(), VestingEventType::Monthly);
            assert_eq!(event.share_count().raw(), 1000);
        }

        let total: i64 = events.iter().map(|e| e.share_count().raw()).sum();
        assert_eq!(total, 12_000);
    }

    #[test]
    fn uneven_shares_remainder_on_last() {
        // 10 shares over 3 months: 3, 3, 4
        let schedule = make_schedule(10, 0, 3);
        let events = materialize_vesting_events(&schedule);

        assert_eq!(events.len(), 3);
        assert_eq!(events[0].share_count().raw(), 3);
        assert_eq!(events[1].share_count().raw(), 3);
        assert_eq!(events[2].share_count().raw(), 4); // remainder

        let total: i64 = events.iter().map(|e| e.share_count().raw()).sum();
        assert_eq!(total, 10);
    }

    #[test]
    fn terminate_schedule() {
        let mut schedule = make_schedule(48_000, 12, 48);
        let date = NaiveDate::from_ymd_opt(2027, 6, 15).unwrap();
        schedule.terminate(date);
        assert_eq!(schedule.status(), VestingStatus::Terminated);
        assert_eq!(schedule.terminated_at(), Some(date));
    }

    #[test]
    fn vest_and_forfeit_events() {
        let schedule = make_schedule(1000, 0, 0);
        let mut events = materialize_vesting_events(&schedule);
        assert_eq!(events[0].status(), VestingEventStatus::Scheduled);

        events[0].vest().unwrap();
        assert_eq!(events[0].status(), VestingEventStatus::Vested);

        // Create another to test forfeit
        let schedule2 = make_schedule(1000, 0, 2);
        let mut events2 = materialize_vesting_events(&schedule2);
        events2[0].vest().unwrap();
        events2[1].forfeit().unwrap();
        assert_eq!(events2[0].status(), VestingEventStatus::Vested);
        assert_eq!(events2[1].status(), VestingEventStatus::Forfeited);
    }

    #[test]
    fn cancel_event() {
        let schedule = make_schedule(1000, 0, 1);
        let mut events = materialize_vesting_events(&schedule);
        events[0].cancel().unwrap();
        assert_eq!(events[0].status(), VestingEventStatus::Cancelled);
    }

    #[test]
    fn vest_rejects_non_scheduled() {
        let schedule = make_schedule(1000, 0, 2);
        let mut events = materialize_vesting_events(&schedule);
        events[0].vest().unwrap();
        // Attempting to vest again from Vested should fail
        assert!(events[0].vest().is_err());
    }

    #[test]
    fn forfeit_rejects_non_scheduled() {
        let schedule = make_schedule(1000, 0, 2);
        let mut events = materialize_vesting_events(&schedule);
        events[0].cancel().unwrap();
        // Attempting to forfeit from Cancelled should fail
        assert!(events[0].forfeit().is_err());
    }

    #[test]
    fn cancel_rejects_non_scheduled() {
        let schedule = make_schedule(1000, 0, 2);
        let mut events = materialize_vesting_events(&schedule);
        events[0].vest().unwrap();
        // Attempting to cancel from Vested should fail
        assert!(events[0].cancel().is_err());
    }
}
