pub mod commands;
pub mod desktop;
pub mod domain;
pub mod repositories;
pub mod services;

use serde::{ser::SerializeMap, Deserialize, Serialize, Serializer};

pub type DomainVersion = u64;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DomainError {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandResult<T> {
    Success { data: T, version: DomainVersion },
    Failure { error: DomainError },
}

impl<T: Serialize> Serialize for CommandResult<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Success { data, version } => {
                let mut result = serializer.serialize_map(Some(3))?;
                result.serialize_entry("ok", &true)?;
                result.serialize_entry("data", data)?;
                result.serialize_entry("version", version)?;
                result.end()
            }
            Self::Failure { error } => {
                let mut result = serializer.serialize_map(Some(2))?;
                result.serialize_entry("ok", &false)?;
                result.serialize_entry("error", error)?;
                result.end()
            }
        }
    }
}

impl<T> CommandResult<T> {
    pub fn success(data: T, version: DomainVersion) -> Self {
        Self::Success { data, version }
    }

    pub fn failure(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Failure {
            error: DomainError {
                code: code.into(),
                message: message.into(),
                field: None,
            },
        }
    }

    pub fn from_result(
        context: &str,
        value: Result<T, DomainError>,
        version: DomainVersion,
    ) -> Self {
        match value {
            Ok(data) => Self::success(data, version),
            Err(error) => {
                log::warn!("{}", diagnostic_failure_event(context, &error));
                Self::Failure { error }
            }
        }
    }
}

fn diagnostic_failure_event(context: &str, error: &DomainError) -> String {
    format!(
        "event=domain_error context={} code={} field={}",
        diagnostic_token(context),
        diagnostic_token(&error.code),
        error
            .field
            .as_deref()
            .map(diagnostic_token)
            .unwrap_or_else(|| "-".into())
    )
}

fn diagnostic_invocation_failure_event(command: &str, error: &str) -> String {
    format!(
        "event=command_invocation_failed command={} reason={}",
        diagnostic_token(command),
        diagnostic_token(error)
    )
}

fn diagnostic_token(value: &str) -> String {
    value
        .chars()
        .take(120)
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.' | ':') {
                character
            } else {
                '_'
            }
        })
        .collect()
}

#[cfg(feature = "desktop-app")]
pub fn run() {
    use tauri::{Manager, WindowEvent};

    let context = tauri::generate_context!();
    let app_data_dir = dirs::data_dir()
        .expect("failed to resolve application data directory")
        .join(&context.config().identifier);
    std::fs::create_dir_all(&app_data_dir).expect("failed to create application data directory");
    let database =
        repositories::database::Database::open(app_data_dir.join("arrive-focus.sqlite3"))
            .expect("failed to open application database");

    let app = tauri::Builder::default()
        .manage(database)
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Err(error) = desktop::tray::show_main_window(app) {
                log::warn!(
                    "event=second_instance_activation_failed code={}",
                    error.code
                );
            }
        }))
        .plugin(tauri_plugin_log::Builder::default().build())
        .plugin(commands::update::plugin())
        .plugin(desktop::shortcuts::plugin())
        .plugin(tauri_plugin_autostart::Builder::new().build())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            desktop::recurrence::reconcile_and_emit(
                app.handle(),
                services::recurrence_service::GenerationTrigger::Startup,
            )
            .map_err(|error| std::io::Error::other(error.message))?;
            let main_geometry_runtime =
                desktop::main_window::MainWindowGeometryRuntime::start(app.handle().clone())
                    .map_err(|error| std::io::Error::other(error.message))?;
            app.manage(main_geometry_runtime);
            desktop::main_window::restore_main_window(app.handle())
                .map_err(|error| std::io::Error::other(error.message))?;
            desktop::main_window::persist_main_window_state(app.handle())
                .map_err(|error| std::io::Error::other(error.message))?;
            app.manage(commands::backup::BackupRestoreState::default());
            app.manage(commands::update::PendingUpdateState::default());
            app.manage(desktop::shortcuts::ShortcutRuntime::default());
            desktop::shortcuts::initialize(app.handle())
                .map_err(|error| std::io::Error::other(error.message))?;
            app.manage(desktop::widget_shell::WidgetShellRuntime::default());
            let geometry_runtime =
                desktop::widget_window::WidgetGeometryRuntime::start(app.handle().clone())
                    .map_err(|error| std::io::Error::other(error.message))?;
            app.manage(geometry_runtime);
            desktop::tray::setup_tray(app).map_err(|error| std::io::Error::other(error.message))?;
            desktop::notifications::start_notification_worker(app.handle().clone())
                .map_err(|error| std::io::Error::other(error.message))?;
            let database = app.state::<repositories::database::Database>();
            if let Ok(config) = services::widget_service::WidgetService::new(&database).get() {
                let _ = commands::widget::apply_widget_config(app.handle(), &config, false);
            }
            desktop::widget_shell::start_widget_shell_monitor(app.handle().clone())
                .map_err(|error| std::io::Error::other(error.message))?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() == "main" {
                if let WindowEvent::CloseRequested { api, .. } = event {
                    let database = window
                        .app_handle()
                        .state::<repositories::database::Database>();
                    let background_running =
                        repositories::preferences_repository::PreferencesRepository::new(&database)
                            .get_general()
                            .map(|preferences| preferences.background_running)
                            .unwrap_or(true);
                    if desktop::tray::main_window_close_action(background_running)
                        == desktop::tray::MainWindowCloseAction::HideToTray
                    {
                        api.prevent_close();
                        if let Err(error) =
                            desktop::main_window::persist_main_window_state(window.app_handle())
                        {
                            log::warn!("event=main_window_state_save_failed code={}", error.code);
                        }
                        let _ = window.hide();
                        return;
                    }
                    api.prevent_close();
                    if let Err(error) = desktop::lifecycle::request_exit(window.app_handle(), 0) {
                        log::warn!("event=exit_request_failed code={}", error.code);
                    }
                    return;
                }
                if matches!(
                    event,
                    WindowEvent::Moved(_)
                        | WindowEvent::Resized(_)
                        | WindowEvent::ScaleFactorChanged { .. }
                ) {
                    window
                        .app_handle()
                        .state::<desktop::main_window::MainWindowGeometryRuntime>()
                        .record_change();
                }
            }
            if window.label() == domain::widget::WIDGET_WINDOW_LABEL {
                if let WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    if let Err(error) =
                        desktop::widget_window::persist_widget_geometry(window.app_handle())
                    {
                        log::warn!("event=widget_window_state_save_failed code={}", error.code);
                    }
                    if window.hide().is_err() {
                        log::warn!("event=widget_window_hide_failed");
                    }
                    return;
                }
                if matches!(
                    event,
                    WindowEvent::Moved(_)
                        | WindowEvent::Resized(_)
                        | WindowEvent::ScaleFactorChanged { .. }
                ) {
                    window
                        .app_handle()
                        .state::<desktop::widget_window::WidgetGeometryRuntime>()
                        .record_change();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::health::health,
            commands::health::diagnostic_command_failure,
            commands::backup::backup_export,
            commands::backup::backup_inspect,
            commands::backup::backup_restore,
            commands::calendar::calendar_get_period,
            commands::statistics::statistics_get_summary,
            commands::planning::note_get,
            commands::planning::note_save,
            commands::planning::weekly_goal_list,
            commands::planning::weekly_goal_save,
            commands::settings::settings_get,
            commands::settings::settings_update,
            commands::desktop_integration::desktop_integration_get_settings,
            commands::desktop_integration::desktop_integration_update_shortcuts,
            commands::desktop_integration::desktop_integration_set_autostart,
            commands::notification::notification_get_settings,
            commands::notification::notification_update_preferences,
            commands::notification::notification_open_settings,
            commands::focus::focus_get_state,
            commands::focus::focus_reconcile,
            commands::focus::focus_start,
            commands::focus::focus_pause,
            commands::focus::focus_resume,
            commands::focus::focus_reset,
            commands::focus::focus_finish,
            commands::memo::memo_list,
            commands::memo::memo_get,
            commands::memo::memo_create,
            commands::memo::memo_update,
            commands::memo::memo_remove,
            commands::memo::memo_tag_list,
            commands::project::project_create,
            commands::project::project_update,
            commands::project::project_set_status,
            commands::project::project_remove,
            commands::project::project_list,
            commands::project::project_get,
            commands::task::task_list,
            commands::task::task_get,
            commands::task::task_create,
            commands::task::task_update,
            commands::task::task_set_completed,
            commands::task::task_remove,
            commands::task::task_set_check_item_completed,
            commands::task::task_reorder_check_items,
            commands::recurrence::recurrence_get,
            commands::recurrence::recurrence_create,
            commands::recurrence::recurrence_update,
            commands::recurrence::recurrence_set_status,
            commands::recurrence::instance_complete,
            commands::recurrence::instance_skip,
            commands::recurrence::instance_delay_today,
            commands::recurrence::instance_reschedule_tomorrow,
            commands::today::today_get_digest,
            commands::update::update_check,
            commands::update::update_download,
            commands::update::update_install,
            commands::widget::widget_get_config,
            commands::widget::widget_update_config,
            commands::widget::widget_show,
            commands::widget::widget_unlock
        ])
        .build(context)
        .expect("failed to build Arrive Focus");
    app.run(|app_handle, event| match event {
        tauri::RunEvent::Resumed => {
            let database = app_handle.state::<repositories::database::Database>();
            let _ = commands::focus::reconcile_and_emit(&database, app_handle);
            let _ = desktop::recurrence::reconcile_and_emit(
                app_handle,
                services::recurrence_service::GenerationTrigger::Startup,
            );
            let _ = desktop::notifications::reconcile_task_notifications(app_handle);
        }
        tauri::RunEvent::ExitRequested { api, .. } => {
            if let Err(error) = desktop::lifecycle::persist_before_exit(app_handle) {
                log::warn!("event=exit_persistence_failed code={}", error.code);
                api.prevent_exit();
            }
        }
        _ => {}
    });
}

#[cfg(test)]
mod tests {
    use super::{
        diagnostic_failure_event, diagnostic_invocation_failure_event, CommandResult, DomainError,
    };

    #[test]
    fn success_result_keeps_domain_version() {
        let result = CommandResult::success("ready", 7);
        assert_eq!(
            result,
            CommandResult::Success {
                data: "ready",
                version: 7
            }
        );
        let serialized = serde_json::to_value(result).expect("serialize command result");
        assert_eq!(serialized["ok"], true);
        assert_eq!(serialized["version"], 7);
    }

    #[test]
    fn failure_result_uses_stable_shape() {
        let result: CommandResult<()> = CommandResult::failure("INVALID_INPUT", "invalid input");
        let serialized = serde_json::to_value(result).expect("serialize command result");
        assert_eq!(serialized["ok"], false);
        assert_eq!(serialized["error"]["code"], "INVALID_INPUT");
    }

    #[test]
    fn command_failure_diagnostics_exclude_sensitive_messages() {
        let error = DomainError {
            code: "NOTE_BODY_INVALID".into(),
            message: "Private note body and task title: launch plan".into(),
            field: Some("input.body\nforged=entry".into()),
        };

        let event = diagnostic_failure_event("commands::planning", &error);

        assert!(event.contains("context=commands::planning"));
        assert!(event.contains("code=NOTE_BODY_INVALID"));
        assert!(event.contains("field=input.body_forged_entry"));
        assert!(!event.contains("Private note body"));
        assert!(!event.contains("launch plan"));
        assert!(!event.contains('\n'));
    }

    #[test]
    fn memo_failure_diagnostics_exclude_content_tags_and_search_terms() {
        let error = DomainError {
            code: "MEMO_SAVE_FAILED".into(),
            message: "Title=Private launch; Body=secret plan; Tag=finance; Search=acquisition"
                .into(),
            field: Some("body".into()),
        };

        let event = diagnostic_failure_event("commands::memo", &error);

        assert_eq!(
            event,
            "event=domain_error context=commands::memo code=MEMO_SAVE_FAILED field=body"
        );
        assert!(!event.contains("Private launch"));
        assert!(!event.contains("secret plan"));
        assert!(!event.contains("finance"));
        assert!(!event.contains("acquisition"));
    }

    #[test]
    fn invocation_failure_diagnostics_are_single_line_and_bounded() {
        let event = diagnostic_invocation_failure_event(
            "settings_get\nforged=entry",
            &format!("unknown command: {}\nprivate=value", "x".repeat(200)),
        );

        assert!(event.contains("command=settings_get_forged_entry"));
        assert!(event.contains("reason=unknown_command:"));
        assert!(!event.contains('\n'));
        assert!(event.len() <= 204);
    }

    #[test]
    fn from_result_preserves_the_public_domain_error() {
        let error = DomainError {
            code: "TASK_TITLE_INVALID".into(),
            message: "task title is required".into(),
            field: Some("title".into()),
        };

        assert_eq!(
            CommandResult::<()>::from_result("commands::task", Err(error.clone()), 1),
            CommandResult::Failure { error }
        );
    }
}
