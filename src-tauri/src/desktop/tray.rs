use crate::domain::focus::FocusState;

#[cfg(feature = "desktop-app")]
const DEFAULT_FOCUS_MINUTES: u16 = 25;
#[cfg(feature = "desktop-app")]
const TRAY_ID: &str = "main-controls";
#[cfg(feature = "desktop-app")]
const SHOW_MAIN_ID: &str = "tray_show_main";
#[cfg(feature = "desktop-app")]
const SHOW_WIDGET_ID: &str = "tray_show_widget";
#[cfg(feature = "desktop-app")]
const FOCUS_TOGGLE_ID: &str = "tray_focus_toggle";
#[cfg(feature = "desktop-app")]
const QUICK_TASK_ID: &str = "tray_quick_task";
#[cfg(feature = "desktop-app")]
const UNLOCK_WIDGET_ID: &str = "tray_unlock_widget";
#[cfg(feature = "desktop-app")]
const EXIT_ID: &str = "tray_exit";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayFocusAction {
    Start,
    Pause,
    Resume,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrayMenuModel {
    pub task_label: String,
    pub remaining_label: String,
    pub focus_label: String,
    pub focus_enabled: bool,
    pub focus_action: TrayFocusAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskbarProgressStatus {
    None,
    Normal,
    Paused,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TaskbarProgressModel {
    pub status: TaskbarProgressStatus,
    pub progress: Option<u64>,
}

impl TaskbarProgressModel {
    pub fn from_focus_state(state: &FocusState) -> Self {
        match state {
            FocusState::Ready { .. } => Self {
                status: TaskbarProgressStatus::None,
                progress: None,
            },
            FocusState::Running {
                planned_seconds,
                remaining_seconds,
                ..
            } => Self::active(
                TaskbarProgressStatus::Normal,
                *planned_seconds,
                *remaining_seconds,
            ),
            FocusState::Paused {
                planned_seconds,
                remaining_seconds,
                ..
            } => Self::active(
                TaskbarProgressStatus::Paused,
                *planned_seconds,
                *remaining_seconds,
            ),
        }
    }

    fn active(status: TaskbarProgressStatus, planned: i64, remaining: i64) -> Self {
        let planned = planned.max(1) as u64;
        let remaining = remaining.clamp(0, planned as i64) as u64;
        Self {
            status,
            progress: Some((remaining * 100).div_ceil(planned)),
        }
    }
}

impl TrayMenuModel {
    pub fn from_focus_state(state: &FocusState, task_title: Option<&str>) -> Self {
        match state {
            FocusState::Ready { .. } => Self {
                task_label: task_title
                    .map(|title| format!("下一项：{title}"))
                    .unwrap_or_else(|| "下一项：暂无待完成任务".into()),
                remaining_label: "剩余：--:--".into(),
                focus_label: if task_title.is_some() {
                    "开始专注（25 分钟）".into()
                } else {
                    "打开专注空间".into()
                },
                focus_enabled: true,
                focus_action: TrayFocusAction::Start,
            },
            FocusState::Running {
                remaining_seconds, ..
            } => Self {
                task_label: format!("当前：{}", task_title.unwrap_or("进行中的专注")),
                remaining_label: format!("剩余：{}", format_remaining(*remaining_seconds)),
                focus_label: "暂停专注".into(),
                focus_enabled: true,
                focus_action: TrayFocusAction::Pause,
            },
            FocusState::Paused {
                remaining_seconds, ..
            } => Self {
                task_label: format!("当前：{}", task_title.unwrap_or("已暂停的专注")),
                remaining_label: format!(
                    "剩余：{}（已暂停）",
                    format_remaining(*remaining_seconds)
                ),
                focus_label: "继续专注".into(),
                focus_enabled: true,
                focus_action: TrayFocusAction::Resume,
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainWindowCloseAction {
    HideToTray,
    Close,
}

pub fn main_window_close_action(background_running: bool) -> MainWindowCloseAction {
    if background_running {
        MainWindowCloseAction::HideToTray
    } else {
        MainWindowCloseAction::Close
    }
}

fn format_remaining(seconds: i64) -> String {
    let seconds = seconds.max(0);
    format!("{:02}:{:02}", seconds / 60, seconds % 60)
}

#[cfg(feature = "desktop-app")]
pub fn setup_tray(app: &tauri::App) -> Result<(), crate::DomainError> {
    use tauri::{
        image::Image,
        menu::{Menu, MenuItem, PredefinedMenuItem},
        tray::TrayIconBuilder,
    };

    let initial = load_menu_model(app.handle())?;
    let task = MenuItem::with_id(app, "tray_task", initial.task_label, false, None::<&str>)
        .map_err(tray_error)?;
    let remaining = MenuItem::with_id(
        app,
        "tray_remaining",
        initial.remaining_label,
        false,
        None::<&str>,
    )
    .map_err(tray_error)?;
    let show_main = MenuItem::with_id(app, SHOW_MAIN_ID, "显示主窗口", true, None::<&str>)
        .map_err(tray_error)?;
    let show_widget = MenuItem::with_id(app, SHOW_WIDGET_ID, "显示小组件", true, None::<&str>)
        .map_err(tray_error)?;
    let focus_toggle = MenuItem::with_id(
        app,
        FOCUS_TOGGLE_ID,
        initial.focus_label,
        initial.focus_enabled,
        None::<&str>,
    )
    .map_err(tray_error)?;
    let quick_task = MenuItem::with_id(app, QUICK_TASK_ID, "创建快速任务", true, None::<&str>)
        .map_err(tray_error)?;
    let unlock_widget = MenuItem::with_id(app, UNLOCK_WIDGET_ID, "解锁小组件", true, None::<&str>)
        .map_err(tray_error)?;
    let exit = MenuItem::with_id(app, EXIT_ID, "退出抵达 Focus", true, None::<&str>)
        .map_err(tray_error)?;
    let separator_one = PredefinedMenuItem::separator(app).map_err(tray_error)?;
    let separator_two = PredefinedMenuItem::separator(app).map_err(tray_error)?;
    let separator_three = PredefinedMenuItem::separator(app).map_err(tray_error)?;
    let menu = Menu::with_items(
        app,
        &[
            &task,
            &remaining,
            &separator_one,
            &show_main,
            &show_widget,
            &separator_two,
            &focus_toggle,
            &quick_task,
            &unlock_widget,
            &separator_three,
            &exit,
        ],
    )
    .map_err(tray_error)?;
    let icon = app
        .default_window_icon()
        .cloned()
        .unwrap_or_else(|| Image::new_owned([90, 194, 159, 255].repeat(16 * 16), 16, 16));

    TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon)
        .tooltip("抵达 Focus")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| {
            if let Err(error) = handle_menu_action(app, event.id().as_ref()) {
                log::warn!("event=tray_action_failed code={}", error.code);
            }
        })
        .on_tray_icon_event(|tray, event| {
            if matches!(
                event,
                tauri::tray::TrayIconEvent::Click {
                    button: tauri::tray::MouseButton::Left,
                    button_state: tauri::tray::MouseButtonState::Up,
                    ..
                }
            ) {
                let _ = show_main_window(tray.app_handle());
            }
        })
        .build(app)
        .map_err(tray_error)?;

    start_refresh_worker(app.handle().clone(), task, remaining, focus_toggle)?;
    Ok(())
}

#[cfg(feature = "desktop-app")]
fn start_refresh_worker(
    app: tauri::AppHandle,
    task: tauri::menu::MenuItem<tauri::Wry>,
    remaining: tauri::menu::MenuItem<tauri::Wry>,
    focus_toggle: tauri::menu::MenuItem<tauri::Wry>,
) -> Result<(), crate::DomainError> {
    std::thread::Builder::new()
        .name("tray-status-refresh".into())
        .spawn(move || loop {
            if let Ok(model) = load_menu_model(&app) {
                let _ = task.set_text(model.task_label);
                let _ = remaining.set_text(model.remaining_label);
                let _ = focus_toggle.set_text(model.focus_label);
                let _ = focus_toggle.set_enabled(model.focus_enabled);
            }
            std::thread::sleep(std::time::Duration::from_secs(1));
        })
        .map_err(tray_error)?;
    Ok(())
}

#[cfg(feature = "desktop-app")]
fn load_menu_model(app: &tauri::AppHandle) -> Result<TrayMenuModel, crate::DomainError> {
    use crate::repositories::database::Database;
    use tauri::Manager;

    let database = app.state::<Database>();
    let reconciliation = crate::commands::focus::reconcile_and_emit(&database, app)?;
    let state = reconciliation.state;
    update_taskbar_progress(app, &state)?;
    let task_title = match &state {
        FocusState::Ready { .. } => next_focus_target(&database)?.map(|(_, title)| title),
        _ => active_focus_title(&database, &state),
    };
    Ok(TrayMenuModel::from_focus_state(
        &state,
        task_title.as_deref(),
    ))
}

#[cfg(feature = "desktop-app")]
fn update_taskbar_progress(
    app: &tauri::AppHandle,
    state: &FocusState,
) -> Result<(), crate::DomainError> {
    use tauri::window::{ProgressBarState, ProgressBarStatus};
    use tauri::Manager;

    let window = app
        .get_webview_window(super::main_window::MAIN_WINDOW_LABEL)
        .ok_or_else(|| crate::DomainError {
            code: "MAIN_WINDOW_MISSING".into(),
            message: "main window is unavailable".into(),
            field: None,
        })?;
    let model = TaskbarProgressModel::from_focus_state(state);
    let status = match model.status {
        TaskbarProgressStatus::None => ProgressBarStatus::None,
        TaskbarProgressStatus::Normal => ProgressBarStatus::Normal,
        TaskbarProgressStatus::Paused => ProgressBarStatus::Paused,
    };
    window
        .set_progress_bar(ProgressBarState {
            status: Some(status),
            progress: model.progress,
        })
        .map_err(|error| crate::DomainError {
            code: "TASKBAR_PROGRESS_FAILED".into(),
            message: error.to_string(),
            field: None,
        })
}

#[cfg(feature = "desktop-app")]
fn next_focus_target(
    database: &crate::repositories::database::Database,
) -> Result<Option<(crate::domain::focus::FocusTarget, String)>, crate::DomainError> {
    use crate::{domain::today::TodaySourceKind, services::today_service::TodayService};

    let local_date = chrono::Local::now().format("%Y-%m-%d").to_string();
    let digest = TodayService::new(database).get_digest(&local_date)?;
    Ok(digest
        .items
        .into_iter()
        .find(is_focus_candidate)
        .map(|item| {
            let target = match item.source_kind {
                TodaySourceKind::Task => crate::domain::focus::FocusTarget {
                    task_id: Some(item.source_id),
                    task_instance_id: None,
                },
                TodaySourceKind::RecurringInstance => crate::domain::focus::FocusTarget {
                    task_id: None,
                    task_instance_id: Some(item.source_id),
                },
            };
            (target, item.title)
        }))
}

#[cfg(any(feature = "desktop-app", test))]
fn is_focus_candidate(item: &crate::domain::today::TodayDigestItem) -> bool {
    item.status == crate::domain::today::TodayItemStatus::Pending
        && item
            .project
            .as_ref()
            .map_or(true, |project| project.status != "paused")
}

#[cfg(feature = "desktop-app")]
fn active_focus_title(
    database: &crate::repositories::database::Database,
    state: &FocusState,
) -> Option<String> {
    use crate::{
        repositories::recurrence_repository::RecurrenceRepository,
        services::task_service::TaskService,
    };

    let (task_id, task_instance_id) = match state {
        FocusState::Ready { .. } => return None,
        FocusState::Running {
            task_id,
            task_instance_id,
            ..
        }
        | FocusState::Paused {
            task_id,
            task_instance_id,
            ..
        } => (task_id, task_instance_id),
    };
    task_id
        .as_deref()
        .and_then(|id| TaskService::new(database).get(id).ok())
        .map(|detail| detail.task.title)
        .or_else(|| {
            task_instance_id.as_deref().and_then(|id| {
                RecurrenceRepository::new(database)
                    .get_instance(id)
                    .ok()
                    .flatten()
                    .map(|instance| instance.snapshot_title)
            })
        })
}

#[cfg(feature = "desktop-app")]
fn handle_menu_action(app: &tauri::AppHandle, menu_id: &str) -> Result<(), crate::DomainError> {
    match menu_id {
        SHOW_MAIN_ID => show_main_window(app),
        SHOW_WIDGET_ID => show_widget(app),
        FOCUS_TOGGLE_ID => toggle_focus(app),
        QUICK_TASK_ID => open_quick_task(app),
        UNLOCK_WIDGET_ID => unlock_widget(app),
        EXIT_ID => super::lifecycle::request_exit(app, 0),
        _ => Ok(()),
    }
}

#[cfg(feature = "desktop-app")]
pub fn toggle_focus(app: &tauri::AppHandle) -> Result<(), crate::DomainError> {
    use crate::{
        domain::focus::FocusState, repositories::database::Database,
        services::focus_service::FocusService,
    };
    use tauri::{Emitter, Manager};

    let database = app.state::<Database>();
    let service = FocusService::new(&database);
    let reconciliation = crate::commands::focus::reconcile_and_emit(&database, app)?;
    let state = match reconciliation.state {
        FocusState::Ready { .. } => {
            if let Some((target, _)) = next_focus_target(&database)? {
                service.start(target, DEFAULT_FOCUS_MINUTES)?
            } else {
                show_main_window(app)?;
                app.emit("tray://open-focus", ()).map_err(tray_error)?;
                return Ok(());
            }
        }
        FocusState::Running { .. } => service.pause()?,
        FocusState::Paused { .. } => service.resume()?,
    };
    crate::desktop::focus_events::emit_focus_changed(app, &state);
    Ok(())
}

#[cfg(feature = "desktop-app")]
pub fn open_quick_task(app: &tauri::AppHandle) -> Result<(), crate::DomainError> {
    use tauri::Emitter;

    show_main_window(app)?;
    app.emit("tray://quick-task", ()).map_err(tray_error)
}

#[cfg(feature = "desktop-app")]
pub fn unlock_widget(app: &tauri::AppHandle) -> Result<(), crate::DomainError> {
    use tauri::Manager;

    let database = app.state::<crate::repositories::database::Database>();
    let config = crate::services::widget_service::WidgetService::new(&database).unlock()?;
    crate::commands::widget::apply_widget_config(app, &config, true)
}

#[cfg(feature = "desktop-app")]
fn show_widget(app: &tauri::AppHandle) -> Result<(), crate::DomainError> {
    use tauri::Manager;

    let database = app.state::<crate::repositories::database::Database>();
    let config = crate::services::widget_service::WidgetService::new(&database).mark_visible()?;
    crate::commands::widget::apply_widget_config(app, &config, true)
}

#[cfg(feature = "desktop-app")]
pub fn show_main_window(app: &tauri::AppHandle) -> Result<(), crate::DomainError> {
    use tauri::Manager;

    let window = app
        .get_webview_window(super::main_window::MAIN_WINDOW_LABEL)
        .ok_or_else(|| crate::DomainError {
            code: "MAIN_WINDOW_MISSING".into(),
            message: "main window is unavailable".into(),
            field: None,
        })?;
    super::main_window::activate_existing_instance_window(&window)
}

#[cfg(feature = "desktop-app")]
fn tray_error(error: impl std::fmt::Display) -> crate::DomainError {
    crate::DomainError {
        code: "TRAY_OPERATION_FAILED".into(),
        message: error.to_string(),
        field: None,
    }
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};

    use super::*;

    fn ready() -> FocusState {
        FocusState::Ready {
            server_time: Utc.with_ymd_and_hms(2026, 7, 19, 10, 0, 0).unwrap(),
        }
    }

    fn active(running: bool, remaining_seconds: i64) -> FocusState {
        let now = Utc.with_ymd_and_hms(2026, 7, 19, 10, 0, 0).unwrap();
        if running {
            FocusState::Running {
                task_id: Some("task-1".into()),
                task_instance_id: None,
                planned_seconds: 1500,
                remaining_seconds,
                started_at: now,
                target_ends_at: now,
                interruption_count: 0,
                server_time: now,
            }
        } else {
            FocusState::Paused {
                task_id: Some("task-1".into()),
                task_instance_id: None,
                planned_seconds: 1500,
                remaining_seconds,
                started_at: now,
                paused_at: now,
                interruption_count: 1,
                server_time: now,
            }
        }
    }

    fn today_item(project_status: Option<&str>) -> crate::domain::today::TodayDigestItem {
        use crate::domain::today::{
            TodayItemKind, TodayItemStatus, TodayProjectSummary, TodaySourceKind,
        };

        crate::domain::today::TodayDigestItem {
            source_kind: TodaySourceKind::Task,
            source_id: "task-1".into(),
            item_kind: TodayItemKind::ProjectTask,
            recurrence_rule_id: None,
            title: "Write tests".into(),
            category: "work".into(),
            priority: 2,
            scheduled_date: "2026-07-19".into(),
            scheduled_time: None,
            status: TodayItemStatus::Pending,
            completed_at: None,
            project: project_status.map(|status| TodayProjectSummary {
                id: "project-1".into(),
                name: "Project".into(),
                color: "mint".into(),
                icon: "folder".into(),
                status: status.into(),
            }),
            is_overdue: false,
            created_at: "2026-07-19T10:00:00Z".into(),
        }
    }

    #[test]
    fn ready_menu_starts_the_next_task_or_opens_focus_without_one() {
        let available = TrayMenuModel::from_focus_state(&ready(), Some("完成托盘菜单"));
        assert_eq!(available.task_label, "下一项：完成托盘菜单");
        assert_eq!(available.focus_action, TrayFocusAction::Start);
        assert!(available.focus_enabled);

        let empty = TrayMenuModel::from_focus_state(&ready(), None);
        assert_eq!(empty.task_label, "下一项：暂无待完成任务");
        assert_eq!(empty.focus_label, "打开专注空间");
        assert!(empty.focus_enabled);
    }

    #[test]
    fn tray_focus_candidates_skip_paused_projects() {
        assert!(is_focus_candidate(&today_item(None)));
        assert!(is_focus_candidate(&today_item(Some("active"))));
        assert!(!is_focus_candidate(&today_item(Some("paused"))));
    }

    #[test]
    fn running_and_paused_menus_show_timer_and_matching_action() {
        let running = TrayMenuModel::from_focus_state(&active(true, 754), Some("编写测试"));
        assert_eq!(running.remaining_label, "剩余：12:34");
        assert_eq!(running.focus_label, "暂停专注");
        assert_eq!(running.focus_action, TrayFocusAction::Pause);

        let paused = TrayMenuModel::from_focus_state(&active(false, 754), Some("编写测试"));
        assert_eq!(paused.remaining_label, "剩余：12:34（已暂停）");
        assert_eq!(paused.focus_label, "继续专注");
        assert_eq!(paused.focus_action, TrayFocusAction::Resume);
    }

    #[test]
    fn close_policy_hides_only_when_background_running_is_enabled() {
        assert_eq!(
            main_window_close_action(true),
            MainWindowCloseAction::HideToTray
        );
        assert_eq!(
            main_window_close_action(false),
            MainWindowCloseAction::Close
        );
    }

    #[test]
    fn taskbar_progress_tracks_remaining_ratio_and_focus_state() {
        assert_eq!(
            TaskbarProgressModel::from_focus_state(&ready()),
            TaskbarProgressModel {
                status: TaskbarProgressStatus::None,
                progress: None,
            }
        );
        assert_eq!(
            TaskbarProgressModel::from_focus_state(&active(true, 750)),
            TaskbarProgressModel {
                status: TaskbarProgressStatus::Normal,
                progress: Some(50),
            }
        );
        assert_eq!(
            TaskbarProgressModel::from_focus_state(&active(false, 1)),
            TaskbarProgressModel {
                status: TaskbarProgressStatus::Paused,
                progress: Some(1),
            }
        );
    }
}
