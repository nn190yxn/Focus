use chrono::{NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    domain::task::{task_error, CheckItemInput, TaskInput, TaskListFilter},
    repositories::{
        database::Database,
        project_repository::ProjectRepository,
        task_repository::{CheckItemRecord, TaskListItem, TaskRecord, TaskRepository},
    },
    DomainError,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskDetail {
    pub task: TaskRecord,
    pub check_items: Vec<CheckItemRecord>,
}

pub struct TaskService<'a> {
    database: &'a Database,
}

impl<'a> TaskService<'a> {
    pub fn new(database: &'a Database) -> Self {
        Self { database }
    }

    pub fn create(&self, input: TaskInput, today: NaiveDate) -> Result<TaskDetail, DomainError> {
        input.validate(today)?;
        self.validate_project(input.project_id.as_deref())?;
        let now = Utc::now().to_rfc3339();
        let task_id = uuid::Uuid::new_v4().to_string();
        let task = TaskRecord {
            id: task_id.clone(),
            project_id: input.project_id,
            title: input.title.trim().to_string(),
            category: input.category,
            priority: input.priority,
            scheduled_date: input.scheduled_date,
            scheduled_time: input.scheduled_time,
            status: "pending".into(),
            completed_at: None,
            created_at: now.clone(),
            updated_at: now.clone(),
        };
        let check_items = make_check_items(&task_id, input.check_items, &now);
        TaskRepository::new(self.database).insert_with_check_items(&task, &check_items)?;
        Ok(TaskDetail { task, check_items })
    }

    pub fn update(
        &self,
        id: &str,
        input: TaskInput,
        today: NaiveDate,
    ) -> Result<TaskDetail, DomainError> {
        input.validate(today)?;
        self.validate_project(input.project_id.as_deref())?;
        let repository = TaskRepository::new(self.database);
        let current = repository.get(id)?.ok_or_else(task_not_found)?;
        ensure_mutable(&current)?;
        let now = Utc::now().to_rfc3339();
        let task = TaskRecord {
            id: current.id,
            project_id: input.project_id,
            title: input.title.trim().to_string(),
            category: input.category,
            priority: input.priority,
            scheduled_date: input.scheduled_date,
            scheduled_time: input.scheduled_time,
            status: current.status,
            completed_at: current.completed_at,
            created_at: current.created_at,
            updated_at: now.clone(),
        };
        let check_items = make_check_items(&task.id, input.check_items, &now);
        repository.update_with_check_items(&task, &check_items)?;
        Ok(TaskDetail { task, check_items })
    }

    pub fn set_completed(&self, id: &str, completed: bool) -> Result<TaskDetail, DomainError> {
        let repository = TaskRepository::new(self.database);
        let current = repository.get(id)?.ok_or_else(task_not_found)?;
        ensure_mutable(&current)?;
        let now = Utc::now().to_rfc3339();
        let completed_at = completed.then_some(now.as_str());
        repository.set_status(
            id,
            if completed { "completed" } else { "pending" },
            completed_at,
            &now,
        )?;
        self.get(id)
    }

    pub fn remove(&self, id: &str) -> Result<(), DomainError> {
        let repository = TaskRepository::new(self.database);
        let current = repository.get(id)?.ok_or_else(task_not_found)?;
        ensure_mutable(&current)?;
        let now = Utc::now().to_rfc3339();
        repository.set_status(id, "removed", None, &now)?;
        Ok(())
    }

    pub fn set_check_item_completed(
        &self,
        task_id: &str,
        item_id: &str,
        completed: bool,
    ) -> Result<TaskDetail, DomainError> {
        let repository = TaskRepository::new(self.database);
        let task = repository.get(task_id)?.ok_or_else(task_not_found)?;
        ensure_mutable(&task)?;
        let now = Utc::now().to_rfc3339();
        if !repository.set_check_item_completed(
            task_id,
            item_id,
            completed.then_some(now.as_str()),
            &now,
        )? {
            return Err(check_item_not_found());
        }
        self.get(task_id)
    }

    pub fn reorder_check_items(
        &self,
        task_id: &str,
        ordered_ids: &[String],
    ) -> Result<TaskDetail, DomainError> {
        let repository = TaskRepository::new(self.database);
        let task = repository.get(task_id)?.ok_or_else(task_not_found)?;
        ensure_mutable(&task)?;
        if !repository.reorder_check_items(task_id, ordered_ids, &Utc::now().to_rfc3339())? {
            return Err(task_error(
                "CHECK_ITEM_ORDER_INVALID",
                "check item order must contain every item exactly once",
                Some("orderedIds"),
            ));
        }
        self.get(task_id)
    }

    pub fn get(&self, id: &str) -> Result<TaskDetail, DomainError> {
        let repository = TaskRepository::new(self.database);
        let task = repository.get(id)?.ok_or_else(task_not_found)?;
        let check_items = repository.list_check_items(id)?;
        Ok(TaskDetail { task, check_items })
    }

    pub fn list(&self, filter: TaskListFilter) -> Result<Vec<TaskListItem>, DomainError> {
        filter.validate()?;
        TaskRepository::new(self.database).list_filtered(&filter)
    }

    fn validate_project(&self, project_id: Option<&str>) -> Result<(), DomainError> {
        if let Some(id) = project_id {
            if ProjectRepository::new(self.database).get(id)?.is_none() {
                return Err(task_error(
                    "TASK_PROJECT_NOT_FOUND",
                    "selected project was not found",
                    Some("projectId"),
                ));
            }
        }
        Ok(())
    }
}

fn make_check_items(task_id: &str, inputs: Vec<CheckItemInput>, now: &str) -> Vec<CheckItemRecord> {
    inputs
        .into_iter()
        .enumerate()
        .map(|(position, input)| CheckItemRecord {
            id: input.id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
            task_id: task_id.to_string(),
            title: input.title.trim().to_string(),
            position: position as i64,
            completed_at: input.completed.then(|| now.to_string()),
            created_at: now.to_string(),
            updated_at: now.to_string(),
        })
        .collect()
}

fn ensure_mutable(task: &TaskRecord) -> Result<(), DomainError> {
    if task.status == "removed" {
        Err(task_error(
            "TASK_REMOVED",
            "removed task cannot be changed",
            None,
        ))
    } else {
        Ok(())
    }
}

fn task_not_found() -> DomainError {
    task_error("TASK_NOT_FOUND", "task was not found", None)
}

fn check_item_not_found() -> DomainError {
    task_error("CHECK_ITEM_NOT_FOUND", "check item was not found", None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repositories::project_repository::{ProjectRecord, ProjectRepository};

    fn input() -> TaskInput {
        TaskInput {
            project_id: None,
            title: "Implement commands".into(),
            category: "work".into(),
            priority: 3,
            scheduled_date: Some("2026-07-18".into()),
            scheduled_time: Some("14:00".into()),
            check_items: vec![
                CheckItemInput {
                    id: Some("check-1".into()),
                    title: "Create service".into(),
                    completed: false,
                },
                CheckItemInput {
                    id: Some("check-2".into()),
                    title: "Register command".into(),
                    completed: false,
                },
            ],
        }
    }

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 7, 18).unwrap()
    }

    #[test]
    fn creates_completes_and_restores_a_task() {
        let database = Database::open_in_memory().unwrap();
        let service = TaskService::new(&database);
        let created = service.create(input(), today()).unwrap();
        assert_eq!(created.check_items.len(), 2);

        let completed = service.set_completed(&created.task.id, true).unwrap();
        assert_eq!(completed.task.status, "completed");
        assert!(completed.task.completed_at.is_some());

        let restored = service.set_completed(&created.task.id, false).unwrap();
        assert_eq!(restored.task.status, "pending");
        assert!(restored.task.completed_at.is_none());
    }

    #[test]
    fn completes_and_reorders_check_items() {
        let database = Database::open_in_memory().unwrap();
        let service = TaskService::new(&database);
        let created = service.create(input(), today()).unwrap();

        let changed = service
            .set_check_item_completed(&created.task.id, "check-1", true)
            .unwrap();
        assert!(changed.check_items[0].completed_at.is_some());

        let reordered = service
            .reorder_check_items(&created.task.id, &["check-2".into(), "check-1".into()])
            .unwrap();
        assert_eq!(reordered.check_items[0].id, "check-2");
    }

    #[test]
    fn removal_preserves_the_task_as_history() {
        let database = Database::open_in_memory().unwrap();
        let service = TaskService::new(&database);
        let created = service.create(input(), today()).unwrap();
        service.remove(&created.task.id).unwrap();
        assert_eq!(
            service.get(&created.task.id).unwrap().task.status,
            "removed"
        );
    }

    #[test]
    fn rejects_an_unknown_project() {
        let database = Database::open_in_memory().unwrap();
        let service = TaskService::new(&database);
        let mut value = input();
        value.project_id = Some("missing".into());
        let error = service.create(value, today()).unwrap_err();
        assert_eq!(error.code, "TASK_PROJECT_NOT_FOUND");
    }

    #[test]
    fn associates_and_clears_a_project_during_updates() {
        let database = Database::open_in_memory().unwrap();
        ProjectRepository::new(&database)
            .insert(&ProjectRecord {
                id: "project-1".into(),
                name: "Desktop app".into(),
                description: String::new(),
                color: "mint".into(),
                icon: "folder".into(),
                status: "active".into(),
                started_on: "2026-07-18".into(),
                target_on: None,
                created_at: "2026-07-18T00:00:00Z".into(),
                updated_at: "2026-07-18T00:00:00Z".into(),
            })
            .unwrap();
        let service = TaskService::new(&database);
        let mut value = input();
        value.project_id = Some("project-1".into());

        let created = service.create(value.clone(), today()).unwrap();
        assert_eq!(created.task.project_id.as_deref(), Some("project-1"));
        let associated = service
            .list(TaskListFilter {
                project_id: Some("project-1".into()),
                ..TaskListFilter::default()
            })
            .unwrap();
        assert_eq!(associated[0].project.as_ref().unwrap().name, "Desktop app");

        value.project_id = None;
        let updated = service.update(&created.task.id, value, today()).unwrap();
        assert!(updated.task.project_id.is_none());
        assert!(service
            .list(TaskListFilter {
                project_id: Some("project-1".into()),
                ..TaskListFilter::default()
            })
            .unwrap()
            .is_empty());
    }

    #[test]
    fn filters_tasks_and_returns_stable_project_summaries() {
        let database = Database::open_in_memory().unwrap();
        ProjectRepository::new(&database)
            .insert(&ProjectRecord {
                id: "project-1".into(),
                name: "Desktop app".into(),
                description: String::new(),
                color: "mint".into(),
                icon: "folder".into(),
                status: "active".into(),
                started_on: "2026-07-18".into(),
                target_on: None,
                created_at: "2026-07-18T00:00:00Z".into(),
                updated_at: "2026-07-18T00:00:00Z".into(),
            })
            .unwrap();
        let service = TaskService::new(&database);

        let mut lower_priority = input();
        lower_priority.title = "Rust query basics".into();
        lower_priority.priority = 1;
        lower_priority.project_id = Some("project-1".into());
        lower_priority.scheduled_time = Some("10:00".into());
        lower_priority.check_items.clear();
        service.create(lower_priority, today()).unwrap();

        let mut higher_priority = input();
        higher_priority.title = "RUST query filters".into();
        higher_priority.priority = 3;
        higher_priority.project_id = Some("project-1".into());
        higher_priority.scheduled_time = Some("10:00".into());
        higher_priority.check_items.clear();
        service.create(higher_priority, today()).unwrap();

        let mut excluded = input();
        excluded.title = "Write React view".into();
        excluded.category = "study".into();
        excluded.check_items.clear();
        service.create(excluded, today()).unwrap();

        let items = service
            .list(TaskListFilter {
                starts_on: Some("2026-07-18".into()),
                ends_on: Some("2026-07-18".into()),
                project_id: Some("project-1".into()),
                category: Some("work".into()),
                completion: Some(crate::domain::task::TaskCompletionFilter::Pending),
                search: Some("rust query".into()),
            })
            .unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(items[0].task.title, "RUST query filters");
        assert_eq!(items[1].task.title, "Rust query basics");
        assert_eq!(items[0].project.as_ref().unwrap().name, "Desktop app");

        service.set_completed(&items[0].task.id, true).unwrap();
        let completed = service
            .list(TaskListFilter {
                completion: Some(crate::domain::task::TaskCompletionFilter::Completed),
                ..TaskListFilter::default()
            })
            .unwrap();
        assert_eq!(completed.len(), 1);
        assert_eq!(completed[0].task.id, items[0].task.id);
    }

    #[test]
    fn list_excludes_removed_tasks() {
        let database = Database::open_in_memory().unwrap();
        let service = TaskService::new(&database);
        let created = service.create(input(), today()).unwrap();
        service.remove(&created.task.id).unwrap();
        assert!(service.list(TaskListFilter::default()).unwrap().is_empty());
    }

    #[test]
    fn removed_tasks_reject_further_state_changes() {
        let database = Database::open_in_memory().unwrap();
        let service = TaskService::new(&database);
        let created = service.create(input(), today()).unwrap();
        service.remove(&created.task.id).unwrap();

        assert_eq!(
            service
                .set_completed(&created.task.id, true)
                .unwrap_err()
                .code,
            "TASK_REMOVED"
        );
        assert_eq!(
            service.remove(&created.task.id).unwrap_err().code,
            "TASK_REMOVED"
        );
    }
}
