use chrono::{Duration, Local};
#[cfg(not(target_os = "windows"))]
use tauri::plugin::PermissionState;
use tauri_plugin_notification::NotificationExt;

use crate::{
    domain::{
        notification::{next_reminder_scan_cursor, ReminderWindow, SystemNotification},
        settings::NotificationPermissionState,
    },
    repositories::database::Database,
    services::notification_service::{NotificationPublisher, NotificationService},
    DomainError,
};

pub struct TauriNotificationPublisher<'a> {
    app: &'a tauri::AppHandle,
}

impl<'a> TauriNotificationPublisher<'a> {
    pub fn new(app: &'a tauri::AppHandle) -> Self {
        Self { app }
    }
}

impl NotificationPublisher for TauriNotificationPublisher<'_> {
    fn permission_state(&self) -> Result<NotificationPermissionState, DomainError> {
        #[cfg(target_os = "windows")]
        {
            // The desktop notification plugin delegates permission management to Windows.
            return Ok(NotificationPermissionState::Unknown);
        }

        #[cfg(not(target_os = "windows"))]
        let state = self
            .app
            .notification()
            .permission_state()
            .map_err(notification_error)?;
        #[cfg(not(target_os = "windows"))]
        Ok(match state {
            PermissionState::Granted => NotificationPermissionState::Granted,
            PermissionState::Denied => NotificationPermissionState::Denied,
            PermissionState::Prompt | PermissionState::PromptWithRationale => {
                NotificationPermissionState::Unknown
            }
        })
    }

    fn publish(&self, notification: &SystemNotification) -> Result<(), DomainError> {
        if self.permission_state()? == NotificationPermissionState::Denied {
            return Err(DomainError {
                code: "NOTIFICATION_DENIED".into(),
                message: "notification permission is unavailable".into(),
                field: None,
            });
        }
        #[cfg(target_os = "windows")]
        if let Some(crate::domain::notification::SystemNotificationActivation::OpenMemo {
            memo_id,
        }) = &notification.activation
        {
            return show_windows_memo_notification(self.app, notification, memo_id);
        }
        let builder = self
            .app
            .notification()
            .builder()
            .title(&notification.title)
            .body(&notification.body);
        let builder = if notification.sound_enabled {
            builder.sound("Default")
        } else {
            builder
        };
        builder.show().map_err(notification_error)
    }
}

#[cfg(target_os = "windows")]
fn show_windows_memo_notification(
    app: &tauri::AppHandle,
    notification: &SystemNotification,
    memo_id: &str,
) -> Result<(), DomainError> {
    use tauri::Manager;
    use tauri_winrt_notification::{Sound, Toast};

    let app_handle = app.clone();
    let activation_memo_id = memo_id.to_owned();
    let sound = notification.sound_enabled.then_some(Sound::Default);
    Toast::new(&app.config().identifier)
        .title(&notification.title)
        .text1(&notification.body)
        .sound(sound)
        .on_activated(move |_| {
            if let Err(error) = crate::desktop::memo_notification_activation::activate(
                &app_handle,
                &activation_memo_id,
            ) {
                log::warn!(
                    "event=memo_notification_activation_failed code={}",
                    error.code
                );
            }
            Ok(())
        })
        .show()
        .map_err(notification_error)
}

pub fn start_notification_worker(app: tauri::AppHandle) -> Result<(), DomainError> {
    std::thread::Builder::new()
        .name("task-notification-worker".into())
        .spawn(move || {
            let mut previous_scan = Local::now().fixed_offset() - Duration::minutes(5);
            loop {
                let _ = crate::desktop::recurrence::reconcile_and_emit(
                    &app,
                    crate::services::recurrence_service::GenerationTrigger::DayBoundary,
                );
                let now = Local::now().fixed_offset();
                let window = continuous_reminder_window(previous_scan, now);
                let reconciliation = reconcile_notifications_in_window(&app, window);
                previous_scan =
                    next_reminder_scan_cursor(previous_scan, now, reconciliation.is_ok());
                std::thread::sleep(std::time::Duration::from_secs(15));
            }
        })
        .map_err(|error| DomainError {
            code: "NOTIFICATION_WORKER_FAILED".into(),
            message: error.to_string(),
            field: None,
        })?;
    Ok(())
}

pub fn reconcile_task_notifications(app: &tauri::AppHandle) -> Result<usize, DomainError> {
    let now = Local::now().fixed_offset();
    reconcile_task_notifications_in_window(
        app,
        ReminderWindow {
            starts_at: now - Duration::minutes(5),
            ends_at: now,
        },
    )
}

fn reconcile_task_notifications_in_window(
    app: &tauri::AppHandle,
    window: ReminderWindow,
) -> Result<usize, DomainError> {
    use tauri::Manager;

    let database = app.state::<Database>();
    NotificationService::new(&database)
        .reconcile_task_reminders(window, &TauriNotificationPublisher::new(app))
}

fn reconcile_notifications_in_window(
    app: &tauri::AppHandle,
    window: ReminderWindow,
) -> Result<usize, DomainError> {
    use tauri::Manager;

    let database = app.state::<Database>();
    let service = NotificationService::new(&database);
    let publisher = TauriNotificationPublisher::new(app);
    service.reconcile_reminder_worker_cycle(window, "Untitled memo", &publisher)
}

fn continuous_reminder_window(
    previous_scan: chrono::DateTime<chrono::FixedOffset>,
    now: chrono::DateTime<chrono::FixedOffset>,
) -> ReminderWindow {
    let starts_at = if previous_scan <= now {
        previous_scan
    } else {
        now - Duration::minutes(5)
    };
    ReminderWindow {
        starts_at,
        ends_at: now,
    }
}

pub fn open_notification_settings() -> Result<(), DomainError> {
    #[cfg(target_os = "windows")]
    {
        use std::{ffi::OsStr, iter::once, os::windows::ffi::OsStrExt, ptr};
        use windows_sys::Win32::UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOWNORMAL};

        let operation: Vec<u16> = OsStr::new("open").encode_wide().chain(once(0)).collect();
        let target: Vec<u16> = OsStr::new("ms-settings:notifications")
            .encode_wide()
            .chain(once(0))
            .collect();
        let result = unsafe {
            ShellExecuteW(
                ptr::null_mut(),
                operation.as_ptr(),
                target.as_ptr(),
                ptr::null(),
                ptr::null(),
                SW_SHOWNORMAL,
            )
        } as isize;
        if result > 32 {
            return Ok(());
        }
        return Err(DomainError {
            code: "NOTIFICATION_SETTINGS_OPEN_FAILED".into(),
            message: format!("Windows settings returned code {result}"),
            field: None,
        });
    }

    #[cfg(not(target_os = "windows"))]
    Err(DomainError {
        code: "NOTIFICATION_SETTINGS_UNSUPPORTED".into(),
        message: "notification settings are only available on Windows".into(),
        field: None,
    })
}

fn notification_error(error: impl std::fmt::Display) -> DomainError {
    DomainError {
        code: "NOTIFICATION_SEND_FAILED".into(),
        message: error.to_string(),
        field: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{FixedOffset, TimeZone};

    fn at(hour: u32, minute: u32) -> chrono::DateTime<FixedOffset> {
        FixedOffset::east_opt(8 * 3600)
            .unwrap()
            .with_ymd_and_hms(2026, 7, 20, hour, minute, 0)
            .unwrap()
    }

    #[test]
    fn continuous_window_covers_a_long_system_pause() {
        let window = continuous_reminder_window(at(9, 0), at(11, 30));
        assert_eq!(window.starts_at, at(9, 0));
        assert_eq!(window.ends_at, at(11, 30));
    }

    #[test]
    fn continuous_window_recovers_from_a_backward_clock_change() {
        let window = continuous_reminder_window(at(9, 30), at(9, 0));
        assert_eq!(window.starts_at, at(8, 55));
        assert_eq!(window.ends_at, at(9, 0));
    }
}
