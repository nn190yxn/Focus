use chrono::{NaiveDate, NaiveTime};
use serde::{Deserialize, Serialize};

use crate::DomainError;

const CATEGORIES: [&str; 4] = ["work", "study", "health", "life"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TaskCompletionFilter {
    Pending,
    Completed,
}

impl TaskCompletionFilter {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Completed => "completed",
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskListFilter {
    pub starts_on: Option<String>,
    pub ends_on: Option<String>,
    pub project_id: Option<String>,
    pub category: Option<String>,
    pub completion: Option<TaskCompletionFilter>,
    pub search: Option<String>,
}

impl TaskListFilter {
    pub fn validate(&self) -> Result<(), DomainError> {
        let starts_on = self
            .starts_on
            .as_deref()
            .map(|value| parse_filter_date(value, "startsOn"))
            .transpose()?;
        let ends_on = self
            .ends_on
            .as_deref()
            .map(|value| parse_filter_date(value, "endsOn"))
            .transpose()?;
        if starts_on
            .zip(ends_on)
            .is_some_and(|(start, end)| start > end)
        {
            return Err(task_error(
                "TASK_FILTER_DATE_RANGE_INVALID",
                "filter end date must be on or after start date",
                Some("endsOn"),
            ));
        }
        if self
            .category
            .as_deref()
            .is_some_and(|value| !CATEGORIES.contains(&value))
        {
            return Err(task_error(
                "TASK_CATEGORY_INVALID",
                "task category is invalid",
                Some("category"),
            ));
        }
        if self
            .search
            .as_deref()
            .is_some_and(|value| value.chars().count() > 200)
        {
            return Err(task_error(
                "TASK_SEARCH_INVALID",
                "task search cannot exceed 200 characters",
                Some("search"),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckItemInput {
    pub id: Option<String>,
    pub title: String,
    pub completed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskInput {
    pub project_id: Option<String>,
    pub title: String,
    pub category: String,
    pub priority: i64,
    pub scheduled_date: Option<String>,
    pub scheduled_time: Option<String>,
    #[serde(default)]
    pub check_items: Vec<CheckItemInput>,
}

impl TaskInput {
    pub fn validate(&self, today: NaiveDate) -> Result<(), DomainError> {
        if !(1..=200).contains(&self.title.trim().chars().count()) {
            return Err(task_error(
                "TASK_TITLE_INVALID",
                "task title must contain 1 to 200 characters",
                Some("title"),
            ));
        }
        if !CATEGORIES.contains(&self.category.as_str()) {
            return Err(task_error(
                "TASK_CATEGORY_INVALID",
                "task category is invalid",
                Some("category"),
            ));
        }
        if !(0..=3).contains(&self.priority) {
            return Err(task_error(
                "TASK_PRIORITY_INVALID",
                "task priority must be between 0 and 3",
                Some("priority"),
            ));
        }
        match &self.scheduled_date {
            Some(value) => {
                let date = NaiveDate::parse_from_str(value, "%Y-%m-%d").map_err(|_| {
                    task_error(
                        "TASK_DATE_INVALID",
                        "scheduled date must use YYYY-MM-DD",
                        Some("scheduledDate"),
                    )
                })?;
                if date < today {
                    return Err(task_error(
                        "TASK_DATE_IN_PAST",
                        "scheduled date cannot be earlier than today",
                        Some("scheduledDate"),
                    ));
                }
            }
            None if self.scheduled_time.is_some() => {
                return Err(task_error(
                    "TASK_TIME_REQUIRES_DATE",
                    "scheduled time requires a scheduled date",
                    Some("scheduledTime"),
                ));
            }
            None => {}
        }
        if let Some(value) = &self.scheduled_time {
            NaiveTime::parse_from_str(value, "%H:%M").map_err(|_| {
                task_error(
                    "TASK_TIME_INVALID",
                    "scheduled time must use HH:MM",
                    Some("scheduledTime"),
                )
            })?;
        }
        for item in &self.check_items {
            if !(1..=200).contains(&item.title.trim().chars().count()) {
                return Err(task_error(
                    "CHECK_ITEM_TITLE_INVALID",
                    "check item title must contain 1 to 200 characters",
                    Some("checkItems"),
                ));
            }
        }
        Ok(())
    }
}

pub fn task_error(
    code: &'static str,
    message: &'static str,
    field: Option<&'static str>,
) -> DomainError {
    DomainError {
        code: code.into(),
        message: message.into(),
        field: field.map(str::to_string),
    }
}

fn parse_filter_date(value: &str, field: &'static str) -> Result<NaiveDate, DomainError> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d").map_err(|_| {
        task_error(
            "TASK_FILTER_DATE_INVALID",
            "filter date must use YYYY-MM-DD",
            Some(field),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input() -> TaskInput {
        TaskInput {
            project_id: None,
            title: "Write migration".into(),
            category: "work".into(),
            priority: 2,
            scheduled_date: Some("2026-07-18".into()),
            scheduled_time: Some("09:30".into()),
            check_items: vec![],
        }
    }

    #[test]
    fn rejects_a_scheduled_date_before_today() {
        let mut value = input();
        value.scheduled_date = Some("2026-07-17".into());
        let error = value
            .validate(NaiveDate::from_ymd_opt(2026, 7, 18).unwrap())
            .unwrap_err();
        assert_eq!(error.code, "TASK_DATE_IN_PAST");
    }

    #[test]
    fn accepts_today_and_rejects_invalid_date_and_time_formats() {
        let today = NaiveDate::from_ymd_opt(2026, 7, 18).unwrap();
        assert!(input().validate(today).is_ok());

        let mut invalid_date = input();
        invalid_date.scheduled_date = Some("2026/07/18".into());
        assert_eq!(
            invalid_date.validate(today).unwrap_err().code,
            "TASK_DATE_INVALID"
        );

        let mut invalid_time = input();
        invalid_time.scheduled_time = Some("24:00".into());
        assert_eq!(
            invalid_time.validate(today).unwrap_err().code,
            "TASK_TIME_INVALID"
        );
    }

    #[test]
    fn requires_a_date_when_time_is_present() {
        let mut value = input();
        value.scheduled_date = None;
        let error = value
            .validate(NaiveDate::from_ymd_opt(2026, 7, 18).unwrap())
            .unwrap_err();
        assert_eq!(error.code, "TASK_TIME_REQUIRES_DATE");
    }

    #[test]
    fn accepts_valid_check_items() {
        let mut value = input();
        value.check_items.push(CheckItemInput {
            id: None,
            title: "Add indexes".into(),
            completed: false,
        });
        assert!(value
            .validate(NaiveDate::from_ymd_opt(2026, 7, 18).unwrap())
            .is_ok());
    }

    #[test]
    fn rejects_an_inverted_filter_date_range() {
        let filter = TaskListFilter {
            starts_on: Some("2026-07-19".into()),
            ends_on: Some("2026-07-18".into()),
            ..TaskListFilter::default()
        };
        assert_eq!(
            filter.validate().unwrap_err().code,
            "TASK_FILTER_DATE_RANGE_INVALID"
        );
    }

    #[test]
    fn accepts_open_filter_date_boundaries() {
        let filter = TaskListFilter {
            starts_on: Some("2026-07-18".into()),
            category: Some("study".into()),
            completion: Some(TaskCompletionFilter::Pending),
            search: Some("Rust".into()),
            ..TaskListFilter::default()
        };
        assert!(filter.validate().is_ok());
    }

    #[test]
    fn rejects_invalid_filter_values() {
        let invalid_date = TaskListFilter {
            starts_on: Some("18-07-2026".into()),
            ..TaskListFilter::default()
        };
        assert_eq!(
            invalid_date.validate().unwrap_err().code,
            "TASK_FILTER_DATE_INVALID"
        );

        let invalid_category = TaskListFilter {
            category: Some("other".into()),
            ..TaskListFilter::default()
        };
        assert_eq!(
            invalid_category.validate().unwrap_err().code,
            "TASK_CATEGORY_INVALID"
        );

        let long_search = TaskListFilter {
            search: Some("x".repeat(201)),
            ..TaskListFilter::default()
        };
        assert_eq!(
            long_search.validate().unwrap_err().code,
            "TASK_SEARCH_INVALID"
        );
    }
}
