pub mod backup_repository;
pub mod calendar_repository;
pub mod database;
pub mod focus_repository;
pub mod memo_repository;
pub mod notification_repository;
pub mod planning_repository;
pub mod preferences_repository;
pub mod project_repository;
pub mod recurrence_repository;
pub mod task_repository;
pub mod today_repository;
pub mod widget_repository;

pub trait Repository {
    type Entity;

    fn get(&self, id: &str) -> Result<Option<Self::Entity>, crate::DomainError>;
}
