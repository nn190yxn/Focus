use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::calendar::{CalendarPeriod, CalendarPeriodResult, CalendarProject, CalendarTaskStatus};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatisticsTrendPoint {
    pub date: String,
    pub planned_task_count: u32,
    pub completed_task_count: u32,
    pub focus_seconds: u64,
    pub effective_session_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectInvestment {
    pub project: CalendarProject,
    pub focus_seconds: u64,
    pub effective_session_count: u32,
    pub focus_percent: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatisticsSummary {
    pub period: CalendarPeriod,
    pub starts_on: String,
    pub ends_on: String,
    pub planned_task_count: u32,
    pub completed_task_count: u32,
    pub completion_percent: u8,
    pub focus_seconds: u64,
    pub focus_minutes: u64,
    pub effective_session_count: u32,
    pub active_day_count: u32,
    pub trend: Vec<StatisticsTrendPoint>,
    pub project_investments: Vec<ProjectInvestment>,
}

impl StatisticsSummary {
    pub fn from_calendar(calendar: &CalendarPeriodResult) -> Self {
        let planned_task_count = calendar
            .days
            .iter()
            .map(|day| day.planned_tasks.len() as u32)
            .sum::<u32>();
        let completed_planned_count = calendar
            .days
            .iter()
            .flat_map(|day| &day.planned_tasks)
            .filter(|task| task.status == CalendarTaskStatus::Completed)
            .count() as u32;
        let completed_task_count = calendar
            .days
            .iter()
            .map(|day| day.completed_tasks.len() as u32)
            .sum::<u32>();
        let focus_seconds = calendar
            .days
            .iter()
            .flat_map(|day| &day.focus_sessions)
            .map(|session| session.actual_seconds)
            .sum::<u64>();
        let effective_session_count = calendar
            .days
            .iter()
            .map(|day| day.focus_sessions.len() as u32)
            .sum::<u32>();
        let active_day_count = calendar
            .days
            .iter()
            .filter(|day| !day.completed_tasks.is_empty() || !day.focus_sessions.is_empty())
            .count() as u32;
        let completion_percent = completed_planned_count
            .saturating_mul(100)
            .checked_div(planned_task_count)
            .unwrap_or(0) as u8;
        let trend = calendar
            .days
            .iter()
            .map(|day| StatisticsTrendPoint {
                date: day.date.clone(),
                planned_task_count: day.planned_tasks.len() as u32,
                completed_task_count: day.completed_tasks.len() as u32,
                focus_seconds: day
                    .focus_sessions
                    .iter()
                    .map(|session| session.actual_seconds)
                    .sum(),
                effective_session_count: day.focus_sessions.len() as u32,
            })
            .collect();

        let mut investment_by_project = BTreeMap::new();
        for session in calendar.days.iter().flat_map(|day| &day.focus_sessions) {
            let Some(project) = &session.project else {
                continue;
            };
            let entry = investment_by_project
                .entry(project.id.clone())
                .or_insert_with(|| (project.clone(), 0_u64, 0_u32));
            entry.1 = entry.1.saturating_add(session.actual_seconds);
            entry.2 = entry.2.saturating_add(1);
        }
        let mut project_investments = investment_by_project
            .into_values()
            .map(
                |(project, project_focus_seconds, project_session_count)| ProjectInvestment {
                    focus_percent: percent(project_focus_seconds, focus_seconds),
                    project,
                    focus_seconds: project_focus_seconds,
                    effective_session_count: project_session_count,
                },
            )
            .collect::<Vec<_>>();
        project_investments.sort_by(|left, right| {
            right
                .focus_seconds
                .cmp(&left.focus_seconds)
                .then_with(|| left.project.name.cmp(&right.project.name))
                .then_with(|| left.project.id.cmp(&right.project.id))
        });

        Self {
            period: calendar.period,
            starts_on: calendar.starts_on.clone(),
            ends_on: calendar.ends_on.clone(),
            planned_task_count,
            completed_task_count,
            completion_percent,
            focus_seconds,
            focus_minutes: focus_seconds / 60,
            effective_session_count,
            active_day_count,
            trend,
            project_investments,
        }
    }
}

fn percent(part: u64, total: u64) -> u8 {
    part.saturating_mul(100).checked_div(total).unwrap_or(0) as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::calendar::{
        CalendarCompletionKind, CalendarDay, CalendarFocusSession, CalendarSourceKind,
        CalendarTaskItem,
    };

    fn project(id: &str, name: &str) -> CalendarProject {
        CalendarProject {
            id: id.into(),
            name: name.into(),
            color: "#4eaa98".into(),
            icon: "AF".into(),
            status: "active".into(),
        }
    }

    fn task(
        id: &str,
        status: CalendarTaskStatus,
        project: Option<CalendarProject>,
    ) -> CalendarTaskItem {
        CalendarTaskItem {
            source_kind: CalendarSourceKind::Task,
            source_id: id.into(),
            title: format!("Task {id}"),
            category: "work".into(),
            project,
            scheduled_date: Some("2026-07-20".into()),
            scheduled_time: None,
            status,
            completed_at: None,
        }
    }

    fn session(id: &str, seconds: u64, project: Option<CalendarProject>) -> CalendarFocusSession {
        CalendarFocusSession {
            id: id.into(),
            title: format!("Session {id}"),
            category: Some("work".into()),
            project,
            actual_seconds: seconds,
            completion_kind: CalendarCompletionKind::Deadline,
            started_at: "2026-07-20T08:00:00Z".into(),
            ended_at: "2026-07-20T08:30:00Z".into(),
        }
    }

    #[test]
    fn empty_calendar_returns_zero_metrics_and_complete_trend() {
        let calendar = CalendarPeriodResult {
            period: CalendarPeriod::Week,
            starts_on: "2026-07-20".into(),
            ends_on: "2026-07-21".into(),
            days: vec![
                CalendarDay {
                    date: "2026-07-20".into(),
                    planned_tasks: vec![],
                    completed_tasks: vec![],
                    focus_sessions: vec![],
                },
                CalendarDay {
                    date: "2026-07-21".into(),
                    planned_tasks: vec![],
                    completed_tasks: vec![],
                    focus_sessions: vec![],
                },
            ],
            projects: vec![],
        };

        let summary = StatisticsSummary::from_calendar(&calendar);
        assert_eq!(summary.completion_percent, 0);
        assert_eq!(summary.focus_minutes, 0);
        assert_eq!(summary.active_day_count, 0);
        assert_eq!(summary.trend.len(), 2);
        assert!(summary.project_investments.is_empty());
    }

    #[test]
    fn aggregates_completion_focus_activity_and_project_investment() {
        let primary = project("primary", "Primary");
        let secondary = project("secondary", "Secondary");
        let completed = task("done", CalendarTaskStatus::Completed, Some(primary.clone()));
        let pending = task("todo", CalendarTaskStatus::Pending, Some(secondary.clone()));
        let calendar = CalendarPeriodResult {
            period: CalendarPeriod::Week,
            starts_on: "2026-07-20".into(),
            ends_on: "2026-07-21".into(),
            days: vec![
                CalendarDay {
                    date: "2026-07-20".into(),
                    planned_tasks: vec![completed.clone(), pending],
                    completed_tasks: vec![completed],
                    focus_sessions: vec![
                        session("primary-1", 1_800, Some(primary.clone())),
                        session("unassigned", 600, None),
                    ],
                },
                CalendarDay {
                    date: "2026-07-21".into(),
                    planned_tasks: vec![],
                    completed_tasks: vec![],
                    focus_sessions: vec![session("secondary-1", 600, Some(secondary))],
                },
            ],
            projects: vec![primary, project("secondary", "Secondary")],
        };

        let summary = StatisticsSummary::from_calendar(&calendar);
        assert_eq!(summary.planned_task_count, 2);
        assert_eq!(summary.completed_task_count, 1);
        assert_eq!(summary.completion_percent, 50);
        assert_eq!(summary.focus_seconds, 3_000);
        assert_eq!(summary.focus_minutes, 50);
        assert_eq!(summary.effective_session_count, 3);
        assert_eq!(summary.active_day_count, 2);
        assert_eq!(summary.project_investments[0].project.id, "primary");
        assert_eq!(summary.project_investments[0].focus_percent, 60);
        assert_eq!(summary.project_investments[1].focus_percent, 20);
    }
}
