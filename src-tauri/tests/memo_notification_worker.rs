use std::{cell::RefCell, path::Path, sync::Mutex};

use arrive_focus_core::{
    desktop::memo_notification_activation::{
        activate, MemoNotificationActivationTarget, MEMO_OPEN_REQUESTED_EVENT,
    },
    domain::{
        memo::{MemoInput, MemoReminderInput, MemoReminderStatus},
        notification::{
            NotificationKind, ReminderWindow, SystemNotification, SystemNotificationActivation,
        },
        settings::NotificationPermissionState,
    },
    repositories::{
        database::Database, memo_repository::MemoRepository,
        notification_repository::NotificationRepository,
    },
    services::{
        memo_reminder_service::MemoReminderService,
        memo_service::MemoService,
        notification_service::{NotificationPublisher, NotificationService},
    },
    DomainError,
};
use chrono::{DateTime, Duration, Utc};

const MEMO_ID: &str = "ebbc2524-ae61-4ae7-b62e-5cc8eb6ed112";
const UNTITLED_LABEL: &str = "Untitled memo";

struct RecordingPublisher {
    permission: NotificationPermissionState,
    notifications: Mutex<Vec<SystemNotification>>,
}

impl RecordingPublisher {
    fn granted() -> Self {
        Self {
            permission: NotificationPermissionState::Granted,
            notifications: Mutex::new(Vec::new()),
        }
    }

    fn denied() -> Self {
        Self {
            permission: NotificationPermissionState::Denied,
            notifications: Mutex::new(Vec::new()),
        }
    }
}

impl NotificationPublisher for RecordingPublisher {
    fn permission_state(&self) -> Result<NotificationPermissionState, DomainError> {
        Ok(self.permission)
    }

    fn publish(&self, notification: &SystemNotification) -> Result<(), DomainError> {
        if self.permission == NotificationPermissionState::Denied {
            return Err(DomainError {
                code: "NOTIFICATION_DENIED".into(),
                message: "notification permission is unavailable".into(),
                field: None,
            });
        }
        self.notifications
            .lock()
            .unwrap()
            .push(notification.clone());
        Ok(())
    }
}

#[derive(Default)]
struct RecordingActivationTarget {
    calls: RefCell<Vec<String>>,
}

impl MemoNotificationActivationTarget for RecordingActivationTarget {
    fn show_main_window(&self) -> Result<(), DomainError> {
        self.calls.borrow_mut().push("show".into());
        Ok(())
    }

    fn emit_open_requested(&self, memo_id: &str) -> Result<(), DomainError> {
        self.calls
            .borrow_mut()
            .push(format!("{MEMO_OPEN_REQUESTED_EVENT}:{memo_id}"));
        Ok(())
    }
}

fn open_database(path: &Path) -> Database {
    Database::open(path).unwrap()
}

fn persist_due_memo(database: &Database, due_at: DateTime<Utc>) -> (String, String) {
    let created_at = due_at - Duration::minutes(5);
    let input = MemoInput {
        title: "Review notes".into(),
        body: "Prepare the next action".into(),
        tags: vec![],
        pinned: false,
        reminder: Some(MemoReminderInput::Once {
            scheduled_local: due_at.format("%Y-%m-%dT%H:%M").to_string(),
            timezone: "UTC".into(),
        }),
    };
    let memo = MemoService::create(MEMO_ID.into(), &input, created_at).unwrap();
    let reminder =
        MemoReminderService::prepare_rule(MEMO_ID, None, input.reminder.as_ref(), created_at)
            .unwrap()
            .unwrap();
    let reminder_id = reminder.id.clone();
    let scheduled_for = reminder.next_scheduled_for.clone().unwrap();
    MemoRepository::new(database)
        .create(&memo, &[], Some(&reminder), UNTITLED_LABEL)
        .unwrap();
    (reminder_id, scheduled_for)
}

fn worker_window(due_at: DateTime<Utc>) -> ReminderWindow {
    ReminderWindow {
        starts_at: (due_at - Duration::minutes(5)).fixed_offset(),
        ends_at: due_at.fixed_offset(),
    }
}

fn delivery_state(database: &Database) -> (i64, String, Option<String>) {
    database
        .read(|connection| {
            connection.query_row(
                "SELECT COUNT(*), status, last_error FROM notification_deliveries",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
        })
        .unwrap()
}

fn reminder_status(database: &Database) -> MemoReminderStatus {
    MemoRepository::new(database)
        .get(MEMO_ID, UNTITLED_LABEL)
        .unwrap()
        .unwrap()
        .reminder
        .unwrap()
        .status
}

#[test]
fn worker_retries_after_permission_recovery_and_routes_notification_click() {
    let directory = tempfile::tempdir().unwrap();
    let database_path = directory.path().join("permission-retry.sqlite3");
    let due_at = DateTime::parse_from_rfc3339("2026-07-23T10:05:00Z")
        .unwrap()
        .with_timezone(&Utc);
    let database = open_database(&database_path);
    persist_due_memo(&database, due_at);

    let error = NotificationService::new(&database)
        .reconcile_reminder_worker_cycle(
            worker_window(due_at),
            UNTITLED_LABEL,
            &RecordingPublisher::denied(),
        )
        .unwrap_err();
    assert_eq!(error.code, "NOTIFICATION_DENIED");
    assert_eq!(
        delivery_state(&database),
        (1, "failed".into(), Some("NOTIFICATION_DENIED".into()))
    );
    assert_eq!(reminder_status(&database), MemoReminderStatus::Active);
    drop(database);

    let database = open_database(&database_path);
    let publisher = RecordingPublisher::granted();
    assert_eq!(
        NotificationService::new(&database)
            .reconcile_reminder_worker_cycle(worker_window(due_at), UNTITLED_LABEL, &publisher)
            .unwrap(),
        1
    );
    assert_eq!(delivery_state(&database), (1, "sent".into(), None));
    assert_eq!(reminder_status(&database), MemoReminderStatus::Completed);

    let notifications = publisher.notifications.lock().unwrap();
    assert_eq!(notifications.len(), 1);
    assert_eq!(
        notifications[0].activation,
        Some(SystemNotificationActivation::OpenMemo {
            memo_id: MEMO_ID.into(),
        })
    );
    let target = RecordingActivationTarget::default();
    let Some(SystemNotificationActivation::OpenMemo { memo_id }) = &notifications[0].activation
    else {
        panic!("memo reminder notification must include activation data");
    };
    activate(&target, memo_id).unwrap();
    assert_eq!(
        *target.calls.borrow(),
        [
            "show".to_string(),
            format!("{MEMO_OPEN_REQUESTED_EVENT}:{MEMO_ID}"),
        ]
    );
}

#[test]
fn worker_reclaims_an_expired_lease_after_process_restart() {
    let directory = tempfile::tempdir().unwrap();
    let database_path = directory.path().join("lease-recovery.sqlite3");
    let due_at = Utc::now() - Duration::minutes(2);
    let database = open_database(&database_path);
    let (reminder_id, scheduled_for) = persist_due_memo(&database, due_at);
    assert_eq!(
        NotificationRepository::new(&database)
            .reserve(
                NotificationKind::MemoReminder,
                &reminder_id,
                &scheduled_for,
                true,
            )
            .unwrap(),
        arrive_focus_core::repositories::notification_repository::NotificationReservation::Acquired
    );
    database
        .write(|transaction| {
            transaction.execute(
                "UPDATE notification_deliveries SET created_at = ?1 WHERE kind = 'memoReminder'",
                [(Utc::now() - Duration::minutes(2)).to_rfc3339()],
            )?;
            Ok(())
        })
        .unwrap();
    drop(database);

    let database = open_database(&database_path);
    let publisher = RecordingPublisher::granted();
    assert_eq!(
        NotificationService::new(&database)
            .reconcile_reminder_worker_cycle(worker_window(Utc::now()), UNTITLED_LABEL, &publisher)
            .unwrap(),
        1
    );
    assert_eq!(publisher.notifications.lock().unwrap().len(), 1);
    assert_eq!(delivery_state(&database), (1, "sent".into(), None));
    assert_eq!(reminder_status(&database), MemoReminderStatus::Completed);
}

#[test]
fn worker_advances_a_sent_occurrence_after_process_interruption_without_republishing() {
    let directory = tempfile::tempdir().unwrap();
    let database_path = directory.path().join("sent-recovery.sqlite3");
    let due_at = DateTime::parse_from_rfc3339("2026-07-23T10:05:00Z")
        .unwrap()
        .with_timezone(&Utc);
    let database = open_database(&database_path);
    let (reminder_id, scheduled_for) = persist_due_memo(&database, due_at);
    let repository = NotificationRepository::new(&database);
    repository
        .reserve(
            NotificationKind::MemoReminder,
            &reminder_id,
            &scheduled_for,
            true,
        )
        .unwrap();
    repository
        .mark_sent(NotificationKind::MemoReminder, &reminder_id, &scheduled_for)
        .unwrap();
    assert_eq!(reminder_status(&database), MemoReminderStatus::Active);
    drop(database);

    let database = open_database(&database_path);
    let publisher = RecordingPublisher::granted();
    assert_eq!(
        NotificationService::new(&database)
            .reconcile_reminder_worker_cycle(worker_window(due_at), UNTITLED_LABEL, &publisher)
            .unwrap(),
        1
    );
    assert!(publisher.notifications.lock().unwrap().is_empty());
    assert_eq!(delivery_state(&database), (1, "sent".into(), None));
    assert_eq!(reminder_status(&database), MemoReminderStatus::Completed);
}
