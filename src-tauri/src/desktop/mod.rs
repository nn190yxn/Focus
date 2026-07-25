pub mod focus_events;
pub mod memo_events;
pub mod memo_notification_activation;
pub mod shell_attachment;
pub mod today_events;
pub mod tray;
pub mod widget_window;

pub mod lifecycle;

pub mod main_window;

#[cfg(feature = "desktop-app")]
pub mod notifications;

#[cfg(feature = "desktop-app")]
pub mod recurrence;

#[cfg(feature = "desktop-app")]
pub mod shortcuts;

#[cfg(feature = "desktop-app")]
pub mod widget_shell;

#[cfg(any(target_os = "windows", test))]
#[cfg_attr(all(test, not(target_os = "windows")), allow(dead_code))]
mod windows_shell;
