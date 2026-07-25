use chrono::{Datelike, Duration, NaiveDate, NaiveTime};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};

use crate::DomainError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum RecurrencePattern {
    Daily {
        interval: u32,
    },
    Weekdays,
    Weekly {
        interval: u32,
        weekdays: Vec<u8>,
    },
    Monthly {
        interval: u32,
        #[serde(rename = "dayOfMonth")]
        day_of_month: u8,
    },
}

impl RecurrencePattern {
    pub fn validate(&self) -> Result<(), DomainError> {
        match self {
            Self::Daily { interval } => validate_interval(*interval),
            Self::Weekdays => Ok(()),
            Self::Weekly { interval, weekdays } => {
                validate_interval(*interval)?;
                if weekdays.is_empty() {
                    return Err(recurrence_error(
                        "weekly pattern requires at least one weekday",
                        "pattern.weekdays",
                    ));
                }
                let mut seen = [false; 8];
                for weekday in weekdays {
                    if !(1..=7).contains(weekday) {
                        return Err(recurrence_error(
                            "weekday must use ISO values from 1 to 7",
                            "pattern.weekdays",
                        ));
                    }
                    if seen[*weekday as usize] {
                        return Err(recurrence_error(
                            "weekly pattern cannot contain duplicate weekdays",
                            "pattern.weekdays",
                        ));
                    }
                    seen[*weekday as usize] = true;
                }
                Ok(())
            }
            Self::Monthly {
                interval,
                day_of_month,
            } => {
                validate_interval(*interval)?;
                if !(1..=31).contains(day_of_month) {
                    return Err(recurrence_error(
                        "monthly day must be between 1 and 31",
                        "pattern.dayOfMonth",
                    ));
                }
                Ok(())
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RecurrenceStatus {
    Active,
    Paused,
    Ended,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TaskInstanceStatus {
    Pending,
    Completed,
    Skipped,
    Rescheduled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "scope",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum RecurrenceChangeScope {
    ThisInstance { instance_id: String },
    Future { effective_on: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecurrenceRule {
    pub id: String,
    pub task_template_id: String,
    pub pattern: RecurrencePattern,
    pub local_time: Option<String>,
    pub timezone: String,
    pub starts_on: String,
    pub ends_on: Option<String>,
    pub status: RecurrenceStatus,
    pub version: u32,
}

impl RecurrenceRule {
    pub fn validate(&self) -> Result<(), DomainError> {
        if self.id.trim().is_empty() {
            return Err(recurrence_error("rule id is required", "id"));
        }
        if self.task_template_id.trim().is_empty() {
            return Err(recurrence_error(
                "task template id is required",
                "taskTemplateId",
            ));
        }
        self.pattern.validate()?;
        if let Some(local_time) = &self.local_time {
            NaiveTime::parse_from_str(local_time, "%H:%M")
                .map_err(|_| recurrence_error("local time must use HH:MM", "localTime"))?;
        }
        self.timezone.parse::<Tz>().map_err(|_| {
            recurrence_error("timezone must be a valid IANA identifier", "timezone")
        })?;
        let starts_on = parse_date(&self.starts_on, "startsOn")?;
        if let Some(ends_on) = &self.ends_on {
            let ends_on = parse_date(ends_on, "endsOn")?;
            if ends_on < starts_on {
                return Err(recurrence_error(
                    "end date must be on or after start date",
                    "endsOn",
                ));
            }
        }
        if self.version == 0 {
            return Err(recurrence_error("rule version must be positive", "version"));
        }
        Ok(())
    }
}

pub fn scheduled_dates(
    rule: &RecurrenceRule,
    range_start: NaiveDate,
    range_end: NaiveDate,
) -> Result<Vec<NaiveDate>, DomainError> {
    rule.validate()?;
    if range_start > range_end {
        return Err(recurrence_error(
            "generation end date must be on or after start date",
            "rangeEnd",
        ));
    }
    let starts_on = parse_date(&rule.starts_on, "startsOn")?;
    let ends_on = rule
        .ends_on
        .as_deref()
        .map(|value| parse_date(value, "endsOn"))
        .transpose()?;
    let first = range_start.max(starts_on);
    let last = ends_on.map_or(range_end, |ends_on| range_end.min(ends_on));
    if first > last || rule.status != RecurrenceStatus::Active {
        return Ok(Vec::new());
    }

    let mut dates = Vec::new();
    let mut date = first;
    loop {
        if matches_pattern(&rule.pattern, starts_on, date) {
            dates.push(date);
        }
        if date == last {
            break;
        }
        date = date.succ_opt().ok_or_else(|| {
            recurrence_error("generation date range exceeds supported values", "rangeEnd")
        })?;
    }
    Ok(dates)
}

pub fn next_scheduled_date(
    rule: &RecurrenceRule,
    on_or_after: NaiveDate,
) -> Result<Option<NaiveDate>, DomainError> {
    rule.validate()?;
    if rule.status != RecurrenceStatus::Active {
        return Ok(None);
    }

    let starts_on = parse_date(&rule.starts_on, "startsOn")?;
    let ends_on = rule
        .ends_on
        .as_deref()
        .map(|value| parse_date(value, "endsOn"))
        .transpose()?;
    let candidate = on_or_after.max(starts_on);
    if ends_on.is_some_and(|end| candidate > end) {
        return Ok(None);
    }

    let next = match &rule.pattern {
        RecurrencePattern::Daily { interval } => {
            let elapsed = (candidate - starts_on).num_days();
            let interval = i64::from(*interval);
            let remainder = elapsed % interval;
            candidate.checked_add_signed(Duration::days(if remainder == 0 {
                0
            } else {
                interval - remainder
            }))
        }
        RecurrencePattern::Weekdays => next_weekday(candidate),
        RecurrencePattern::Weekly { interval, weekdays } => {
            next_weekly_date(starts_on, candidate, *interval, weekdays)
        }
        RecurrencePattern::Monthly {
            interval,
            day_of_month,
        } => next_monthly_date(starts_on, candidate, *interval, *day_of_month),
    }
    .ok_or_else(|| recurrence_error("next scheduled date exceeds supported values", "onOrAfter"))?;

    Ok((!ends_on.is_some_and(|end| next > end)).then_some(next))
}

fn next_weekday(mut candidate: NaiveDate) -> Option<NaiveDate> {
    while candidate.weekday().number_from_monday() > 5 {
        candidate = candidate.succ_opt()?;
    }
    Some(candidate)
}

fn next_weekly_date(
    starts_on: NaiveDate,
    candidate: NaiveDate,
    interval: u32,
    weekdays: &[u8],
) -> Option<NaiveDate> {
    let start_week = starts_on.checked_sub_signed(Duration::days(i64::from(
        starts_on.weekday().num_days_from_monday(),
    )))?;
    let candidate_week = candidate.checked_sub_signed(Duration::days(i64::from(
        candidate.weekday().num_days_from_monday(),
    )))?;
    let week_offset = (candidate_week - start_week).num_days() / 7;
    let interval = i64::from(interval);
    let remainder = week_offset % interval;
    let mut week_start = if remainder == 0 {
        candidate_week
    } else {
        candidate_week.checked_add_signed(Duration::weeks(interval - remainder))?
    };

    loop {
        for weekday in 1..=7 {
            if !weekdays.contains(&weekday) {
                continue;
            }
            let date = week_start.checked_add_signed(Duration::days(i64::from(weekday) - 1))?;
            if date >= candidate {
                return Some(date);
            }
        }
        week_start = week_start.checked_add_signed(Duration::weeks(interval))?;
    }
}

fn next_monthly_date(
    starts_on: NaiveDate,
    candidate: NaiveDate,
    interval: u32,
    day_of_month: u8,
) -> Option<NaiveDate> {
    let start_month = i64::from(starts_on.year()) * 12 + i64::from(starts_on.month0());
    let candidate_month = i64::from(candidate.year()) * 12 + i64::from(candidate.month0());
    let interval = i64::from(interval);
    let month_offset = candidate_month - start_month;
    let remainder = month_offset % interval;
    let mut target_month = candidate_month.checked_add(if remainder == 0 {
        0
    } else {
        interval - remainder
    })?;
    let mut target = date_in_month(target_month, day_of_month)?;
    if target < candidate {
        target_month = target_month.checked_add(interval)?;
        target = date_in_month(target_month, day_of_month)?;
    }
    Some(target)
}

fn date_in_month(month_index: i64, day_of_month: u8) -> Option<NaiveDate> {
    let year = i32::try_from(month_index.div_euclid(12)).ok()?;
    let month = u32::try_from(month_index.rem_euclid(12) + 1).ok()?;
    let first = NaiveDate::from_ymd_opt(year, month, 1)?;
    NaiveDate::from_ymd_opt(
        year,
        month,
        u32::from(day_of_month).min(last_day_of_month(first)),
    )
}

fn matches_pattern(pattern: &RecurrencePattern, starts_on: NaiveDate, date: NaiveDate) -> bool {
    match pattern {
        RecurrencePattern::Daily { interval } => {
            (date - starts_on).num_days() % i64::from(*interval) == 0
        }
        RecurrencePattern::Weekdays => date.weekday().number_from_monday() <= 5,
        RecurrencePattern::Weekly { interval, weekdays } => {
            let start_week =
                starts_on - Duration::days(i64::from(starts_on.weekday().num_days_from_monday()));
            let date_week = date - Duration::days(i64::from(date.weekday().num_days_from_monday()));
            let week_offset = (date_week - start_week).num_days() / 7;
            week_offset % i64::from(*interval) == 0
                && weekdays.contains(&(date.weekday().number_from_monday() as u8))
        }
        RecurrencePattern::Monthly {
            interval,
            day_of_month,
        } => {
            let month_offset = (date.year() - starts_on.year()) * 12 + date.month() as i32
                - starts_on.month() as i32;
            month_offset >= 0
                && month_offset % *interval as i32 == 0
                && date.day() == u32::from(*day_of_month).min(last_day_of_month(date))
        }
    }
}

fn last_day_of_month(date: NaiveDate) -> u32 {
    let (year, month) = if date.month() == 12 {
        (date.year() + 1, 1)
    } else {
        (date.year(), date.month() + 1)
    };
    NaiveDate::from_ymd_opt(year, month, 1)
        .expect("the month after a valid date is valid")
        .pred_opt()
        .expect("the day before a valid month is valid")
        .day()
}

fn validate_interval(interval: u32) -> Result<(), DomainError> {
    if interval == 0 {
        Err(recurrence_error(
            "recurrence interval must be positive",
            "pattern.interval",
        ))
    } else {
        Ok(())
    }
}

fn parse_date(value: &str, field: &'static str) -> Result<NaiveDate, DomainError> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map_err(|_| recurrence_error("recurrence date must use YYYY-MM-DD", field))
}

fn recurrence_error(message: &'static str, field: &'static str) -> DomainError {
    DomainError {
        code: "RECURRENCE_INVALID".into(),
        message: message.into(),
        field: Some(field.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn rule(pattern: RecurrencePattern) -> RecurrenceRule {
        RecurrenceRule {
            id: "rule-1".into(),
            task_template_id: "task-1".into(),
            pattern,
            local_time: Some("09:30".into()),
            timezone: "Asia/Shanghai".into(),
            starts_on: "2026-07-20".into(),
            ends_on: Some("2026-12-31".into()),
            status: RecurrenceStatus::Active,
            version: 1,
        }
    }

    fn valid_pattern_strategy() -> impl Strategy<Value = RecurrencePattern> {
        prop_oneof![
            (1u32..15).prop_map(|interval| RecurrencePattern::Daily { interval }),
            Just(RecurrencePattern::Weekdays),
            (1u32..8, prop::collection::btree_set(1u8..8, 1..8)).prop_map(
                |(interval, weekdays)| RecurrencePattern::Weekly {
                    interval,
                    weekdays: weekdays.into_iter().collect(),
                }
            ),
            (1u32..8, 1u8..32).prop_map(|(interval, day_of_month)| {
                RecurrencePattern::Monthly {
                    interval,
                    day_of_month,
                }
            }),
        ]
    }

    fn valid_start_date_strategy() -> impl Strategy<Value = NaiveDate> {
        (2024i32..2032, 1u32..13, 1u32..29)
            .prop_map(|(year, month, day)| NaiveDate::from_ymd_opt(year, month, day).unwrap())
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(256))]

        // Validates P2 and acceptance criteria R13.1-R13.3 and R13.10.
        #[test]
        fn p2_rule_generation_is_deterministic_and_strictly_ordered(
            pattern in valid_pattern_strategy(),
            starts_on in valid_start_date_strategy(),
            ends_after in prop::option::of(0i64..900),
            range_offset in -60i64..120,
            range_span in 0i64..730,
            local_time in prop_oneof![Just(None), (0u8..24, 0u8..60).prop_map(|(hour, minute)| Some(format!("{hour:02}:{minute:02}")))],
            timezone in prop::sample::select(vec!["UTC", "Asia/Shanghai", "America/New_York", "Europe/London", "Australia/Sydney"]),
        ) {
            let ends_on = ends_after.map(|days| starts_on + Duration::days(days));
            let rule = RecurrenceRule {
                id: "p2-rule".into(),
                task_template_id: "p2-task".into(),
                pattern,
                local_time,
                timezone: timezone.into(),
                starts_on: starts_on.format("%Y-%m-%d").to_string(),
                ends_on: ends_on.map(|date| date.format("%Y-%m-%d").to_string()),
                status: RecurrenceStatus::Active,
                version: 1,
            };
            let range_start = starts_on + Duration::days(range_offset);
            let range_end = range_start + Duration::days(range_span);

            let first = scheduled_dates(&rule, range_start, range_end).unwrap();
            let second = scheduled_dates(&rule, range_start, range_end).unwrap();
            let lower_bound = range_start.max(starts_on);
            let upper_bound = ends_on.map_or(range_end, |end| range_end.min(end));

            prop_assert_eq!(&first, &second);
            prop_assert!(first.windows(2).all(|dates| dates[0] < dates[1]));
            prop_assert!(first.iter().all(|date| *date >= lower_bound && *date <= upper_bound));
        }
    }

    #[test]
    fn accepts_supported_patterns_and_custom_intervals() {
        let patterns = [
            RecurrencePattern::Daily { interval: 3 },
            RecurrencePattern::Weekdays,
            RecurrencePattern::Weekly {
                interval: 2,
                weekdays: vec![1, 3, 7],
            },
            RecurrencePattern::Monthly {
                interval: 2,
                day_of_month: 31,
            },
        ];

        for pattern in patterns {
            assert!(rule(pattern).validate().is_ok());
        }
    }

    #[test]
    fn rejects_zero_intervals_and_invalid_pattern_values() {
        let invalid_patterns = [
            RecurrencePattern::Daily { interval: 0 },
            RecurrencePattern::Weekly {
                interval: 1,
                weekdays: vec![],
            },
            RecurrencePattern::Weekly {
                interval: 1,
                weekdays: vec![1, 1],
            },
            RecurrencePattern::Weekly {
                interval: 1,
                weekdays: vec![0, 3],
            },
            RecurrencePattern::Monthly {
                interval: 1,
                day_of_month: 0,
            },
        ];

        for pattern in invalid_patterns {
            let error = rule(pattern).validate().unwrap_err();
            assert_eq!(error.code, "RECURRENCE_INVALID");
            assert!(error.field.unwrap().starts_with("pattern."));
        }
    }

    #[test]
    fn validates_time_timezone_date_range_and_version() {
        let mut invalid_time = rule(RecurrencePattern::Weekdays);
        invalid_time.local_time = Some("25:00".into());
        assert_eq!(
            invalid_time.validate().unwrap_err().field.as_deref(),
            Some("localTime")
        );

        let mut invalid_timezone = rule(RecurrencePattern::Weekdays);
        invalid_timezone.timezone = "Asia/Unknown".into();
        assert_eq!(
            invalid_timezone.validate().unwrap_err().field.as_deref(),
            Some("timezone")
        );

        let mut invalid_range = rule(RecurrencePattern::Weekdays);
        invalid_range.ends_on = Some("2026-07-19".into());
        assert_eq!(
            invalid_range.validate().unwrap_err().field.as_deref(),
            Some("endsOn")
        );

        let mut invalid_version = rule(RecurrencePattern::Weekdays);
        invalid_version.version = 0;
        assert_eq!(
            invalid_version.validate().unwrap_err().field.as_deref(),
            Some("version")
        );
    }

    #[test]
    fn accepts_optional_schedule_fields_and_validates_rule_identity() {
        let mut open_ended = rule(RecurrencePattern::Weekdays);
        open_ended.local_time = None;
        open_ended.ends_on = None;
        assert!(open_ended.validate().is_ok());

        let mut missing_template = rule(RecurrencePattern::Weekdays);
        missing_template.task_template_id = " ".into();
        assert_eq!(
            missing_template.validate().unwrap_err().field.as_deref(),
            Some("taskTemplateId")
        );

        let mut invalid_start = rule(RecurrencePattern::Weekdays);
        invalid_start.starts_on = "2026/07/20".into();
        assert_eq!(
            invalid_start.validate().unwrap_err().field.as_deref(),
            Some("startsOn")
        );
    }

    #[test]
    fn serializes_the_persisted_pattern_shape() {
        let value = serde_json::to_value(rule(RecurrencePattern::Monthly {
            interval: 1,
            day_of_month: 15,
        }))
        .unwrap();

        assert_eq!(value["taskTemplateId"], "task-1");
        assert_eq!(value["pattern"]["kind"], "monthly");
        assert_eq!(value["pattern"]["dayOfMonth"], 15);
        assert_eq!(value["status"], "active");
    }

    #[test]
    fn serializes_instance_status_and_change_scope_for_command_boundaries() {
        assert_eq!(
            serde_json::to_value(TaskInstanceStatus::Rescheduled).unwrap(),
            "rescheduled"
        );
        assert_eq!(
            serde_json::to_value(RecurrenceChangeScope::ThisInstance {
                instance_id: "instance-1".into(),
            })
            .unwrap(),
            serde_json::json!({
                "scope": "thisInstance",
                "instanceId": "instance-1"
            })
        );
        assert_eq!(
            serde_json::to_value(RecurrenceChangeScope::Future {
                effective_on: "2026-07-20".into(),
            })
            .unwrap(),
            serde_json::json!({
                "scope": "future",
                "effectiveOn": "2026-07-20"
            })
        );
    }

    #[test]
    fn expands_daily_weekday_and_weekly_rules_in_stable_order() {
        let start = NaiveDate::from_ymd_opt(2026, 7, 20).unwrap();
        let end = NaiveDate::from_ymd_opt(2026, 8, 2).unwrap();

        assert_eq!(
            scheduled_dates(&rule(RecurrencePattern::Daily { interval: 3 }), start, end).unwrap(),
            [
                "2026-07-20",
                "2026-07-23",
                "2026-07-26",
                "2026-07-29",
                "2026-08-01"
            ]
            .map(|value| NaiveDate::parse_from_str(value, "%Y-%m-%d").unwrap())
        );
        assert_eq!(
            scheduled_dates(&rule(RecurrencePattern::Weekdays), start, end)
                .unwrap()
                .len(),
            10
        );
        assert_eq!(
            scheduled_dates(
                &rule(RecurrencePattern::Weekly {
                    interval: 2,
                    weekdays: vec![1, 5],
                }),
                start,
                end,
            )
            .unwrap(),
            ["2026-07-20", "2026-07-24"]
                .map(|value| NaiveDate::parse_from_str(value, "%Y-%m-%d").unwrap())
        );
    }

    #[test]
    fn normalizes_monthly_dates_to_each_month_end() {
        let mut monthly = rule(RecurrencePattern::Monthly {
            interval: 1,
            day_of_month: 31,
        });
        monthly.starts_on = "2026-01-31".into();
        monthly.ends_on = None;

        assert_eq!(
            scheduled_dates(
                &monthly,
                NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
                NaiveDate::from_ymd_opt(2026, 4, 30).unwrap(),
            )
            .unwrap(),
            ["2026-01-31", "2026-02-28", "2026-03-31", "2026-04-30"]
                .map(|value| NaiveDate::parse_from_str(value, "%Y-%m-%d").unwrap())
        );
    }

    #[test]
    fn clips_generation_to_rule_bounds_and_active_status() {
        let mut bounded = rule(RecurrencePattern::Daily { interval: 1 });
        bounded.starts_on = "2026-07-22".into();
        bounded.ends_on = Some("2026-07-24".into());
        assert_eq!(
            scheduled_dates(
                &bounded,
                NaiveDate::from_ymd_opt(2026, 7, 20).unwrap(),
                NaiveDate::from_ymd_opt(2026, 7, 26).unwrap(),
            )
            .unwrap()
            .len(),
            3
        );

        bounded.status = RecurrenceStatus::Paused;
        assert!(scheduled_dates(
            &bounded,
            NaiveDate::from_ymd_opt(2026, 7, 20).unwrap(),
            NaiveDate::from_ymd_opt(2026, 7, 26).unwrap(),
        )
        .unwrap()
        .is_empty());
    }
}
