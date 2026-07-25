use std::{path::Path, sync::Mutex};

use rusqlite::{Connection, Transaction};

use crate::DomainError;

const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        name: "initial_schema",
        sql: include_str!("../../migrations/0001_initial.sql"),
    },
    Migration {
        version: 2,
        name: "notification_deliveries",
        sql: include_str!("../../migrations/0002_notification_deliveries.sql"),
    },
    Migration {
        version: 3,
        name: "weekly_goal_category",
        sql: include_str!("../../migrations/0003_weekly_goal_category.sql"),
    },
    Migration {
        version: 4,
        name: "memo_center",
        sql: include_str!("../../migrations/0004_memo_center.sql"),
    },
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DatabaseState {
    Ready,
    ReadOnlyProtection { reason: String },
}

#[derive(Debug, Clone, Copy)]
pub struct Migration {
    pub version: i64,
    pub name: &'static str,
    pub sql: &'static str,
}

pub struct Database {
    connection: Mutex<Connection>,
    state: DatabaseState,
}

impl Database {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, DomainError> {
        let mut connection = Connection::open(path).map_err(database_error)?;
        configure_connection(&connection).map_err(database_error)?;
        let state = match run_migrations(&mut connection, MIGRATIONS) {
            Ok(()) => DatabaseState::Ready,
            Err(error) => DatabaseState::ReadOnlyProtection {
                reason: error.to_string(),
            },
        };
        Ok(Self {
            connection: Mutex::new(connection),
            state,
        })
    }

    pub fn open_in_memory() -> Result<Self, DomainError> {
        let mut connection = Connection::open_in_memory().map_err(database_error)?;
        configure_connection(&connection).map_err(database_error)?;
        run_migrations(&mut connection, MIGRATIONS).map_err(database_error)?;
        Ok(Self {
            connection: Mutex::new(connection),
            state: DatabaseState::Ready,
        })
    }

    pub fn state(&self) -> &DatabaseState {
        &self.state
    }

    pub fn read<T>(
        &self,
        operation: impl FnOnce(&Connection) -> rusqlite::Result<T>,
    ) -> Result<T, DomainError> {
        let connection = self.connection.lock().map_err(|_| lock_error())?;
        operation(&connection).map_err(database_error)
    }

    pub fn write<T>(
        &self,
        operation: impl FnOnce(&Transaction<'_>) -> rusqlite::Result<T>,
    ) -> Result<T, DomainError> {
        if let DatabaseState::ReadOnlyProtection { reason } = &self.state {
            return Err(DomainError {
                code: "DATABASE_READ_ONLY".into(),
                message: reason.clone(),
                field: None,
            });
        }
        let mut connection = self.connection.lock().map_err(|_| lock_error())?;
        let transaction = connection.transaction().map_err(database_error)?;
        let value = operation(&transaction).map_err(database_error)?;
        transaction.commit().map_err(database_error)?;
        Ok(value)
    }

    pub fn write_domain<T>(
        &self,
        operation: impl FnOnce(&Transaction<'_>) -> Result<T, DomainError>,
    ) -> Result<T, DomainError> {
        if let DatabaseState::ReadOnlyProtection { reason } = &self.state {
            return Err(DomainError {
                code: "DATABASE_READ_ONLY".into(),
                message: reason.clone(),
                field: None,
            });
        }
        let mut connection = self.connection.lock().map_err(|_| lock_error())?;
        let transaction = connection.transaction().map_err(database_error)?;
        let value = operation(&transaction)?;
        transaction.commit().map_err(database_error)?;
        Ok(value)
    }
}

pub fn configure_connection(connection: &Connection) -> rusqlite::Result<()> {
    connection.pragma_update(None, "foreign_keys", "ON")?;
    connection.busy_timeout(std::time::Duration::from_secs(5))?;
    connection.pragma_update(None, "journal_mode", "WAL")?;
    Ok(())
}

pub fn run_migrations(
    connection: &mut Connection,
    migrations: &[Migration],
) -> rusqlite::Result<()> {
    connection.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            applied_at TEXT NOT NULL
        );",
    )?;
    let transaction = connection.transaction()?;
    for migration in migrations {
        let applied = transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version = ?1)",
            [migration.version],
            |row| row.get::<_, bool>(0),
        )?;
        if applied {
            continue;
        }
        transaction.execute_batch(migration.sql)?;
        transaction.execute(
            "INSERT INTO schema_migrations(version, name, applied_at) VALUES (?1, ?2, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
            (migration.version, migration.name),
        )?;
    }
    transaction.commit()
}

fn database_error(error: rusqlite::Error) -> DomainError {
    DomainError {
        code: "DATABASE_ERROR".into(),
        message: error.to_string(),
        field: None,
    }
}

fn lock_error() -> DomainError {
    DomainError {
        code: "DATABASE_LOCK_FAILED".into(),
        message: "database connection lock is unavailable".into(),
        field: None,
    }
}

#[cfg(test)]
mod tests {
    use rusqlite::params;

    use super::*;

    #[test]
    fn migration_failure_rolls_back_the_whole_batch() {
        let mut connection = Connection::open_in_memory().unwrap();
        let migrations = [
            Migration {
                version: 1,
                name: "valid",
                sql: "CREATE TABLE valid_table(id INTEGER PRIMARY KEY);",
            },
            Migration {
                version: 2,
                name: "invalid",
                sql: "CREATE TABL broken(id INTEGER);",
            },
        ];

        assert!(run_migrations(&mut connection, &migrations).is_err());
        let exists: bool = connection
            .query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'valid_table')", [], |row| row.get(0))
            .unwrap();
        assert!(!exists);
    }

    #[test]
    fn migrations_are_idempotent() {
        let mut connection = Connection::open_in_memory().unwrap();
        configure_connection(&connection).unwrap();

        run_migrations(&mut connection, MIGRATIONS).unwrap();
        run_migrations(&mut connection, MIGRATIONS).unwrap();

        let count: i64 = connection
            .query_row("SELECT COUNT(*) FROM schema_migrations", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(count, 4);
    }

    #[test]
    fn memo_center_migration_preserves_notifications_and_constraints() {
        let mut connection = Connection::open_in_memory().unwrap();
        configure_connection(&connection).unwrap();
        run_migrations(&mut connection, &MIGRATIONS[..3]).unwrap();
        connection.execute("INSERT INTO notification_deliveries(id, kind, source_id, scheduled_for, status, sound_enabled, created_at) VALUES ('delivery', 'taskDue', 'task', '2026-07-23T10:00:00Z', 'sent', 1, '2026-07-23T10:00:00Z')", []).unwrap();

        run_migrations(&mut connection, MIGRATIONS).unwrap();

        let preserved: String = connection
            .query_row(
                "SELECT kind FROM notification_deliveries WHERE id = 'delivery'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(preserved, "taskDue");
        connection.execute("INSERT INTO notification_deliveries(id, kind, source_id, scheduled_for, status, sound_enabled, created_at) VALUES ('memo-delivery', 'memoReminder', 'reminder', '2026-07-23T11:00:00Z', 'pending', 1, '2026-07-23T11:00:00Z')", []).unwrap();

        connection.execute("INSERT INTO memos(id, title, body, created_at, updated_at) VALUES ('memo', 'Title', 'Body', '2026-07-23T10:00:00Z', '2026-07-23T10:00:00Z')", []).unwrap();
        connection.execute("INSERT INTO memo_tags(id, name, normalized_name, created_at) VALUES ('tag', 'Work', 'work', '2026-07-23T10:00:00Z')", []).unwrap();
        connection
            .execute(
                "INSERT INTO memo_tag_links(memo_id, tag_id) VALUES ('memo', 'tag')",
                [],
            )
            .unwrap();
        connection.execute("INSERT INTO memo_reminders(id, memo_id, schedule_kind, frequency, interval_value, local_time, starts_on, timezone, next_scheduled_for, status, created_at, updated_at) VALUES ('reminder', 'memo', 'recurring', 'daily', 1, '09:00', '2026-07-23', 'Asia/Shanghai', '2026-07-24T01:00:00Z', 'active', '2026-07-23T10:00:00Z', '2026-07-23T10:00:00Z')", []).unwrap();

        connection
            .execute("DELETE FROM memos WHERE id = 'memo'", [])
            .unwrap();
        let links: i64 = connection
            .query_row("SELECT COUNT(*) FROM memo_tag_links", [], |row| row.get(0))
            .unwrap();
        let reminders: i64 = connection
            .query_row("SELECT COUNT(*) FROM memo_reminders", [], |row| row.get(0))
            .unwrap();
        assert_eq!((links, reminders), (0, 0));
    }

    #[test]
    fn memo_center_migration_enforces_field_and_identity_constraints() {
        let mut connection = Connection::open_in_memory().unwrap();
        configure_connection(&connection).unwrap();
        run_migrations(&mut connection, MIGRATIONS).unwrap();

        let now = "2026-07-23T10:00:00Z";
        assert!(connection
            .execute(
                "INSERT INTO memos(id, title, body, created_at, updated_at) VALUES ('long-title', ?1, '', ?2, ?2)",
                params!["x".repeat(201), now],
            )
            .is_err());
        assert!(connection
            .execute(
                "INSERT INTO memos(id, title, body, created_at, updated_at) VALUES ('long-body', '', ?1, ?2, ?2)",
                params!["x".repeat(20_001), now],
            )
            .is_err());
        assert!(connection
            .execute(
                "INSERT INTO memo_tags(id, name, normalized_name, created_at) VALUES ('empty-tag', '', 'empty', ?1)",
                [now],
            )
            .is_err());

        connection
            .execute(
                "INSERT INTO memos(id, title, body, created_at, updated_at) VALUES ('memo', '', '', ?1, ?1)",
                [now],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO memo_tags(id, name, normalized_name, created_at) VALUES ('tag', 'Work', 'work', ?1)",
                [now],
            )
            .unwrap();
        assert!(connection
            .execute(
                "INSERT INTO memo_tags(id, name, normalized_name, created_at) VALUES ('duplicate-tag', 'work', 'work', ?1)",
                [now],
            )
            .is_err());

        assert!(connection
            .execute(
                "INSERT INTO memo_reminders(id, memo_id, schedule_kind, interval_value, local_time, starts_on, timezone, status, created_at, updated_at) VALUES ('bad-kind', 'memo', 'later', 1, '09:00', '2026-07-23', 'Asia/Shanghai', 'active', ?1, ?1)",
                [now],
            )
            .is_err());
        assert!(connection
            .execute(
                "INSERT INTO memo_reminders(id, memo_id, schedule_kind, frequency, interval_value, monthly_day, local_time, starts_on, timezone, status, created_at, updated_at) VALUES ('bad-day', 'memo', 'recurring', 'monthly', 1, 32, '09:00', '2026-07-23', 'Asia/Shanghai', 'active', ?1, ?1)",
                [now],
            )
            .is_err());

        connection
            .execute(
                "INSERT INTO memo_reminders(id, memo_id, schedule_kind, frequency, interval_value, local_time, starts_on, timezone, status, created_at, updated_at) VALUES ('reminder', 'memo', 'recurring', 'daily', 1, '09:00', '2026-07-23', 'Asia/Shanghai', 'active', ?1, ?1)",
                [now],
            )
            .unwrap();
        assert!(connection
            .execute(
                "INSERT INTO memo_reminders(id, memo_id, schedule_kind, local_time, starts_on, timezone, status, created_at, updated_at) VALUES ('second-reminder', 'memo', 'once', '10:00', '2026-07-24', 'Asia/Shanghai', 'active', ?1, ?1)",
                [now],
            )
            .is_err());
    }

    #[test]
    fn recurrence_instances_are_unique_per_rule_and_date() {
        let database = Database::open_in_memory().unwrap();
        database.write(|tx| {
            tx.execute("INSERT INTO tasks(id, title, category, priority, status, created_at, updated_at) VALUES ('task', 'Review', 'work', 0, 'pending', '2026-07-18T00:00:00Z', '2026-07-18T00:00:00Z')", [])?;
            tx.execute("INSERT INTO recurrence_rules(id, task_template_id, pattern_json, timezone, starts_on, status, version, created_at, updated_at) VALUES ('rule', 'task', '{}', 'Asia/Shanghai', '2026-07-18', 'active', 1, '2026-07-18T00:00:00Z', '2026-07-18T00:00:00Z')", [])?;
            tx.execute("INSERT INTO task_instances(id, recurrence_rule_id, rule_version, scheduled_date, snapshot_title, status, created_at, updated_at) VALUES ('one', 'rule', 1, '2026-07-18', 'Review', 'pending', '2026-07-18T00:00:00Z', '2026-07-18T00:00:00Z')", [])?;
            Ok(())
        }).unwrap();

        let duplicate = database.write(|tx| {
            tx.execute("INSERT INTO task_instances(id, recurrence_rule_id, rule_version, scheduled_date, snapshot_title, status, created_at, updated_at) VALUES ('two', 'rule', 1, '2026-07-18', 'Review', 'pending', '2026-07-18T00:00:00Z', '2026-07-18T00:00:00Z')", [])
        });
        assert!(duplicate.is_err());
    }

    #[test]
    fn failed_write_operation_is_rolled_back() {
        let database = Database::open_in_memory().unwrap();
        let result = database.write(|tx| {
            tx.execute("INSERT INTO projects(id, name, description, color, icon, status, started_on, created_at, updated_at) VALUES ('project', 'Project', '', 'mint', 'folder', 'active', '2026-07-18', '2026-07-18T00:00:00Z', '2026-07-18T00:00:00Z')", [])?;
            Err::<(), _>(rusqlite::Error::InvalidQuery)
        });
        assert!(result.is_err());

        let count: i64 = database
            .read(|connection| {
                connection.query_row("SELECT COUNT(*) FROM projects", [], |row| row.get(0))
            })
            .unwrap();
        assert_eq!(count, 0);
    }
}
