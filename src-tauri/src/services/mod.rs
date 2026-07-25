pub mod backup_service;
pub mod calendar_service;
pub mod focus_service;
pub mod memo_reminder_service;
pub mod memo_service;
pub mod notification_service;
pub mod planning_service;
pub mod project_service;
pub mod recurrence_service;
pub mod settings_service;
pub mod statistics_service;
pub mod task_service;
pub mod today_service;
pub mod widget_service;

pub trait DomainEventPublisher: Send + Sync {
    fn publish(&self, event: DomainEvent);
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DomainEvent {
    pub name: &'static str,
    pub entity_id: String,
    pub version: crate::DomainVersion,
}
