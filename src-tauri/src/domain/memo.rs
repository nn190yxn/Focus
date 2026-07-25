use std::{collections::HashSet, str::FromStr};

use chrono::{DateTime, LocalResult, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};

use crate::DomainError;

const MAX_TITLE_LENGTH: usize = 200;
const MAX_BODY_LENGTH: usize = 20_000;
const MAX_TAGS: usize = 10;
const MAX_TAG_LENGTH: usize = 30;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoTag {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoTagSummary {
    pub id: String,
    pub name: String,
    pub memo_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoRecord {
    pub id: String,
    pub title: String,
    pub body: String,
    pub display_title: String,
    pub tags: Vec<MemoTag>,
    pub pinned_at: Option<String>,
    pub reminder: Option<MemoReminderRule>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoSummary {
    pub id: String,
    pub display_title: String,
    pub body_preview: String,
    pub tags: Vec<MemoTag>,
    pub pinned_at: Option<String>,
    pub reminder: Option<MemoReminderRule>,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoListQuery {
    pub search: String,
    pub tag_id: Option<String>,
}

impl MemoListQuery {
    pub fn validate(&self) -> Result<(), DomainError> {
        if self.search.chars().count() > MAX_BODY_LENGTH {
            return Err(validation_error(
                "MEMO_SEARCH_INVALID",
                "memo search must contain at most 20000 characters",
                "search",
            ));
        }
        if self.tag_id.as_ref().is_some_and(|id| id.trim().is_empty()) {
            return Err(validation_error(
                "MEMO_TAG_ID_INVALID",
                "memo tag id must not be empty",
                "tagId",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoInput {
    pub title: String,
    pub body: String,
    pub tags: Vec<String>,
    pub pinned: bool,
    pub reminder: Option<MemoReminderInput>,
}

impl MemoInput {
    pub fn validate(&self) -> Result<(), DomainError> {
        self.validate_at(Utc::now())
    }

    pub fn validate_at(&self, now: DateTime<Utc>) -> Result<(), DomainError> {
        if self.title.chars().count() > MAX_TITLE_LENGTH {
            return Err(validation_error(
                "MEMO_TITLE_INVALID",
                "memo title must contain at most 200 characters",
                "title",
            ));
        }
        if self.body.chars().count() > MAX_BODY_LENGTH {
            return Err(validation_error(
                "MEMO_BODY_INVALID",
                "memo body must contain at most 20000 characters",
                "body",
            ));
        }
        validate_tags(&self.tags)?;
        if let Some(reminder) = &self.reminder {
            reminder.validate_at(now)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MemoReminderFrequency {
    Daily,
    Weekdays,
    Weekly,
    Monthly,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum MemoReminderInput {
    Once {
        scheduled_local: String,
        timezone: String,
    },
    Recurring {
        frequency: MemoReminderFrequency,
        interval: u32,
        weekdays: Vec<u8>,
        monthly_day: Option<u8>,
        local_time: String,
        starts_on: String,
        ends_on: Option<String>,
        timezone: String,
    },
}

impl MemoReminderInput {
    pub fn validate(&self) -> Result<(), DomainError> {
        self.validate_at(Utc::now())
    }

    pub fn validate_at(&self, now: DateTime<Utc>) -> Result<(), DomainError> {
        match self {
            Self::Once {
                scheduled_local,
                timezone,
            } => {
                let timezone = parse_timezone(timezone)?;
                let local = parse_local_datetime(scheduled_local)?;
                let scheduled = match timezone.from_local_datetime(&local) {
                    LocalResult::Single(value) => value.with_timezone(&Utc),
                    LocalResult::Ambiguous(first, second) => first.min(second).with_timezone(&Utc),
                    LocalResult::None => {
                        return Err(validation_error(
                            "MEMO_REMINDER_TIME_INVALID",
                            "memo reminder local time does not exist in the selected timezone",
                            "scheduledLocal",
                        ));
                    }
                };
                if scheduled <= now {
                    return Err(validation_error(
                        "MEMO_REMINDER_TIME_INVALID",
                        "one-time memo reminder must be scheduled in the future",
                        "scheduledLocal",
                    ));
                }
            }
            Self::Recurring {
                frequency,
                interval,
                weekdays,
                monthly_day,
                local_time,
                starts_on,
                ends_on,
                timezone,
            } => {
                parse_timezone(timezone)?;
                parse_time(local_time)?;
                let starts_on = parse_date(starts_on, "startsOn")?;
                if ends_on
                    .as_deref()
                    .map(|value| parse_date(value, "endsOn"))
                    .transpose()?
                    .is_some_and(|ends_on| ends_on < starts_on)
                {
                    return Err(validation_error(
                        "MEMO_REMINDER_DATE_INVALID",
                        "memo reminder end date must not precede its start date",
                        "endsOn",
                    ));
                }
                if *interval == 0 {
                    return Err(validation_error(
                        "MEMO_REMINDER_INTERVAL_INVALID",
                        "memo reminder interval must be greater than zero",
                        "interval",
                    ));
                }
                validate_recurring_shape(*frequency, *interval, weekdays, *monthly_day)?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoReminderRule {
    pub id: String,
    pub memo_id: String,
    pub schedule: MemoReminderInput,
    pub next_scheduled_for: Option<String>,
    pub status: MemoReminderStatus,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DueMemoReminder {
    pub reminder: MemoReminderRule,
    pub display_title: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MemoReminderStatus {
    Active,
    Completed,
    Cancelled,
}

fn validate_tags(tags: &[String]) -> Result<(), DomainError> {
    let mut normalized = HashSet::with_capacity(tags.len());
    for tag in tags {
        let trimmed = tag.trim();
        if trimmed.is_empty() || trimmed.chars().count() > MAX_TAG_LENGTH {
            return Err(validation_error(
                "MEMO_TAG_INVALID",
                "memo tag must contain between 1 and 30 characters",
                "tags",
            ));
        }
        normalized.insert(trimmed.to_lowercase());
    }
    if normalized.len() > MAX_TAGS {
        return Err(validation_error(
            "MEMO_TAG_LIMIT_EXCEEDED",
            "memo must contain at most 10 unique tags",
            "tags",
        ));
    }
    Ok(())
}

fn validate_recurring_shape(
    frequency: MemoReminderFrequency,
    interval: u32,
    weekdays: &[u8],
    monthly_day: Option<u8>,
) -> Result<(), DomainError> {
    if frequency == MemoReminderFrequency::Weekdays && interval != 1 {
        return Err(validation_error(
            "MEMO_REMINDER_INTERVAL_INVALID",
            "weekday memo reminders use an interval of one",
            "interval",
        ));
    }
    if weekdays.iter().any(|weekday| !(1..=7).contains(weekday))
        || weekdays.iter().collect::<HashSet<_>>().len() != weekdays.len()
    {
        return Err(validation_error(
            "MEMO_REMINDER_WEEKDAYS_INVALID",
            "memo reminder weekdays must be unique values from 1 through 7",
            "weekdays",
        ));
    }
    if frequency == MemoReminderFrequency::Weekly && weekdays.is_empty() {
        return Err(validation_error(
            "MEMO_REMINDER_WEEKDAYS_INVALID",
            "weekly memo reminder must contain at least one weekday",
            "weekdays",
        ));
    }
    if frequency != MemoReminderFrequency::Weekly && !weekdays.is_empty() {
        return Err(validation_error(
            "MEMO_REMINDER_WEEKDAYS_INVALID",
            "memo reminder weekdays are only valid for weekly reminders",
            "weekdays",
        ));
    }
    if frequency == MemoReminderFrequency::Monthly
        && !monthly_day.is_some_and(|day| (1..=31).contains(&day))
    {
        return Err(validation_error(
            "MEMO_REMINDER_MONTHLY_DAY_INVALID",
            "monthly memo reminder must contain a day from 1 through 31",
            "monthlyDay",
        ));
    }
    if frequency != MemoReminderFrequency::Monthly && monthly_day.is_some() {
        return Err(validation_error(
            "MEMO_REMINDER_MONTHLY_DAY_INVALID",
            "memo reminder monthly day is only valid for monthly reminders",
            "monthlyDay",
        ));
    }
    Ok(())
}

fn parse_timezone(value: &str) -> Result<Tz, DomainError> {
    Tz::from_str(value).map_err(|_| {
        validation_error(
            "MEMO_REMINDER_TIMEZONE_INVALID",
            "memo reminder timezone must be a valid IANA timezone",
            "timezone",
        )
    })
}

fn parse_local_datetime(value: &str) -> Result<NaiveDateTime, DomainError> {
    ["%Y-%m-%dT%H:%M", "%Y-%m-%dT%H:%M:%S"]
        .into_iter()
        .find_map(|format| NaiveDateTime::parse_from_str(value, format).ok())
        .ok_or_else(|| {
            validation_error(
                "MEMO_REMINDER_TIME_INVALID",
                "memo reminder time must use YYYY-MM-DDTHH:MM format",
                "scheduledLocal",
            )
        })
}

fn parse_time(value: &str) -> Result<NaiveTime, DomainError> {
    NaiveTime::parse_from_str(value, "%H:%M").map_err(|_| {
        validation_error(
            "MEMO_REMINDER_TIME_INVALID",
            "memo reminder local time must use HH:MM format",
            "localTime",
        )
    })
}

fn parse_date(value: &str, field: &str) -> Result<NaiveDate, DomainError> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d").map_err(|_| {
        validation_error(
            "MEMO_REMINDER_DATE_INVALID",
            "memo reminder date must use YYYY-MM-DD format",
            field,
        )
    })
}

fn validation_error(code: &str, message: &str, field: &str) -> DomainError {
    DomainError {
        code: code.into(),
        message: message.into(),
        field: Some(field.into()),
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    fn valid_input() -> MemoInput {
        MemoInput {
            title: "Project notes".into(),
            body: "Review the launch checklist".into(),
            tags: vec!["Work".into(), "Launch".into()],
            pinned: false,
            reminder: None,
        }
    }

    fn daily_reminder(interval: u32, ends_on: Option<&str>, timezone: &str) -> MemoReminderInput {
        MemoReminderInput::Recurring {
            frequency: MemoReminderFrequency::Daily,
            interval,
            weekdays: vec![],
            monthly_day: None,
            local_time: "09:30".into(),
            starts_on: "2026-07-23".into(),
            ends_on: ends_on.map(str::to_owned),
            timezone: timezone.into(),
        }
    }

    #[test]
    fn memo_accepts_empty_draft_and_unicode_boundaries() {
        let mut input = valid_input();
        input.title.clear();
        input.body.clear();
        assert!(input.validate().is_ok());

        input.title = "备".repeat(MAX_TITLE_LENGTH + 1);
        assert_eq!(
            input.validate().unwrap_err().field.as_deref(),
            Some("title")
        );
        input.title.clear();
        input.body = "录".repeat(MAX_BODY_LENGTH + 1);
        assert_eq!(input.validate().unwrap_err().field.as_deref(), Some("body"));
    }

    #[test]
    fn memo_tags_enforce_normalized_uniqueness_and_limits() {
        let mut input = valid_input();
        input.tags = vec![" Work ".into(), "work".into()];
        assert!(input.validate().is_ok());

        input.tags = (0..=MAX_TAGS).map(|index| format!("tag-{index}")).collect();
        assert_eq!(
            input.validate().unwrap_err().code,
            "MEMO_TAG_LIMIT_EXCEEDED"
        );
        input.tags = vec!["标".repeat(MAX_TAG_LENGTH + 1)];
        assert_eq!(input.validate().unwrap_err().code, "MEMO_TAG_INVALID");
    }

    #[test]
    fn once_reminder_requires_future_resolvable_local_time() {
        let now = Utc.with_ymd_and_hms(2026, 7, 23, 10, 0, 0).unwrap();
        let reminder = MemoReminderInput::Once {
            scheduled_local: "2026-07-23T19:00".into(),
            timezone: "Asia/Shanghai".into(),
        };
        assert!(reminder.validate_at(now).is_ok());

        let past = MemoReminderInput::Once {
            scheduled_local: "2026-07-23T17:00".into(),
            timezone: "Asia/Shanghai".into(),
        };
        assert_eq!(
            past.validate_at(now).unwrap_err().code,
            "MEMO_REMINDER_TIME_INVALID"
        );

        let current = MemoReminderInput::Once {
            scheduled_local: "2026-07-23T18:00".into(),
            timezone: "Asia/Shanghai".into(),
        };
        assert_eq!(
            current.validate_at(now).unwrap_err().code,
            "MEMO_REMINDER_TIME_INVALID"
        );

        let serialized = serde_json::to_value(reminder).unwrap();
        assert_eq!(serialized["kind"], "once");
        assert_eq!(serialized["scheduledLocal"], "2026-07-23T19:00");
    }

    #[test]
    fn recurring_reminder_validates_frequency_specific_fields() {
        let valid = MemoReminderInput::Recurring {
            frequency: MemoReminderFrequency::Weekly,
            interval: 2,
            weekdays: vec![1, 5],
            monthly_day: None,
            local_time: "09:30".into(),
            starts_on: "2026-07-23".into(),
            ends_on: Some("2026-12-31".into()),
            timezone: "Europe/Berlin".into(),
        };
        assert!(valid.validate().is_ok());

        let invalid_monthly = MemoReminderInput::Recurring {
            frequency: MemoReminderFrequency::Monthly,
            interval: 1,
            weekdays: vec![],
            monthly_day: None,
            local_time: "09:30".into(),
            starts_on: "2026-07-23".into(),
            ends_on: None,
            timezone: "Europe/Berlin".into(),
        };
        assert_eq!(
            invalid_monthly.validate().unwrap_err().field.as_deref(),
            Some("monthlyDay")
        );
    }

    #[test]
    fn recurring_reminder_rejects_bad_ranges_timezone_and_interval() {
        assert_eq!(
            daily_reminder(1, None, "Invalid/Timezone")
                .validate()
                .unwrap_err()
                .code,
            "MEMO_REMINDER_TIMEZONE_INVALID"
        );
        assert_eq!(
            daily_reminder(1, Some("2026-07-22"), "Asia/Shanghai")
                .validate()
                .unwrap_err()
                .field
                .as_deref(),
            Some("endsOn")
        );
        assert_eq!(
            daily_reminder(0, None, "Asia/Shanghai")
                .validate()
                .unwrap_err()
                .field
                .as_deref(),
            Some("interval")
        );
    }

    #[test]
    fn recurring_reminder_rejects_invalid_frequency_field_combinations() {
        let weekly_without_days = MemoReminderInput::Recurring {
            frequency: MemoReminderFrequency::Weekly,
            interval: 1,
            weekdays: vec![],
            monthly_day: None,
            local_time: "09:30".into(),
            starts_on: "2026-07-23".into(),
            ends_on: None,
            timezone: "Asia/Shanghai".into(),
        };
        assert_eq!(
            weekly_without_days.validate().unwrap_err().field.as_deref(),
            Some("weekdays")
        );

        let duplicate_days = MemoReminderInput::Recurring {
            frequency: MemoReminderFrequency::Weekly,
            interval: 1,
            weekdays: vec![1, 1],
            monthly_day: None,
            local_time: "09:30".into(),
            starts_on: "2026-07-23".into(),
            ends_on: None,
            timezone: "Asia/Shanghai".into(),
        };
        assert_eq!(
            duplicate_days.validate().unwrap_err().code,
            "MEMO_REMINDER_WEEKDAYS_INVALID"
        );

        let daily_with_monthly_day = MemoReminderInput::Recurring {
            frequency: MemoReminderFrequency::Daily,
            interval: 1,
            weekdays: vec![],
            monthly_day: Some(15),
            local_time: "09:30".into(),
            starts_on: "2026-07-23".into(),
            ends_on: None,
            timezone: "Asia/Shanghai".into(),
        };
        assert_eq!(
            daily_with_monthly_day
                .validate()
                .unwrap_err()
                .field
                .as_deref(),
            Some("monthlyDay")
        );
    }

    #[test]
    fn reminder_rejects_nonexistent_local_time_and_bad_clock_format() {
        let now = Utc.with_ymd_and_hms(2026, 3, 1, 0, 0, 0).unwrap();
        let nonexistent = MemoReminderInput::Once {
            scheduled_local: "2026-03-29T02:30".into(),
            timezone: "Europe/Berlin".into(),
        };
        assert_eq!(
            nonexistent.validate_at(now).unwrap_err().field.as_deref(),
            Some("scheduledLocal")
        );

        let bad_clock = MemoReminderInput::Recurring {
            frequency: MemoReminderFrequency::Daily,
            interval: 1,
            weekdays: vec![],
            monthly_day: None,
            local_time: "24:00".into(),
            starts_on: "2026-07-23".into(),
            ends_on: None,
            timezone: "Asia/Shanghai".into(),
        };
        assert_eq!(
            bad_clock.validate().unwrap_err().field.as_deref(),
            Some("localTime")
        );
    }
}
