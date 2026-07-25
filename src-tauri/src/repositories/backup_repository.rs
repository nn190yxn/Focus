use rusqlite::{params, types::Type, OptionalExtension, Transaction};

use super::database::Database;
use crate::{
    domain::backup::{
        BackupActiveFocus, BackupCheckItem, BackupData, BackupFocusSession, BackupMemo,
        BackupMemoReminder, BackupMemoTag, BackupMemoTagLink, BackupNote, BackupPreference,
        BackupProject, BackupRecurrenceRule, BackupTask, BackupTaskInstance, BackupWeeklyGoal,
    },
    DomainError,
};

pub struct BackupRepository<'a> {
    database: &'a Database,
}

#[derive(Debug, Clone)]
pub struct BackupHistoryEntry {
    pub id: String,
    pub kind: String,
    pub path: String,
    pub format_version: u32,
    pub checksum: String,
    pub created_at: String,
}

impl<'a> BackupRepository<'a> {
    pub fn new(database: &'a Database) -> Self {
        Self { database }
    }

    pub fn snapshot(&self) -> Result<BackupData, DomainError> {
        self.database.read(snapshot_from)
    }

    pub fn record_history(&self, entry: &BackupHistoryEntry) -> Result<(), DomainError> {
        self.database
            .write(|transaction| insert_history(transaction, entry))
    }

    pub fn restore_with_snapshot(
        &self,
        data: &BackupData,
        create_snapshot: impl FnOnce(&BackupData) -> Result<BackupHistoryEntry, DomainError>,
    ) -> Result<BackupHistoryEntry, DomainError> {
        self.database.write_domain(|transaction| {
            let current = snapshot_from(transaction).map_err(restore_sql_error)?;
            let history = create_snapshot(&current)?;
            insert_history(transaction, &history).map_err(restore_sql_error)?;
            replace_business_data(transaction, data)?;
            Ok(history)
        })
    }
}

pub fn snapshot_from(connection: &rusqlite::Connection) -> rusqlite::Result<BackupData> {
    Ok(BackupData {
        projects: query_projects(connection)?,
        tasks: query_tasks(connection)?,
        check_items: query_check_items(connection)?,
        recurrence_rules: query_recurrence_rules(connection)?,
        task_instances: query_task_instances(connection)?,
        focus_sessions: query_focus_sessions(connection)?,
        active_focus: query_active_focus(connection)?,
        notes: query_notes(connection)?,
        weekly_goals: query_weekly_goals(connection)?,
        preferences: query_preferences(connection)?,
        memos: query_memos(connection)?,
        memo_tags: query_memo_tags(connection)?,
        memo_tag_links: query_memo_tag_links(connection)?,
        memo_reminders: query_memo_reminders(connection)?,
    })
}

fn insert_history(
    transaction: &Transaction<'_>,
    entry: &BackupHistoryEntry,
) -> rusqlite::Result<()> {
    transaction.execute(
        "INSERT INTO backup_history(id, kind, path, format_version, checksum, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![entry.id, entry.kind, entry.path, entry.format_version, entry.checksum, entry.created_at],
    )?;
    Ok(())
}

fn replace_business_data(
    transaction: &Transaction<'_>,
    data: &BackupData,
) -> Result<(), DomainError> {
    transaction
        .execute_batch(
            "DELETE FROM notification_deliveries;
             DELETE FROM memo_tag_links;
             DELETE FROM memo_reminders;
             DELETE FROM memos;
             DELETE FROM memo_tags;
             DELETE FROM active_focus;
             DELETE FROM focus_sessions;
             DELETE FROM task_check_items;
             DELETE FROM task_instances;
             DELETE FROM recurrence_rules;
             DELETE FROM tasks;
             DELETE FROM projects;
             DELETE FROM notes;
             DELETE FROM weekly_goals;
             DELETE FROM preferences;",
        )
        .map_err(restore_sql_error)?;

    for item in &data.projects {
        transaction.execute(
            "INSERT INTO projects(id, name, description, color, icon, status, started_on, target_on, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![item.id, item.name, item.description, item.color, item.icon, item.status, item.started_on, item.target_on, item.created_at, item.updated_at],
        ).map_err(restore_sql_error)?;
    }
    for item in &data.tasks {
        transaction.execute(
            "INSERT INTO tasks(id, project_id, title, category, priority, scheduled_date, scheduled_time, status, completed_at, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![item.id, item.project_id, item.title, item.category, item.priority, item.scheduled_date, item.scheduled_time, item.status, item.completed_at, item.created_at, item.updated_at],
        ).map_err(restore_sql_error)?;
    }
    for item in &data.check_items {
        transaction.execute(
            "INSERT INTO task_check_items(id, task_id, title, position, completed_at, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![item.id, item.task_id, item.title, item.position, item.completed_at, item.created_at, item.updated_at],
        ).map_err(restore_sql_error)?;
    }
    for item in &data.recurrence_rules {
        let pattern = serde_json::to_string(&item.pattern).map_err(restore_serialization_error)?;
        transaction.execute(
            "INSERT INTO recurrence_rules(id, task_template_id, pattern_json, local_time, timezone, starts_on, ends_on, status, version, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![item.id, item.task_template_id, pattern, item.local_time, item.timezone, item.starts_on, item.ends_on, item.status, item.version, item.created_at, item.updated_at],
        ).map_err(restore_sql_error)?;
    }
    for item in &data.task_instances {
        transaction.execute(
            "INSERT INTO task_instances(id, recurrence_rule_id, rule_version, scheduled_date, scheduled_at, snapshot_title, snapshot_project_id, status, completed_at, source_instance_id, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, NULL, ?10, ?11)",
            params![item.id, item.recurrence_rule_id, item.rule_version, item.scheduled_date, item.scheduled_at, item.snapshot_title, item.snapshot_project_id, item.status, item.completed_at, item.created_at, item.updated_at],
        ).map_err(restore_sql_error)?;
    }
    for item in &data.task_instances {
        if let Some(source_instance_id) = &item.source_instance_id {
            transaction
                .execute(
                    "UPDATE task_instances SET source_instance_id = ?1 WHERE id = ?2",
                    params![source_instance_id, item.id],
                )
                .map_err(restore_sql_error)?;
        }
    }
    for item in &data.focus_sessions {
        transaction.execute(
            "INSERT INTO focus_sessions(id, task_id, task_instance_id, project_id, planned_seconds, actual_seconds, interruption_count, completion_kind, started_at, ended_at, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![item.id, item.task_id, item.task_instance_id, item.project_id, item.planned_seconds, item.actual_seconds, item.interruption_count, item.completion_kind, item.started_at, item.ended_at, item.created_at],
        ).map_err(restore_sql_error)?;
    }
    if let Some(item) = &data.active_focus {
        transaction.execute(
            "INSERT INTO active_focus(singleton_id, task_id, task_instance_id, state, planned_seconds, remaining_seconds, started_at, target_ends_at, paused_at, interruption_count, updated_at) VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![item.task_id, item.task_instance_id, item.state, item.planned_seconds, item.remaining_seconds, item.started_at, item.target_ends_at, item.paused_at, item.interruption_count, item.updated_at],
        ).map_err(restore_sql_error)?;
    }
    for item in &data.notes {
        transaction.execute(
            "INSERT INTO notes(id, body, note_date, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![item.id, item.body, item.note_date, item.created_at, item.updated_at],
        ).map_err(restore_sql_error)?;
    }
    for item in &data.weekly_goals {
        transaction.execute(
            "INSERT INTO weekly_goals(id, week_starts_on, title, category, target_count, completed_count, position, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![item.id, item.week_starts_on, item.title, item.category, item.target_count, item.completed_count, item.position, item.created_at, item.updated_at],
        ).map_err(restore_sql_error)?;
    }
    for item in &data.preferences {
        let value = serde_json::to_string(&item.value).map_err(restore_serialization_error)?;
        transaction
            .execute(
                "INSERT INTO preferences(key, value_json, updated_at) VALUES (?1, ?2, ?3)",
                params![item.key, value, item.updated_at],
            )
            .map_err(restore_sql_error)?;
    }
    for item in &data.memos {
        transaction
            .execute(
                "INSERT INTO memos(id, title, body, pinned_at, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![item.id, item.title, item.body, item.pinned_at, item.created_at, item.updated_at],
            )
            .map_err(restore_sql_error)?;
    }
    for item in &data.memo_tags {
        transaction
            .execute(
                "INSERT INTO memo_tags(id, name, normalized_name, created_at) VALUES (?1, ?2, ?3, ?4)",
                params![item.id, item.name, item.normalized_name, item.created_at],
            )
            .map_err(restore_sql_error)?;
    }
    for item in &data.memo_tag_links {
        transaction
            .execute(
                "INSERT INTO memo_tag_links(memo_id, tag_id) VALUES (?1, ?2)",
                params![item.memo_id, item.tag_id],
            )
            .map_err(restore_sql_error)?;
    }
    for item in &data.memo_reminders {
        let weekdays = (item.schedule_kind == "recurring")
            .then(|| serde_json::to_string(&item.weekdays))
            .transpose()
            .map_err(restore_serialization_error)?;
        transaction
            .execute(
                "INSERT INTO memo_reminders(id, memo_id, schedule_kind, frequency, interval_value, weekdays_json, monthly_day, local_time, starts_on, ends_on, timezone, next_scheduled_for, status, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
                params![item.id, item.memo_id, item.schedule_kind, item.frequency, item.interval, weekdays, item.monthly_day, item.local_time, item.starts_on, item.ends_on, item.timezone, item.next_scheduled_for, item.status, item.created_at, item.updated_at],
            )
            .map_err(restore_sql_error)?;
    }

    let violations: i64 = transaction
        .query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |row| {
            row.get(0)
        })
        .map_err(restore_sql_error)?;
    if violations > 0 {
        return Err(DomainError {
            code: "BACKUP_RESTORE_FAILED".into(),
            message: "restored data violates database references".into(),
            field: None,
        });
    }
    Ok(())
}

fn restore_sql_error(error: rusqlite::Error) -> DomainError {
    DomainError {
        code: "BACKUP_RESTORE_FAILED".into(),
        message: error.to_string(),
        field: None,
    }
}

fn restore_serialization_error(error: serde_json::Error) -> DomainError {
    DomainError {
        code: "BACKUP_RESTORE_FAILED".into(),
        message: error.to_string(),
        field: None,
    }
}

fn query_projects(connection: &rusqlite::Connection) -> rusqlite::Result<Vec<BackupProject>> {
    let mut statement = connection.prepare(
        "SELECT id, name, description, color, icon, status, started_on, target_on, created_at, updated_at FROM projects ORDER BY id",
    )?;
    let rows = statement.query_map([], |row| {
        Ok(BackupProject {
            id: row.get(0)?,
            name: row.get(1)?,
            description: row.get(2)?,
            color: row.get(3)?,
            icon: row.get(4)?,
            status: row.get(5)?,
            started_on: row.get(6)?,
            target_on: row.get(7)?,
            created_at: row.get(8)?,
            updated_at: row.get(9)?,
        })
    })?;
    rows.collect()
}

fn query_tasks(connection: &rusqlite::Connection) -> rusqlite::Result<Vec<BackupTask>> {
    let mut statement = connection.prepare(
        "SELECT id, project_id, title, category, priority, scheduled_date, scheduled_time, status, completed_at, created_at, updated_at FROM tasks ORDER BY id",
    )?;
    let rows = statement.query_map([], |row| {
        Ok(BackupTask {
            id: row.get(0)?,
            project_id: row.get(1)?,
            title: row.get(2)?,
            category: row.get(3)?,
            priority: row.get(4)?,
            scheduled_date: row.get(5)?,
            scheduled_time: row.get(6)?,
            status: row.get(7)?,
            completed_at: row.get(8)?,
            created_at: row.get(9)?,
            updated_at: row.get(10)?,
        })
    })?;
    rows.collect()
}

fn query_check_items(connection: &rusqlite::Connection) -> rusqlite::Result<Vec<BackupCheckItem>> {
    let mut statement = connection.prepare(
        "SELECT id, task_id, title, position, completed_at, created_at, updated_at FROM task_check_items ORDER BY task_id, position, id",
    )?;
    let rows = statement.query_map([], |row| {
        Ok(BackupCheckItem {
            id: row.get(0)?,
            task_id: row.get(1)?,
            title: row.get(2)?,
            position: row.get(3)?,
            completed_at: row.get(4)?,
            created_at: row.get(5)?,
            updated_at: row.get(6)?,
        })
    })?;
    rows.collect()
}

fn query_recurrence_rules(
    connection: &rusqlite::Connection,
) -> rusqlite::Result<Vec<BackupRecurrenceRule>> {
    let mut statement = connection.prepare(
        "SELECT id, task_template_id, pattern_json, local_time, timezone, starts_on, ends_on, status, version, created_at, updated_at FROM recurrence_rules ORDER BY id",
    )?;
    let rows = statement.query_map([], |row| {
        let raw_pattern: String = row.get(2)?;
        Ok(BackupRecurrenceRule {
            id: row.get(0)?,
            task_template_id: row.get(1)?,
            pattern: parse_json_column(&raw_pattern, 2)?,
            local_time: row.get(3)?,
            timezone: row.get(4)?,
            starts_on: row.get(5)?,
            ends_on: row.get(6)?,
            status: row.get(7)?,
            version: row.get(8)?,
            created_at: row.get(9)?,
            updated_at: row.get(10)?,
        })
    })?;
    rows.collect()
}

fn query_task_instances(
    connection: &rusqlite::Connection,
) -> rusqlite::Result<Vec<BackupTaskInstance>> {
    let mut statement = connection.prepare(
        "SELECT id, recurrence_rule_id, rule_version, scheduled_date, scheduled_at, snapshot_title, snapshot_project_id, status, completed_at, source_instance_id, created_at, updated_at FROM task_instances ORDER BY recurrence_rule_id, scheduled_date, id",
    )?;
    let rows = statement.query_map([], |row| {
        Ok(BackupTaskInstance {
            id: row.get(0)?,
            recurrence_rule_id: row.get(1)?,
            rule_version: row.get(2)?,
            scheduled_date: row.get(3)?,
            scheduled_at: row.get(4)?,
            snapshot_title: row.get(5)?,
            snapshot_project_id: row.get(6)?,
            status: row.get(7)?,
            completed_at: row.get(8)?,
            source_instance_id: row.get(9)?,
            created_at: row.get(10)?,
            updated_at: row.get(11)?,
        })
    })?;
    rows.collect()
}

fn query_focus_sessions(
    connection: &rusqlite::Connection,
) -> rusqlite::Result<Vec<BackupFocusSession>> {
    let mut statement = connection.prepare(
        "SELECT id, task_id, task_instance_id, project_id, planned_seconds, actual_seconds, interruption_count, completion_kind, started_at, ended_at, created_at FROM focus_sessions ORDER BY started_at, id",
    )?;
    let rows = statement.query_map([], |row| {
        Ok(BackupFocusSession {
            id: row.get(0)?,
            task_id: row.get(1)?,
            task_instance_id: row.get(2)?,
            project_id: row.get(3)?,
            planned_seconds: row.get(4)?,
            actual_seconds: row.get(5)?,
            interruption_count: row.get(6)?,
            completion_kind: row.get(7)?,
            started_at: row.get(8)?,
            ended_at: row.get(9)?,
            created_at: row.get(10)?,
        })
    })?;
    rows.collect()
}

fn query_active_focus(
    connection: &rusqlite::Connection,
) -> rusqlite::Result<Option<BackupActiveFocus>> {
    connection
        .query_row(
            "SELECT task_id, task_instance_id, state, planned_seconds, remaining_seconds, started_at, target_ends_at, paused_at, interruption_count, updated_at FROM active_focus WHERE singleton_id = 1",
            [],
            |row| {
                Ok(BackupActiveFocus {
                    task_id: row.get(0)?,
                    task_instance_id: row.get(1)?,
                    state: row.get(2)?,
                    planned_seconds: row.get(3)?,
                    remaining_seconds: row.get(4)?,
                    started_at: row.get(5)?,
                    target_ends_at: row.get(6)?,
                    paused_at: row.get(7)?,
                    interruption_count: row.get(8)?,
                    updated_at: row.get(9)?,
                })
            },
        )
        .optional()
}

fn query_notes(connection: &rusqlite::Connection) -> rusqlite::Result<Vec<BackupNote>> {
    let mut statement = connection.prepare(
        "SELECT id, body, note_date, created_at, updated_at FROM notes ORDER BY note_date, id",
    )?;
    let rows = statement.query_map([], |row| {
        Ok(BackupNote {
            id: row.get(0)?,
            body: row.get(1)?,
            note_date: row.get(2)?,
            created_at: row.get(3)?,
            updated_at: row.get(4)?,
        })
    })?;
    rows.collect()
}

fn query_weekly_goals(
    connection: &rusqlite::Connection,
) -> rusqlite::Result<Vec<BackupWeeklyGoal>> {
    let mut statement = connection.prepare(
        "SELECT id, week_starts_on, title, category, target_count, completed_count, position, created_at, updated_at FROM weekly_goals ORDER BY week_starts_on, position, id",
    )?;
    let rows = statement.query_map([], |row| {
        Ok(BackupWeeklyGoal {
            id: row.get(0)?,
            week_starts_on: row.get(1)?,
            title: row.get(2)?,
            category: row.get(3)?,
            target_count: row.get(4)?,
            completed_count: row.get(5)?,
            position: row.get(6)?,
            created_at: row.get(7)?,
            updated_at: row.get(8)?,
        })
    })?;
    rows.collect()
}

fn query_preferences(connection: &rusqlite::Connection) -> rusqlite::Result<Vec<BackupPreference>> {
    let mut statement =
        connection.prepare("SELECT key, value_json, updated_at FROM preferences ORDER BY key")?;
    let rows = statement.query_map([], |row| {
        let raw_value: String = row.get(1)?;
        Ok(BackupPreference {
            key: row.get(0)?,
            value: parse_json_column(&raw_value, 1)?,
            updated_at: row.get(2)?,
        })
    })?;
    rows.collect()
}

fn query_memos(connection: &rusqlite::Connection) -> rusqlite::Result<Vec<BackupMemo>> {
    let mut statement = connection.prepare(
        "SELECT id, title, body, pinned_at, created_at, updated_at FROM memos ORDER BY id",
    )?;
    let rows = statement.query_map([], |row| {
        Ok(BackupMemo {
            id: row.get(0)?,
            title: row.get(1)?,
            body: row.get(2)?,
            pinned_at: row.get(3)?,
            created_at: row.get(4)?,
            updated_at: row.get(5)?,
        })
    })?;
    rows.collect()
}

fn query_memo_tags(connection: &rusqlite::Connection) -> rusqlite::Result<Vec<BackupMemoTag>> {
    let mut statement = connection
        .prepare("SELECT id, name, normalized_name, created_at FROM memo_tags ORDER BY id")?;
    let rows = statement.query_map([], |row| {
        Ok(BackupMemoTag {
            id: row.get(0)?,
            name: row.get(1)?,
            normalized_name: row.get(2)?,
            created_at: row.get(3)?,
        })
    })?;
    rows.collect()
}

fn query_memo_tag_links(
    connection: &rusqlite::Connection,
) -> rusqlite::Result<Vec<BackupMemoTagLink>> {
    let mut statement = connection
        .prepare("SELECT memo_id, tag_id FROM memo_tag_links ORDER BY memo_id, tag_id")?;
    let rows = statement.query_map([], |row| {
        Ok(BackupMemoTagLink {
            memo_id: row.get(0)?,
            tag_id: row.get(1)?,
        })
    })?;
    rows.collect()
}

fn query_memo_reminders(
    connection: &rusqlite::Connection,
) -> rusqlite::Result<Vec<BackupMemoReminder>> {
    let mut statement = connection.prepare(
        "SELECT id, memo_id, schedule_kind, frequency, interval_value, weekdays_json,
            monthly_day, local_time, starts_on, ends_on, timezone, next_scheduled_for,
            status, created_at, updated_at
         FROM memo_reminders ORDER BY id",
    )?;
    let rows = statement.query_map([], |row| {
        let weekdays = row
            .get::<_, Option<String>>(5)?
            .map(|value| parse_json_column(&value, 5))
            .transpose()?
            .unwrap_or_default();
        Ok(BackupMemoReminder {
            id: row.get(0)?,
            memo_id: row.get(1)?,
            schedule_kind: row.get(2)?,
            frequency: row.get(3)?,
            interval: row.get(4)?,
            weekdays,
            monthly_day: row.get(6)?,
            local_time: row.get(7)?,
            starts_on: row.get(8)?,
            ends_on: row.get(9)?,
            timezone: row.get(10)?,
            next_scheduled_for: row.get(11)?,
            status: row.get(12)?,
            created_at: row.get(13)?,
            updated_at: row.get(14)?,
        })
    })?;
    rows.collect()
}

fn parse_json_column<T: serde::de::DeserializeOwned>(
    value: &str,
    column: usize,
) -> rusqlite::Result<T> {
    serde_json::from_str(value).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(column, Type::Text, Box::new(error))
    })
}
