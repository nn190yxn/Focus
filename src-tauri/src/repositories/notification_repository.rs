use chrono::{DateTime, Duration, Utc};
use rusqlite::params;

use super::database::Database;
use crate::{
    domain::notification::{NotificationCandidate, NotificationKind, ReminderWindow},
    DomainError,
};

pub struct NotificationRepository<'a> {
    database: &'a Database,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationReservation {
    Acquired,
    InFlight,
    AlreadySent,
}

const PENDING_LEASE_SECONDS: i64 = 60;

impl<'a> NotificationRepository<'a> {
    pub fn new(database: &'a Database) -> Self {
        Self { database }
    }

    pub fn list_due_tasks(
        &self,
        window: ReminderWindow,
    ) -> Result<Vec<NotificationCandidate>, DomainError> {
        let local_start = window.starts_at.format("%Y-%m-%dT%H:%M:%S").to_string();
        let local_end = window.ends_at.format("%Y-%m-%dT%H:%M:%S").to_string();
        let absolute_start = window.starts_at.to_rfc3339();
        let absolute_end = window.ends_at.to_rfc3339();
        self.database.read(|connection| {
            let mut candidates = Vec::new();
            let mut task_statement = connection.prepare(
                "SELECT t.id, t.title, t.scheduled_date || 'T' || t.scheduled_time || ':00'
                 FROM tasks t
                 WHERE t.status = 'pending'
                   AND t.scheduled_date IS NOT NULL
                   AND t.scheduled_time IS NOT NULL
                   AND NOT EXISTS (SELECT 1 FROM recurrence_rules r WHERE r.task_template_id = t.id)
                   AND datetime(t.scheduled_date || 'T' || t.scheduled_time || ':00') > datetime(?1)
                   AND datetime(t.scheduled_date || 'T' || t.scheduled_time || ':00') <= datetime(?2)",
            )?;
            let tasks = task_statement.query_map(params![local_start, local_end], |row| {
                Ok(NotificationCandidate {
                    kind: NotificationKind::TaskDue,
                    source_id: row.get(0)?,
                    title: row.get(1)?,
                    scheduled_for: row.get(2)?,
                })
            })?;
            for task in tasks {
                candidates.push(task?);
            }

            let mut instance_statement = connection.prepare(
                "SELECT id, snapshot_title, scheduled_at
                 FROM task_instances
                 WHERE status = 'pending'
                   AND scheduled_at IS NOT NULL
                   AND julianday(scheduled_at) > julianday(?1)
                   AND julianday(scheduled_at) <= julianday(?2)",
            )?;
            let instances = instance_statement.query_map(
                params![absolute_start, absolute_end],
                |row| {
                    Ok(NotificationCandidate {
                        kind: NotificationKind::RecurringTaskDue,
                        source_id: row.get(0)?,
                        title: row.get(1)?,
                        scheduled_for: row.get(2)?,
                    })
                },
            )?;
            for instance in instances {
                candidates.push(instance?);
            }
            Ok(candidates)
        })
    }

    pub fn reserve(
        &self,
        kind: NotificationKind,
        source_id: &str,
        scheduled_for: &str,
        sound_enabled: bool,
    ) -> Result<NotificationReservation, DomainError> {
        self.reserve_at(kind, source_id, scheduled_for, sound_enabled, Utc::now())
    }

    fn reserve_at(
        &self,
        kind: NotificationKind,
        source_id: &str,
        scheduled_for: &str,
        sound_enabled: bool,
        now: DateTime<Utc>,
    ) -> Result<NotificationReservation, DomainError> {
        let lease_expires_before = now - Duration::seconds(PENDING_LEASE_SECONDS);
        self.database.write(|tx| {
            let acquired = tx.execute(
                "INSERT INTO notification_deliveries(id, kind, source_id, scheduled_for, status, sound_enabled, created_at)
                 VALUES (?1, ?2, ?3, ?4, 'pending', ?5, ?6)
                 ON CONFLICT(kind, source_id, scheduled_for) DO UPDATE SET
                     status = 'pending',
                     sound_enabled = excluded.sound_enabled,
                     created_at = excluded.created_at,
                     sent_at = NULL,
                     last_error = NULL
                 WHERE notification_deliveries.status = 'failed'
                    OR (notification_deliveries.status = 'pending'
                        AND julianday(notification_deliveries.created_at) <= julianday(?7))",
                params![
                    uuid::Uuid::new_v4().to_string(),
                    kind.as_str(),
                    source_id,
                    scheduled_for,
                    sound_enabled,
                    now.to_rfc3339(),
                    lease_expires_before.to_rfc3339(),
                ],
            )? == 1;
            if acquired {
                return Ok(NotificationReservation::Acquired);
            }
            let status: String = tx.query_row(
                "SELECT status FROM notification_deliveries
                 WHERE kind = ?1 AND source_id = ?2 AND scheduled_for = ?3",
                params![kind.as_str(), source_id, scheduled_for],
                |row| row.get(0),
            )?;
            Ok(match status.as_str() {
                "sent" => NotificationReservation::AlreadySent,
                _ => NotificationReservation::InFlight,
            })
        })
    }

    pub fn mark_sent(
        &self,
        kind: NotificationKind,
        source_id: &str,
        scheduled_for: &str,
    ) -> Result<(), DomainError> {
        self.update_status(kind, source_id, scheduled_for, "sent", None)
    }

    pub fn mark_failed(
        &self,
        kind: NotificationKind,
        source_id: &str,
        scheduled_for: &str,
        error_code: &str,
    ) -> Result<(), DomainError> {
        self.update_status(kind, source_id, scheduled_for, "failed", Some(error_code))
    }

    fn update_status(
        &self,
        kind: NotificationKind,
        source_id: &str,
        scheduled_for: &str,
        status: &str,
        last_error: Option<&str>,
    ) -> Result<(), DomainError> {
        self.database.write(|tx| {
            tx.execute(
                "UPDATE notification_deliveries
                 SET status = ?4, sent_at = CASE WHEN ?4 = 'sent' THEN ?5 ELSE NULL END, last_error = ?6
                 WHERE kind = ?1 AND source_id = ?2 AND scheduled_for = ?3",
                params![
                    kind.as_str(),
                    source_id,
                    scheduled_for,
                    status,
                    Utc::now().to_rfc3339(),
                    last_error,
                ],
            )?;
            Ok(())
        })
    }

    #[cfg(test)]
    pub fn count_deliveries(&self) -> Result<i64, DomainError> {
        self.database.read(|connection| {
            connection.query_row("SELECT COUNT(*) FROM notification_deliveries", [], |row| {
                row.get(0)
            })
        })
    }

    #[cfg(test)]
    pub fn delivery_status(&self) -> Result<Option<(String, Option<String>)>, DomainError> {
        use rusqlite::OptionalExtension;
        self.database.read(|connection| {
            connection
                .query_row(
                    "SELECT status, last_error FROM notification_deliveries LIMIT 1",
                    [],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .optional()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, FixedOffset, TimeZone};

    fn at(hour: u32, minute: u32) -> DateTime<FixedOffset> {
        FixedOffset::east_opt(8 * 3600)
            .unwrap()
            .with_ymd_and_hms(2026, 7, 20, hour, minute, 0)
            .unwrap()
    }

    #[test]
    fn delivery_reservation_is_unique() {
        let database = Database::open_in_memory().unwrap();
        let repository = NotificationRepository::new(&database);
        assert_eq!(
            repository
                .reserve(
                    NotificationKind::TaskDue,
                    "task",
                    "2026-07-20T10:00:00",
                    true
                )
                .unwrap(),
            NotificationReservation::Acquired
        );
        assert_eq!(
            repository
                .reserve(
                    NotificationKind::TaskDue,
                    "task",
                    "2026-07-20T10:00:00",
                    true
                )
                .unwrap(),
            NotificationReservation::InFlight
        );
        assert_eq!(repository.count_deliveries().unwrap(), 1);
    }

    #[test]
    fn failed_delivery_can_be_reserved_for_retry() {
        let database = Database::open_in_memory().unwrap();
        let repository = NotificationRepository::new(&database);
        assert_eq!(
            repository
                .reserve(
                    NotificationKind::TaskDue,
                    "task",
                    "2026-07-20T10:00:00",
                    true
                )
                .unwrap(),
            NotificationReservation::Acquired
        );
        repository
            .mark_failed(
                NotificationKind::TaskDue,
                "task",
                "2026-07-20T10:00:00",
                "NOTIFICATION_SEND_FAILED",
            )
            .unwrap();

        assert_eq!(
            repository
                .reserve(
                    NotificationKind::TaskDue,
                    "task",
                    "2026-07-20T10:00:00",
                    false
                )
                .unwrap(),
            NotificationReservation::Acquired
        );
        assert_eq!(
            repository
                .reserve(
                    NotificationKind::TaskDue,
                    "task",
                    "2026-07-20T10:00:00",
                    false
                )
                .unwrap(),
            NotificationReservation::InFlight
        );
        assert_eq!(repository.count_deliveries().unwrap(), 1);
        assert_eq!(
            repository.delivery_status().unwrap(),
            Some(("pending".into(), None))
        );
    }

    #[test]
    fn expired_pending_delivery_can_be_reclaimed() {
        let database = Database::open_in_memory().unwrap();
        let repository = NotificationRepository::new(&database);
        let reserved_at = Utc.with_ymd_and_hms(2026, 7, 20, 2, 0, 0).unwrap();
        let reserve = |now| {
            repository
                .reserve_at(
                    NotificationKind::TaskDue,
                    "task",
                    "2026-07-20T10:00:00",
                    true,
                    now,
                )
                .unwrap()
        };

        assert_eq!(reserve(reserved_at), NotificationReservation::Acquired);
        assert_eq!(
            reserve(reserved_at + Duration::seconds(PENDING_LEASE_SECONDS - 1)),
            NotificationReservation::InFlight
        );
        assert_eq!(
            reserve(reserved_at + Duration::seconds(PENDING_LEASE_SECONDS)),
            NotificationReservation::Acquired
        );
        repository
            .mark_sent(NotificationKind::TaskDue, "task", "2026-07-20T10:00:00")
            .unwrap();
        assert_eq!(
            reserve(reserved_at + Duration::minutes(10)),
            NotificationReservation::AlreadySent
        );
        assert_eq!(repository.count_deliveries().unwrap(), 1);
    }

    #[test]
    fn due_query_excludes_templates_and_completed_items() {
        let database = Database::open_in_memory().unwrap();
        database.write(|tx| {
            let stamp = "2026-07-20T00:00:00Z";
            tx.execute("INSERT INTO tasks(id, title, category, priority, scheduled_date, scheduled_time, status, created_at, updated_at) VALUES ('due', 'Due task', 'work', 0, '2026-07-20', '10:00', 'pending', ?1, ?1)", [stamp])?;
            tx.execute("INSERT INTO tasks(id, title, category, priority, scheduled_date, scheduled_time, status, completed_at, created_at, updated_at) VALUES ('done', 'Done task', 'work', 0, '2026-07-20', '10:00', 'completed', ?1, ?1, ?1)", [stamp])?;
            tx.execute("INSERT INTO tasks(id, title, category, priority, scheduled_date, scheduled_time, status, created_at, updated_at) VALUES ('template', 'Template', 'work', 0, '2026-07-20', '10:00', 'pending', ?1, ?1)", [stamp])?;
            tx.execute("INSERT INTO recurrence_rules(id, task_template_id, pattern_json, local_time, timezone, starts_on, status, version, created_at, updated_at) VALUES ('rule', 'template', '{}', '10:00', 'Asia/Shanghai', '2026-07-20', 'active', 1, ?1, ?1)", [stamp])?;
            tx.execute("INSERT INTO task_instances(id, recurrence_rule_id, rule_version, scheduled_date, scheduled_at, snapshot_title, status, created_at, updated_at) VALUES ('instance', 'rule', 1, '2026-07-20', '2026-07-20T10:00:00+08:00', 'Recurring due', 'pending', ?1, ?1)", [stamp])?;
            Ok(())
        }).unwrap();

        let candidates = NotificationRepository::new(&database)
            .list_due_tasks(ReminderWindow {
                starts_at: at(9, 55),
                ends_at: at(10, 0),
            })
            .unwrap();
        assert_eq!(candidates.len(), 2);
        assert!(candidates.iter().any(|item| item.source_id == "due"));
        assert!(candidates.iter().any(|item| item.source_id == "instance"));
    }
}
