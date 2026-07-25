use chrono::{NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    domain::project::{
        ensure_transition, ProjectAggregation, ProjectInput, ProjectRemovalResolution,
        ProjectStatus,
    },
    repositories::{
        database::Database,
        project_repository::{ProjectRecord, ProjectRepository},
        task_repository::TaskRecord,
    },
    DomainError,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSummary {
    pub project: ProjectRecord,
    pub aggregation: ProjectAggregation,
    pub next_task_title: Option<String>,
    pub next_task_date: Option<String>,
    pub deadline_state: String,
    pub deadline_days: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectDetail {
    pub summary: ProjectSummary,
    pub tasks: Vec<TaskRecord>,
}

pub struct ProjectService<'a> {
    database: &'a Database,
}

impl<'a> ProjectService<'a> {
    pub fn new(database: &'a Database) -> Self {
        Self { database }
    }

    pub fn create(&self, input: ProjectInput) -> Result<ProjectRecord, DomainError> {
        input.validate()?;
        let now = Utc::now().to_rfc3339();
        let project = ProjectRecord {
            id: uuid::Uuid::new_v4().to_string(),
            name: input.name.trim().to_string(),
            description: input.description,
            color: input.color,
            icon: input.icon,
            status: ProjectStatus::Active.as_str().into(),
            started_on: input.started_on,
            target_on: input.target_on,
            created_at: now.clone(),
            updated_at: now,
        };
        ProjectRepository::new(self.database).insert(&project)?;
        Ok(project)
    }

    pub fn update(&self, id: &str, input: ProjectInput) -> Result<ProjectRecord, DomainError> {
        input.validate()?;
        let repository = ProjectRepository::new(self.database);
        let current = repository.get(id)?.ok_or_else(project_not_found)?;
        let project = ProjectRecord {
            id: current.id,
            name: input.name.trim().to_string(),
            description: input.description,
            color: input.color,
            icon: input.icon,
            status: current.status,
            started_on: input.started_on,
            target_on: input.target_on,
            created_at: current.created_at,
            updated_at: Utc::now().to_rfc3339(),
        };
        repository.update(&project)?;
        Ok(project)
    }

    pub fn set_status(
        &self,
        id: &str,
        target: ProjectStatus,
    ) -> Result<ProjectRecord, DomainError> {
        let repository = ProjectRepository::new(self.database);
        let current = repository.get(id)?.ok_or_else(project_not_found)?;
        ensure_transition(ProjectStatus::parse(&current.status)?, target)?;
        repository.set_status(id, target.as_str(), &Utc::now().to_rfc3339())?;
        repository.get(id)?.ok_or_else(project_not_found)
    }

    pub fn remove(
        &self,
        id: &str,
        resolution: ProjectRemovalResolution,
    ) -> Result<(), DomainError> {
        let repository = ProjectRepository::new(self.database);
        repository.get(id)?.ok_or_else(project_not_found)?;
        if resolution == ProjectRemovalResolution::Archive {
            self.set_status(id, ProjectStatus::Archived)?;
            return Ok(());
        }
        if repository.has_history(id)? {
            if resolution == ProjectRemovalResolution::Delete {
                return Err(DomainError {
                    code: "PROJECT_HAS_HISTORY".into(),
                    message: "project history must be archived or detached before removal".into(),
                    field: None,
                });
            }
            repository.detach_history(id)?;
        }
        repository.delete(id)?;
        Ok(())
    }

    pub fn list(
        &self,
        status: Option<ProjectStatus>,
        today: NaiveDate,
    ) -> Result<Vec<ProjectSummary>, DomainError> {
        let repository = ProjectRepository::new(self.database);
        repository
            .list()?
            .into_iter()
            .filter(|project| status.map_or(true, |value| project.status == value.as_str()))
            .map(|project| self.summary(project, today))
            .collect()
    }

    pub fn get(&self, id: &str, today: NaiveDate) -> Result<ProjectDetail, DomainError> {
        let repository = ProjectRepository::new(self.database);
        let project = repository.get(id)?.ok_or_else(project_not_found)?;
        let tasks = crate::repositories::task_repository::TaskRepository::new(self.database)
            .list()?
            .into_iter()
            .filter(|task| task.project_id.as_deref() == Some(id))
            .collect();
        Ok(ProjectDetail {
            summary: self.summary(project, today)?,
            tasks,
        })
    }

    fn summary(
        &self,
        project: ProjectRecord,
        today: NaiveDate,
    ) -> Result<ProjectSummary, DomainError> {
        let repository = ProjectRepository::new(self.database);
        let statuses = repository.task_statuses(&project.id)?;
        let status_refs: Vec<&str> = statuses.iter().map(String::as_str).collect();
        let focus_durations = repository.focus_durations(&project.id)?;
        let aggregation = ProjectAggregation::from_records(&status_refs, &focus_durations);
        let next_task = repository.next_task(&project.id)?;
        let (deadline_state, deadline_days) = deadline(&project, today);
        Ok(ProjectSummary {
            project,
            aggregation,
            next_task_title: next_task.as_ref().map(|item| item.0.clone()),
            next_task_date: next_task.and_then(|item| item.1),
            deadline_state,
            deadline_days,
        })
    }
}

fn deadline(project: &ProjectRecord, today: NaiveDate) -> (String, Option<i64>) {
    let Some(target) = project
        .target_on
        .as_deref()
        .and_then(|value| NaiveDate::parse_from_str(value, "%Y-%m-%d").ok())
    else {
        return ("none".into(), None);
    };
    let days = (target - today).num_days();
    let state = if days < 0 {
        "overdue"
    } else if days <= 7 {
        "atRisk"
    } else {
        "onTrack"
    };
    (state.into(), Some(days))
}

fn project_not_found() -> DomainError {
    DomainError {
        code: "PROJECT_NOT_FOUND".into(),
        message: "project was not found".into(),
        field: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input() -> ProjectInput {
        ProjectInput {
            name: "Arrive Focus".into(),
            description: "Local focus system".into(),
            color: "mint".into(),
            icon: "folder".into(),
            started_on: "2026-07-01".into(),
            target_on: Some("2026-07-25".into()),
        }
    }

    #[test]
    fn lifecycle_and_deadline_are_reported() {
        let database = Database::open_in_memory().unwrap();
        let service = ProjectService::new(&database);
        let project = service.create(input()).unwrap();
        service
            .set_status(&project.id, ProjectStatus::Paused)
            .unwrap();
        let summary = service
            .list(
                Some(ProjectStatus::Paused),
                NaiveDate::from_ymd_opt(2026, 7, 20).unwrap(),
            )
            .unwrap()
            .remove(0);
        assert_eq!(summary.project.status, "paused");
        assert_eq!(summary.deadline_state, "atRisk");
        assert_eq!(summary.deadline_days, Some(5));
    }

    #[test]
    fn project_with_history_requires_resolution() {
        let database = Database::open_in_memory().unwrap();
        let service = ProjectService::new(&database);
        let project = service.create(input()).unwrap();
        database.write(|tx| {
            tx.execute(
                "INSERT INTO tasks(id, project_id, title, category, priority, status, created_at, updated_at) VALUES ('task', ?1, 'Task', 'work', 0, 'pending', '2026-07-18T00:00:00Z', '2026-07-18T00:00:00Z')",
                [&project.id],
            )?;
            Ok(())
        }).unwrap();
        assert_eq!(
            service
                .remove(&project.id, ProjectRemovalResolution::Delete)
                .unwrap_err()
                .code,
            "PROJECT_HAS_HISTORY"
        );
        service
            .remove(&project.id, ProjectRemovalResolution::DetachHistory)
            .unwrap();
        assert!(ProjectRepository::new(&database)
            .get(&project.id)
            .unwrap()
            .is_none());
    }
}
