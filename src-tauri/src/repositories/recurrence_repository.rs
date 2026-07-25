use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};

use super::database::Database;
use crate::{
    domain::recurrence::{RecurrencePattern, RecurrenceRule, RecurrenceStatus, TaskInstanceStatus},
    DomainError,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskInstanceRecord {
    pub id: String,
    pub recurrence_rule_id: String,
    pub rule_version: u32,
    pub scheduled_date: String,
    pub scheduled_at: Option<String>,
    pub snapshot_title: String,
    pub snapshot_project_id: Option<String>,
    pub status: TaskInstanceStatus,
    pub completed_at: Option<String>,
    pub source_instance_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

pub struct RecurrenceRepository<'a> {
    database: &'a Database,
}

impl<'a> RecurrenceRepository<'a> {
    pub fn new(database: &'a Database) -> Self {
        Self { database }
    }

    pub fn insert_rule(&self, rule: &RecurrenceRule) -> Result<(), DomainError> {
        rule.validate()?;
        let pattern_json = serde_json::to_string(&rule.pattern).map_err(serialization_error)?;
        let now = chrono::Utc::now().to_rfc3339();
        self.database.write(|tx| {
            tx.execute(
                "INSERT INTO recurrence_rules(id, task_template_id, pattern_json, local_time, timezone, starts_on, ends_on, status, version, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?10)",
                params![rule.id, rule.task_template_id, pattern_json, rule.local_time, rule.timezone, rule.starts_on, rule.ends_on, status_name(rule.status), rule.version, now],
            )?;
            Ok(())
        })
    }

    pub fn insert_rule_and_instances(
        &self,
        rule: &RecurrenceRule,
        instances: &[TaskInstanceRecord],
    ) -> Result<usize, DomainError> {
        rule.validate()?;
        let pattern_json = serde_json::to_string(&rule.pattern).map_err(serialization_error)?;
        let now = chrono::Utc::now().to_rfc3339();
        self.database.write(|tx| {
            tx.execute(
                "INSERT INTO recurrence_rules(id, task_template_id, pattern_json, local_time, timezone, starts_on, ends_on, status, version, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?10)",
                params![rule.id, rule.task_template_id, pattern_json, rule.local_time, rule.timezone, rule.starts_on, rule.ends_on, status_name(rule.status), rule.version, now],
            )?;
            let mut affected = 0;
            for instance in instances {
                affected += upsert_instance(tx, instance, false)?;
            }
            Ok(affected)
        })
    }

    pub fn get_rule(&self, id: &str) -> Result<Option<RecurrenceRule>, DomainError> {
        let raw = self.database.read(|connection| {
            connection
                .query_row(
                    "SELECT id, task_template_id, pattern_json, local_time, timezone, starts_on, ends_on, status, version FROM recurrence_rules WHERE id = ?1",
                    [id],
                    map_raw_rule,
                )
                .optional()
        })?;
        raw.map(parse_rule).transpose()
    }

    pub fn update_rule(&self, rule: &RecurrenceRule) -> Result<bool, DomainError> {
        rule.validate()?;
        let pattern_json = serde_json::to_string(&rule.pattern).map_err(serialization_error)?;
        let now = chrono::Utc::now().to_rfc3339();
        self.database.write(|tx| {
            Ok(tx.execute(
                "UPDATE recurrence_rules SET task_template_id = ?2, pattern_json = ?3, local_time = ?4, timezone = ?5, starts_on = ?6, ends_on = ?7, status = ?8, version = ?9, updated_at = ?10 WHERE id = ?1",
                params![rule.id, rule.task_template_id, pattern_json, rule.local_time, rule.timezone, rule.starts_on, rule.ends_on, status_name(rule.status), rule.version, now],
            )? == 1)
        })
    }

    pub fn update_rule_and_instances(
        &self,
        rule: &RecurrenceRule,
        instances: &[TaskInstanceRecord],
    ) -> Result<usize, DomainError> {
        rule.validate()?;
        let pattern_json = serde_json::to_string(&rule.pattern).map_err(serialization_error)?;
        let now = chrono::Utc::now().to_rfc3339();
        self.database.write(|tx| {
            let updated = tx.execute(
                "UPDATE recurrence_rules SET task_template_id = ?2, pattern_json = ?3, local_time = ?4, timezone = ?5, starts_on = ?6, ends_on = ?7, status = ?8, version = ?9, updated_at = ?10 WHERE id = ?1",
                params![rule.id, rule.task_template_id, pattern_json, rule.local_time, rule.timezone, rule.starts_on, rule.ends_on, status_name(rule.status), rule.version, now],
            )?;
            if updated != 1 {
                return Ok(0);
            }
            let mut affected = updated;
            for instance in instances {
                affected += upsert_instance(tx, instance, true)?;
            }
            Ok(affected)
        })
    }

    pub fn list_active_rules(&self) -> Result<Vec<RecurrenceRule>, DomainError> {
        let rows: Vec<RawRule> = self.database.read(|connection| {
            let mut statement = connection.prepare(
                "SELECT id, task_template_id, pattern_json, local_time, timezone, starts_on, ends_on, status, version FROM recurrence_rules WHERE status = 'active' ORDER BY id",
            )?;
            let rows = statement.query_map([], map_raw_rule)?.collect();
            rows
        })?;
        rows.into_iter().map(parse_rule).collect()
    }

    pub fn upsert_instances(
        &self,
        instances: &[TaskInstanceRecord],
        refresh_pending: bool,
    ) -> Result<usize, DomainError> {
        self.database.write(|tx| {
            let mut affected = 0;
            for instance in instances {
                affected += upsert_instance(tx, instance, refresh_pending)?;
            }
            Ok(affected)
        })
    }

    pub fn list_instances_for_rule(
        &self,
        rule_id: &str,
    ) -> Result<Vec<TaskInstanceRecord>, DomainError> {
        self.database.read(|connection| {
            let mut statement = connection.prepare(
                "SELECT id, recurrence_rule_id, rule_version, scheduled_date, scheduled_at, snapshot_title, snapshot_project_id, status, completed_at, source_instance_id, created_at, updated_at FROM task_instances WHERE recurrence_rule_id = ?1 ORDER BY scheduled_date, id",
            )?;
            let instances = statement.query_map([rule_id], map_instance)?.collect();
            instances
        })
    }

    pub fn get_instance(&self, id: &str) -> Result<Option<TaskInstanceRecord>, DomainError> {
        self.database.read(|connection| {
            connection
                .query_row(
                    "SELECT id, recurrence_rule_id, rule_version, scheduled_date, scheduled_at, snapshot_title, snapshot_project_id, status, completed_at, source_instance_id, created_at, updated_at FROM task_instances WHERE id = ?1",
                    [id],
                    map_instance,
                )
                .optional()
        })
    }

    pub fn set_instance_status(
        &self,
        id: &str,
        status: TaskInstanceStatus,
        completed_at: Option<&str>,
        updated_at: &str,
    ) -> Result<bool, DomainError> {
        self.database.write(|tx| {
            Ok(tx.execute(
                "UPDATE task_instances SET status = ?2, completed_at = ?3, updated_at = ?4 WHERE id = ?1 AND status = 'pending' AND NOT EXISTS (SELECT 1 FROM active_focus WHERE task_instance_id = task_instances.id) AND NOT EXISTS (SELECT 1 FROM focus_sessions WHERE task_instance_id = task_instances.id)",
                params![id, instance_status_name(status), completed_at, updated_at],
            )? == 1)
        })
    }

    pub fn delay_instance(
        &self,
        id: &str,
        scheduled_at: Option<&str>,
        updated_at: &str,
    ) -> Result<bool, DomainError> {
        self.database.write(|tx| {
            Ok(tx.execute(
                "UPDATE task_instances SET scheduled_at = ?2, updated_at = ?3 WHERE id = ?1 AND status = 'pending' AND NOT EXISTS (SELECT 1 FROM active_focus WHERE task_instance_id = task_instances.id) AND NOT EXISTS (SELECT 1 FROM focus_sessions WHERE task_instance_id = task_instances.id)",
                params![id, scheduled_at, updated_at],
            )? == 1)
        })
    }

    pub fn reschedule_instance(
        &self,
        source_id: &str,
        target: &TaskInstanceRecord,
        updated_at: &str,
    ) -> Result<bool, DomainError> {
        self.database.write(|tx| {
            let source_actionable = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM task_instances WHERE id = ?1 AND status = 'pending' AND NOT EXISTS (SELECT 1 FROM active_focus WHERE task_instance_id = task_instances.id) AND NOT EXISTS (SELECT 1 FROM focus_sessions WHERE task_instance_id = task_instances.id))",
                [source_id],
                |row| row.get::<_, bool>(0),
            )?;
            if !source_actionable {
                return Ok(false);
            }
            let target_actionable = tx.query_row(
                "SELECT NOT EXISTS(SELECT 1 FROM task_instances WHERE recurrence_rule_id = ?1 AND scheduled_date = ?2 AND (status != 'pending' OR EXISTS (SELECT 1 FROM active_focus WHERE task_instance_id = task_instances.id) OR EXISTS (SELECT 1 FROM focus_sessions WHERE task_instance_id = task_instances.id)))",
                params![target.recurrence_rule_id, target.scheduled_date],
                |row| row.get::<_, bool>(0),
            )?;
            if !target_actionable {
                return Ok(false);
            }
            let target_affected = tx.execute(
                "INSERT INTO task_instances(id, recurrence_rule_id, rule_version, scheduled_date, scheduled_at, snapshot_title, snapshot_project_id, status, completed_at, source_instance_id, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'pending', NULL, ?8, ?9, ?10) ON CONFLICT(recurrence_rule_id, scheduled_date) DO UPDATE SET source_instance_id = COALESCE(task_instances.source_instance_id, excluded.source_instance_id), updated_at = excluded.updated_at WHERE task_instances.status = 'pending' AND NOT EXISTS (SELECT 1 FROM active_focus WHERE task_instance_id = task_instances.id) AND NOT EXISTS (SELECT 1 FROM focus_sessions WHERE task_instance_id = task_instances.id)",
                params![target.id, target.recurrence_rule_id, target.rule_version, target.scheduled_date, target.scheduled_at, target.snapshot_title, target.snapshot_project_id, source_id, target.created_at, target.updated_at],
            )?;
            let source_affected = tx.execute(
                "UPDATE task_instances SET status = 'rescheduled', completed_at = NULL, updated_at = ?2 WHERE id = ?1 AND status = 'pending' AND NOT EXISTS (SELECT 1 FROM active_focus WHERE task_instance_id = task_instances.id) AND NOT EXISTS (SELECT 1 FROM focus_sessions WHERE task_instance_id = task_instances.id)",
                params![source_id, updated_at],
            )?;
            Ok(target_affected == 1 && source_affected == 1)
        })
    }
}

struct RawRule {
    id: String,
    task_template_id: String,
    pattern_json: String,
    local_time: Option<String>,
    timezone: String,
    starts_on: String,
    ends_on: Option<String>,
    status: String,
    version: u32,
}

fn map_raw_rule(row: &rusqlite::Row<'_>) -> rusqlite::Result<RawRule> {
    Ok(RawRule {
        id: row.get(0)?,
        task_template_id: row.get(1)?,
        pattern_json: row.get(2)?,
        local_time: row.get(3)?,
        timezone: row.get(4)?,
        starts_on: row.get(5)?,
        ends_on: row.get(6)?,
        status: row.get(7)?,
        version: row.get(8)?,
    })
}

fn parse_rule(raw: RawRule) -> Result<RecurrenceRule, DomainError> {
    let pattern = serde_json::from_str::<RecurrencePattern>(&raw.pattern_json)
        .map_err(serialization_error)?;
    let status = match raw.status.as_str() {
        "active" => RecurrenceStatus::Active,
        "paused" => RecurrenceStatus::Paused,
        "ended" => RecurrenceStatus::Ended,
        _ => return Err(serialization_error("invalid recurrence status")),
    };
    Ok(RecurrenceRule {
        id: raw.id,
        task_template_id: raw.task_template_id,
        pattern,
        local_time: raw.local_time,
        timezone: raw.timezone,
        starts_on: raw.starts_on,
        ends_on: raw.ends_on,
        status,
        version: raw.version,
    })
}

fn status_name(status: RecurrenceStatus) -> &'static str {
    match status {
        RecurrenceStatus::Active => "active",
        RecurrenceStatus::Paused => "paused",
        RecurrenceStatus::Ended => "ended",
    }
}

fn upsert_instance(
    transaction: &rusqlite::Transaction<'_>,
    instance: &TaskInstanceRecord,
    refresh_pending: bool,
) -> rusqlite::Result<usize> {
    let sql = if refresh_pending {
        "INSERT INTO task_instances(id, recurrence_rule_id, rule_version, scheduled_date, scheduled_at, snapshot_title, snapshot_project_id, status, completed_at, source_instance_id, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12) ON CONFLICT(recurrence_rule_id, scheduled_date) DO UPDATE SET rule_version = excluded.rule_version, scheduled_at = excluded.scheduled_at, snapshot_title = excluded.snapshot_title, snapshot_project_id = excluded.snapshot_project_id, updated_at = excluded.updated_at WHERE task_instances.status = 'pending' AND NOT EXISTS (SELECT 1 FROM active_focus WHERE task_instance_id = task_instances.id) AND NOT EXISTS (SELECT 1 FROM focus_sessions WHERE task_instance_id = task_instances.id)"
    } else {
        "INSERT INTO task_instances(id, recurrence_rule_id, rule_version, scheduled_date, scheduled_at, snapshot_title, snapshot_project_id, status, completed_at, source_instance_id, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12) ON CONFLICT(recurrence_rule_id, scheduled_date) DO NOTHING"
    };
    transaction.execute(
        sql,
        params![
            instance.id,
            instance.recurrence_rule_id,
            instance.rule_version,
            instance.scheduled_date,
            instance.scheduled_at,
            instance.snapshot_title,
            instance.snapshot_project_id,
            instance_status_name(instance.status),
            instance.completed_at,
            instance.source_instance_id,
            instance.created_at,
            instance.updated_at,
        ],
    )
}

fn map_instance(row: &rusqlite::Row<'_>) -> rusqlite::Result<TaskInstanceRecord> {
    Ok(TaskInstanceRecord {
        id: row.get(0)?,
        recurrence_rule_id: row.get(1)?,
        rule_version: row.get(2)?,
        scheduled_date: row.get(3)?,
        scheduled_at: row.get(4)?,
        snapshot_title: row.get(5)?,
        snapshot_project_id: row.get(6)?,
        status: map_instance_status(row.get::<_, String>(7)?)?,
        completed_at: row.get(8)?,
        source_instance_id: row.get(9)?,
        created_at: row.get(10)?,
        updated_at: row.get(11)?,
    })
}

fn instance_status_name(status: TaskInstanceStatus) -> &'static str {
    match status {
        TaskInstanceStatus::Pending => "pending",
        TaskInstanceStatus::Completed => "completed",
        TaskInstanceStatus::Skipped => "skipped",
        TaskInstanceStatus::Rescheduled => "rescheduled",
    }
}

fn map_instance_status(status: String) -> rusqlite::Result<TaskInstanceStatus> {
    match status.as_str() {
        "pending" => Ok(TaskInstanceStatus::Pending),
        "completed" => Ok(TaskInstanceStatus::Completed),
        "skipped" => Ok(TaskInstanceStatus::Skipped),
        "rescheduled" => Ok(TaskInstanceStatus::Rescheduled),
        _ => Err(rusqlite::Error::InvalidColumnType(
            7,
            "status".into(),
            rusqlite::types::Type::Text,
        )),
    }
}

fn serialization_error(error: impl std::fmt::Display) -> DomainError {
    DomainError {
        code: "RECURRENCE_DATA_INVALID".into(),
        message: error.to_string(),
        field: None,
    }
}
