use rusqlite::params;

use super::database::Database;
use crate::{
    domain::today::{TodayItemKind, TodayItemStatus, TodayProjectSummary, TodaySourceKind},
    DomainError,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TodayCandidate {
    pub source_kind: TodaySourceKind,
    pub source_id: String,
    pub item_kind: TodayItemKind,
    pub recurrence_rule_id: Option<String>,
    pub title: String,
    pub category: String,
    pub priority: i64,
    pub scheduled_date: String,
    pub scheduled_time: Option<String>,
    pub scheduled_at: Option<String>,
    pub timezone: Option<String>,
    pub status: TodayItemStatus,
    pub completed_at: Option<String>,
    pub project: Option<TodayProjectSummary>,
    pub created_at: String,
}

pub struct TodayRepository<'a> {
    database: &'a Database,
}

impl<'a> TodayRepository<'a> {
    pub fn new(database: &'a Database) -> Self {
        Self { database }
    }

    pub fn list_candidates(&self, date: &str) -> Result<Vec<TodayCandidate>, DomainError> {
        self.database.read(|connection| {
            let mut statement = connection.prepare(
                "SELECT 'task', t.id, CASE WHEN t.project_id IS NULL THEN 'ordinaryTask' ELSE 'projectTask' END, NULL, t.title, t.category, t.priority, t.scheduled_date, t.scheduled_time, NULL, NULL, t.status, t.completed_at, p.id, p.name, p.color, p.icon, p.status, t.created_at
                 FROM tasks t
                 LEFT JOIN projects p ON p.id = t.project_id
                 WHERE t.status IN ('pending', 'completed')
                   AND (t.scheduled_date = ?1 OR (t.status = 'pending' AND t.scheduled_date < ?1))
                   AND NOT EXISTS (SELECT 1 FROM recurrence_rules r WHERE r.task_template_id = t.id)
                 UNION ALL
                 SELECT 'recurringInstance', i.id, 'recurringInstance', i.recurrence_rule_id, i.snapshot_title, t.category, t.priority, i.scheduled_date, NULL, i.scheduled_at, r.timezone, i.status, i.completed_at, p.id, p.name, p.color, p.icon, p.status, i.created_at
                 FROM task_instances i
                 JOIN recurrence_rules r ON r.id = i.recurrence_rule_id
                 JOIN tasks t ON t.id = r.task_template_id
                 LEFT JOIN projects p ON p.id = i.snapshot_project_id
                 WHERE i.status IN ('pending', 'completed')
                   AND (i.scheduled_date = ?1 OR (i.status = 'pending' AND i.scheduled_date < ?1))",
            )?;
            let candidates = statement
                .query_map(params![date], map_candidate)?
                .collect();
            candidates
        })
    }
}

fn map_candidate(row: &rusqlite::Row<'_>) -> rusqlite::Result<TodayCandidate> {
    let source_kind = match row.get::<_, String>(0)?.as_str() {
        "task" => TodaySourceKind::Task,
        "recurringInstance" => TodaySourceKind::RecurringInstance,
        _ => return Err(invalid_type(0, "source_kind")),
    };
    let item_kind = match row.get::<_, String>(2)?.as_str() {
        "ordinaryTask" => TodayItemKind::OrdinaryTask,
        "projectTask" => TodayItemKind::ProjectTask,
        "recurringInstance" => TodayItemKind::RecurringInstance,
        _ => return Err(invalid_type(2, "item_kind")),
    };
    let status = match row.get::<_, String>(11)?.as_str() {
        "pending" => TodayItemStatus::Pending,
        "completed" => TodayItemStatus::Completed,
        _ => return Err(invalid_type(11, "status")),
    };
    let project_id = row.get::<_, Option<String>>(13)?;
    let project = if let Some(id) = project_id {
        Some(TodayProjectSummary {
            id,
            name: row.get(14)?,
            color: row.get(15)?,
            icon: row.get(16)?,
            status: row.get(17)?,
        })
    } else {
        None
    };
    Ok(TodayCandidate {
        source_kind,
        source_id: row.get(1)?,
        item_kind,
        recurrence_rule_id: row.get(3)?,
        title: row.get(4)?,
        category: row.get(5)?,
        priority: row.get(6)?,
        scheduled_date: row.get(7)?,
        scheduled_time: row.get(8)?,
        scheduled_at: row.get(9)?,
        timezone: row.get(10)?,
        status,
        completed_at: row.get(12)?,
        project,
        created_at: row.get(18)?,
    })
}

fn invalid_type(index: usize, name: &str) -> rusqlite::Error {
    rusqlite::Error::InvalidColumnType(index, name.into(), rusqlite::types::Type::Text)
}
