use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};

use super::database::Database;
use crate::DomainError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectRecord {
    pub id: String,
    pub name: String,
    pub description: String,
    pub color: String,
    pub icon: String,
    pub status: String,
    pub started_on: String,
    pub target_on: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

pub struct ProjectRepository<'a> {
    database: &'a Database,
}

impl<'a> ProjectRepository<'a> {
    pub fn new(database: &'a Database) -> Self {
        Self { database }
    }

    pub fn insert(&self, project: &ProjectRecord) -> Result<(), DomainError> {
        self.database.write(|tx| {
            tx.execute(
                "INSERT INTO projects(id, name, description, color, icon, status, started_on, target_on, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![project.id, project.name, project.description, project.color, project.icon, project.status, project.started_on, project.target_on, project.created_at, project.updated_at],
            )?;
            Ok(())
        })
    }

    pub fn get(&self, id: &str) -> Result<Option<ProjectRecord>, DomainError> {
        self.database.read(|connection| {
            connection.query_row(
                "SELECT id, name, description, color, icon, status, started_on, target_on, created_at, updated_at FROM projects WHERE id = ?1",
                [id],
                map_project,
            ).optional()
        })
    }

    pub fn list(&self) -> Result<Vec<ProjectRecord>, DomainError> {
        self.database.read(|connection| {
            let mut statement = connection.prepare("SELECT id, name, description, color, icon, status, started_on, target_on, created_at, updated_at FROM projects ORDER BY created_at, id")?;
            let projects = statement.query_map([], map_project)?.collect();
            projects
        })
    }

    pub fn update(&self, project: &ProjectRecord) -> Result<bool, DomainError> {
        self.database.write(|tx| {
            let changed = tx.execute(
                "UPDATE projects SET name = ?2, description = ?3, color = ?4, icon = ?5, status = ?6, started_on = ?7, target_on = ?8, updated_at = ?9 WHERE id = ?1",
                params![project.id, project.name, project.description, project.color, project.icon, project.status, project.started_on, project.target_on, project.updated_at],
            )?;
            Ok(changed == 1)
        })
    }

    pub fn set_status(
        &self,
        id: &str,
        status: &str,
        updated_at: &str,
    ) -> Result<bool, DomainError> {
        self.database.write(|tx| {
            Ok(tx.execute(
                "UPDATE projects SET status = ?2, updated_at = ?3 WHERE id = ?1",
                params![id, status, updated_at],
            )? == 1)
        })
    }

    pub fn task_statuses(&self, id: &str) -> Result<Vec<String>, DomainError> {
        self.database.read(|connection| {
            let mut statement =
                connection.prepare("SELECT status FROM tasks WHERE project_id = ?1 ORDER BY id")?;
            let statuses = statement.query_map([id], |row| row.get(0))?.collect();
            statuses
        })
    }

    pub fn focus_durations(&self, id: &str) -> Result<Vec<u64>, DomainError> {
        self.database.read(|connection| {
            let mut statement = connection.prepare(
                "SELECT actual_seconds FROM focus_sessions WHERE project_id = ?1 ORDER BY id",
            )?;
            let durations = statement.query_map([id], |row| row.get(0))?.collect();
            durations
        })
    }

    pub fn next_task(&self, id: &str) -> Result<Option<(String, Option<String>)>, DomainError> {
        self.database.read(|connection| {
            connection.query_row(
                "SELECT title, scheduled_date FROM tasks WHERE project_id = ?1 AND status = 'pending' ORDER BY scheduled_date IS NULL, scheduled_date, scheduled_time IS NULL, scheduled_time, created_at LIMIT 1",
                [id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            ).optional()
        })
    }

    pub fn has_history(&self, id: &str) -> Result<bool, DomainError> {
        self.database.read(|connection| {
            connection.query_row(
                "SELECT EXISTS(
                    SELECT 1 FROM tasks WHERE project_id = ?1
                    UNION ALL SELECT 1 FROM focus_sessions WHERE project_id = ?1
                    UNION ALL SELECT 1 FROM task_instances WHERE snapshot_project_id = ?1
                )",
                [id],
                |row| row.get(0),
            )
        })
    }

    pub fn detach_history(&self, id: &str) -> Result<(), DomainError> {
        self.database.write(|tx| {
            tx.execute("UPDATE tasks SET project_id = NULL WHERE project_id = ?1", [id])?;
            tx.execute("UPDATE focus_sessions SET project_id = NULL WHERE project_id = ?1", [id])?;
            tx.execute("UPDATE task_instances SET snapshot_project_id = NULL WHERE snapshot_project_id = ?1", [id])?;
            Ok(())
        })
    }

    pub fn delete(&self, id: &str) -> Result<bool, DomainError> {
        self.database
            .write(|tx| Ok(tx.execute("DELETE FROM projects WHERE id = ?1", [id])? == 1))
    }
}

fn map_project(row: &rusqlite::Row<'_>) -> rusqlite::Result<ProjectRecord> {
    Ok(ProjectRecord {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project() -> ProjectRecord {
        ProjectRecord {
            id: "project-1".into(),
            name: "Arrive".into(),
            description: "Desktop focus".into(),
            color: "mint".into(),
            icon: "folder".into(),
            status: "active".into(),
            started_on: "2026-07-18".into(),
            target_on: Some("2026-08-18".into()),
            created_at: "2026-07-18T00:00:00Z".into(),
            updated_at: "2026-07-18T00:00:00Z".into(),
        }
    }

    #[test]
    fn project_crud_round_trip() {
        let database = Database::open_in_memory().unwrap();
        let repository = ProjectRepository::new(&database);
        let mut value = project();
        repository.insert(&value).unwrap();
        assert_eq!(repository.get(&value.id).unwrap(), Some(value.clone()));

        value.name = "Arrive Focus".into();
        value.updated_at = "2026-07-18T01:00:00Z".into();
        assert!(repository.update(&value).unwrap());
        assert_eq!(repository.list().unwrap(), vec![value.clone()]);
        assert!(repository.delete(&value.id).unwrap());
        assert!(repository.get(&value.id).unwrap().is_none());
    }
}
