use std::{sync::Mutex, time::Duration};

use serde::Serialize;
use tauri::{Emitter, Manager};

use crate::{
    desktop::shell_attachment::{
        AppliedShellMode, ShellAttachmentManager, ShellAttachmentOutcome, ShellFallbackReason,
    },
    domain::widget::WidgetMode,
    DomainError,
};

#[cfg(not(target_os = "windows"))]
use crate::desktop::shell_attachment::{ShellAttachError, ShellHostAdapter};

#[cfg(target_os = "windows")]
use crate::desktop::windows_shell::WindowsShellAdapter as PlatformShellAdapter;

#[cfg(not(target_os = "windows"))]
#[derive(Default)]
struct PlatformShellAdapter {
    _private: (),
}

#[cfg(not(target_os = "windows"))]
impl ShellHostAdapter for PlatformShellAdapter {
    type Host = usize;

    fn discover_host(&self) -> Result<Self::Host, ShellAttachError> {
        Err(ShellAttachError::new(
            ShellFallbackReason::UnsupportedPlatform,
            "desktop attachment requires Windows",
        ))
    }

    fn is_host_valid(&self, _host: Self::Host) -> bool {
        false
    }

    fn attach(&self, _window: usize, _host: Self::Host) -> Result<(), ShellAttachError> {
        Err(ShellAttachError::new(
            ShellFallbackReason::UnsupportedPlatform,
            "desktop attachment requires Windows",
        ))
    }

    fn detach(&self, _window: usize) -> Result<(), ShellAttachError> {
        Ok(())
    }
}

pub struct WidgetShellRuntime {
    manager: Mutex<ShellAttachmentManager<PlatformShellAdapter>>,
}

impl Default for WidgetShellRuntime {
    fn default() -> Self {
        Self {
            manager: Mutex::new(ShellAttachmentManager::new(PlatformShellAdapter::default())),
        }
    }
}

pub use crate::desktop::shell_attachment::AppliedShellMode as AppliedWidgetMode;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct WidgetModeFallbackEvent {
    from_mode: WidgetMode,
    to_mode: WidgetMode,
    reason: ShellFallbackReason,
}

pub fn apply_widget_mode(
    app: &tauri::AppHandle,
    window: &tauri::WebviewWindow,
    mode: WidgetMode,
) -> Result<AppliedWidgetMode, DomainError> {
    let native_window = native_window_handle(window)?;
    let state = app.state::<WidgetShellRuntime>();
    let mut manager = state.manager.lock().map_err(|_| shell_state_error())?;
    let outcome = match mode {
        WidgetMode::Desktop => manager.set_desktop_mode(native_window),
        WidgetMode::Floating => manager
            .set_floating_mode(native_window)
            .map_err(shell_operation_error)?,
    };
    apply_outcome(window, outcome)
}

pub fn start_widget_shell_monitor(app: tauri::AppHandle) -> Result<(), DomainError> {
    std::thread::Builder::new()
        .name("widget-shell-monitor".into())
        .spawn(move || loop {
            std::thread::sleep(Duration::from_secs(2));
            let Some(window) = app.get_webview_window("widget") else {
                break;
            };
            let Ok(native_window) = native_window_handle(&window) else {
                continue;
            };
            let state = app.state::<WidgetShellRuntime>();
            let Ok(mut manager) = state.manager.lock() else {
                break;
            };
            let outcome = manager.recover_if_needed(native_window);
            let _ = apply_outcome(&window, outcome);
        })
        .map(|_| ())
        .map_err(shell_operation_error)
}

fn apply_outcome(
    window: &tauri::WebviewWindow,
    outcome: ShellAttachmentOutcome,
) -> Result<AppliedWidgetMode, DomainError> {
    let applied_mode = outcome.applied_mode();
    window
        .set_always_on_top(applied_mode == AppliedShellMode::Floating)
        .map_err(shell_operation_error)?;
    if outcome.recovered() {
        window
            .emit("widget://mode-restored", ())
            .map_err(shell_operation_error)?;
    }
    match outcome {
        ShellAttachmentOutcome::DesktopAttached { .. } | ShellAttachmentOutcome::Floating => {
            Ok(applied_mode)
        }
        ShellAttachmentOutcome::FloatingFallback {
            reason,
            should_notify,
        } => {
            if should_notify {
                window
                    .emit(
                        "widget://mode-fallback",
                        WidgetModeFallbackEvent {
                            from_mode: WidgetMode::Desktop,
                            to_mode: WidgetMode::Floating,
                            reason,
                        },
                    )
                    .map_err(shell_operation_error)?;
            }
            Ok(applied_mode)
        }
    }
}

#[cfg(target_os = "windows")]
fn native_window_handle(window: &tauri::WebviewWindow) -> Result<usize, DomainError> {
    window
        .hwnd()
        .map(|handle| handle.0 as usize)
        .map_err(shell_operation_error)
}

#[cfg(not(target_os = "windows"))]
fn native_window_handle(_window: &tauri::WebviewWindow) -> Result<usize, DomainError> {
    Ok(0)
}

fn shell_operation_error(error: impl std::fmt::Display) -> DomainError {
    DomainError {
        code: "WIDGET_SHELL_OPERATION_FAILED".into(),
        message: error.to_string(),
        field: None,
    }
}

fn shell_state_error() -> DomainError {
    DomainError {
        code: "WIDGET_SHELL_STATE_UNAVAILABLE".into(),
        message: "widget shell state is unavailable".into(),
        field: None,
    }
}
