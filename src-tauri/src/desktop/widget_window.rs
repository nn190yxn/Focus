use crate::domain::widget::{WidgetConfigInput, WidgetMode};

const MAIN_ACTION_AREA_WIDTH: f64 = 160.0;
const MAIN_ACTION_AREA_HEIGHT: f64 = 48.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WindowRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl WindowRect {
    fn main_action_area(self) -> Self {
        Self {
            width: self.width.min(MAIN_ACTION_AREA_WIDTH),
            height: self.height.min(MAIN_ACTION_AREA_HEIGHT),
            ..self
        }
    }

    fn contains(self, other: Self) -> bool {
        other.x >= self.x
            && other.y >= self.y
            && other.x + other.width <= self.x + self.width
            && other.y + other.height <= self.y + self.height
    }
}

pub fn main_action_area_is_visible(rect: WindowRect, work_areas: &[WindowRect]) -> bool {
    let main_action_area = rect.main_action_area();
    work_areas
        .iter()
        .any(|work_area| work_area.contains(main_action_area))
}

pub fn restore_visible_rect(rect: WindowRect, work_areas: &[WindowRect]) -> WindowRect {
    if work_areas.is_empty() || main_action_area_is_visible(rect, work_areas) {
        return rect;
    }

    let main_action_area = rect.main_action_area();
    let primary = work_areas
        .iter()
        .find(|work_area| {
            work_area.width >= main_action_area.width && work_area.height >= main_action_area.height
        })
        .unwrap_or(&work_areas[0]);
    WindowRect {
        x: if rect.width <= primary.width {
            primary.x + (primary.width - rect.width) / 2.0
        } else {
            primary.x
        },
        y: if rect.height <= primary.height {
            primary.y + (primary.height - rect.height) / 2.0
        } else {
            primary.y
        },
        ..rect
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WidgetWindowBehavior {
    pub always_on_top: bool,
    pub ignore_cursor_events: bool,
    pub resizable: bool,
}

impl WidgetWindowBehavior {
    pub fn new(mode: WidgetMode, locked: bool) -> Self {
        Self {
            always_on_top: mode == WidgetMode::Floating,
            ignore_cursor_events: locked,
            resizable: !locked,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct WidgetWindowGeometry {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub monitor_id: Option<String>,
    pub scale_factor: f64,
}

impl WidgetWindowGeometry {
    pub fn from_physical(
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        monitor_id: Option<String>,
        scale_factor: f64,
    ) -> Self {
        Self {
            x: f64::from(x),
            y: f64::from(y),
            width: f64::from(width) / scale_factor,
            height: f64::from(height) / scale_factor,
            monitor_id,
            scale_factor,
        }
    }

    pub fn apply_position_to(&self, input: &mut WidgetConfigInput) {
        input.x = self.x;
        input.y = self.y;
        input.monitor_id.clone_from(&self.monitor_id);
        input.scale_factor = self.scale_factor;
    }

    pub fn apply_to(self, input: &mut WidgetConfigInput) {
        self.apply_position_to(input);
        input.width = self.width;
        input.height = self.height;
    }
}

#[cfg(feature = "desktop-app")]
pub struct WidgetGeometryRuntime {
    sender: std::sync::mpsc::Sender<()>,
}

#[cfg(feature = "desktop-app")]
impl WidgetGeometryRuntime {
    pub fn start(app: tauri::AppHandle) -> Result<Self, crate::DomainError> {
        let (sender, receiver) = std::sync::mpsc::channel();
        std::thread::Builder::new()
            .name("widget-geometry-monitor".into())
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
        let _ = persist_widget_geometry(&app);
    }
}

#[cfg(feature = "desktop-app")]
pub fn apply_widget_window_behavior(
    window: &tauri::WebviewWindow,
    mode: WidgetMode,
    locked: bool,
) -> Result<(), crate::DomainError> {
    let behavior = WidgetWindowBehavior::new(mode, locked);
    window
        .set_always_on_top(behavior.always_on_top)
        .map_err(window_error)?;
    window
        .set_resizable(behavior.resizable)
        .map_err(window_error)?;
    window
        .set_ignore_cursor_events(behavior.ignore_cursor_events)
        .map_err(window_error)?;
    Ok(())
}

#[cfg(feature = "desktop-app")]
pub fn restored_widget_position(
    window: &tauri::WebviewWindow,
    input: &WidgetConfigInput,
) -> Result<tauri::PhysicalPosition<i32>, crate::DomainError> {
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

    let restored = restore_visible_rect(
        WindowRect {
            x: input.x,
            y: input.y,
            width: input.width * input.scale_factor,
            height: input.height * input.scale_factor,
        },
        &work_areas,
    );
    Ok(tauri::PhysicalPosition::new(
        restored.x.round() as i32,
        restored.y.round() as i32,
    ))
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
pub(crate) fn persist_widget_geometry(app: &tauri::AppHandle) -> Result<(), crate::DomainError> {
    use crate::{repositories::database::Database, services::widget_service::WidgetService};
    use tauri::Manager;

    let geometry = current_widget_geometry(app)?;
    let database = app.state::<Database>();
    let service = WidgetService::new(&database);
    let mut input = service.get()?.input;
    geometry.apply_to(&mut input);
    service.update(input)?;
    Ok(())
}

#[cfg(feature = "desktop-app")]
pub(crate) fn current_widget_geometry(
    app: &tauri::AppHandle,
) -> Result<WidgetWindowGeometry, crate::DomainError> {
    use crate::domain::widget::WIDGET_WINDOW_LABEL;
    use tauri::Manager;

    let window = app
        .get_webview_window(WIDGET_WINDOW_LABEL)
        .ok_or_else(|| crate::DomainError {
            code: "WIDGET_WINDOW_MISSING".into(),
            message: "widget window is unavailable".into(),
            field: None,
        })?;
    let position = window.outer_position().map_err(window_error)?;
    let size = window.inner_size().map_err(window_error)?;
    let scale_factor = window.scale_factor().map_err(window_error)?;
    let monitor_id = window
        .current_monitor()
        .map_err(window_error)?
        .and_then(|monitor| monitor.name().cloned());
    Ok(WidgetWindowGeometry::from_physical(
        position.x,
        position.y,
        size.width,
        size.height,
        monitor_id,
        scale_factor,
    ))
}

#[cfg(feature = "desktop-app")]
fn window_error(error: impl std::fmt::Display) -> crate::DomainError {
    crate::DomainError {
        code: "WIDGET_WINDOW_OPERATION_FAILED".into(),
        message: error.to_string(),
        field: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn floating_mode_is_always_on_top() {
        let behavior = WidgetWindowBehavior::new(WidgetMode::Floating, false);
        assert_eq!(
            behavior,
            WidgetWindowBehavior {
                always_on_top: true,
                ignore_cursor_events: false,
                resizable: true,
            }
        );
    }

    #[test]
    fn desktop_mode_stays_below_regular_windows() {
        let behavior = WidgetWindowBehavior::new(WidgetMode::Desktop, false);
        assert!(!behavior.always_on_top);
        assert!(behavior.resizable);
    }

    #[test]
    fn locked_mode_is_click_through_and_fixed_size() {
        for mode in [WidgetMode::Desktop, WidgetMode::Floating] {
            let behavior = WidgetWindowBehavior::new(mode, true);
            assert!(behavior.ignore_cursor_events);
            assert!(!behavior.resizable);
        }
    }

    #[test]
    fn physical_geometry_is_stored_with_logical_dimensions() {
        let geometry =
            WidgetWindowGeometry::from_physical(-120, 80, 720, 840, Some("DISPLAY2".into()), 2.0);
        let mut input = WidgetConfigInput::default();
        geometry.apply_to(&mut input);

        assert_eq!(input.x, -120.0);
        assert_eq!(input.y, 80.0);
        assert_eq!(input.width, 360.0);
        assert_eq!(input.height, 420.0);
        assert_eq!(input.monitor_id.as_deref(), Some("DISPLAY2"));
        assert_eq!(input.scale_factor, 2.0);
    }

    #[test]
    fn position_merge_preserves_requested_widget_size() {
        let geometry =
            WidgetWindowGeometry::from_physical(920, 180, 720, 840, Some("DISPLAY2".into()), 2.0);
        let mut input = WidgetConfigInput {
            width: 440.0,
            height: 640.0,
            ..WidgetConfigInput::default()
        };

        geometry.apply_position_to(&mut input);

        assert_eq!(input.x, 920.0);
        assert_eq!(input.y, 180.0);
        assert_eq!(input.width, 440.0);
        assert_eq!(input.height, 640.0);
        assert_eq!(input.monitor_id.as_deref(), Some("DISPLAY2"));
        assert_eq!(input.scale_factor, 2.0);
    }

    #[test]
    fn offscreen_window_is_centered_in_the_primary_work_area() {
        let restored = restore_visible_rect(
            WindowRect {
                x: 5000.0,
                y: -3000.0,
                width: 360.0,
                height: 420.0,
            },
            &[WindowRect {
                x: 0.0,
                y: 0.0,
                width: 1920.0,
                height: 1040.0,
            }],
        );

        assert_eq!(restored.x, 780.0);
        assert_eq!(restored.y, 310.0);
    }

    #[test]
    fn visible_window_keeps_its_saved_position() {
        let saved = WindowRect {
            x: -1200.0,
            y: 80.0,
            width: 440.0,
            height: 640.0,
        };
        let work_areas = [
            WindowRect {
                x: 0.0,
                y: 0.0,
                width: 1920.0,
                height: 1040.0,
            },
            WindowRect {
                x: -1280.0,
                y: 0.0,
                width: 1280.0,
                height: 984.0,
            },
        ];

        assert_eq!(restore_visible_rect(saved, &work_areas), saved);
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(256))]

        #[test]
        fn p8_restored_window_keeps_its_main_action_area_visible(
            saved in (
                -100_000i32..100_000,
                -100_000i32..100_000,
                1u32..8_000,
                1u32..8_000,
            ),
            monitors in prop::collection::vec(
                (
                    -20_000i32..20_000,
                    -20_000i32..20_000,
                    160u32..8_000,
                    48u32..5_000,
                ),
                1..6,
            ),
        ) {
            let saved = WindowRect {
                x: f64::from(saved.0),
                y: f64::from(saved.1),
                width: f64::from(saved.2),
                height: f64::from(saved.3),
            };
            let work_areas = monitors
                .into_iter()
                .map(|monitor| WindowRect {
                    x: f64::from(monitor.0),
                    y: f64::from(monitor.1),
                    width: f64::from(monitor.2),
                    height: f64::from(monitor.3),
                })
                .collect::<Vec<_>>();

            let restored = restore_visible_rect(saved, &work_areas);

            prop_assert!(main_action_area_is_visible(restored, &work_areas));
            if main_action_area_is_visible(saved, &work_areas) {
                prop_assert_eq!(restored, saved);
            }
        }
    }
}
