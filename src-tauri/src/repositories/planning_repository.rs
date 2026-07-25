use chrono::Utc;
use rusqlite::{params, OptionalExtension, Row};
use uuid::Uuid;

use crate::{
    domain::planning::{DailyNote, DailyNoteInput, WeeklyGoal, WeeklyGoalInput},
    repositories::database::Database,
    DomainError,
};

pub struct PlanningRepository<'a> {
    database: &'a Database,
}

impl<'a> PlanningRepository<'a> {
    pub fn new(database: &'a Database) -> Self {
        Self { database }
    }

    pub fn get_note(&self, note_date: &str) -> Result<Option<DailyNote>, DomainError> {
        self.database.read(|connection| {
            connection
                .query_row(
                    "SELECT id, body, note_date, created_at, updated_at
                     FROM notes WHERE note_date = ?1
                     ORDER BY updated_at DESC, id DESC LIMIT 1",
                    [note_date],
                    map_note,
                )
                .optional()
        })
    }

    pub fn save_note(&self, input: &DailyNoteInput) -> Result<DailyNote, DomainError> {
        let now = Utc::now().to_rfc3339();
        self.database.write(|transaction| {
            let existing = transaction
                .query_row(
                    "SELECT id, created_at FROM notes WHERE note_date = ?1
                     ORDER BY updated_at DESC, id DESC LIMIT 1",
                    [&input.note_date],
                    |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
                )
                .optional()?;
            let (id, created_at) = existing
                .unwrap_or_else(|| (Uuid::new_v4().to_string(), now.clone()));
            transaction.execute(
                "INSERT INTO notes(id, body, note_date, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(id) DO UPDATE SET body = excluded.body, updated_at = excluded.updated_at",
                params![id, input.body, input.note_date, created_at, now],
            )?;
            Ok(DailyNote {
                id,
                body: input.body.clone(),
                note_date: input.note_date.clone(),
                created_at,
                updated_at: now,
            })
        })
    }

    pub fn list_goals(&self, week_starts_on: &str) -> Result<Vec<WeeklyGoal>, DomainError> {
        let stored = self.database.read(|connection| {
            let mut statement = connection.prepare(
                "SELECT id, week_starts_on, title, category, target_count, completed_count,
                    position, created_at, updated_at
                 FROM weekly_goals WHERE week_starts_on = ?1
                 ORDER BY position, created_at, id",
            )?;
            let rows = statement
                .query_map([week_starts_on], map_goal)?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            Ok(rows)
        })?;
        stored.into_iter().collect()
    }

    pub fn save_goal(&self, input: &WeeklyGoalInput) -> Result<WeeklyGoal, DomainError> {
        let now = Utc::now().to_rfc3339();
        let requested_id = input.id.clone();
        self.database.write(|transaction| {
            let existing = requested_id
                .as_deref()
                .map(|id| {
                    transaction
                        .query_row(
                            "SELECT position, created_at FROM weekly_goals WHERE id = ?1",
                            [id],
                            |row| Ok((row.get::<_, u32>(0)?, row.get::<_, String>(1)?)),
                        )
                        .optional()
                })
                .transpose()?
                .flatten();
            let position = match existing.as_ref() {
                Some((position, _)) => *position,
                None => transaction.query_row(
                    "SELECT COALESCE(MAX(position) + 1, 0) FROM weekly_goals WHERE week_starts_on = ?1",
                    [&input.week_starts_on],
                    |row| row.get(0),
                )?,
            };
            let created_at = existing
                .map(|(_, created_at)| created_at)
                .unwrap_or_else(|| now.clone());
            let id = requested_id.unwrap_or_else(|| Uuid::new_v4().to_string());
            transaction.execute(
                "INSERT INTO weekly_goals(id, week_starts_on, title, target_count, completed_count,
                    position, created_at, updated_at, category)
                 VALUES (?1, ?2, ?3, ?4, 0, ?5, ?6, ?7, ?8)
                 ON CONFLICT(id) DO UPDATE SET week_starts_on = excluded.week_starts_on,
                    title = excluded.title, target_count = excluded.target_count,
                    completed_count = MIN(weekly_goals.completed_count, excluded.target_count),
                    updated_at = excluded.updated_at, category = excluded.category",
                params![
                    id,
                    input.week_starts_on,
                    input.title.trim(),
                    input.target_count,
                    position,
                    created_at,
                    now,
                    input.category.as_database_value(),
                ],
            )?;
            Ok(WeeklyGoal {
                id,
                week_starts_on: input.week_starts_on.clone(),
                title: input.title.trim().into(),
                category: input.category,
                target_count: input.target_count,
                completed_count: 0,
                position,
                created_at,
                updated_at: now,
            })
        })
    }

    pub fn update_goal_progress(&self, goals: &[WeeklyGoal]) -> Result<String, DomainError> {
        let now = Utc::now().to_rfc3339();
        self.database.write(|transaction| {
            for goal in goals {
                transaction.execute(
                    "UPDATE weekly_goals SET completed_count = ?2, updated_at = ?3 WHERE id = ?1",
                    params![goal.id, goal.completed_count, now],
                )?;
            }
            Ok(now)
        })
    }
}

fn map_note(row: &Row<'_>) -> rusqlite::Result<DailyNote> {
    Ok(DailyNote {
        id: row.get(0)?,
        body: row.get(1)?,
        note_date: row.get(2)?,
        created_at: row.get(3)?,
        updated_at: row.get(4)?,
    })
}

fn map_goal(row: &Row<'_>) -> rusqlite::Result<Result<WeeklyGoal, DomainError>> {
    let id = row.get(0)?;
    let week_starts_on = row.get(1)?;
    let title = row.get(2)?;
    let category = row.get::<_, String>(3)?;
    let target_count = row.get(4)?;
    let completed_count = row.get(5)?;
    let position = row.get(6)?;
    let created_at = row.get(7)?;
    let updated_at = row.get(8)?;
    Ok(
        crate::domain::planning::WeeklyGoalCategory::parse_database_value(&category).map(
            |category| WeeklyGoal {
                id,
                week_starts_on,
                title,
                category,
                target_count,
                completed_count,
                position,
                created_at,
                updated_at,
            },
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::planning::WeeklyGoalCategory;

    #[test]
    fn note_save_updates_the_latest_record_for_the_date() {
        let database = Database::open_in_memory().unwrap();
        let repository = PlanningRepository::new(&database);
        let first = repository
            .save_note(&DailyNoteInput {
                body: "first".into(),
                note_date: "2026-07-20".into(),
            })
            .unwrap();
        let second = repository
            .save_note(&DailyNoteInput {
                body: "second".into(),
                note_date: "2026-07-20".into(),
            })
            .unwrap();

        assert_eq!(first.id, second.id);
        assert_eq!(
            repository.get_note("2026-07-20").unwrap().unwrap().body,
            "second"
        );
    }

    #[test]
    fn goals_receive_stable_positions_within_the_week() {
        let database = Database::open_in_memory().unwrap();
        let repository = PlanningRepository::new(&database);
        for title in ["任务", "专注"] {
            repository
                .save_goal(&WeeklyGoalInput {
                    id: None,
                    week_starts_on: "2026-07-20".into(),
                    title: title.into(),
                    category: WeeklyGoalCategory::CompletedTasks,
                    target_count: 5,
                })
                .unwrap();
        }
        let goals = repository.list_goals("2026-07-20").unwrap();
        assert_eq!(
            goals.iter().map(|goal| goal.position).collect::<Vec<_>>(),
            vec![0, 1]
        );
    }
}
