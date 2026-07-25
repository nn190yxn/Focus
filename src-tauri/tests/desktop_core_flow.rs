use std::sync::Mutex;

use arrive_focus_core::{
    domain::{
        calendar::{CalendarPeriod, CalendarQuery},
        focus::{FocusCompletionKind, FocusState, FocusTarget},
        notification::{ReminderWindow, SystemNotification},
        project::ProjectInput,
        recurrence::{RecurrencePattern, RecurrenceRule, RecurrenceStatus},
        settings::NotificationPermissionState,
        task::TaskInput,
        today::TodaySourceKind,
        widget::WidgetModule,
    },
    repositories::{database::Database, recurrence_repository::RecurrenceRepository},
    services::{
        backup_service::BackupService,
        calendar_service::CalendarService,
        focus_service::FocusService,
        notification_service::{NotificationPublisher, NotificationService},
        project_service::ProjectService,
        recurrence_service::{GenerationTrigger, RecurrenceScheduler, RecurrenceService},
        statistics_service::StatisticsService,
        task_service::TaskService,
        today_service::TodayService,
        widget_service::WidgetService,
    },
    DomainError,
};
use chrono::{Duration, Utc};

#[derive(Default)]
struct RecordingNotificationPublisher {
    notifications: Mutex<Vec<SystemNotification>>,
}

impl NotificationPublisher for RecordingNotificationPublisher {
    fn permission_state(&self) -> Result<NotificationPermissionState, DomainError> {
        Ok(NotificationPermissionState::Granted)
    }

    fn publish(&self, notification: &SystemNotification) -> Result<(), DomainError> {
        self.notifications
            .lock()
            .unwrap()
            .push(notification.clone());
        Ok(())
    }
}

#[test]
fn desktop_core_journey_reaches_review_and_portable_backup() {
    let database = Database::open_in_memory().unwrap();
    let now = Utc::now();
    let today = now.date_naive();
    let today_text = today.format("%Y-%m-%d").to_string();

    let project = ProjectService::new(&database)
        .create(ProjectInput {
            name: "Release readiness".into(),
            description: "Validate the complete desktop journey".into(),
            color: "mint".into(),
            icon: "target".into(),
            started_on: today_text.clone(),
            target_on: Some(today_text.clone()),
        })
        .unwrap();

    let task_service = TaskService::new(&database);
    let focus_task = task_service
        .create(
            TaskInput {
                project_id: Some(project.id.clone()),
                title: "Review release checklist".into(),
                category: "work".into(),
                priority: 3,
                scheduled_date: Some(today_text.clone()),
                scheduled_time: None,
                check_items: vec![],
            },
            today,
        )
        .unwrap();
    let recurring_template = task_service
        .create(
            TaskInput {
                project_id: Some(project.id.clone()),
                title: "Daily desktop check-in".into(),
                category: "work".into(),
                priority: 2,
                scheduled_date: None,
                scheduled_time: None,
                check_items: vec![],
            },
            today,
        )
        .unwrap();

    let rule_id = "release-daily-check-in";
    let generation = RecurrenceService::new(&database)
        .create_rule(
            RecurrenceRule {
                id: rule_id.into(),
                task_template_id: recurring_template.task.id.clone(),
                pattern: RecurrencePattern::Daily { interval: 1 },
                local_time: Some(now.format("%H:%M").to_string()),
                timezone: "UTC".into(),
                starts_on: today_text.clone(),
                ends_on: Some(today_text.clone()),
                status: RecurrenceStatus::Active,
                version: 1,
            },
            today,
            today,
        )
        .unwrap();
    assert_eq!(generation.scheduled_count, 1);
    assert_eq!(generation.affected_count, 1);

    let repeated_generation = RecurrenceScheduler::new(&database)
        .run(GenerationTrigger::DayBoundary, today, today)
        .unwrap();
    assert_eq!(repeated_generation[0].scheduled_count, 1);
    assert_eq!(repeated_generation[0].affected_count, 0);

    let instances = RecurrenceRepository::new(&database)
        .list_instances_for_rule(rule_id)
        .unwrap();
    assert_eq!(instances.len(), 1);
    let recurring_instance = &instances[0];

    let digest = TodayService::new(&database)
        .get_digest(&today_text)
        .unwrap();
    assert_eq!(digest.items.len(), 2);
    let recurring_digest_item = digest
        .items
        .iter()
        .find(|item| item.source_kind == TodaySourceKind::RecurringInstance)
        .unwrap();
    assert_eq!(recurring_digest_item.source_id, recurring_instance.id);
    assert_eq!(
        recurring_digest_item
            .project
            .as_ref()
            .map(|value| value.id.as_str()),
        Some(project.id.as_str())
    );

    let widget_service = WidgetService::new(&database);
    let widget = widget_service.get().unwrap();
    assert!(widget.input.modules.contains(&WidgetModule::TodayProgress));
    assert!(widget.input.modules.contains(&WidgetModule::Tasks));
    assert!(widget_service
        .mark_visible()
        .unwrap()
        .last_visible_at
        .is_some());

    let publisher = RecordingNotificationPublisher::default();
    let reminder_window = ReminderWindow {
        starts_at: (now - Duration::minutes(2)).fixed_offset(),
        ends_at: (now + Duration::minutes(2)).fixed_offset(),
    };
    let notification_service = NotificationService::new(&database);
    assert_eq!(
        notification_service
            .reconcile_task_reminders(reminder_window, &publisher)
            .unwrap(),
        1
    );
    assert_eq!(
        notification_service
            .reconcile_task_reminders(reminder_window, &publisher)
            .unwrap(),
        0
    );
    let notifications = publisher.notifications.lock().unwrap();
    assert_eq!(notifications.len(), 1);
    assert_eq!(notifications[0].title, "任务到时");
    assert_eq!(notifications[0].body, recurring_template.task.title);
    drop(notifications);

    let focus_service = FocusService::new(&database);
    let focus_state = focus_service
        .start(
            FocusTarget {
                task_id: Some(focus_task.task.id.clone()),
                task_instance_id: None,
            },
            1,
        )
        .unwrap();
    assert!(matches!(
        focus_state,
        FocusState::Running {
            task_id: Some(ref task_id),
            ..
        } if task_id == &focus_task.task.id
    ));
    let focus_session = focus_service.finish(FocusCompletionKind::Early).unwrap();
    assert_eq!(
        focus_session.project_id.as_deref(),
        Some(project.id.as_str())
    );
    assert_eq!(focus_session.completion_kind, FocusCompletionKind::Early);
    task_service
        .set_completed(&focus_task.task.id, true)
        .unwrap();

    let query = CalendarQuery {
        period: CalendarPeriod::Week,
        anchor_date: today_text.clone(),
        timezone: "UTC".into(),
        category: None,
        project_id: Some(project.id.clone()),
    };
    let calendar = CalendarService::new(&database)
        .get_period(query.clone())
        .unwrap();
    let review_day = calendar
        .days
        .iter()
        .find(|day| day.date == today_text)
        .unwrap();
    assert!(review_day
        .completed_tasks
        .iter()
        .any(|item| item.source_id == focus_task.task.id));
    assert!(review_day
        .focus_sessions
        .iter()
        .any(|item| item.id == focus_session.id));

    let statistics = StatisticsService::new(&database)
        .get_summary(query)
        .unwrap();
    assert_eq!(statistics.completed_task_count, 1);
    assert_eq!(statistics.effective_session_count, 1);
    assert_eq!(
        statistics.focus_seconds,
        focus_session.actual_seconds as u64
    );
    assert_eq!(statistics.project_investments.len(), 1);
    assert_eq!(statistics.project_investments[0].project.id, project.id);

    let backup_directory = tempfile::tempdir().unwrap();
    let backup_path = backup_directory.path().join("release-journey.json");
    let backup_result = BackupService::new(&database)
        .export_to_path(&backup_path, Utc::now())
        .unwrap();
    assert_eq!(backup_result.summary.counts.projects, 1);
    assert_eq!(backup_result.summary.counts.tasks, 2);
    assert_eq!(backup_result.summary.counts.recurrence_rules, 1);
    assert_eq!(backup_result.summary.counts.task_instances, 1);
    assert_eq!(backup_result.summary.counts.focus_sessions, 1);

    let inspected_backup = BackupService::inspect_path(&backup_path).unwrap();
    assert_eq!(inspected_backup.summary, backup_result.summary);
    assert_eq!(inspected_backup.envelope.data.projects[0].id, project.id);
    assert!(inspected_backup
        .envelope
        .data
        .tasks
        .iter()
        .any(|task| task.id == focus_task.task.id));
}
