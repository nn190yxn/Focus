use chrono::{DateTime, Utc};
use rusqlite::{params, OptionalExtension};

use super::database::Database;
use crate::{
    domain::focus::{ActiveFocus, ActiveFocusStatus, FocusCompletionKind, FocusSession},
    DomainError,
};

pub struct FocusRepository<'a> {
    database: &'a Database,
}

impl<'a> FocusRepository<'a> {
    pub fn new(database: &'a Database) -> Self {
        Self { database }
    }

    pub fn get_active(&self) -> Result<Option<ActiveFocus>, DomainError> {
        self.database.read(|connection| {
            connection
                .query_row(
                    "SELECT task_id, task_instance_id, state, planned_seconds, remaining_seconds, started_at, target_ends_at, paused_at, interruption_count, updated_at FROM active_focus WHERE singleton_id = 1",
                    [],
                    map_active,
                )
                .optional()
        })
    }

    pub fn insert_active(&self, focus: &ActiveFocus) -> Result<(), DomainError> {
        self.database.write(|tx| {
            tx.execute(
                "INSERT INTO active_focus(singleton_id, task_id, task_instance_id, state, planned_seconds, remaining_seconds, started_at, target_ends_at, paused_at, interruption_count, updated_at) VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    focus.task_id,
                    focus.task_instance_id,
                    active_status_name(focus.status),
                    focus.planned_seconds,
                    focus.remaining_seconds,
                    focus.started_at.to_rfc3339(),
                    focus.target_ends_at.map(|value| value.to_rfc3339()),
                    focus.paused_at.map(|value| value.to_rfc3339()),
                    focus.interruption_count,
                    focus.updated_at.to_rfc3339(),
                ],
            )?;
            Ok(())
        })
    }

    pub fn update_active(&self, focus: &ActiveFocus) -> Result<bool, DomainError> {
        self.database.write(|tx| {
            Ok(tx.execute(
                "UPDATE active_focus SET task_id = ?1, task_instance_id = ?2, state = ?3, planned_seconds = ?4, remaining_seconds = ?5, started_at = ?6, target_ends_at = ?7, paused_at = ?8, interruption_count = ?9, updated_at = ?10 WHERE singleton_id = 1",
                params![
                    focus.task_id,
                    focus.task_instance_id,
                    active_status_name(focus.status),
                    focus.planned_seconds,
                    focus.remaining_seconds,
                    focus.started_at.to_rfc3339(),
                    focus.target_ends_at.map(|value| value.to_rfc3339()),
                    focus.paused_at.map(|value| value.to_rfc3339()),
                    focus.interruption_count,
                    focus.updated_at.to_rfc3339(),
                ],
            )? == 1)
        })
    }

    pub fn complete_due(
        &self,
        now: DateTime<Utc>,
        session_id: &str,
    ) -> Result<Option<FocusSession>, DomainError> {
        self.database.write(|tx| {
            let active = tx
                .query_row(
                    "SELECT task_id, task_instance_id, state, planned_seconds, remaining_seconds, started_at, target_ends_at, paused_at, interruption_count, updated_at FROM active_focus WHERE singleton_id = 1",
                    [],
                    map_active,
                )
                .optional()?;
            let Some(active) = active else {
                return Ok(None);
            };
            if active.status != ActiveFocusStatus::Running || active.remaining_at(now) > 0 {
                return Ok(None);
            }
            let ended_at = active.target_ends_at.ok_or(rusqlite::Error::InvalidQuery)?;
            let project_id = if let Some(task_id) = active.task_id.as_deref() {
                tx.query_row(
                    "SELECT project_id FROM tasks WHERE id = ?1",
                    [task_id],
                    |row| row.get::<_, Option<String>>(0),
                )
                .optional()?
                .flatten()
            } else if let Some(instance_id) = active.task_instance_id.as_deref() {
                tx.query_row(
                    "SELECT snapshot_project_id FROM task_instances WHERE id = ?1",
                    [instance_id],
                    |row| row.get::<_, Option<String>>(0),
                )
                .optional()?
                .flatten()
            } else {
                None
            };
            let session = FocusSession {
                id: session_id.into(),
                task_id: active.task_id,
                task_instance_id: active.task_instance_id,
                project_id,
                planned_seconds: active.planned_seconds,
                actual_seconds: active.planned_seconds,
                interruption_count: active.interruption_count,
                completion_kind: FocusCompletionKind::Deadline,
                started_at: active.started_at,
                ended_at,
                created_at: now,
            };
            tx.execute("DELETE FROM active_focus WHERE singleton_id = 1", [])?;
            insert_session(tx, &session)?;
            Ok(Some(session))
        })
    }

    pub fn finalize(&self, session: &FocusSession) -> Result<(), DomainError> {
        self.database.write(|tx| {
            let removed = tx.execute("DELETE FROM active_focus WHERE singleton_id = 1", [])?;
            if removed != 1 {
                return Err(rusqlite::Error::QueryReturnedNoRows);
            }
            insert_session(tx, session)?;
            Ok(())
        })
    }

    #[cfg(test)]
    pub fn get_session(&self, id: &str) -> Result<Option<FocusSession>, DomainError> {
        self.database.read(|connection| {
            connection
                .query_row(
                    "SELECT id, task_id, task_instance_id, project_id, planned_seconds, actual_seconds, interruption_count, completion_kind, started_at, ended_at, created_at FROM focus_sessions WHERE id = ?1",
                    [id],
                    map_session,
                )
                .optional()
        })
    }

    #[cfg(test)]
    pub fn count_sessions(&self, kind: FocusCompletionKind) -> Result<i64, DomainError> {
        self.database.read(|connection| {
            connection.query_row(
                "SELECT COUNT(*) FROM focus_sessions WHERE completion_kind = ?1",
                [completion_kind_name(kind)],
                |row| row.get(0),
            )
        })
    }
}

fn insert_session(tx: &rusqlite::Transaction<'_>, session: &FocusSession) -> rusqlite::Result<()> {
    tx.execute(
        "INSERT INTO focus_sessions(id, task_id, task_instance_id, project_id, planned_seconds, actual_seconds, interruption_count, completion_kind, started_at, ended_at, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            session.id,
            session.task_id,
            session.task_instance_id,
            session.project_id,
            session.planned_seconds,
            session.actual_seconds,
            session.interruption_count,
            completion_kind_name(session.completion_kind),
            session.started_at.to_rfc3339(),
            session.ended_at.to_rfc3339(),
            session.created_at.to_rfc3339(),
        ],
    )?;
    Ok(())
}

fn map_active(row: &rusqlite::Row<'_>) -> rusqlite::Result<ActiveFocus> {
    let status = match row.get::<_, String>(2)?.as_str() {
        "running" => ActiveFocusStatus::Running,
        "paused" => ActiveFocusStatus::Paused,
        _ => return Err(invalid_type(2, "state")),
    };
    Ok(ActiveFocus {
        task_id: row.get(0)?,
        task_instance_id: row.get(1)?,
        status,
        planned_seconds: row.get(3)?,
        remaining_seconds: row.get(4)?,
        started_at: parse_datetime(row.get::<_, String>(5)?, 5)?,
        target_ends_at: parse_optional_datetime(row.get(6)?, 6)?,
        paused_at: parse_optional_datetime(row.get(7)?, 7)?,
        interruption_count: row.get(8)?,
        updated_at: parse_datetime(row.get::<_, String>(9)?, 9)?,
    })
}

#[cfg(test)]
fn map_session(row: &rusqlite::Row<'_>) -> rusqlite::Result<FocusSession> {
    let completion_kind = match row.get::<_, String>(7)?.as_str() {
        "deadline" => FocusCompletionKind::Deadline,
        "early" => FocusCompletionKind::Early,
        "cancelled" => FocusCompletionKind::Cancelled,
        _ => return Err(invalid_type(7, "completion_kind")),
    };
    Ok(FocusSession {
        id: row.get(0)?,
        task_id: row.get(1)?,
        task_instance_id: row.get(2)?,
        project_id: row.get(3)?,
        planned_seconds: row.get(4)?,
        actual_seconds: row.get(5)?,
        interruption_count: row.get(6)?,
        completion_kind,
        started_at: parse_datetime(row.get::<_, String>(8)?, 8)?,
        ended_at: parse_datetime(row.get::<_, String>(9)?, 9)?,
        created_at: parse_datetime(row.get::<_, String>(10)?, 10)?,
    })
}

fn parse_optional_datetime(
    value: Option<String>,
    index: usize,
) -> rusqlite::Result<Option<DateTime<Utc>>> {
    value.map(|value| parse_datetime(value, index)).transpose()
}

fn parse_datetime(value: String, index: usize) -> rusqlite::Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(&value)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                index,
                rusqlite::types::Type::Text,
                Box::new(error),
            )
        })
}

fn active_status_name(status: ActiveFocusStatus) -> &'static str {
    match status {
        ActiveFocusStatus::Running => "running",
        ActiveFocusStatus::Paused => "paused",
    }
}

fn completion_kind_name(kind: FocusCompletionKind) -> &'static str {
    match kind {
        FocusCompletionKind::Deadline => "deadline",
        FocusCompletionKind::Early => "early",
        FocusCompletionKind::Cancelled => "cancelled",
    }
}

fn invalid_type(index: usize, name: &str) -> rusqlite::Error {
    rusqlite::Error::InvalidColumnType(index, name.into(), rusqlite::types::Type::Text)
}
