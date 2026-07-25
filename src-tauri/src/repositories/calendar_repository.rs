use rusqlite::{params, Row};

use super::database::Database;
use crate::{
    domain::calendar::{
        CalendarCompletionKind, CalendarFocusSession, CalendarProject, CalendarSourceKind,
        CalendarTaskItem, CalendarTaskStatus,
    },
    DomainError,
};

pub struct CalendarRepository<'a> {
    database: &'a Database,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalendarTaskCandidate {
    pub item: CalendarTaskItem,
    pub scheduled_at: Option<String>,
    pub timezone: Option<String>,
}

impl<'a> CalendarRepository<'a> {
    pub fn new(database: &'a Database) -> Self {
        Self { database }
    }

    pub fn list_planned_tasks(
        &self,
        starts_on: &str,
        ends_on: &str,
        category: Option<&str>,
        project_id: Option<&str>,
    ) -> Result<Vec<CalendarTaskCandidate>, DomainError> {
        self.database.read(|connection| {
            let mut statement = connection.prepare(
                "SELECT 'task', t.id, t.title, t.category, p.id, p.name, p.color, p.icon, p.status,
                        t.scheduled_date, t.scheduled_time, t.status, t.completed_at, NULL, NULL
                 FROM tasks t
                 LEFT JOIN projects p ON p.id = t.project_id
                 WHERE t.status != 'removed'
                   AND t.scheduled_date BETWEEN ?1 AND ?2
                   AND NOT EXISTS (SELECT 1 FROM recurrence_rules r WHERE r.task_template_id = t.id)
                   AND (?3 IS NULL OR t.category = ?3)
                   AND (?4 IS NULL OR t.project_id = ?4)
                 UNION ALL
                 SELECT 'recurringInstance', i.id, i.snapshot_title, t.category,
                        p.id, p.name, p.color, p.icon, p.status,
                        i.scheduled_date, r.local_time,
                        i.status, i.completed_at, i.scheduled_at, r.timezone
                 FROM task_instances i
                 JOIN recurrence_rules r ON r.id = i.recurrence_rule_id
                 JOIN tasks t ON t.id = r.task_template_id
                 LEFT JOIN projects p ON p.id = i.snapshot_project_id
                 WHERE i.scheduled_date BETWEEN ?1 AND ?2
                   AND (?3 IS NULL OR t.category = ?3)
                   AND (?4 IS NULL OR i.snapshot_project_id = ?4)",
            )?;
            let items = statement
                .query_map(
                    params![starts_on, ends_on, category, project_id],
                    map_task_candidate,
                )?
                .collect();
            items
        })
    }

    pub fn list_completed_tasks(
        &self,
        utc_start: &str,
        utc_end: &str,
        category: Option<&str>,
        project_id: Option<&str>,
    ) -> Result<Vec<CalendarTaskCandidate>, DomainError> {
        self.database.read(|connection| {
            let mut statement = connection.prepare(
                "SELECT 'task', t.id, t.title, t.category, p.id, p.name, p.color, p.icon, p.status,
                        t.scheduled_date, t.scheduled_time, t.status, t.completed_at, NULL, NULL
                 FROM tasks t
                 LEFT JOIN projects p ON p.id = t.project_id
                 WHERE t.status = 'completed'
                   AND julianday(t.completed_at) >= julianday(?1)
                   AND julianday(t.completed_at) < julianday(?2)
                   AND NOT EXISTS (SELECT 1 FROM recurrence_rules r WHERE r.task_template_id = t.id)
                   AND (?3 IS NULL OR t.category = ?3)
                   AND (?4 IS NULL OR t.project_id = ?4)
                 UNION ALL
                 SELECT 'recurringInstance', i.id, i.snapshot_title, t.category,
                        p.id, p.name, p.color, p.icon, p.status,
                        i.scheduled_date, r.local_time,
                        i.status, i.completed_at, i.scheduled_at, r.timezone
                 FROM task_instances i
                 JOIN recurrence_rules r ON r.id = i.recurrence_rule_id
                 JOIN tasks t ON t.id = r.task_template_id
                 LEFT JOIN projects p ON p.id = i.snapshot_project_id
                 WHERE i.status = 'completed'
                   AND julianday(i.completed_at) >= julianday(?1)
                   AND julianday(i.completed_at) < julianday(?2)
                   AND (?3 IS NULL OR t.category = ?3)
                   AND (?4 IS NULL OR i.snapshot_project_id = ?4)",
            )?;
            let items = statement
                .query_map(
                    params![utc_start, utc_end, category, project_id],
                    map_task_candidate,
                )?
                .collect();
            items
        })
    }

    pub fn list_focus_sessions(
        &self,
        utc_start: &str,
        utc_end: &str,
        category: Option<&str>,
        project_id: Option<&str>,
    ) -> Result<Vec<CalendarFocusSession>, DomainError> {
        self.database.read(|connection| {
            let mut statement = connection.prepare(
                "SELECT f.id, COALESCE(t.title, i.snapshot_title, 'Independent focus'),
                        COALESCE(t.category, template.category),
                        p.id, p.name, p.color, p.icon, p.status,
                        f.actual_seconds, f.completion_kind, f.started_at, f.ended_at
                 FROM focus_sessions f
                 LEFT JOIN tasks t ON t.id = f.task_id
                 LEFT JOIN task_instances i ON i.id = f.task_instance_id
                 LEFT JOIN recurrence_rules r ON r.id = i.recurrence_rule_id
                 LEFT JOIN tasks template ON template.id = r.task_template_id
                 LEFT JOIN projects p ON p.id = f.project_id
                 WHERE f.completion_kind IN ('deadline', 'early')
                   AND julianday(f.ended_at) >= julianday(?1)
                   AND julianday(f.ended_at) < julianday(?2)
                   AND (?3 IS NULL OR COALESCE(t.category, template.category) = ?3)
                   AND (?4 IS NULL OR f.project_id = ?4)",
            )?;
            let sessions = statement
                .query_map(
                    params![utc_start, utc_end, category, project_id],
                    map_focus_session,
                )?
                .collect();
            sessions
        })
    }
}

fn map_task_candidate(row: &Row<'_>) -> rusqlite::Result<CalendarTaskCandidate> {
    Ok(CalendarTaskCandidate {
        item: CalendarTaskItem {
            source_kind: match row.get::<_, String>(0)?.as_str() {
                "task" => CalendarSourceKind::Task,
                "recurringInstance" => CalendarSourceKind::RecurringInstance,
                _ => return Err(invalid_type(0, "source_kind")),
            },
            source_id: row.get(1)?,
            title: row.get(2)?,
            category: row.get(3)?,
            project: map_project(row, 4)?,
            scheduled_date: row.get(9)?,
            scheduled_time: row.get(10)?,
            status: match row.get::<_, String>(11)?.as_str() {
                "pending" => CalendarTaskStatus::Pending,
                "completed" => CalendarTaskStatus::Completed,
                "skipped" => CalendarTaskStatus::Skipped,
                "rescheduled" => CalendarTaskStatus::Rescheduled,
                _ => return Err(invalid_type(11, "status")),
            },
            completed_at: row.get(12)?,
        },
        scheduled_at: row.get(13)?,
        timezone: row.get(14)?,
    })
}

fn map_focus_session(row: &Row<'_>) -> rusqlite::Result<CalendarFocusSession> {
    let actual_seconds = row.get::<_, i64>(8)?;
    Ok(CalendarFocusSession {
        id: row.get(0)?,
        title: row.get(1)?,
        category: row.get(2)?,
        project: map_project(row, 3)?,
        actual_seconds: u64::try_from(actual_seconds)
            .map_err(|_| invalid_type(8, "actual_seconds"))?,
        completion_kind: match row.get::<_, String>(9)?.as_str() {
            "deadline" => CalendarCompletionKind::Deadline,
            "early" => CalendarCompletionKind::Early,
            _ => return Err(invalid_type(9, "completion_kind")),
        },
        started_at: row.get(10)?,
        ended_at: row.get(11)?,
    })
}

fn map_project(row: &Row<'_>, start: usize) -> rusqlite::Result<Option<CalendarProject>> {
    let Some(id) = row.get::<_, Option<String>>(start)? else {
        return Ok(None);
    };
    Ok(Some(CalendarProject {
        id,
        name: row.get(start + 1)?,
        color: row.get(start + 2)?,
        icon: row.get(start + 3)?,
        status: row.get(start + 4)?,
    }))
}

fn invalid_type(index: usize, name: &str) -> rusqlite::Error {
    rusqlite::Error::InvalidColumnType(index, name.into(), rusqlite::types::Type::Text)
}
