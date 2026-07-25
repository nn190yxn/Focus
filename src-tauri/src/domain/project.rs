use serde::{Deserialize, Serialize};

use crate::DomainError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ProjectStatus {
    Active,
    Paused,
    Completed,
    Archived,
}

impl ProjectStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Paused => "paused",
            Self::Completed => "completed",
            Self::Archived => "archived",
        }
    }

    pub fn parse(value: &str) -> Result<Self, DomainError> {
        match value {
            "active" => Ok(Self::Active),
            "paused" => Ok(Self::Paused),
            "completed" => Ok(Self::Completed),
            "archived" => Ok(Self::Archived),
            _ => Err(project_error(
                "PROJECT_STATUS_INVALID",
                "unknown project status",
                Some("status"),
            )),
        }
    }

    pub fn can_transition_to(self, target: Self) -> bool {
        self == target
            || matches!(
                (self, target),
                (
                    Self::Active,
                    Self::Paused | Self::Completed | Self::Archived
                ) | (
                    Self::Paused,
                    Self::Active | Self::Completed | Self::Archived
                ) | (Self::Completed, Self::Active | Self::Archived)
            )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectInput {
    pub name: String,
    pub description: String,
    pub color: String,
    pub icon: String,
    pub started_on: String,
    pub target_on: Option<String>,
}

impl ProjectInput {
    pub fn validate(&self) -> Result<(), DomainError> {
        let name_length = self.name.trim().chars().count();
        if !(1..=80).contains(&name_length) {
            return Err(project_error(
                "PROJECT_NAME_INVALID",
                "project name must contain 1 to 80 characters",
                Some("name"),
            ));
        }
        if self.description.chars().count() > 2000 {
            return Err(project_error(
                "PROJECT_DESCRIPTION_INVALID",
                "project description cannot exceed 2000 characters",
                Some("description"),
            ));
        }
        let start = parse_date(&self.started_on, "startedOn")?;
        if let Some(target_on) = &self.target_on {
            let target = parse_date(target_on, "targetOn")?;
            if target < start {
                return Err(project_error(
                    "PROJECT_DATE_RANGE_INVALID",
                    "target date must be on or after start date",
                    Some("targetOn"),
                ));
            }
        }
        if self.color.trim().is_empty() || self.icon.trim().is_empty() {
            return Err(project_error(
                "PROJECT_APPEARANCE_INVALID",
                "project color and icon are required",
                None,
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ProjectRemovalResolution {
    Archive,
    DetachHistory,
    Delete,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectAggregation {
    pub active_task_count: u32,
    pub completed_task_count: u32,
    pub total_task_count: u32,
    pub completion_percent: u8,
    pub focus_seconds: u64,
}

impl ProjectAggregation {
    pub fn from_records(task_statuses: &[&str], focus_durations: &[u64]) -> Self {
        let active_task_count = task_statuses
            .iter()
            .filter(|status| **status == "pending")
            .count() as u32;
        let completed_task_count = task_statuses
            .iter()
            .filter(|status| **status == "completed")
            .count() as u32;
        let total_task_count = active_task_count + completed_task_count;
        let completion_percent = (completed_task_count * 100)
            .checked_div(total_task_count)
            .unwrap_or(0) as u8;
        Self {
            active_task_count,
            completed_task_count,
            total_task_count,
            completion_percent,
            focus_seconds: focus_durations.iter().copied().sum(),
        }
    }
}

pub fn ensure_transition(current: ProjectStatus, target: ProjectStatus) -> Result<(), DomainError> {
    if current.can_transition_to(target) {
        Ok(())
    } else {
        Err(project_error(
            "PROJECT_STATUS_TRANSITION_INVALID",
            "project status transition is not allowed",
            Some("status"),
        ))
    }
}

fn parse_date(value: &str, field: &'static str) -> Result<chrono::NaiveDate, DomainError> {
    chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d").map_err(|_| {
        project_error(
            "PROJECT_DATE_INVALID",
            "project date must use YYYY-MM-DD",
            Some(field),
        )
    })
}

fn project_error(
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

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn archived_project_is_terminal() {
        assert!(ensure_transition(ProjectStatus::Archived, ProjectStatus::Active).is_err());
    }

    #[test]
    fn project_dates_are_ordered() {
        let input = ProjectInput {
            name: "Focus".into(),
            description: String::new(),
            color: "mint".into(),
            icon: "folder".into(),
            started_on: "2026-07-18".into(),
            target_on: Some("2026-07-17".into()),
        };
        assert_eq!(
            input.validate().unwrap_err().code,
            "PROJECT_DATE_RANGE_INVALID"
        );
    }

    proptest! {
        #[test]
        fn p5_aggregation_matches_its_record_definition(
            task_codes in prop::collection::vec(0u8..3, 0..200),
            focus_durations in prop::collection::vec(0u64..100_000, 0..200),
        ) {
            let statuses: Vec<&str> = task_codes.iter().map(|code| match code { 0 => "pending", 1 => "completed", _ => "removed" }).collect();
            let result = ProjectAggregation::from_records(&statuses, &focus_durations);
            let expected_active = statuses.iter().filter(|status| **status == "pending").count() as u32;
            let expected_completed = statuses.iter().filter(|status| **status == "completed").count() as u32;
            let expected_total = expected_active + expected_completed;
            let expected_percent = (expected_completed * 100)
                .checked_div(expected_total)
                .unwrap_or(0) as u8;

            prop_assert_eq!(result.active_task_count, expected_active);
            prop_assert_eq!(result.completed_task_count, expected_completed);
            prop_assert_eq!(result.total_task_count, expected_total);
            prop_assert_eq!(result.completion_percent, expected_percent);
            prop_assert_eq!(result.focus_seconds, focus_durations.iter().sum::<u64>());
        }
    }
}
