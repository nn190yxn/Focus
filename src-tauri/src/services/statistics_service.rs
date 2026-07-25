use crate::{
    domain::{calendar::CalendarQuery, statistics::StatisticsSummary},
    repositories::database::Database,
    services::calendar_service::CalendarService,
    DomainError,
};

pub struct StatisticsService<'a> {
    database: &'a Database,
}

impl<'a> StatisticsService<'a> {
    pub fn new(database: &'a Database) -> Self {
        Self { database }
    }

    pub fn get_summary(&self, query: CalendarQuery) -> Result<StatisticsSummary, DomainError> {
        let calendar = CalendarService::new(self.database).get_period(query)?;
        Ok(StatisticsSummary::from_calendar(&calendar))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::calendar::CalendarPeriod;
    use rusqlite::params;

    const STAMP: &str = "2026-01-01T00:00:00Z";

    fn query(timezone: &str) -> CalendarQuery {
        CalendarQuery {
            period: CalendarPeriod::Week,
            anchor_date: "2026-07-20".into(),
            timezone: timezone.into(),
            category: None,
            project_id: None,
        }
    }

    fn insert_project(database: &Database, id: &str) {
        database
            .write(|tx| {
                tx.execute(
                    "INSERT INTO projects(id, name, description, color, icon, status, started_on, created_at, updated_at)
                     VALUES (?1, ?2, '', 'mint', 'folder', 'active', '2026-01-01', ?3, ?3)",
                    params![id, format!("Project {id}"), STAMP],
                )?;
                Ok(())
            })
            .unwrap();
    }

    fn insert_completed_task(database: &Database, id: &str, project_id: &str, completed_at: &str) {
        database
            .write(|tx| {
                tx.execute(
                    "INSERT INTO tasks(id, project_id, title, category, priority, scheduled_date, status, completed_at, created_at, updated_at)
                     VALUES (?1, ?2, ?3, 'work', 0, '2026-07-20', 'completed', ?4, ?5, ?5)",
                    params![id, project_id, format!("Task {id}"), completed_at, STAMP],
                )?;
                Ok(())
            })
            .unwrap();
    }

    fn insert_focus(
        database: &Database,
        id: &str,
        project_id: Option<&str>,
        actual_seconds: u64,
        started_at: &str,
        ended_at: &str,
    ) {
        database
            .write(|tx| {
                tx.execute(
                    "INSERT INTO focus_sessions(id, project_id, planned_seconds, actual_seconds, interruption_count, completion_kind, started_at, ended_at, created_at)
                     VALUES (?1, ?2, 1500, ?3, 0, 'deadline', ?4, ?5, ?6)",
                    params![id, project_id, actual_seconds, started_at, ended_at, STAMP],
                )?;
                Ok(())
            })
            .unwrap();
    }

    #[test]
    fn empty_week_returns_zero_metrics_and_a_complete_daily_trend() {
        let database = Database::open_in_memory().unwrap();
        let summary = StatisticsService::new(&database)
            .get_summary(query("Asia/Shanghai"))
            .unwrap();

        assert_eq!(summary.planned_task_count, 0);
        assert_eq!(summary.completed_task_count, 0);
        assert_eq!(summary.completion_percent, 0);
        assert_eq!(summary.focus_seconds, 0);
        assert_eq!(summary.focus_minutes, 0);
        assert_eq!(summary.effective_session_count, 0);
        assert_eq!(summary.active_day_count, 0);
        assert_eq!(summary.trend.len(), 7);
        assert!(summary
            .trend
            .iter()
            .all(|point| point.planned_task_count == 0
                && point.completed_task_count == 0
                && point.focus_seconds == 0
                && point.effective_session_count == 0));
        assert!(summary.project_investments.is_empty());
    }

    #[test]
    fn focus_session_crossing_local_midnight_belongs_to_its_end_date() {
        let database = Database::open_in_memory().unwrap();
        insert_focus(
            &database,
            "cross-midnight",
            None,
            1_200,
            "2026-07-19T15:50:00Z",
            "2026-07-19T16:10:00Z",
        );

        let summary = StatisticsService::new(&database)
            .get_summary(query("Asia/Shanghai"))
            .unwrap();

        assert_eq!(summary.focus_seconds, 1_200);
        assert_eq!(summary.focus_minutes, 20);
        assert_eq!(summary.effective_session_count, 1);
        assert_eq!(summary.active_day_count, 1);
        assert_eq!(summary.trend[0].date, "2026-07-20");
        assert_eq!(summary.trend[0].focus_seconds, 1_200);
        assert_eq!(summary.trend[0].effective_session_count, 1);
        assert!(summary.trend[1..]
            .iter()
            .all(|point| point.focus_seconds == 0));
    }

    #[test]
    fn project_filter_limits_task_focus_and_investment_aggregates() {
        let database = Database::open_in_memory().unwrap();
        insert_project(&database, "p1");
        insert_project(&database, "p2");
        insert_completed_task(&database, "task-p1", "p1", "2026-07-20T10:00:00Z");
        insert_completed_task(&database, "task-p2", "p2", "2026-07-20T11:00:00Z");
        insert_focus(
            &database,
            "focus-p1",
            Some("p1"),
            600,
            "2026-07-20T12:00:00Z",
            "2026-07-20T12:10:00Z",
        );
        insert_focus(
            &database,
            "focus-p2",
            Some("p2"),
            1_200,
            "2026-07-20T13:00:00Z",
            "2026-07-20T13:20:00Z",
        );
        let mut filtered_query = query("UTC");
        filtered_query.project_id = Some("p1".into());

        let summary = StatisticsService::new(&database)
            .get_summary(filtered_query)
            .unwrap();

        assert_eq!(summary.planned_task_count, 1);
        assert_eq!(summary.completed_task_count, 1);
        assert_eq!(summary.completion_percent, 100);
        assert_eq!(summary.focus_seconds, 600);
        assert_eq!(summary.focus_minutes, 10);
        assert_eq!(summary.effective_session_count, 1);
        assert_eq!(summary.project_investments.len(), 1);
        assert_eq!(summary.project_investments[0].project.id, "p1");
        assert_eq!(summary.project_investments[0].focus_percent, 100);
    }
}
