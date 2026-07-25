use crate::domain::window::MainWindowState;

use super::widget_window::{restore_visible_rect, WindowRect};

pub const MAIN_WINDOW_LABEL: &str = "main";

impl MainWindowState {
    pub fn from_physical(
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        maximized: bool,
        monitor_id: Option<String>,
        scale_factor: f64,
    ) -> Self {
        Self {
            x: f64::from(x),
            y: f64::from(y),
            width: f64::from(width) / scale_factor,
            height: f64::from(height) / scale_factor,
            maximized,
            monitor_id,
            scale_factor,
        }
    }

    fn physical_rect(&self) -> WindowRect {
        WindowRect {
            x: self.x,
            y: self.y,
            width: self.width * self.scale_factor,
            height: self.height * self.scale_factor,
        }
    }
}

pub fn restored_main_window_rect(state: &MainWindowState, work_areas: &[WindowRect]) -> WindowRect {
    restore_visible_rect(state.physical_rect(), work_areas)
}

pub trait MainWindowActivationTarget {
    fn show_for_activation(&self) -> Result<(), crate::DomainError>;
    fn unminimize_for_activation(&self) -> Result<(), crate::DomainError>;
    fn focus_for_activation(&self) -> Result<(), crate::DomainError>;
}

pub fn activate_existing_instance_window(
    target: &impl MainWindowActivationTarget,
) -> Result<(), crate::DomainError> {
    target.show_for_activation()?;
    target.unminimize_for_activation()?;
    target.focus_for_activation()
}

#[cfg(feature = "desktop-app")]
impl MainWindowActivationTarget for tauri::WebviewWindow {
    fn show_for_activation(&self) -> Result<(), crate::DomainError> {
        self.show().map_err(window_error)
    }

    fn unminimize_for_activation(&self) -> Result<(), crate::DomainError> {
        self.unminimize().map_err(window_error)
    }

    fn focus_for_activation(&self) -> Result<(), crate::DomainError> {
        self.set_focus().map_err(window_error)
    }
}

#[cfg(feature = "desktop-app")]
pub struct MainWindowGeometryRuntime {
    sender: std::sync::mpsc::Sender<()>,
}

#[cfg(feature = "desktop-app")]
impl MainWindowGeometryRuntime {
    pub fn start(app: tauri::AppHandle) -> Result<Self, crate::DomainError> {
        let (sender, receiver) = std::sync::mpsc::channel();
        std::thread::Builder::new()
            .name("main-window-geometry-monitor".into())
            .spawn(move || geometry_worker(app, receiver))
            .map_err(window_error)?;
        Ok(Self { sender })
    }

    pub fn record_change(&self) {
        let _ = self.sender.send(());
    }
}

#[cfg(feature = "desktop-app")]
fn geometry_worker(app: tauri::AppHandle, receiver: std::sync::mpsc::Receiver<()>) {
    use std::sync::mpsc::RecvTimeoutError;
    use std::time::Duration;

    while receiver.recv().is_ok() {
        loop {
            match receiver.recv_timeout(Duration::from_millis(180)) {
                Ok(()) => continue,
                Err(RecvTimeoutError::Timeout) => break,
                Err(RecvTimeoutError::Disconnected) => return,
            }
        }
        if let Err(error) = persist_main_window_state(&app) {
            log::warn!("event=main_window_state_save_failed code={}", error.code);
        }
    }
}

#[cfg(feature = "desktop-app")]
pub fn restore_main_window(app: &tauri::AppHandle) -> Result<(), crate::DomainError> {
    use crate::repositories::{database::Database, preferences_repository::PreferencesRepository};
    use tauri::Manager;

    let window = app
        .get_webview_window(MAIN_WINDOW_LABEL)
        .ok_or_else(main_window_missing)?;
    let database = app.state::<Database>();
    if let Some(state) = PreferencesRepository::new(&database).get_main_window_state()? {
        let work_areas = monitor_work_areas(&window)?;
        window
            .set_size(tauri::LogicalSize::new(state.width, state.height))
            .map_err(window_error)?;
        if work_areas.is_empty() {
            window.center().map_err(window_error)?;
        } else {
            let restored = restored_main_window_rect(&state, &work_areas);
            window
                .set_position(tauri::PhysicalPosition::new(
                    restored.x.round() as i32,
                    restored.y.round() as i32,
                ))
                .map_err(window_error)?;
        }
        if state.maximized {
            window.maximize().map_err(window_error)?;
        }
    }
    window.show().map_err(window_error)?;
    window.set_focus().map_err(window_error)
}

#[cfg(feature = "desktop-app")]
pub fn persist_main_window_state(app: &tauri::AppHandle) -> Result<(), crate::DomainError> {
    use crate::repositories::{database::Database, preferences_repository::PreferencesRepository};
    use tauri::Manager;

    let window = app
        .get_webview_window(MAIN_WINDOW_LABEL)
        .ok_or_else(main_window_missing)?;
    let database = app.state::<Database>();
    let repository = PreferencesRepository::new(&database);
    let maximized = window.is_maximized().map_err(window_error)?;

    if window.is_minimized().map_err(window_error)? {
        return Ok(());
    }

    if maximized {
        if let Some(mut state) = repository.get_main_window_state()? {
            state.maximized = true;
            return repository.set_main_window_state(&state);
        }
    }

    let position = window.outer_position().map_err(window_error)?;
    let size = window.inner_size().map_err(window_error)?;
    let scale_factor = window.scale_factor().map_err(window_error)?;
    let monitor_id = window
        .current_monitor()
        .map_err(window_error)?
        .and_then(|monitor| monitor.name().cloned());
    repository.set_main_window_state(&MainWindowState::from_physical(
        position.x,
        position.y,
        size.width,
        size.height,
        maximized,
        monitor_id,
        scale_factor,
    ))
}

#[cfg(feature = "desktop-app")]
fn monitor_work_areas(
    window: &tauri::WebviewWindow,
) -> Result<Vec<WindowRect>, crate::DomainError> {
    let mut work_areas = Vec::new();
    if let Some(primary) = window.primary_monitor().map_err(window_error)? {
        work_areas.push(monitor_work_area(&primary));
    }
    for monitor in window.available_monitors().map_err(window_error)? {
        let work_area = monitor_work_area(&monitor);
        if !work_areas.contains(&work_area) {
            work_areas.push(work_area);
        }
    }
    Ok(work_areas)
}

#[cfg(feature = "desktop-app")]
fn monitor_work_area(monitor: &tauri::Monitor) -> WindowRect {
    let work_area = monitor.work_area();
    WindowRect {
        x: f64::from(work_area.position.x),
        y: f64::from(work_area.position.y),
        width: f64::from(work_area.size.width),
        height: f64::from(work_area.size.height),
    }
}

#[cfg(feature = "desktop-app")]
fn main_window_missing() -> crate::DomainError {
    crate::DomainError {
        code: "MAIN_WINDOW_MISSING".into(),
        message: "main window is unavailable".into(),
        field: None,
    }
}

#[cfg(feature = "desktop-app")]
fn window_error(error: impl std::fmt::Display) -> crate::DomainError {
    crate::DomainError {
        code: "MAIN_WINDOW_OPERATION_FAILED".into(),
        message: error.to_string(),
        field: None,
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, collections::VecDeque};

    use super::*;

    struct ActivationRecorder {
        calls: RefCell<Vec<&'static str>>,
        results: RefCell<VecDeque<Result<(), crate::DomainError>>>,
    }

    impl ActivationRecorder {
        fn successful() -> Self {
            Self {
                calls: RefCell::new(Vec::new()),
                results: RefCell::new(VecDeque::from([Ok(()), Ok(()), Ok(())])),
            }
        }

        fn record(&self, call: &'static str) -> Result<(), crate::DomainError> {
            self.calls.borrow_mut().push(call);
            self.results.borrow_mut().pop_front().unwrap()
        }
    }

    impl MainWindowActivationTarget for ActivationRecorder {
        fn show_for_activation(&self) -> Result<(), crate::DomainError> {
            self.record("show")
        }

        fn unminimize_for_activation(&self) -> Result<(), crate::DomainError> {
            self.record("unminimize")
        }

        fn focus_for_activation(&self) -> Result<(), crate::DomainError> {
            self.record("focus")
        }
    }

    #[test]
    fn physical_state_preserves_logical_size_and_maximized_flag() {
        let state = MainWindowState::from_physical(
            -120,
            80,
            1700,
            1075,
            true,
            Some("DISPLAY2".into()),
            1.25,
        );

        assert_eq!(state.x, -120.0);
        assert_eq!(state.y, 80.0);
        assert_eq!(state.width, 1360.0);
        assert_eq!(state.height, 860.0);
        assert!(state.maximized);
        assert_eq!(state.physical_rect().width, 1700.0);
    }

    #[test]
    fn second_instance_activation_shows_unminimizes_and_focuses_the_window() {
        let target = ActivationRecorder::successful();

        activate_existing_instance_window(&target).unwrap();

        assert_eq!(*target.calls.borrow(), ["show", "unminimize", "focus"]);
    }

    #[test]
    fn second_instance_activation_stops_after_a_window_error() {
        let target = ActivationRecorder {
            calls: RefCell::new(Vec::new()),
            results: RefCell::new(VecDeque::from([
                Ok(()),
                Err(crate::DomainError {
                    code: "MAIN_WINDOW_OPERATION_FAILED".into(),
                    message: "cannot restore".into(),
                    field: None,
                }),
                Ok(()),
            ])),
        };

        let error = activate_existing_instance_window(&target).unwrap_err();

        assert_eq!(error.code, "MAIN_WINDOW_OPERATION_FAILED");
        assert_eq!(*target.calls.borrow(), ["show", "unminimize"]);
    }

    #[test]
    fn main_window_restore_centers_an_offscreen_saved_state() {
        let state = MainWindowState {
            x: 8_000.0,
            y: -4_000.0,
            width: 1_360.0,
            height: 860.0,
            maximized: false,
            monitor_id: Some("REMOVED_DISPLAY".into()),
            scale_factor: 1.0,
        };
        let primary = WindowRect {
            x: 0.0,
            y: 0.0,
            width: 1_920.0,
            height: 1_040.0,
        };

        let restored = restored_main_window_rect(&state, &[primary]);

        assert_eq!(restored.x, 280.0);
        assert_eq!(restored.y, 90.0);
        assert!(super::super::widget_window::main_action_area_is_visible(
            restored,
            &[primary]
        ));
    }
}
