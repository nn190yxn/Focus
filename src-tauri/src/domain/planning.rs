use chrono::{Datelike, NaiveDate, Weekday};
use serde::{Deserialize, Serialize};

use crate::DomainError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyNote {
    pub id: String,
    pub body: String,
    pub note_date: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyNoteInput {
    pub body: String,
    pub note_date: String,
}

impl DailyNoteInput {
    pub fn validate(&self) -> Result<(), DomainError> {
        parse_date(&self.note_date, "noteDate")?;
        if self.body.chars().count() > 4_000 {
            return Err(validation_error(
                "NOTE_BODY_INVALID",
                "note body must contain at most 4000 characters",
                "body",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WeeklyGoalCategory {
    CompletedTasks,
    FocusMinutes,
    ActiveDays,
}

impl WeeklyGoalCategory {
    pub fn as_database_value(self) -> &'static str {
        match self {
            Self::CompletedTasks => "completed_tasks",
            Self::FocusMinutes => "focus_minutes",
            Self::ActiveDays => "active_days",
        }
    }

    pub fn parse_database_value(value: &str) -> Result<Self, DomainError> {
        match value {
            "completed_tasks" => Ok(Self::CompletedTasks),
            "focus_minutes" => Ok(Self::FocusMinutes),
            "active_days" => Ok(Self::ActiveDays),
            _ => Err(DomainError {
                code: "WEEKLY_GOAL_DATA_INVALID".into(),
                message: format!("unknown weekly goal category: {value}"),
                field: None,
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WeeklyGoal {
    pub id: String,
    pub week_starts_on: String,
    pub title: String,
    pub category: WeeklyGoalCategory,
    pub target_count: u32,
    pub completed_count: u32,
    pub position: u32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WeeklyGoalInput {
    pub id: Option<String>,
    pub week_starts_on: String,
    pub title: String,
    pub category: WeeklyGoalCategory,
    pub target_count: u32,
}

impl WeeklyGoalInput {
    pub fn validate(&self) -> Result<(), DomainError> {
        let starts_on = parse_date(&self.week_starts_on, "weekStartsOn")?;
        if starts_on.weekday() != Weekday::Mon {
            return Err(validation_error(
                "WEEKLY_GOAL_WEEK_INVALID",
                "week start must be a Monday",
                "weekStartsOn",
            ));
        }
        let title_length = self.title.trim().chars().count();
        if !(1..=200).contains(&title_length) {
            return Err(validation_error(
                "WEEKLY_GOAL_TITLE_INVALID",
                "goal title must contain between 1 and 200 characters",
                "title",
            ));
        }
        if self.target_count == 0 {
            return Err(validation_error(
                "WEEKLY_GOAL_TARGET_INVALID",
                "goal target must be greater than zero",
                "targetCount",
            ));
        }
        Ok(())
    }
}

fn parse_date(value: &str, field: &str) -> Result<NaiveDate, DomainError> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d").map_err(|_| {
        validation_error(
            "PLANNING_DATE_INVALID",
            "date must use YYYY-MM-DD format",
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
    use super::*;

    #[test]
    fn note_accepts_empty_content_and_rejects_more_than_4000_characters() {
        assert!(DailyNoteInput {
            body: String::new(),
            note_date: "2026-07-20".into(),
        }
        .validate()
        .is_ok());
        assert!(DailyNoteInput {
            body: "想".repeat(4_001),
            note_date: "2026-07-20".into(),
        }
        .validate()
        .is_err());
    }

    #[test]
    fn weekly_goal_requires_a_monday_positive_target_and_title() {
        let valid = WeeklyGoalInput {
            id: None,
            week_starts_on: "2026-07-20".into(),
            title: "完成重点任务".into(),
            category: WeeklyGoalCategory::CompletedTasks,
            target_count: 5,
        };
        assert!(valid.validate().is_ok());
        assert!(WeeklyGoalInput {
            week_starts_on: "2026-07-21".into(),
            ..valid.clone()
        }
        .validate()
        .is_err());
        assert!(WeeklyGoalInput {
            target_count: 0,
            ..valid
        }
        .validate()
        .is_err());
    }
}
