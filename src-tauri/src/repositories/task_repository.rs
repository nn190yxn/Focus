use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};

use super::database::Database;
use crate::{domain::task::TaskListFilter, DomainError};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskRecord {
    pub id: String,
    pub project_id: Option<String>,
    pub title: String,
    pub category: String,
    pub priority: i64,
    pub scheduled_date: Option<String>,
    pub scheduled_time: Option<String>,
    pub status: String,
    pub completed_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckItemRecord {
    pub id: String,
    pub task_id: String,
    pub title: String,
    pub position: i64,
    pub completed_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskProjectSummary {
    pub id: String,
    pub name: String,
    pub color: String,
    pub icon: String,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskListItem {
    pub task: TaskRecord,
    pub project: Option<TaskProjectSummary>,
}

pub struct TaskRepository<'a> {
    database: &'a Database,
}

impl<'a> TaskRepository<'a> {
    pub fn new(database: &'a Database) -> Self {
        Self { database }
    }

    pub fn insert(&self, task: &TaskRecord) -> Result<(), DomainError> {
        self.database.write(|tx| {
            tx.execute(
                "INSERT INTO tasks(id, project_id, title, category, priority, scheduled_date, scheduled_time, status, completed_at, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![task.id, task.project_id, task.title, task.category, task.priority, task.scheduled_date, task.scheduled_time, task.status, task.completed_at, task.created_at, task.updated_at],
            )?;
            Ok(())
        })
    }

    pub fn insert_with_check_items(
        &self,
        task: &TaskRecord,
        check_items: &[CheckItemRecord],
    ) -> Result<(), DomainError> {
        self.database.write(|tx| {
            insert_task(tx, task)?;
            for item in check_items {
                insert_check_item(tx, item)?;
            }
            Ok(())
        })
    }

    pub fn get(&self, id: &str) -> Result<Option<TaskRecord>, DomainError> {
        self.database.read(|connection| {
            connection.query_row(
                "SELECT id, project_id, title, category, priority, scheduled_date, scheduled_time, status, completed_at, created_at, updated_at FROM tasks WHERE id = ?1",
                [id], map_task,
            ).optional()
        })
    }

    pub fn list(&self) -> Result<Vec<TaskRecord>, DomainError> {
        self.database.read(|connection| {
            let mut statement = connection.prepare("SELECT id, project_id, title, category, priority, scheduled_date, scheduled_time, status, completed_at, created_at, updated_at FROM tasks ORDER BY scheduled_date, scheduled_time, created_at, id")?;
            let tasks = statement.query_map([], map_task)?.collect();
            tasks
        })
    }

    pub fn list_filtered(&self, filter: &TaskListFilter) -> Result<Vec<TaskListItem>, DomainError> {
        self.database.read(|connection| {
            let mut sql = String::from(
                "SELECT t.id, t.project_id, t.title, t.category, t.priority, t.scheduled_date, t.scheduled_time, t.status, t.completed_at, t.created_at, t.updated_at, p.id, p.name, p.color, p.icon, p.status FROM tasks t LEFT JOIN projects p ON p.id = t.project_id WHERE t.status != 'removed'",
            );
            let mut values = Vec::<rusqlite::types::Value>::new();
            if let Some(value) = &filter.starts_on {
                sql.push_str(" AND t.scheduled_date >= ?");
                values.push(value.clone().into());
            }
            if let Some(value) = &filter.ends_on {
                sql.push_str(" AND t.scheduled_date <= ?");
                values.push(value.clone().into());
            }
            if let Some(value) = &filter.project_id {
                sql.push_str(" AND t.project_id = ?");
                values.push(value.clone().into());
            }
            if let Some(value) = &filter.category {
                sql.push_str(" AND t.category = ?");
                values.push(value.clone().into());
            }
            if let Some(value) = filter.completion {
                sql.push_str(" AND t.status = ?");
                values.push(value.as_str().to_string().into());
            }
            if let Some(value) = filter.search.as_deref().map(str::trim).filter(|value| !value.is_empty()) {
                sql.push_str(" AND instr(lower(t.title), lower(?)) > 0");
                values.push(value.to_string().into());
            }
            sql.push_str(
                " ORDER BY t.scheduled_date IS NULL, t.scheduled_date, t.scheduled_time IS NULL, t.scheduled_time, t.priority DESC, t.created_at, t.id",
            );
            let mut statement = connection.prepare(&sql)?;
            let items = statement
                .query_map(rusqlite::params_from_iter(values.iter()), map_task_list_item)?
                .collect();
            items
        })
    }

    pub fn list_check_items(&self, task_id: &str) -> Result<Vec<CheckItemRecord>, DomainError> {
        self.database.read(|connection| {
            let mut statement = connection.prepare(
                "SELECT id, task_id, title, position, completed_at, created_at, updated_at FROM task_check_items WHERE task_id = ?1 ORDER BY position, id",
            )?;
            let items = statement.query_map([task_id], map_check_item)?.collect();
            items
        })
    }

    pub fn update(&self, task: &TaskRecord) -> Result<bool, DomainError> {
        self.database.write(|tx| {
            let changed = tx.execute(
                "UPDATE tasks SET project_id = ?2, title = ?3, category = ?4, priority = ?5, scheduled_date = ?6, scheduled_time = ?7, status = ?8, completed_at = ?9, updated_at = ?10 WHERE id = ?1",
                params![task.id, task.project_id, task.title, task.category, task.priority, task.scheduled_date, task.scheduled_time, task.status, task.completed_at, task.updated_at],
            )?;
            Ok(changed == 1)
        })
    }

    pub fn update_with_check_items(
        &self,
        task: &TaskRecord,
        check_items: &[CheckItemRecord],
    ) -> Result<bool, DomainError> {
        self.database.write(|tx| {
            let changed = update_task(tx, task)?;
            tx.execute(
                "DELETE FROM task_check_items WHERE task_id = ?1",
                [&task.id],
            )?;
            for item in check_items {
                insert_check_item(tx, item)?;
            }
            Ok(changed)
        })
    }

    pub fn set_status(
        &self,
        id: &str,
        status: &str,
        completed_at: Option<&str>,
        updated_at: &str,
    ) -> Result<bool, DomainError> {
        self.database.write(|tx| {
            Ok(tx.execute(
                "UPDATE tasks SET status = ?2, completed_at = ?3, updated_at = ?4 WHERE id = ?1",
                params![id, status, completed_at, updated_at],
            )? == 1)
        })
    }

    pub fn set_check_item_completed(
        &self,
        task_id: &str,
        item_id: &str,
        completed_at: Option<&str>,
        updated_at: &str,
    ) -> Result<bool, DomainError> {
        self.database.write(|tx| {
            Ok(tx.execute(
                "UPDATE task_check_items SET completed_at = ?3, updated_at = ?4 WHERE task_id = ?1 AND id = ?2",
                params![task_id, item_id, completed_at, updated_at],
            )? == 1)
        })
    }

    pub fn reorder_check_items(
        &self,
        task_id: &str,
        ordered_ids: &[String],
        updated_at: &str,
    ) -> Result<bool, DomainError> {
        self.database.write(|tx| {
            let mut statement = tx.prepare(
                "SELECT id FROM task_check_items WHERE task_id = ?1 ORDER BY id",
            )?;
            let mut current_ids: Vec<String> = statement
                .query_map([task_id], |row| row.get(0))?
                .collect::<rusqlite::Result<_>>()?;
            let mut requested_ids = ordered_ids.to_vec();
            requested_ids.sort();
            current_ids.sort();
            drop(statement);
            if current_ids != requested_ids {
                return Ok(false);
            }
            tx.execute(
                "UPDATE task_check_items SET position = position + 1000000 WHERE task_id = ?1",
                [task_id],
            )?;
            for (position, id) in ordered_ids.iter().enumerate() {
                tx.execute(
                    "UPDATE task_check_items SET position = ?3, updated_at = ?4 WHERE task_id = ?1 AND id = ?2",
                    params![task_id, id, position as i64, updated_at],
                )?;
            }
            Ok(true)
        })
    }

    pub fn delete(&self, id: &str) -> Result<bool, DomainError> {
        self.database
            .write(|tx| Ok(tx.execute("DELETE FROM tasks WHERE id = ?1", [id])? == 1))
    }
}

fn insert_task(tx: &rusqlite::Transaction<'_>, task: &TaskRecord) -> rusqlite::Result<()> {
    tx.execute(
        "INSERT INTO tasks(id, project_id, title, category, priority, scheduled_date, scheduled_time, status, completed_at, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![task.id, task.project_id, task.title, task.category, task.priority, task.scheduled_date, task.scheduled_time, task.status, task.completed_at, task.created_at, task.updated_at],
    )?;
    Ok(())
}

fn update_task(tx: &rusqlite::Transaction<'_>, task: &TaskRecord) -> rusqlite::Result<bool> {
    Ok(tx.execute(
        "UPDATE tasks SET project_id = ?2, title = ?3, category = ?4, priority = ?5, scheduled_date = ?6, scheduled_time = ?7, status = ?8, completed_at = ?9, updated_at = ?10 WHERE id = ?1",
        params![task.id, task.project_id, task.title, task.category, task.priority, task.scheduled_date, task.scheduled_time, task.status, task.completed_at, task.updated_at],
    )? == 1)
}

fn insert_check_item(
    tx: &rusqlite::Transaction<'_>,
    item: &CheckItemRecord,
) -> rusqlite::Result<()> {
    tx.execute(
        "INSERT INTO task_check_items(id, task_id, title, position, completed_at, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![item.id, item.task_id, item.title, item.position, item.completed_at, item.created_at, item.updated_at],
    )?;
    Ok(())
}

fn map_task(row: &rusqlite::Row<'_>) -> rusqlite::Result<TaskRecord> {
    Ok(TaskRecord {
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
}

fn map_check_item(row: &rusqlite::Row<'_>) -> rusqlite::Result<CheckItemRecord> {
    Ok(CheckItemRecord {
        id: row.get(0)?,
        task_id: row.get(1)?,
        title: row.get(2)?,
        position: row.get(3)?,
        completed_at: row.get(4)?,
        created_at: row.get(5)?,
        updated_at: row.get(6)?,
    })
}

fn map_task_list_item(row: &rusqlite::Row<'_>) -> rusqlite::Result<TaskListItem> {
    let task = map_task(row)?;
    let project_id: Option<String> = row.get(11)?;
    let project = match project_id {
        Some(id) => Some(TaskProjectSummary {
            id,
            name: row.get(12)?,
            color: row.get(13)?,
            icon: row.get(14)?,
            status: row.get(15)?,
        }),
        None => None,
    };
    Ok(TaskListItem { task, project })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn task() -> TaskRecord {
        TaskRecord {
            id: "task-1".into(),
            project_id: None,
            title: "Define schema".into(),
            category: "work".into(),
            priority: 2,
            scheduled_date: Some("2026-07-18".into()),
            scheduled_time: Some("10:30".into()),
            status: "pending".into(),
            completed_at: None,
            created_at: "2026-07-18T00:00:00Z".into(),
            updated_at: "2026-07-18T00:00:00Z".into(),
        }
    }

    #[test]
    fn task_crud_round_trip() {
        let database = Database::open_in_memory().unwrap();
        let repository = TaskRepository::new(&database);
        let mut value = task();
        repository.insert(&value).unwrap();
        assert_eq!(repository.get(&value.id).unwrap(), Some(value.clone()));

        value.status = "completed".into();
        value.completed_at = Some("2026-07-18T02:00:00Z".into());
        value.updated_at = "2026-07-18T02:00:00Z".into();
        assert!(repository.update(&value).unwrap());
        assert_eq!(repository.list().unwrap(), vec![value.clone()]);
        assert!(repository.delete(&value.id).unwrap());
        assert!(repository.get(&value.id).unwrap().is_none());
    }
}
