use crate::{
    domain::{
        calendar::{CalendarPeriod, CalendarQuery},
        planning::{DailyNote, DailyNoteInput, WeeklyGoal, WeeklyGoalCategory, WeeklyGoalInput},
        statistics::StatisticsSummary,
    },
    repositories::{database::Database, planning_repository::PlanningRepository},
    services::calendar_service::CalendarService,
    DomainError,
};

pub struct PlanningService<'a> {
    database: &'a Database,
}

impl<'a> PlanningService<'a> {
    pub fn new(database: &'a Database) -> Self {
        Self { database }
    }

    pub fn get_note(&self, note_date: String) -> Result<Option<DailyNote>, DomainError> {
        DailyNoteInput {
            body: String::new(),
            note_date: note_date.clone(),
        }
        .validate()?;
        PlanningRepository::new(self.database).get_note(&note_date)
    }

    pub fn save_note(&self, input: DailyNoteInput) -> Result<DailyNote, DomainError> {
        input.validate()?;
        PlanningRepository::new(self.database).save_note(&input)
    }

    pub fn list_weekly_goals(
        &self,
        week_starts_on: String,
        timezone: String,
    ) -> Result<Vec<WeeklyGoal>, DomainError> {
        validate_week_start(&week_starts_on)?;
        let calendar = CalendarService::new(self.database).get_period(CalendarQuery {
            period: CalendarPeriod::Week,
            anchor_date: week_starts_on.clone(),
            timezone,
            category: None,
            project_id: None,
        })?;
        let summary = StatisticsSummary::from_calendar(&calendar);
        let repository = PlanningRepository::new(self.database);
        let mut goals = repository.list_goals(&week_starts_on)?;
        for goal in &mut goals {
            goal.completed_count = progress_for_category(goal.category, &summary)
                .min(u64::from(goal.target_count)) as u32;
        }
        let updated_at = repository.update_goal_progress(&goals)?;
        for goal in &mut goals {
            goal.updated_at.clone_from(&updated_at);
        }
        Ok(goals)
    }

    pub fn save_weekly_goal(
        &self,
        input: WeeklyGoalInput,
        timezone: String,
    ) -> Result<WeeklyGoal, DomainError> {
        input.validate()?;
        timezone.parse::<chrono_tz::Tz>().map_err(|_| DomainError {
            code: "CALENDAR_TIMEZONE_INVALID".into(),
            message: "timezone must be a valid IANA timezone".into(),
            field: Some("timezone".into()),
        })?;
        let saved = PlanningRepository::new(self.database).save_goal(&input)?;
        self.list_weekly_goals(input.week_starts_on, timezone)?
            .into_iter()
            .find(|goal| goal.id == saved.id)
            .ok_or_else(|| DomainError {
                code: "WEEKLY_GOAL_NOT_FOUND".into(),
                message: "saved weekly goal could not be loaded".into(),
                field: None,
            })
    }
}

fn validate_week_start(value: &str) -> Result<(), DomainError> {
    WeeklyGoalInput {
        id: None,
        week_starts_on: value.into(),
        title: "validation".into(),
        category: WeeklyGoalCategory::CompletedTasks,
        target_count: 1,
    }
    .validate()
}

fn progress_for_category(category: WeeklyGoalCategory, summary: &StatisticsSummary) -> u64 {
    match category {
        WeeklyGoalCategory::CompletedTasks => u64::from(summary.completed_task_count),
        WeeklyGoalCategory::FocusMinutes => summary.focus_minutes,
        WeeklyGoalCategory::ActiveDays => u64::from(summary.active_day_count),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::statistics::StatisticsSummary;
    use rusqlite::params;

    const STAMP: &str = "2026-01-01T00:00:00Z";

    fn save_goal(
        service: &PlanningService<'_>,
        title: &str,
        category: WeeklyGoalCategory,
        target_count: u32,
    ) -> WeeklyGoal {
        service
            .save_weekly_goal(
                WeeklyGoalInput {
                    id: None,
                    week_starts_on: "2026-07-20".into(),
                    title: title.into(),
                    category,
                    target_count,
                },
                "UTC".into(),
            )
            .unwrap()
    }

    fn insert_completed_task(database: &Database) {
        database
            .write(|tx| {
                tx.execute(
                    "INSERT INTO tasks(id, title, category, priority, scheduled_date, status, completed_at, created_at, updated_at)
                     VALUES ('completed-task', 'Completed task', 'work', 0, '2026-07-20', 'completed', '2026-07-20T10:00:00Z', ?1, ?1)",
                    params![STAMP],
                )?;
                Ok(())
            })
            .unwrap();
    }

    fn insert_focus(database: &Database, id: &str, ended_at: &str) {
        database
            .write(|tx| {
                tx.execute(
                    "INSERT INTO focus_sessions(id, planned_seconds, actual_seconds, interruption_count, completion_kind, started_at, ended_at, created_at)
                     VALUES (?1, 1500, 1200, 0, 'deadline', ?2, ?3, ?4)",
                    params![id, "2026-07-20T11:00:00Z", ended_at, STAMP],
                )?;
                Ok(())
            })
            .unwrap();
    }

    #[test]
    fn category_progress_uses_the_matching_statistic() {
        let summary = StatisticsSummary {
            period: CalendarPeriod::Week,
            starts_on: "2026-07-20".into(),
            ends_on: "2026-07-26".into(),
            planned_task_count: 8,
            completed_task_count: 5,
            completion_percent: 62,
            focus_seconds: 7_500,
            focus_minutes: 125,
            effective_session_count: 4,
            active_day_count: 3,
            trend: Vec::new(),
            project_investments: Vec::new(),
        };
        assert_eq!(
            progress_for_category(WeeklyGoalCategory::CompletedTasks, &summary),
            5
        );
        assert_eq!(
            progress_for_category(WeeklyGoalCategory::FocusMinutes, &summary),
            125
        );
        assert_eq!(
            progress_for_category(WeeklyGoalCategory::ActiveDays, &summary),
            3
        );
    }

    #[test]
    fn note_service_round_trips_unicode_content() {
        let database = Database::open_in_memory().unwrap();
        let service = PlanningService::new(&database);
        service
            .save_note(DailyNoteInput {
                body: "记录一个干扰想法".into(),
                note_date: "2026-07-20".into(),
            })
            .unwrap();
        assert_eq!(
            service.get_note("2026-07-20".into()).unwrap().unwrap().body,
            "记录一个干扰想法"
        );
    }

    #[test]
    fn weekly_goals_recalculate_and_persist_after_related_data_changes() {
        let database = Database::open_in_memory().unwrap();
        let service = PlanningService::new(&database);
        save_goal(
            &service,
            "Complete one task",
            WeeklyGoalCategory::CompletedTasks,
            1,
        );
        save_goal(
            &service,
            "Focus for thirty minutes",
            WeeklyGoalCategory::FocusMinutes,
            30,
        );
        save_goal(&service, "Stay active", WeeklyGoalCategory::ActiveDays, 7);

        insert_completed_task(&database);
        insert_focus(&database, "focus-day-two", "2026-07-21T11:20:00Z");
        let first_update = service
            .list_weekly_goals("2026-07-20".into(), "UTC".into())
            .unwrap();
        assert_eq!(first_update[0].completed_count, 1);
        assert_eq!(first_update[1].completed_count, 20);
        assert_eq!(first_update[2].completed_count, 2);

        insert_focus(&database, "focus-day-three", "2026-07-22T11:20:00Z");
        let second_update = service
            .list_weekly_goals("2026-07-20".into(), "UTC".into())
            .unwrap();
        assert_eq!(second_update[0].completed_count, 1);
        assert_eq!(second_update[1].completed_count, 30);
        assert_eq!(second_update[2].completed_count, 3);

        let persisted = PlanningRepository::new(&database)
            .list_goals("2026-07-20")
            .unwrap();
        assert_eq!(persisted, second_update);
    }
}
