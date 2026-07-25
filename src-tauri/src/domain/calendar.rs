use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CalendarPeriod {
    Week,
    Month,
    Year,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarQuery {
    pub period: CalendarPeriod,
    pub anchor_date: String,
    pub timezone: String,
    pub category: Option<String>,
    pub project_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CalendarSourceKind {
    Task,
    RecurringInstance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CalendarTaskStatus {
    Pending,
    Completed,
    Skipped,
    Rescheduled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CalendarCompletionKind {
    Deadline,
    Early,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarProject {
    pub id: String,
    pub name: String,
    pub color: String,
    pub icon: String,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarTaskItem {
    pub source_kind: CalendarSourceKind,
    pub source_id: String,
    pub title: String,
    pub category: String,
    pub project: Option<CalendarProject>,
    pub scheduled_date: Option<String>,
    pub scheduled_time: Option<String>,
    pub status: CalendarTaskStatus,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarFocusSession {
    pub id: String,
    pub title: String,
    pub category: Option<String>,
    pub project: Option<CalendarProject>,
    pub actual_seconds: u64,
    pub completion_kind: CalendarCompletionKind,
    pub started_at: String,
    pub ended_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarDay {
    pub date: String,
    pub planned_tasks: Vec<CalendarTaskItem>,
    pub completed_tasks: Vec<CalendarTaskItem>,
    pub focus_sessions: Vec<CalendarFocusSession>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarPeriodResult {
    pub period: CalendarPeriod,
    pub starts_on: String,
    pub ends_on: String,
    pub days: Vec<CalendarDay>,
    pub projects: Vec<CalendarProject>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calendar_dtos_use_camel_case() {
        let query = CalendarQuery {
            period: CalendarPeriod::Week,
            anchor_date: "2026-07-20".into(),
            timezone: "UTC".into(),
            category: None,
            project_id: Some("project-1".into()),
        };
        let value = serde_json::to_value(query).unwrap();
        assert_eq!(value["period"], "week");
        assert_eq!(value["anchorDate"], "2026-07-20");
        assert_eq!(value["projectId"], "project-1");
    }
}
