use chrono::{DateTime, Duration, LocalResult, NaiveDateTime, NaiveTime, TimeZone, Utc};
use chrono_tz::Tz;

use crate::{
    domain::{
        memo::{
            DueMemoReminder, MemoReminderFrequency, MemoReminderInput, MemoReminderRule,
            MemoReminderStatus,
        },
        recurrence::{next_scheduled_date, RecurrencePattern, RecurrenceRule, RecurrenceStatus},
    },
    repositories::{database::Database, memo_repository::MemoRepository},
    DomainError,
};

pub struct MemoReminderService<'a> {
    database: &'a Database,
}

impl<'a> MemoReminderService<'a> {
    pub fn new(database: &'a Database) -> Self {
        Self { database }
    }

    pub fn prepare_rule(
        memo_id: &str,
        current: Option<&MemoReminderRule>,
        schedule: Option<&MemoReminderInput>,
        now: DateTime<Utc>,
    ) -> Result<Option<MemoReminderRule>, DomainError> {
        let Some(schedule) = schedule else {
            return Ok(None);
        };
        let next_scheduled_for =
            Self::next_occurrence(schedule, now)?.map(|value| value.to_rfc3339());
        let status = if next_scheduled_for.is_some() {
            MemoReminderStatus::Active
        } else {
            MemoReminderStatus::Completed
        };
        let timestamp = now.to_rfc3339();
        Ok(Some(MemoReminderRule {
            id: current
                .map(|reminder| reminder.id.clone())
                .unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
            memo_id: memo_id.into(),
            schedule: schedule.clone(),
            next_scheduled_for,
            status,
            created_at: current
                .map(|reminder| reminder.created_at.clone())
                .unwrap_or_else(|| timestamp.clone()),
            updated_at: timestamp,
        }))
    }

    pub fn reconcile_due<F>(
        &self,
        now: DateTime<Utc>,
        untitled_label: &str,
        mut deliver: F,
    ) -> Result<usize, DomainError>
    where
        F: FnMut(&DueMemoReminder) -> Result<(), DomainError>,
    {
        let repository = MemoRepository::new(self.database);
        let reminders = repository.list_due_reminders(now, untitled_label)?;
        let mut delivered = 0;
        let mut first_error = None;
        for occurrence in reminders {
            match deliver(&occurrence) {
                Ok(()) => match self.advance_after_delivery(&occurrence, now) {
                    Ok(_) => delivered += 1,
                    Err(error) => {
                        first_error.get_or_insert(error);
                    }
                },
                Err(error) => {
                    first_error.get_or_insert(error);
                }
            }
        }
        first_error.map_or(Ok(delivered), Err)
    }

    pub fn advance_after_delivery(
        &self,
        occurrence: &DueMemoReminder,
        updated_at: DateTime<Utc>,
    ) -> Result<bool, DomainError> {
        let scheduled_for = occurrence
            .reminder
            .next_scheduled_for
            .as_deref()
            .ok_or_else(stored_occurrence_error)?;
        let current = DateTime::parse_from_rfc3339(scheduled_for)
            .map_err(|_| stored_occurrence_error())?
            .with_timezone(&Utc);
        let (next, status) = match &occurrence.reminder.schedule {
            MemoReminderInput::Once { .. } => (None, MemoReminderStatus::Completed),
            MemoReminderInput::Recurring { .. } => {
                let next = Self::next_occurrence(&occurrence.reminder.schedule, current)?
                    .map(|value| value.to_rfc3339());
                let status = if next.is_some() {
                    MemoReminderStatus::Active
                } else {
                    MemoReminderStatus::Completed
                };
                (next, status)
            }
        };
        MemoRepository::new(self.database).advance_reminder(
            &occurrence.reminder.id,
            scheduled_for,
            next.as_deref(),
            status,
            updated_at,
        )
    }

    pub fn next_occurrence(
        schedule: &MemoReminderInput,
        after: DateTime<Utc>,
    ) -> Result<Option<DateTime<Utc>>, DomainError> {
        schedule.validate_at(after)?;
        match schedule {
            MemoReminderInput::Once {
                scheduled_local,
                timezone,
            } => {
                let timezone = parse_timezone(timezone)?;
                let local = parse_local_datetime(scheduled_local)?;
                Ok(Some(resolve_once(timezone, local)?))
            }
            MemoReminderInput::Recurring {
                frequency,
                interval,
                weekdays,
                monthly_day,
                local_time,
                starts_on,
                ends_on,
                timezone,
            } => {
                let timezone = parse_timezone(timezone)?;
                let local_time = parse_time(local_time)?;
                let pattern = match frequency {
                    MemoReminderFrequency::Daily => RecurrencePattern::Daily {
                        interval: *interval,
                    },
                    MemoReminderFrequency::Weekdays => RecurrencePattern::Weekdays,
                    MemoReminderFrequency::Weekly => RecurrencePattern::Weekly {
                        interval: *interval,
                        weekdays: weekdays.clone(),
                    },
                    MemoReminderFrequency::Monthly => RecurrencePattern::Monthly {
                        interval: *interval,
                        day_of_month: monthly_day.expect("validated monthly reminder has a day"),
                    },
                };
                let rule = RecurrenceRule {
                    id: "memo-reminder-calculation".into(),
                    task_template_id: "memo-reminder-calculation".into(),
                    pattern,
                    local_time: Some(local_time.format("%H:%M").to_string()),
                    timezone: timezone.name().into(),
                    starts_on: starts_on.clone(),
                    ends_on: ends_on.clone(),
                    status: RecurrenceStatus::Active,
                    version: 1,
                };
                let mut date = after.with_timezone(&timezone).date_naive();

                loop {
                    let Some(next_date) = next_scheduled_date(&rule, date).map_err(date_error)?
                    else {
                        return Ok(None);
                    };
                    let occurrence = resolve_recurring(timezone, next_date.and_time(local_time))?;
                    if occurrence > after {
                        return Ok(Some(occurrence));
                    }
                    date = next_date.succ_opt().ok_or_else(supported_range_error)?;
                }
            }
        }
    }
}

fn stored_occurrence_error() -> DomainError {
    DomainError {
        code: "MEMO_REMINDER_DATA_INVALID".into(),
        message: "stored memo reminder occurrence is invalid".into(),
        field: None,
    }
}

fn resolve_once(timezone: Tz, local: NaiveDateTime) -> Result<DateTime<Utc>, DomainError> {
    match timezone.from_local_datetime(&local) {
        LocalResult::Single(value) => Ok(value.with_timezone(&Utc)),
        LocalResult::Ambiguous(first, second) => Ok(first.min(second).with_timezone(&Utc)),
        LocalResult::None => Err(time_error(
            "memo reminder local time does not exist in the selected timezone",
            "scheduledLocal",
        )),
    }
}

fn resolve_recurring(timezone: Tz, local: NaiveDateTime) -> Result<DateTime<Utc>, DomainError> {
    for offset in 0..=180 {
        if let Some(value) = timezone
            .from_local_datetime(&(local + Duration::minutes(offset)))
            .earliest()
        {
            return Ok(value.with_timezone(&Utc));
        }
    }
    Err(time_error(
        "memo reminder local time could not be resolved in the selected timezone",
        "localTime",
    ))
}

fn parse_timezone(value: &str) -> Result<Tz, DomainError> {
    value.parse::<Tz>().map_err(|_| DomainError {
        code: "MEMO_REMINDER_TIMEZONE_INVALID".into(),
        message: "memo reminder timezone must be a valid IANA timezone".into(),
        field: Some("timezone".into()),
    })
}

fn parse_local_datetime(value: &str) -> Result<NaiveDateTime, DomainError> {
    ["%Y-%m-%dT%H:%M", "%Y-%m-%dT%H:%M:%S"]
        .into_iter()
        .find_map(|format| NaiveDateTime::parse_from_str(value, format).ok())
        .ok_or_else(|| {
            time_error(
                "memo reminder time must use YYYY-MM-DDTHH:MM format",
                "scheduledLocal",
            )
        })
}

fn parse_time(value: &str) -> Result<NaiveTime, DomainError> {
    NaiveTime::parse_from_str(value, "%H:%M").map_err(|_| {
        time_error(
            "memo reminder local time must use HH:MM format",
            "localTime",
        )
    })
}

fn date_error(error: DomainError) -> DomainError {
    DomainError {
        code: "MEMO_REMINDER_DATE_INVALID".into(),
        message: error.message,
        field: error.field,
    }
}

fn supported_range_error() -> DomainError {
    DomainError {
        code: "MEMO_REMINDER_DATE_INVALID".into(),
        message: "memo reminder date exceeds supported values".into(),
        field: Some("startsOn".into()),
    }
}

fn time_error(message: &str, field: &str) -> DomainError {
    DomainError {
        code: "MEMO_REMINDER_TIME_INVALID".into(),
        message: message.into(),
        field: Some(field.into()),
    }
}

#[cfg(test)]
mod tests {
    use chrono::{Datelike, Duration, NaiveDate, TimeZone, Timelike};
    use chrono_tz::Tz;
    use proptest::prelude::*;

    use super::*;
    use crate::{
        domain::memo::{MemoInput, MemoRecord},
        services::memo_service::MemoService,
    };

    #[allow(clippy::too_many_arguments)]
    fn recurring(
        frequency: MemoReminderFrequency,
        interval: u32,
        weekdays: Vec<u8>,
        monthly_day: Option<u8>,
        local_time: &str,
        starts_on: &str,
        ends_on: Option<&str>,
        timezone: &str,
    ) -> MemoReminderInput {
        MemoReminderInput::Recurring {
            frequency,
            interval,
            weekdays,
            monthly_day,
            local_time: local_time.into(),
            starts_on: starts_on.into(),
            ends_on: ends_on.map(str::to_owned),
            timezone: timezone.into(),
        }
    }

    fn persist_with_reminder(
        database: &Database,
        id: &str,
        title: &str,
        body: &str,
        schedule: MemoReminderInput,
        now: DateTime<Utc>,
    ) -> MemoRecord {
        let input = MemoInput {
            title: title.into(),
            body: body.into(),
            tags: Vec::new(),
            pinned: false,
            reminder: Some(schedule),
        };
        let core = MemoService::create(id.into(), &input, now).unwrap();
        let reminder =
            MemoReminderService::prepare_rule(id, None, input.reminder.as_ref(), now).unwrap();
        MemoRepository::new(database)
            .create(&core, &[], reminder.as_ref(), "Untitled memo")
            .unwrap()
    }

    fn valid_recurring_schedule_strategy() -> impl Strategy<Value = MemoReminderInput> {
        (
            0_u8..4,
            1_u32..8,
            1_u8..128,
            1_u8..32,
            2024_i32..2032,
            1_u32..13,
            1_u32..29,
            6_u32..23,
            0_u32..60,
            prop::sample::select(vec![
                "UTC",
                "Asia/Shanghai",
                "America/New_York",
                "Europe/London",
                "Australia/Sydney",
            ]),
        )
            .prop_map(
                |(
                    frequency,
                    interval,
                    weekday_mask,
                    monthly_day,
                    year,
                    month,
                    day,
                    hour,
                    minute,
                    timezone,
                )| {
                    let (frequency, interval, weekdays, monthly_day) = match frequency {
                        0 => (MemoReminderFrequency::Daily, interval, vec![], None),
                        1 => (MemoReminderFrequency::Weekdays, 1, vec![], None),
                        2 => (
                            MemoReminderFrequency::Weekly,
                            interval,
                            (1_u8..=7)
                                .filter(|weekday| weekday_mask & (1 << (weekday - 1)) != 0)
                                .collect(),
                            None,
                        ),
                        _ => (
                            MemoReminderFrequency::Monthly,
                            interval,
                            vec![],
                            Some(monthly_day),
                        ),
                    };
                    recurring(
                        frequency,
                        interval,
                        weekdays,
                        monthly_day,
                        &format!("{hour:02}:{minute:02}"),
                        &format!("{year:04}-{month:02}-{day:02}"),
                        None,
                        timezone,
                    )
                },
            )
    }

    fn occurrence_matches_recurring_schedule(
        schedule: &MemoReminderInput,
        occurrence: DateTime<Utc>,
    ) -> bool {
        let MemoReminderInput::Recurring {
            frequency,
            interval,
            weekdays,
            monthly_day,
            local_time,
            starts_on,
            ends_on,
            timezone,
        } = schedule
        else {
            return false;
        };
        let Ok(timezone) = timezone.parse::<Tz>() else {
            return false;
        };
        let Ok(starts_on) = NaiveDate::parse_from_str(starts_on, "%Y-%m-%d") else {
            return false;
        };
        let local = occurrence.with_timezone(&timezone);
        let date = local.date_naive();
        if date < starts_on
            || local.format("%H:%M").to_string() != *local_time
            || local.second() != 0
            || ends_on.as_deref().is_some_and(|ends_on| {
                NaiveDate::parse_from_str(ends_on, "%Y-%m-%d").is_ok_and(|ends_on| date > ends_on)
            })
        {
            return false;
        }

        match frequency {
            MemoReminderFrequency::Daily => {
                (date - starts_on).num_days() % i64::from(*interval) == 0
            }
            MemoReminderFrequency::Weekdays => date.weekday().number_from_monday() <= 5,
            MemoReminderFrequency::Weekly => {
                let start_week = starts_on
                    - Duration::days(i64::from(starts_on.weekday().num_days_from_monday()));
                let occurrence_week =
                    date - Duration::days(i64::from(date.weekday().num_days_from_monday()));
                let week_offset = (occurrence_week - start_week).num_days() / 7;
                week_offset % i64::from(*interval) == 0
                    && weekdays.contains(&(date.weekday().number_from_monday() as u8))
            }
            MemoReminderFrequency::Monthly => {
                let month_offset = (date.year() - starts_on.year()) * 12 + date.month() as i32
                    - starts_on.month() as i32;
                let next_month = if date.month() == 12 {
                    NaiveDate::from_ymd_opt(date.year() + 1, 1, 1)
                } else {
                    NaiveDate::from_ymd_opt(date.year(), date.month() + 1, 1)
                };
                let last_day = next_month
                    .and_then(|date| date.pred_opt())
                    .map(|date| date.day());
                month_offset >= 0
                    && month_offset % *interval as i32 == 0
                    && Some(date.day())
                        == monthly_day.map(|day| u32::from(day).min(last_day.unwrap_or(31)))
            }
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(64))]

        #[test]
        fn property_m5_recurring_reminders_advance_monotonically(
            schedule in valid_recurring_schedule_strategy(),
        ) {
            let database = Database::open_in_memory().unwrap();
            let created_at = Utc.with_ymd_and_hms(2023, 1, 1, 0, 0, 0).unwrap();
            let current_occurrence = MemoReminderService::next_occurrence(&schedule, created_at)
                .unwrap()
                .unwrap();
            persist_with_reminder(
                &database,
                "property-m5-memo",
                "Property M5",
                "Body",
                schedule.clone(),
                created_at,
            );
            let occurrence = MemoRepository::new(&database)
                .list_due_reminders(current_occurrence, "Untitled memo")
                .unwrap()
                .remove(0);

            prop_assert!(MemoReminderService::new(&database)
                .advance_after_delivery(&occurrence, current_occurrence)
                .unwrap());

            let stored = MemoRepository::new(&database)
                .get("property-m5-memo", "Untitled memo")
                .unwrap()
                .unwrap()
                .reminder
                .unwrap();
            let next_occurrence = DateTime::parse_from_rfc3339(
                stored.next_scheduled_for.as_deref().unwrap(),
            )
            .unwrap()
            .with_timezone(&Utc);

            prop_assert!(next_occurrence > current_occurrence);
            prop_assert!(occurrence_matches_recurring_schedule(
                &schedule,
                next_occurrence,
            ));
        }
    }

    #[test]
    fn resolves_a_future_one_time_reminder_to_utc() {
        let schedule = MemoReminderInput::Once {
            scheduled_local: "2026-07-25T10:00".into(),
            timezone: "Asia/Shanghai".into(),
        };

        let next = MemoReminderService::next_occurrence(
            &schedule,
            Utc.with_ymd_and_hms(2026, 7, 25, 1, 0, 0).unwrap(),
        )
        .unwrap();

        assert_eq!(
            next,
            Some(Utc.with_ymd_and_hms(2026, 7, 25, 2, 0, 0).unwrap())
        );
    }

    #[test]
    fn advances_daily_and_weekday_rules_strictly_after_the_reference_time() {
        let daily = recurring(
            MemoReminderFrequency::Daily,
            2,
            vec![],
            None,
            "09:00",
            "2026-07-23",
            None,
            "Asia/Shanghai",
        );
        let weekday = recurring(
            MemoReminderFrequency::Weekdays,
            1,
            vec![],
            None,
            "09:00",
            "2026-07-20",
            None,
            "Asia/Shanghai",
        );

        assert_eq!(
            MemoReminderService::next_occurrence(
                &daily,
                Utc.with_ymd_and_hms(2026, 7, 23, 1, 0, 0).unwrap(),
            )
            .unwrap(),
            Some(Utc.with_ymd_and_hms(2026, 7, 25, 1, 0, 0).unwrap())
        );
        assert_eq!(
            MemoReminderService::next_occurrence(
                &weekday,
                Utc.with_ymd_and_hms(2026, 7, 24, 1, 0, 0).unwrap(),
            )
            .unwrap(),
            Some(Utc.with_ymd_and_hms(2026, 7, 27, 1, 0, 0).unwrap())
        );
    }

    #[test]
    fn selects_the_earliest_weekday_in_each_eligible_week() {
        let schedule = recurring(
            MemoReminderFrequency::Weekly,
            2,
            vec![3, 1],
            None,
            "09:00",
            "2026-07-20",
            None,
            "Asia/Shanghai",
        );

        assert_eq!(
            MemoReminderService::next_occurrence(
                &schedule,
                Utc.with_ymd_and_hms(2026, 7, 20, 1, 0, 0).unwrap(),
            )
            .unwrap(),
            Some(Utc.with_ymd_and_hms(2026, 7, 22, 1, 0, 0).unwrap())
        );
    }

    #[test]
    fn applies_custom_intervals_across_daily_weekly_and_monthly_rules() {
        let schedules = [
            (
                recurring(
                    MemoReminderFrequency::Daily,
                    3,
                    vec![],
                    None,
                    "09:00",
                    "2026-07-23",
                    None,
                    "Asia/Shanghai",
                ),
                Utc.with_ymd_and_hms(2026, 7, 23, 1, 0, 0).unwrap(),
                Utc.with_ymd_and_hms(2026, 7, 26, 1, 0, 0).unwrap(),
            ),
            (
                recurring(
                    MemoReminderFrequency::Weekly,
                    2,
                    vec![1, 3],
                    None,
                    "09:00",
                    "2026-07-20",
                    None,
                    "Asia/Shanghai",
                ),
                Utc.with_ymd_and_hms(2026, 7, 22, 1, 0, 0).unwrap(),
                Utc.with_ymd_and_hms(2026, 8, 3, 1, 0, 0).unwrap(),
            ),
            (
                recurring(
                    MemoReminderFrequency::Monthly,
                    2,
                    vec![],
                    Some(31),
                    "09:00",
                    "2026-01-31",
                    None,
                    "UTC",
                ),
                Utc.with_ymd_and_hms(2026, 1, 31, 9, 0, 0).unwrap(),
                Utc.with_ymd_and_hms(2026, 3, 31, 9, 0, 0).unwrap(),
            ),
        ];

        for (schedule, after, expected) in schedules {
            assert_eq!(
                MemoReminderService::next_occurrence(&schedule, after).unwrap(),
                Some(expected)
            );
        }
    }

    #[test]
    fn clamps_monthly_reminders_to_the_last_day_of_the_month() {
        let schedule = recurring(
            MemoReminderFrequency::Monthly,
            1,
            vec![],
            Some(31),
            "09:00",
            "2026-01-31",
            None,
            "UTC",
        );

        assert_eq!(
            MemoReminderService::next_occurrence(
                &schedule,
                Utc.with_ymd_and_hms(2026, 1, 31, 9, 0, 0).unwrap(),
            )
            .unwrap(),
            Some(Utc.with_ymd_and_hms(2026, 2, 28, 9, 0, 0).unwrap())
        );

        let leap_year = recurring(
            MemoReminderFrequency::Monthly,
            1,
            vec![],
            Some(31),
            "09:00",
            "2028-01-31",
            None,
            "UTC",
        );
        assert_eq!(
            MemoReminderService::next_occurrence(
                &leap_year,
                Utc.with_ymd_and_hms(2028, 1, 31, 9, 0, 0).unwrap(),
            )
            .unwrap(),
            Some(Utc.with_ymd_and_hms(2028, 2, 29, 9, 0, 0).unwrap())
        );
    }

    #[test]
    fn returns_none_after_a_recurring_rule_end_date() {
        let schedule = recurring(
            MemoReminderFrequency::Daily,
            1,
            vec![],
            None,
            "09:00",
            "2026-07-20",
            Some("2026-07-24"),
            "UTC",
        );

        assert_eq!(
            MemoReminderService::next_occurrence(
                &schedule,
                Utc.with_ymd_and_hms(2026, 7, 24, 9, 0, 0).unwrap(),
            )
            .unwrap(),
            None
        );
    }

    #[test]
    fn normalizes_nonexistent_dst_times_and_uses_the_earlier_ambiguous_time() {
        let spring = recurring(
            MemoReminderFrequency::Daily,
            1,
            vec![],
            None,
            "02:30",
            "2026-03-29",
            None,
            "Europe/Berlin",
        );
        let autumn = recurring(
            MemoReminderFrequency::Daily,
            1,
            vec![],
            None,
            "02:30",
            "2026-10-25",
            None,
            "Europe/Berlin",
        );

        assert_eq!(
            MemoReminderService::next_occurrence(
                &spring,
                Utc.with_ymd_and_hms(2026, 3, 28, 12, 0, 0).unwrap(),
            )
            .unwrap(),
            Some(Utc.with_ymd_and_hms(2026, 3, 29, 1, 0, 0).unwrap())
        );
        assert_eq!(
            MemoReminderService::next_occurrence(
                &autumn,
                Utc.with_ymd_and_hms(2026, 10, 24, 12, 0, 0).unwrap(),
            )
            .unwrap(),
            Some(Utc.with_ymd_and_hms(2026, 10, 25, 0, 30, 0).unwrap())
        );
    }

    #[test]
    fn rejects_custom_intervals_for_weekday_rules() {
        let schedule = recurring(
            MemoReminderFrequency::Weekdays,
            2,
            vec![],
            None,
            "09:00",
            "2026-07-20",
            None,
            "UTC",
        );

        assert_eq!(
            MemoReminderService::next_occurrence(
                &schedule,
                Utc.with_ymd_and_hms(2026, 7, 20, 0, 0, 0).unwrap(),
            )
            .unwrap_err()
            .code,
            "MEMO_REMINDER_INTERVAL_INVALID"
        );
    }

    #[test]
    fn scans_due_reminders_and_advances_once_and_recurring_states() {
        let database = Database::open_in_memory().unwrap();
        let created_at = Utc.with_ymd_and_hms(2026, 7, 23, 10, 0, 0).unwrap();
        let due_at = Utc.with_ymd_and_hms(2026, 7, 23, 10, 5, 0).unwrap();
        persist_with_reminder(
            &database,
            "once-memo",
            "One-time title",
            "Body",
            MemoReminderInput::Once {
                scheduled_local: "2026-07-23T10:05".into(),
                timezone: "UTC".into(),
            },
            created_at,
        );
        persist_with_reminder(
            &database,
            "recurring-memo",
            "",
            "  Recurring body title\nMore",
            recurring(
                MemoReminderFrequency::Daily,
                1,
                vec![],
                None,
                "10:05",
                "2026-07-23",
                None,
                "UTC",
            ),
            created_at,
        );

        let mut delivered = Vec::new();
        let count = MemoReminderService::new(&database)
            .reconcile_due(due_at, "Untitled memo", |occurrence| {
                delivered.push((
                    occurrence.reminder.memo_id.clone(),
                    occurrence.display_title.clone(),
                ));
                Ok(())
            })
            .unwrap();

        assert_eq!(count, 2);
        delivered.sort();
        assert_eq!(
            delivered,
            vec![
                ("once-memo".into(), "One-time title".into()),
                ("recurring-memo".into(), "Recurring body title".into()),
            ]
        );
        let once = MemoRepository::new(&database)
            .get("once-memo", "Untitled memo")
            .unwrap()
            .unwrap()
            .reminder
            .unwrap();
        assert_eq!(once.status, MemoReminderStatus::Completed);
        assert_eq!(once.next_scheduled_for, None);
        let recurring = MemoRepository::new(&database)
            .get("recurring-memo", "Untitled memo")
            .unwrap()
            .unwrap()
            .reminder
            .unwrap();
        assert_eq!(recurring.status, MemoReminderStatus::Active);
        assert_eq!(
            recurring.next_scheduled_for.as_deref(),
            Some("2026-07-24T10:05:00+00:00")
        );
    }

    #[test]
    fn keeps_failed_deliveries_due_and_continues_other_reminders() {
        let database = Database::open_in_memory().unwrap();
        let created_at = Utc.with_ymd_and_hms(2026, 7, 23, 10, 0, 0).unwrap();
        let due_at = Utc.with_ymd_and_hms(2026, 7, 23, 10, 5, 0).unwrap();
        for id in ["failed-memo", "successful-memo"] {
            persist_with_reminder(
                &database,
                id,
                id,
                "Body",
                MemoReminderInput::Once {
                    scheduled_local: "2026-07-23T10:05".into(),
                    timezone: "UTC".into(),
                },
                created_at,
            );
        }

        let error = MemoReminderService::new(&database)
            .reconcile_due(due_at, "Untitled memo", |occurrence| {
                if occurrence.reminder.memo_id == "failed-memo" {
                    Err(DomainError {
                        code: "TEST_DELIVERY_FAILED".into(),
                        message: "injected delivery failure".into(),
                        field: None,
                    })
                } else {
                    Ok(())
                }
            })
            .unwrap_err();

        assert_eq!(error.code, "TEST_DELIVERY_FAILED");
        let due = MemoRepository::new(&database)
            .list_due_reminders(due_at, "Untitled memo")
            .unwrap();
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].reminder.memo_id, "failed-memo");
        let successful = MemoRepository::new(&database)
            .get("successful-memo", "Untitled memo")
            .unwrap()
            .unwrap()
            .reminder
            .unwrap();
        assert_eq!(successful.status, MemoReminderStatus::Completed);
    }

    #[test]
    fn conditional_advancement_ignores_a_stale_occurrence() {
        let database = Database::open_in_memory().unwrap();
        let created_at = Utc.with_ymd_and_hms(2026, 7, 23, 10, 0, 0).unwrap();
        let due_at = Utc.with_ymd_and_hms(2026, 7, 23, 10, 5, 0).unwrap();
        persist_with_reminder(
            &database,
            "memo",
            "Title",
            "Body",
            MemoReminderInput::Once {
                scheduled_local: "2026-07-23T10:05".into(),
                timezone: "UTC".into(),
            },
            created_at,
        );
        let occurrence = MemoRepository::new(&database)
            .list_due_reminders(due_at, "Untitled memo")
            .unwrap()
            .remove(0);
        let service = MemoReminderService::new(&database);

        assert!(service.advance_after_delivery(&occurrence, due_at).unwrap());
        assert!(!service.advance_after_delivery(&occurrence, due_at).unwrap());
    }
}
