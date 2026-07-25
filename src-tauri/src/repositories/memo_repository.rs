use chrono::{DateTime, Utc};
use rusqlite::{params, OptionalExtension, Transaction};
use uuid::Uuid;

use crate::{
    domain::memo::{
        DueMemoReminder, MemoListQuery, MemoRecord, MemoReminderFrequency, MemoReminderInput,
        MemoReminderRule, MemoReminderStatus, MemoSummary, MemoTag, MemoTagSummary,
    },
    repositories::database::Database,
    services::memo_service::{MemoCoreRecord, MemoService, NormalizedMemoTag},
    DomainError,
};

pub struct MemoRepository<'a> {
    database: &'a Database,
}

impl<'a> MemoRepository<'a> {
    pub fn new(database: &'a Database) -> Self {
        Self { database }
    }

    pub fn replace_tags(
        &self,
        memo_id: &str,
        tags: &[NormalizedMemoTag],
    ) -> Result<Vec<MemoTag>, DomainError> {
        let now = Utc::now().to_rfc3339();
        self.database.write(|transaction| {
            let memo_exists = transaction.query_row(
                "SELECT EXISTS(SELECT 1 FROM memos WHERE id = ?1)",
                [memo_id],
                |row| row.get::<_, bool>(0),
            )?;
            if !memo_exists {
                return Err(rusqlite::Error::QueryReturnedNoRows);
            }
            replace_tags_in_transaction(transaction, memo_id, tags, &now)
        })
    }

    pub fn create(
        &self,
        memo: &MemoCoreRecord,
        tags: &[NormalizedMemoTag],
        reminder: Option<&MemoReminderRule>,
        untitled_label: &str,
    ) -> Result<MemoRecord, DomainError> {
        self.database.write_domain(|transaction| {
            transaction
                .execute(
                    "INSERT INTO memos(id, title, body, pinned_at, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![
                        memo.id,
                        memo.title,
                        memo.body,
                        memo.pinned_at,
                        memo.created_at,
                        memo.updated_at,
                    ],
                )
                .map_err(memo_save_error)?;
            replace_tags_in_transaction(transaction, &memo.id, tags, &memo.created_at)
                .map_err(memo_save_error)?;
            replace_reminder_in_transaction(transaction, &memo.id, reminder)
                .map_err(memo_save_error)?;
            Ok(())
        })?;
        self.get(&memo.id, untitled_label)?
            .ok_or_else(memo_not_found_error)
    }

    pub fn update(
        &self,
        memo: &MemoCoreRecord,
        tags: &[NormalizedMemoTag],
        reminder: Option<&MemoReminderRule>,
        untitled_label: &str,
    ) -> Result<MemoRecord, DomainError> {
        self.database.write_domain(|transaction| {
            let changed = transaction
                .execute(
                    "UPDATE memos SET title = ?2, body = ?3, pinned_at = ?4, updated_at = ?5
                     WHERE id = ?1",
                    params![
                        memo.id,
                        memo.title,
                        memo.body,
                        memo.pinned_at,
                        memo.updated_at,
                    ],
                )
                .map_err(memo_save_error)?;
            if changed == 0 {
                return Err(memo_not_found_error());
            }
            replace_tags_in_transaction(transaction, &memo.id, tags, &memo.updated_at)
                .map_err(memo_save_error)?;
            replace_reminder_in_transaction(transaction, &memo.id, reminder)
                .map_err(memo_save_error)?;
            Ok(())
        })?;
        self.get(&memo.id, untitled_label)?
            .ok_or_else(memo_not_found_error)
    }

    pub fn get(
        &self,
        memo_id: &str,
        untitled_label: &str,
    ) -> Result<Option<MemoRecord>, DomainError> {
        let stored = self.database.read(|connection| {
            let core = connection
                .query_row(
                    "SELECT id, title, body, pinned_at, created_at, updated_at
                     FROM memos WHERE id = ?1",
                    [memo_id],
                    |row| {
                        Ok(MemoCoreRecord {
                            id: row.get(0)?,
                            title: row.get(1)?,
                            body: row.get(2)?,
                            pinned_at: row.get(3)?,
                            created_at: row.get(4)?,
                            updated_at: row.get(5)?,
                        })
                    },
                )
                .optional()?;
            let Some(core) = core else {
                return Ok(None);
            };
            let mut tag_statement = connection.prepare(
                "SELECT memo_tags.id, memo_tags.name
                 FROM memo_tags JOIN memo_tag_links ON memo_tag_links.tag_id = memo_tags.id
                 WHERE memo_tag_links.memo_id = ?1
                 ORDER BY memo_tags.normalized_name, memo_tags.id",
            )?;
            let tags = tag_statement
                .query_map([memo_id], |row| {
                    Ok(MemoTag {
                        id: row.get(0)?,
                        name: row.get(1)?,
                    })
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            let reminder = connection
                .query_row(
                    "SELECT id, memo_id, schedule_kind, frequency, interval_value, weekdays_json,
                        monthly_day, local_time, starts_on, ends_on, timezone, next_scheduled_for,
                        status, created_at, updated_at
                     FROM memo_reminders WHERE memo_id = ?1",
                    [memo_id],
                    map_stored_reminder,
                )
                .optional()?;
            Ok(Some((core, tags, reminder)))
        })?;

        stored
            .map(|(core, tags, reminder)| {
                Ok(MemoRecord {
                    id: core.id,
                    title: core.title.clone(),
                    body: core.body.clone(),
                    display_title: MemoService::display_title(
                        &core.title,
                        &core.body,
                        untitled_label,
                    ),
                    tags,
                    pinned_at: core.pinned_at,
                    reminder: reminder.map(parse_reminder).transpose()?,
                    created_at: core.created_at,
                    updated_at: core.updated_at,
                })
            })
            .transpose()
    }

    pub fn list(
        &self,
        query: &MemoListQuery,
        untitled_label: &str,
    ) -> Result<Vec<MemoSummary>, DomainError> {
        query.validate()?;
        let search = query.search.trim();
        let pattern = format!("%{}%", escape_like(search));
        let ids = self.database.read(|connection| {
            let mut statement = connection.prepare(
                "SELECT memos.id
                 FROM memos
                 WHERE (
                    ?1 = ''
                    OR memos.title LIKE ?2 ESCAPE '\\' COLLATE NOCASE
                    OR memos.body LIKE ?2 ESCAPE '\\' COLLATE NOCASE
                    OR EXISTS (
                        SELECT 1
                        FROM memo_tag_links search_links
                        JOIN memo_tags search_tags ON search_tags.id = search_links.tag_id
                        WHERE search_links.memo_id = memos.id
                          AND search_tags.name LIKE ?2 ESCAPE '\\' COLLATE NOCASE
                    )
                 )
                 AND (
                    ?3 IS NULL
                    OR EXISTS (
                        SELECT 1 FROM memo_tag_links filter_links
                        WHERE filter_links.memo_id = memos.id AND filter_links.tag_id = ?3
                    )
                 )
                 ORDER BY (memos.pinned_at IS NOT NULL) DESC,
                    memos.pinned_at DESC, memos.updated_at DESC, memos.id ASC",
            )?;
            let rows = statement
                .query_map(params![search, pattern, query.tag_id], |row| row.get(0))?
                .collect::<rusqlite::Result<Vec<String>>>();
            rows
        })?;

        ids.into_iter()
            .map(|id| {
                self.get(&id, untitled_label)?
                    .map(|memo| MemoSummary {
                        id: memo.id,
                        display_title: memo.display_title,
                        body_preview: body_preview(&memo.body),
                        tags: memo.tags,
                        pinned_at: memo.pinned_at,
                        reminder: memo.reminder,
                        updated_at: memo.updated_at,
                    })
                    .ok_or_else(memo_not_found_error)
            })
            .collect()
    }

    pub fn list_tags(&self) -> Result<Vec<MemoTagSummary>, DomainError> {
        self.database.read(|connection| {
            let mut statement = connection.prepare(
                "SELECT memo_tags.id, memo_tags.name, COUNT(memo_tag_links.memo_id)
                 FROM memo_tags
                 JOIN memo_tag_links ON memo_tag_links.tag_id = memo_tags.id
                 GROUP BY memo_tags.id, memo_tags.name, memo_tags.normalized_name
                 HAVING COUNT(memo_tag_links.memo_id) > 0
                 ORDER BY memo_tags.normalized_name, memo_tags.id",
            )?;
            let tags = statement
                .query_map([], |row| {
                    Ok(MemoTagSummary {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        memo_count: row.get(2)?,
                    })
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            Ok(tags)
        })
    }

    pub fn list_due_reminders(
        &self,
        now: DateTime<Utc>,
        untitled_label: &str,
    ) -> Result<Vec<DueMemoReminder>, DomainError> {
        let stored = self.database.read(|connection| {
            let mut statement = connection.prepare(
                "SELECT memo_reminders.id, memo_reminders.memo_id,
                    memo_reminders.schedule_kind, memo_reminders.frequency,
                    memo_reminders.interval_value, memo_reminders.weekdays_json,
                    memo_reminders.monthly_day, memo_reminders.local_time,
                    memo_reminders.starts_on, memo_reminders.ends_on,
                    memo_reminders.timezone, memo_reminders.next_scheduled_for,
                    memo_reminders.status, memo_reminders.created_at,
                    memo_reminders.updated_at, memos.title, memos.body
                 FROM memo_reminders
                 JOIN memos ON memos.id = memo_reminders.memo_id
                 WHERE memo_reminders.status = 'active'
                   AND memo_reminders.next_scheduled_for IS NOT NULL
                   AND julianday(memo_reminders.next_scheduled_for) <= julianday(?1)
                 ORDER BY julianday(memo_reminders.next_scheduled_for), memo_reminders.id",
            )?;
            let rows = statement
                .query_map([now.to_rfc3339()], |row| {
                    Ok((
                        map_stored_reminder(row)?,
                        row.get::<_, String>(15)?,
                        row.get::<_, String>(16)?,
                    ))
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            Ok(rows)
        })?;

        stored
            .into_iter()
            .map(|(stored, title, body)| {
                Ok(DueMemoReminder {
                    reminder: parse_reminder(stored)?,
                    display_title: MemoService::display_title(&title, &body, untitled_label),
                })
            })
            .collect()
    }

    pub fn advance_reminder(
        &self,
        reminder_id: &str,
        expected_scheduled_for: &str,
        next_scheduled_for: Option<&str>,
        status: MemoReminderStatus,
        updated_at: DateTime<Utc>,
    ) -> Result<bool, DomainError> {
        self.database.write_domain(|transaction| {
            transaction
                .execute(
                    "UPDATE memo_reminders
                     SET next_scheduled_for = ?3, status = ?4, updated_at = ?5
                     WHERE id = ?1 AND status = 'active' AND next_scheduled_for = ?2",
                    params![
                        reminder_id,
                        expected_scheduled_for,
                        next_scheduled_for,
                        reminder_status(status),
                        updated_at.to_rfc3339(),
                    ],
                )
                .map(|changed| changed == 1)
                .map_err(memo_save_error)
        })
    }

    pub fn remove(&self, memo_id: &str) -> Result<(), DomainError> {
        self.database.write_domain(|transaction| {
            let changed = transaction
                .execute("DELETE FROM memos WHERE id = ?1", [memo_id])
                .map_err(memo_delete_error)?;
            if changed == 0 {
                return Err(DomainError {
                    code: "MEMO_NOT_FOUND".into(),
                    message: "memo was not found".into(),
                    field: None,
                });
            }
            transaction
                .execute(
                    "DELETE FROM memo_tags WHERE NOT EXISTS (
                        SELECT 1 FROM memo_tag_links WHERE memo_tag_links.tag_id = memo_tags.id
                    )",
                    [],
                )
                .map_err(memo_delete_error)?;
            Ok(())
        })
    }
}

struct StoredReminder {
    id: String,
    memo_id: String,
    schedule_kind: String,
    frequency: Option<String>,
    interval: Option<u32>,
    weekdays_json: Option<String>,
    monthly_day: Option<u8>,
    local_time: String,
    starts_on: String,
    ends_on: Option<String>,
    timezone: String,
    next_scheduled_for: Option<String>,
    status: String,
    created_at: String,
    updated_at: String,
}

fn map_stored_reminder(row: &rusqlite::Row<'_>) -> rusqlite::Result<StoredReminder> {
    Ok(StoredReminder {
        id: row.get(0)?,
        memo_id: row.get(1)?,
        schedule_kind: row.get(2)?,
        frequency: row.get(3)?,
        interval: row.get(4)?,
        weekdays_json: row.get(5)?,
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
}

fn parse_reminder(stored: StoredReminder) -> Result<MemoReminderRule, DomainError> {
    let schedule = match stored.schedule_kind.as_str() {
        "once" => MemoReminderInput::Once {
            scheduled_local: format!("{}T{}", stored.starts_on, stored.local_time),
            timezone: stored.timezone,
        },
        "recurring" => MemoReminderInput::Recurring {
            frequency: parse_frequency(stored.frequency.as_deref())?,
            interval: stored.interval.ok_or_else(memo_reminder_data_error)?,
            weekdays: stored
                .weekdays_json
                .as_deref()
                .map(serde_json::from_str)
                .transpose()
                .map_err(|_| memo_reminder_data_error())?
                .unwrap_or_default(),
            monthly_day: stored.monthly_day,
            local_time: stored.local_time,
            starts_on: stored.starts_on,
            ends_on: stored.ends_on,
            timezone: stored.timezone,
        },
        _ => return Err(memo_reminder_data_error()),
    };
    Ok(MemoReminderRule {
        id: stored.id,
        memo_id: stored.memo_id,
        schedule,
        next_scheduled_for: stored.next_scheduled_for,
        status: match stored.status.as_str() {
            "active" => MemoReminderStatus::Active,
            "completed" => MemoReminderStatus::Completed,
            "cancelled" => MemoReminderStatus::Cancelled,
            _ => return Err(memo_reminder_data_error()),
        },
        created_at: stored.created_at,
        updated_at: stored.updated_at,
    })
}

fn parse_frequency(value: Option<&str>) -> Result<MemoReminderFrequency, DomainError> {
    match value {
        Some("daily") => Ok(MemoReminderFrequency::Daily),
        Some("weekdays") => Ok(MemoReminderFrequency::Weekdays),
        Some("weekly") => Ok(MemoReminderFrequency::Weekly),
        Some("monthly") => Ok(MemoReminderFrequency::Monthly),
        _ => Err(memo_reminder_data_error()),
    }
}

fn replace_tags_in_transaction(
    transaction: &Transaction<'_>,
    memo_id: &str,
    tags: &[NormalizedMemoTag],
    now: &str,
) -> rusqlite::Result<Vec<MemoTag>> {
    let mut resolved = Vec::with_capacity(tags.len());
    for tag in tags {
        let existing = transaction
            .query_row(
                "SELECT id, name FROM memo_tags WHERE normalized_name = ?1",
                [&tag.normalized_name],
                |row| {
                    Ok(MemoTag {
                        id: row.get(0)?,
                        name: row.get(1)?,
                    })
                },
            )
            .optional()?;
        let resolved_tag = match existing {
            Some(existing) => existing,
            None => {
                let id = Uuid::new_v4().to_string();
                transaction.execute(
                    "INSERT INTO memo_tags(id, name, normalized_name, created_at) VALUES (?1, ?2, ?3, ?4)",
                    params![id, tag.name, tag.normalized_name, now],
                )?;
                MemoTag {
                    id,
                    name: tag.name.clone(),
                }
            }
        };
        resolved.push(resolved_tag);
    }

    transaction.execute("DELETE FROM memo_tag_links WHERE memo_id = ?1", [memo_id])?;
    for tag in &resolved {
        transaction.execute(
            "INSERT INTO memo_tag_links(memo_id, tag_id) VALUES (?1, ?2)",
            params![memo_id, tag.id],
        )?;
    }
    transaction.execute(
        "DELETE FROM memo_tags WHERE NOT EXISTS (
            SELECT 1 FROM memo_tag_links WHERE memo_tag_links.tag_id = memo_tags.id
        )",
        [],
    )?;
    Ok(resolved)
}

fn replace_reminder_in_transaction(
    transaction: &Transaction<'_>,
    memo_id: &str,
    reminder: Option<&MemoReminderRule>,
) -> rusqlite::Result<()> {
    let Some(reminder) = reminder else {
        transaction.execute("DELETE FROM memo_reminders WHERE memo_id = ?1", [memo_id])?;
        return Ok(());
    };

    let (
        schedule_kind,
        frequency,
        interval,
        weekdays_json,
        monthly_day,
        local_time,
        starts_on,
        ends_on,
        timezone,
    ) = match &reminder.schedule {
        MemoReminderInput::Once {
            scheduled_local,
            timezone,
        } => {
            let (starts_on, local_time) = scheduled_local
                .split_once('T')
                .ok_or_else(|| rusqlite::Error::InvalidParameterName("scheduledLocal".into()))?;
            (
                "once",
                None,
                None,
                None,
                None,
                local_time.to_owned(),
                starts_on.to_owned(),
                None,
                timezone.clone(),
            )
        }
        MemoReminderInput::Recurring {
            frequency,
            interval,
            weekdays,
            monthly_day,
            local_time,
            starts_on,
            ends_on,
            timezone,
        } => (
            "recurring",
            Some(reminder_frequency(*frequency)),
            Some(*interval),
            Some(
                serde_json::to_string(weekdays)
                    .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?,
            ),
            *monthly_day,
            local_time.clone(),
            starts_on.clone(),
            ends_on.clone(),
            timezone.clone(),
        ),
    };
    transaction.execute(
        "INSERT INTO memo_reminders(
            id, memo_id, schedule_kind, frequency, interval_value, weekdays_json,
            monthly_day, local_time, starts_on, ends_on, timezone, next_scheduled_for,
            status, created_at, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
         ON CONFLICT(memo_id) DO UPDATE SET
            id = excluded.id,
            schedule_kind = excluded.schedule_kind,
            frequency = excluded.frequency,
            interval_value = excluded.interval_value,
            weekdays_json = excluded.weekdays_json,
            monthly_day = excluded.monthly_day,
            local_time = excluded.local_time,
            starts_on = excluded.starts_on,
            ends_on = excluded.ends_on,
            timezone = excluded.timezone,
            next_scheduled_for = excluded.next_scheduled_for,
            status = excluded.status,
            updated_at = excluded.updated_at",
        params![
            reminder.id,
            memo_id,
            schedule_kind,
            frequency,
            interval,
            weekdays_json,
            monthly_day,
            local_time,
            starts_on,
            ends_on,
            timezone,
            reminder.next_scheduled_for,
            reminder_status(reminder.status),
            reminder.created_at,
            reminder.updated_at,
        ],
    )?;
    Ok(())
}

fn reminder_frequency(frequency: MemoReminderFrequency) -> &'static str {
    match frequency {
        MemoReminderFrequency::Daily => "daily",
        MemoReminderFrequency::Weekdays => "weekdays",
        MemoReminderFrequency::Weekly => "weekly",
        MemoReminderFrequency::Monthly => "monthly",
    }
}

fn reminder_status(status: MemoReminderStatus) -> &'static str {
    match status {
        MemoReminderStatus::Active => "active",
        MemoReminderStatus::Completed => "completed",
        MemoReminderStatus::Cancelled => "cancelled",
    }
}

fn memo_save_error(_error: rusqlite::Error) -> DomainError {
    DomainError {
        code: "MEMO_SAVE_FAILED".into(),
        message: "memo could not be saved".into(),
        field: None,
    }
}

fn memo_not_found_error() -> DomainError {
    DomainError {
        code: "MEMO_NOT_FOUND".into(),
        message: "memo was not found".into(),
        field: None,
    }
}

fn memo_reminder_data_error() -> DomainError {
    DomainError {
        code: "MEMO_REMINDER_DATA_INVALID".into(),
        message: "stored memo reminder is invalid".into(),
        field: None,
    }
}

fn escape_like(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

fn body_preview(body: &str) -> String {
    body.trim().chars().take(120).collect()
}

fn memo_delete_error(_error: rusqlite::Error) -> DomainError {
    DomainError {
        code: "MEMO_DELETE_FAILED".into(),
        message: "memo could not be deleted".into(),
        field: None,
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use proptest::prelude::*;

    use super::*;
    use crate::services::memo_service::MemoService;

    fn insert_memo(database: &Database) {
        database
            .write(|transaction| {
                transaction.execute(
                    "INSERT INTO memos(id, title, body, created_at, updated_at)
                     VALUES ('memo-1', '', '', '2026-07-23T10:00:00Z', '2026-07-23T10:00:00Z')",
                    [],
                )?;
                Ok(())
            })
            .unwrap();
    }

    fn count(database: &Database, table: &str) -> i64 {
        database
            .read(|connection| {
                connection.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                    row.get(0)
                })
            })
            .unwrap()
    }

    fn memo_input(title: &str, body: &str, tags: Vec<String>) -> crate::domain::memo::MemoInput {
        crate::domain::memo::MemoInput {
            title: title.into(),
            body: body.into(),
            tags,
            pinned: false,
            reminder: None,
        }
    }

    fn insert_raw_memo(
        database: &Database,
        id: &str,
        title: &str,
        body: &str,
        pinned_at: Option<&str>,
        updated_at: &str,
    ) {
        database
            .write(|transaction| {
                transaction.execute(
                    "INSERT INTO memos(id, title, body, pinned_at, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, '2026-07-23T09:00:00Z', ?5)",
                    params![id, title, body, pinned_at, updated_at],
                )?;
                Ok(())
            })
            .unwrap();
    }

    #[test]
    fn list_searches_literal_like_characters_and_combines_tag_filter() {
        let database = Database::open_in_memory().unwrap();
        insert_raw_memo(
            &database,
            "percent",
            "Budget 100%",
            "Finance notes",
            None,
            "2026-07-23T10:00:00Z",
        );
        insert_raw_memo(
            &database,
            "plain",
            "Budget 1000",
            "General",
            None,
            "2026-07-23T11:00:00Z",
        );
        insert_raw_memo(
            &database,
            "underscore",
            "Code",
            "literal_code_a",
            None,
            "2026-07-23T12:00:00Z",
        );
        insert_raw_memo(
            &database,
            "slash",
            "Path C:\\notes",
            "Paths",
            None,
            "2026-07-23T13:00:00Z",
        );
        let repository = MemoRepository::new(&database);
        let finance = MemoService::normalize_tags(&["Finance".into()]).unwrap();
        let finance_tag = repository.replace_tags("percent", &finance).unwrap()[0].clone();
        let work = MemoService::normalize_tags(&["Work".into()]).unwrap();
        repository.replace_tags("plain", &work).unwrap();

        let list_ids = |search: &str, tag_id: Option<String>| {
            repository
                .list(
                    &MemoListQuery {
                        search: search.into(),
                        tag_id,
                    },
                    "Untitled memo",
                )
                .unwrap()
                .into_iter()
                .map(|memo| memo.id)
                .collect::<Vec<_>>()
        };
        assert_eq!(list_ids("%", None), vec!["percent"]);
        assert_eq!(list_ids("_", None), vec!["underscore"]);
        assert_eq!(list_ids("\\", None), vec!["slash"]);
        assert_eq!(list_ids("FINANCE", None), vec!["percent"]);
        assert_eq!(list_ids("budget", Some(finance_tag.id)), vec!["percent"]);
    }

    #[test]
    fn list_applies_pinned_updated_and_id_stable_order() {
        let database = Database::open_in_memory().unwrap();
        insert_raw_memo(
            &database,
            "z-pinned",
            "Z",
            "",
            Some("2026-07-23T11:00:00Z"),
            "2026-07-23T12:00:00Z",
        );
        insert_raw_memo(
            &database,
            "a-pinned",
            "A",
            "",
            Some("2026-07-23T11:00:00Z"),
            "2026-07-23T12:00:00Z",
        );
        insert_raw_memo(
            &database,
            "older-pin",
            "Older pin",
            "",
            Some("2026-07-23T10:00:00Z"),
            "2026-07-23T14:00:00Z",
        );
        insert_raw_memo(
            &database,
            "new-unpinned",
            "New",
            " preview ",
            None,
            "2026-07-23T14:00:00Z",
        );
        insert_raw_memo(
            &database,
            "old-unpinned",
            "Old",
            "",
            None,
            "2026-07-23T13:00:00Z",
        );

        let list = MemoRepository::new(&database)
            .list(
                &MemoListQuery {
                    search: String::new(),
                    tag_id: None,
                },
                "Untitled memo",
            )
            .unwrap();

        assert_eq!(
            list.iter().map(|memo| memo.id.as_str()).collect::<Vec<_>>(),
            vec![
                "a-pinned",
                "z-pinned",
                "older-pin",
                "new-unpinned",
                "old-unpinned",
            ]
        );
        assert_eq!(list[3].body_preview, "preview");
    }

    #[test]
    fn tag_filter_list_returns_live_counts_and_excludes_orphans() {
        let database = Database::open_in_memory().unwrap();
        insert_raw_memo(&database, "memo-1", "One", "", None, "2026-07-23T10:00:00Z");
        insert_raw_memo(&database, "memo-2", "Two", "", None, "2026-07-23T11:00:00Z");
        let repository = MemoRepository::new(&database);
        let first = MemoService::normalize_tags(&["Shared".into(), "First".into()]).unwrap();
        repository.replace_tags("memo-1", &first).unwrap();
        let second = MemoService::normalize_tags(&["shared".into(), "Second".into()]).unwrap();
        repository.replace_tags("memo-2", &second).unwrap();

        let initial = repository.list_tags().unwrap();
        assert_eq!(
            initial
                .iter()
                .map(|tag| (tag.name.as_str(), tag.memo_count))
                .collect::<Vec<_>>(),
            vec![("First", 1), ("Second", 1), ("Shared", 2)]
        );

        let replacement = MemoService::normalize_tags(&["Second".into()]).unwrap();
        repository.replace_tags("memo-1", &replacement).unwrap();
        let refreshed = repository.list_tags().unwrap();
        assert_eq!(
            refreshed
                .iter()
                .map(|tag| (tag.name.as_str(), tag.memo_count))
                .collect::<Vec<_>>(),
            vec![("Second", 2), ("Shared", 1)]
        );
        assert!(refreshed.iter().all(|tag| tag.memo_count > 0));
    }

    #[test]
    fn create_get_and_update_round_trip_aggregated_detail() {
        let database = Database::open_in_memory().unwrap();
        let repository = MemoRepository::new(&database);
        let created_at = Utc.with_ymd_and_hms(2026, 7, 23, 10, 0, 0).unwrap();
        let input = memo_input(
            "  Launch  ",
            "Initial body",
            vec!["Work".into(), "Launch".into()],
        );
        let core = MemoService::create("memo-1".into(), &input, created_at).unwrap();
        let tags = MemoService::normalize_tags(&input.tags).unwrap();

        let created = repository
            .create(&core, &tags, None, "Untitled memo")
            .unwrap();

        assert_eq!(created.title, "Launch");
        assert_eq!(created.display_title, "Launch");
        assert_eq!(
            created
                .tags
                .iter()
                .map(|tag| tag.name.as_str())
                .collect::<Vec<_>>(),
            vec!["Launch", "Work"]
        );
        let update_input = memo_input("", "\n  Updated body title\nMore", vec!["Home".into()]);
        let updated_core = MemoService::update(
            &core,
            &update_input,
            Utc.with_ymd_and_hms(2026, 7, 23, 11, 0, 0).unwrap(),
        )
        .unwrap();
        let updated_tags = MemoService::normalize_tags(&update_input.tags).unwrap();
        let updated = repository
            .update(&updated_core, &updated_tags, None, "Untitled memo")
            .unwrap();

        assert_eq!(updated.display_title, "Updated body title");
        assert_eq!(updated.tags[0].name, "Home");
        assert_eq!(updated.created_at, created.created_at);
        assert_ne!(updated.updated_at, created.updated_at);
        assert_eq!(repository.get("missing", "Untitled memo").unwrap(), None);
    }

    #[test]
    fn get_aggregates_a_recurring_reminder() {
        let database = Database::open_in_memory().unwrap();
        insert_memo(&database);
        database
            .write(|transaction| {
                transaction.execute(
                    "INSERT INTO memo_reminders(id, memo_id, schedule_kind, frequency,
                        interval_value, weekdays_json, local_time, starts_on, ends_on, timezone,
                        next_scheduled_for, status, created_at, updated_at)
                     VALUES ('reminder-1', 'memo-1', 'recurring', 'weekly', 2, '[1,5]', '09:30',
                        '2026-07-23', '2026-12-31', 'Europe/Berlin', '2026-07-24T07:30:00Z',
                        'active', '2026-07-23T10:00:00Z', '2026-07-23T10:00:00Z')",
                    [],
                )?;
                Ok(())
            })
            .unwrap();

        let detail = MemoRepository::new(&database)
            .get("memo-1", "Untitled memo")
            .unwrap()
            .unwrap();

        let reminder = detail.reminder.unwrap();
        assert_eq!(reminder.status, MemoReminderStatus::Active);
        assert_eq!(
            reminder.next_scheduled_for.as_deref(),
            Some("2026-07-24T07:30:00Z")
        );
        assert_eq!(
            reminder.schedule,
            MemoReminderInput::Recurring {
                frequency: MemoReminderFrequency::Weekly,
                interval: 2,
                weekdays: vec![1, 5],
                monthly_day: None,
                local_time: "09:30".into(),
                starts_on: "2026-07-23".into(),
                ends_on: Some("2026-12-31".into()),
                timezone: "Europe/Berlin".into(),
            }
        );
    }

    #[test]
    fn update_rolls_back_core_and_links_when_tag_write_fails() {
        let database = Database::open_in_memory().unwrap();
        let repository = MemoRepository::new(&database);
        let now = Utc.with_ymd_and_hms(2026, 7, 23, 10, 0, 0).unwrap();
        let input = memo_input("Original", "Body", vec!["Work".into()]);
        let core = MemoService::create("memo-1".into(), &input, now).unwrap();
        let tags = MemoService::normalize_tags(&input.tags).unwrap();
        repository
            .create(&core, &tags, None, "Untitled memo")
            .unwrap();
        let updated_core = MemoService::update(
            &core,
            &memo_input("Changed", "Body", Vec::new()),
            Utc.with_ymd_and_hms(2026, 7, 23, 11, 0, 0).unwrap(),
        )
        .unwrap();
        let invalid_tags = vec![NormalizedMemoTag {
            name: "x".repeat(31),
            normalized_name: "invalid".into(),
        }];

        let error = repository
            .update(&updated_core, &invalid_tags, None, "Untitled memo")
            .unwrap_err();

        assert_eq!(error.code, "MEMO_SAVE_FAILED");
        let preserved = repository.get("memo-1", "Untitled memo").unwrap().unwrap();
        assert_eq!(preserved.title, "Original");
        assert_eq!(preserved.tags[0].name, "Work");
    }

    #[test]
    fn update_rolls_back_core_and_links_when_reminder_write_fails() {
        let database = Database::open_in_memory().unwrap();
        let repository = MemoRepository::new(&database);
        let now = Utc.with_ymd_and_hms(2026, 7, 23, 10, 0, 0).unwrap();
        let input = memo_input("Original", "Body", vec!["Work".into()]);
        let core = MemoService::create("memo-1".into(), &input, now).unwrap();
        let tags = MemoService::normalize_tags(&input.tags).unwrap();
        repository
            .create(&core, &tags, None, "Untitled memo")
            .unwrap();
        database
            .write(|transaction| {
                transaction.execute_batch(
                    "CREATE TRIGGER reject_memo_reminder
                     BEFORE INSERT ON memo_reminders
                     BEGIN
                         SELECT RAISE(ABORT, 'injected reminder failure');
                     END;",
                )
            })
            .unwrap();
        let updated_at = Utc.with_ymd_and_hms(2026, 7, 23, 11, 0, 0).unwrap();
        let updated_input = memo_input("Changed", "Changed body", vec!["Home".into()]);
        let updated_core = MemoService::update(&core, &updated_input, updated_at).unwrap();
        let updated_tags = MemoService::normalize_tags(&updated_input.tags).unwrap();
        let reminder = MemoReminderRule {
            id: "reminder-1".into(),
            memo_id: "memo-1".into(),
            schedule: MemoReminderInput::Once {
                scheduled_local: "2026-07-24T09:00".into(),
                timezone: "UTC".into(),
            },
            next_scheduled_for: Some("2026-07-24T09:00:00+00:00".into()),
            status: MemoReminderStatus::Active,
            created_at: now.to_rfc3339(),
            updated_at: updated_at.to_rfc3339(),
        };

        let error = repository
            .update(
                &updated_core,
                &updated_tags,
                Some(&reminder),
                "Untitled memo",
            )
            .unwrap_err();

        assert_eq!(error.code, "MEMO_SAVE_FAILED");
        let preserved = repository.get("memo-1", "Untitled memo").unwrap().unwrap();
        assert_eq!(preserved.title, "Original");
        assert_eq!(preserved.body, "Body");
        assert_eq!(preserved.tags[0].name, "Work");
        assert_eq!(preserved.reminder, None);
    }

    #[test]
    fn replacement_reuses_tags_and_removes_stale_orphans() {
        let database = Database::open_in_memory().unwrap();
        insert_memo(&database);
        let repository = MemoRepository::new(&database);
        let initial = MemoService::normalize_tags(&["Work".into(), "Personal".into()]).unwrap();
        let initial_tags = repository.replace_tags("memo-1", &initial).unwrap();
        let work_id = initial_tags[0].id.clone();

        let replacement = MemoService::normalize_tags(&[" work ".into(), "Home".into()]).unwrap();
        let replaced = repository.replace_tags("memo-1", &replacement).unwrap();

        assert_eq!(replaced[0].id, work_id);
        assert_eq!(replaced[0].name, "Work");
        assert_eq!(replaced[1].name, "Home");
        let stored: Vec<(String, String)> = database
            .read(|connection| {
                let mut statement = connection.prepare(
                    "SELECT memo_tags.name, memo_tag_links.memo_id
                     FROM memo_tags JOIN memo_tag_links ON memo_tag_links.tag_id = memo_tags.id
                     ORDER BY memo_tags.name",
                )?;
                let rows = statement
                    .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
                    .collect();
                rows
            })
            .unwrap();
        assert_eq!(
            stored,
            vec![
                ("Home".into(), "memo-1".into()),
                ("Work".into(), "memo-1".into()),
            ]
        );
    }

    #[test]
    fn removing_all_links_cleans_up_all_orphaned_tags() {
        let database = Database::open_in_memory().unwrap();
        insert_memo(&database);
        let repository = MemoRepository::new(&database);
        let tags = MemoService::normalize_tags(&["Work".into()]).unwrap();
        repository.replace_tags("memo-1", &tags).unwrap();

        repository.replace_tags("memo-1", &[]).unwrap();

        let counts = database
            .read(|connection| {
                Ok((
                    connection.query_row("SELECT COUNT(*) FROM memo_tags", [], |row| {
                        row.get::<_, i64>(0)
                    })?,
                    connection.query_row("SELECT COUNT(*) FROM memo_tag_links", [], |row| {
                        row.get::<_, i64>(0)
                    })?,
                ))
            })
            .unwrap();
        assert_eq!(counts, (0, 0));
    }

    #[test]
    fn missing_memo_rolls_back_tag_replacement() {
        let database = Database::open_in_memory().unwrap();
        let repository = MemoRepository::new(&database);
        let tags = MemoService::normalize_tags(&["Work".into()]).unwrap();

        assert!(repository.replace_tags("missing", &tags).is_err());

        let count = database
            .read(|connection| {
                connection.query_row("SELECT COUNT(*) FROM memo_tags", [], |row| {
                    row.get::<_, i64>(0)
                })
            })
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn removal_cascades_dependents_and_preserves_shared_tags() {
        let database = Database::open_in_memory().unwrap();
        insert_memo(&database);
        database
            .write(|transaction| {
                transaction.execute(
                    "INSERT INTO memos(id, title, body, created_at, updated_at)
                     VALUES ('memo-2', '', '', '2026-07-23T10:00:00Z', '2026-07-23T10:00:00Z')",
                    [],
                )?;
                transaction.execute(
                    "INSERT INTO memo_reminders(id, memo_id, schedule_kind, local_time, starts_on,
                        timezone, status, created_at, updated_at)
                     VALUES ('reminder-1', 'memo-1', 'once', '12:00', '2026-07-24',
                        'Asia/Shanghai', 'active', '2026-07-23T10:00:00Z', '2026-07-23T10:00:00Z')",
                    [],
                )?;
                Ok(())
            })
            .unwrap();
        let repository = MemoRepository::new(&database);
        let first_tags = MemoService::normalize_tags(&["Shared".into(), "First".into()]).unwrap();
        repository.replace_tags("memo-1", &first_tags).unwrap();
        let second_tags = MemoService::normalize_tags(&["shared".into(), "Second".into()]).unwrap();
        repository.replace_tags("memo-2", &second_tags).unwrap();

        repository.remove("memo-1").unwrap();

        assert_eq!(count(&database, "memos"), 1);
        assert_eq!(count(&database, "memo_reminders"), 0);
        assert_eq!(count(&database, "memo_tag_links"), 2);
        let names = database
            .read(|connection| {
                let mut statement =
                    connection.prepare("SELECT name FROM memo_tags ORDER BY name")?;
                let names = statement
                    .query_map([], |row| row.get(0))?
                    .collect::<rusqlite::Result<Vec<String>>>()?;
                Ok(names)
            })
            .unwrap();
        assert_eq!(names, vec!["Second", "Shared"]);
    }

    #[test]
    fn removal_returns_stable_not_found_error() {
        let database = Database::open_in_memory().unwrap();
        let error = MemoRepository::new(&database)
            .remove("missing")
            .unwrap_err();
        assert_eq!(error.code, "MEMO_NOT_FOUND");
    }

    #[test]
    fn removal_failure_rolls_back_memo_links_and_reminder() {
        let database = Database::open_in_memory().unwrap();
        insert_memo(&database);
        let repository = MemoRepository::new(&database);
        let tags = MemoService::normalize_tags(&["Work".into()]).unwrap();
        repository.replace_tags("memo-1", &tags).unwrap();
        database
            .write(|transaction| {
                transaction.execute(
                    "INSERT INTO memo_reminders(id, memo_id, schedule_kind, local_time, starts_on,
                        timezone, status, created_at, updated_at)
                     VALUES ('reminder-1', 'memo-1', 'once', '12:00', '2026-07-24',
                        'Asia/Shanghai', 'active', '2026-07-23T10:00:00Z', '2026-07-23T10:00:00Z')",
                    [],
                )?;
                transaction.execute_batch(
                    "CREATE TRIGGER reject_memo_tag_cleanup
                     BEFORE DELETE ON memo_tags
                     BEGIN
                         SELECT RAISE(ABORT, 'injected tag cleanup failure');
                     END;",
                )?;
                Ok(())
            })
            .unwrap();

        let error = repository.remove("memo-1").unwrap_err();

        assert_eq!(error.code, "MEMO_DELETE_FAILED");
        assert_eq!(count(&database, "memos"), 1);
        assert_eq!(count(&database, "memo_reminders"), 1);
        assert_eq!(count(&database, "memo_tag_links"), 1);
        assert_eq!(count(&database, "memo_tags"), 1);
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(64))]

        #[test]
        fn property_tag_normalization_keeps_entities_and_links_unique(
            base_names in prop::collection::btree_set("[a-z]{1,8}", 1..11),
        ) {
            let database = Database::open_in_memory().unwrap();
            insert_memo(&database);
            let repository = MemoRepository::new(&database);
            let variants: Vec<String> = base_names
                .iter()
                .flat_map(|name| [format!("  {name} "), name.to_uppercase()])
                .collect();
            let normalized = MemoService::normalize_tags(&variants).unwrap();

            let resolved = repository.replace_tags("memo-1", &normalized).unwrap();

            prop_assert_eq!(normalized.len(), base_names.len());
            prop_assert_eq!(resolved.len(), base_names.len());
            prop_assert_eq!(count(&database, "memo_tags") as usize, base_names.len());
            prop_assert_eq!(count(&database, "memo_tag_links") as usize, base_names.len());
            let distinct = database
                .read(|connection| {
                    Ok((
                        connection.query_row(
                            "SELECT COUNT(DISTINCT normalized_name) FROM memo_tags",
                            [],
                            |row| row.get::<_, i64>(0),
                        )?,
                        connection.query_row(
                            "SELECT COUNT(DISTINCT memo_id || ':' || tag_id) FROM memo_tag_links",
                            [],
                            |row| row.get::<_, i64>(0),
                        )?,
                    ))
                })
                .unwrap();
            prop_assert_eq!(distinct.0 as usize, base_names.len());
            prop_assert_eq!(distinct.1 as usize, base_names.len());
        }

        #[test]
        fn property_memo_removal_is_atomic_for_random_association_graphs(
            (tag_count, other_memo_count, shared_edges, inject_failure) in
                (1usize..8, 0usize..5, any::<bool>()).prop_flat_map(
                    |(tag_count, other_memo_count, inject_failure)| {
                        (
                            Just(tag_count),
                            Just(other_memo_count),
                            prop::collection::vec(
                                any::<bool>(),
                                tag_count.saturating_sub(1) * other_memo_count,
                            ),
                            Just(inject_failure),
                        )
                    },
                ),
        ) {
            let database = Database::open_in_memory().unwrap();
            database
                .write(|transaction| {
                    transaction.execute(
                        "INSERT INTO memos(id, title, body, created_at, updated_at)
                         VALUES ('target', '', '', '2026-07-23T10:00:00Z', '2026-07-23T10:00:00Z')",
                        [],
                    )?;
                    for memo_index in 0..other_memo_count {
                        transaction.execute(
                            "INSERT INTO memos(id, title, body, created_at, updated_at)
                             VALUES (?1, '', '', '2026-07-23T10:00:00Z', '2026-07-23T10:00:00Z')",
                            [format!("other-{memo_index}")],
                        )?;
                    }
                    for tag_index in 0..tag_count {
                        let tag_id = format!("tag-{tag_index}");
                        transaction.execute(
                            "INSERT INTO memo_tags(id, name, normalized_name, created_at)
                             VALUES (?1, ?2, ?2, '2026-07-23T10:00:00Z')",
                            params![tag_id, format!("tag{tag_index}")],
                        )?;
                        transaction.execute(
                            "INSERT INTO memo_tag_links(memo_id, tag_id) VALUES ('target', ?1)",
                            [&tag_id],
                        )?;
                        if tag_index > 0 {
                            for memo_index in 0..other_memo_count {
                                let edge_index = (tag_index - 1) * other_memo_count + memo_index;
                                if shared_edges[edge_index] {
                                    transaction.execute(
                                        "INSERT INTO memo_tag_links(memo_id, tag_id) VALUES (?1, ?2)",
                                        params![format!("other-{memo_index}"), tag_id],
                                    )?;
                                }
                            }
                        }
                    }
                    transaction.execute(
                        "INSERT INTO memo_reminders(id, memo_id, schedule_kind, local_time, starts_on,
                            timezone, status, created_at, updated_at)
                         VALUES ('target-reminder', 'target', 'once', '12:00', '2026-07-24',
                            'Asia/Shanghai', 'active', '2026-07-23T10:00:00Z', '2026-07-23T10:00:00Z')",
                        [],
                    )?;
                    if inject_failure {
                        transaction.execute_batch(
                            "CREATE TRIGGER reject_property_tag_cleanup
                             BEFORE DELETE ON memo_tags
                             BEGIN
                                 SELECT RAISE(ABORT, 'injected property failure');
                             END;",
                        )?;
                    }
                    Ok(())
                })
                .unwrap();

            let original_links = tag_count + shared_edges.iter().filter(|edge| **edge).count();
            let result = MemoRepository::new(&database).remove("target");

            if inject_failure {
                prop_assert_eq!(result.unwrap_err().code, "MEMO_DELETE_FAILED");
                prop_assert_eq!(count(&database, "memos") as usize, other_memo_count + 1);
                prop_assert_eq!(count(&database, "memo_reminders"), 1);
                prop_assert_eq!(count(&database, "memo_tags") as usize, tag_count);
                prop_assert_eq!(count(&database, "memo_tag_links") as usize, original_links);
            } else {
                result.unwrap();
                let shared_tag_count = (1..tag_count)
                    .filter(|tag_index| {
                        (0..other_memo_count).any(|memo_index| {
                            shared_edges[(tag_index - 1) * other_memo_count + memo_index]
                        })
                    })
                    .count();
                let shared_link_count = shared_edges.iter().filter(|edge| **edge).count();
                prop_assert_eq!(count(&database, "memos") as usize, other_memo_count);
                prop_assert_eq!(count(&database, "memo_reminders"), 0);
                prop_assert_eq!(count(&database, "memo_tags") as usize, shared_tag_count);
                prop_assert_eq!(count(&database, "memo_tag_links") as usize, shared_link_count);
            }
        }

        #[test]
        fn property_memo_list_sorting_is_stable(
            (sort_values, reverse_insertion) in (
                prop::collection::vec((any::<bool>(), 0u8..60, 0u8..60), 1..21),
                any::<bool>(),
            ),
        ) {
            let database = Database::open_in_memory().unwrap();
            let mut records: Vec<(String, Option<u8>, u8)> = sort_values
                .into_iter()
                .enumerate()
                .map(|(index, (pinned, pinned_minute, updated_minute))| {
                    (
                        format!("memo-{index:02}"),
                        pinned.then_some(pinned_minute),
                        updated_minute,
                    )
                })
                .collect();
            let mut insertion_order: Vec<usize> = (0..records.len()).collect();
            if reverse_insertion {
                insertion_order.reverse();
            }
            for index in insertion_order {
                let (id, pinned_minute, updated_minute) = &records[index];
                let pinned_at = pinned_minute
                    .map(|minute| format!("2026-07-23T10:{minute:02}:00Z"));
                let updated_at = format!("2026-07-23T11:{updated_minute:02}:00Z");
                insert_raw_memo(
                    &database,
                    id,
                    id,
                    "",
                    pinned_at.as_deref(),
                    &updated_at,
                );
            }

            records.sort_by(|left, right| {
                right
                    .1
                    .is_some()
                    .cmp(&left.1.is_some())
                    .then_with(|| right.1.cmp(&left.1))
                    .then_with(|| right.2.cmp(&left.2))
                    .then_with(|| left.0.cmp(&right.0))
            });
            let expected: Vec<String> = records.into_iter().map(|record| record.0).collect();
            let query = MemoListQuery {
                search: String::new(),
                tag_id: None,
            };
            let repository = MemoRepository::new(&database);
            let first: Vec<String> = repository
                .list(&query, "Untitled memo")
                .unwrap()
                .into_iter()
                .map(|memo| memo.id)
                .collect();
            let second: Vec<String> = repository
                .list(&query, "Untitled memo")
                .unwrap()
                .into_iter()
                .map(|memo| memo.id)
                .collect();

            prop_assert_eq!(&first, &expected);
            prop_assert_eq!(&second, &expected);
        }

        #[test]
        fn property_search_and_tag_filter_return_exact_intersection(
            records in prop::collection::vec((0u8..4, any::<bool>(), any::<bool>()), 1..21),
        ) {
            let database = Database::open_in_memory().unwrap();
            let repository = MemoRepository::new(&database);
            let mut filter_tag_id = None;
            let mut expected_search = std::collections::BTreeSet::new();
            let mut expected_filter = std::collections::BTreeSet::new();

            for (index, (match_location, has_filter_tag, uppercase)) in
                records.iter().copied().enumerate()
            {
                let id = format!("memo-{index:02}");
                let needle = if uppercase { "NEEDLE%_\\" } else { "needle%_\\" };
                let title = if match_location == 0 {
                    format!("title {needle} suffix")
                } else {
                    "unrelated title".into()
                };
                let body = if match_location == 1 {
                    format!("body {needle} suffix")
                } else {
                    "unrelated body".into()
                };
                insert_raw_memo(
                    &database,
                    &id,
                    &title,
                    &body,
                    None,
                    "2026-07-23T10:00:00Z",
                );
                let mut tag_names = Vec::new();
                if has_filter_tag {
                    tag_names.push("Filter".into());
                    expected_filter.insert(id.clone());
                }
                if match_location == 2 {
                    tag_names.push(format!("tag {needle} suffix"));
                }
                if match_location < 3 {
                    expected_search.insert(id.clone());
                }
                let normalized = MemoService::normalize_tags(&tag_names).unwrap();
                let resolved = repository.replace_tags(&id, &normalized).unwrap();
                if has_filter_tag && filter_tag_id.is_none() {
                    filter_tag_id = resolved
                        .iter()
                        .find(|tag| tag.name == "Filter")
                        .map(|tag| tag.id.clone());
                }
            }

            let filter_id = filter_tag_id.unwrap_or_else(|| "missing-filter".into());
            let result_ids = |search: &str, tag_id: Option<String>| {
                repository
                    .list(
                        &MemoListQuery {
                            search: search.into(),
                            tag_id,
                        },
                        "Untitled memo",
                    )
                    .unwrap()
                    .into_iter()
                    .map(|memo| memo.id)
                    .collect::<std::collections::BTreeSet<_>>()
            };
            let expected_intersection = expected_search
                .intersection(&expected_filter)
                .cloned()
                .collect::<std::collections::BTreeSet<_>>();

            prop_assert_eq!(result_ids("needle%_\\", None), expected_search);
            prop_assert_eq!(result_ids("", Some(filter_id.clone())), expected_filter);
            prop_assert_eq!(
                result_ids("needle%_\\", Some(filter_id)),
                expected_intersection,
            );
        }
    }
}
