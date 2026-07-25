#[cfg(feature = "desktop-app")]
use crate::{
    desktop::{
        widget_shell::{apply_widget_mode, AppliedWidgetMode},
        widget_window::{apply_widget_window_behavior, restored_widget_position},
    },
    domain::widget::{WidgetConfig, WidgetConfigInput, WidgetMode},
    repositories::database::Database,
    services::widget_service::WidgetService,
    CommandResult, DomainError,
};

#[cfg(feature = "desktop-app")]
#[tauri::command]
pub fn widget_get_config(database: tauri::State<'_, Database>) -> CommandResult<WidgetConfig> {
    result(WidgetService::new(&database).get())
}

#[cfg(feature = "desktop-app")]
#[tauri::command]
pub fn widget_update_config(
    database: tauri::State<'_, Database>,
    app: tauri::AppHandle,
    input: WidgetConfigInput,
) -> CommandResult<WidgetConfig> {
    result(
        WidgetService::new(&database)
            .update(input)
            .and_then(|config| {
                apply_widget_config(&app, &config, false)?;
                crate::commands::settings::emit_current_settings(&app, &database)?;
                Ok(config)
            }),
    )
}

#[cfg(feature = "desktop-app")]
#[tauri::command]
pub fn widget_show(
    database: tauri::State<'_, Database>,
    app: tauri::AppHandle,
) -> CommandResult<WidgetConfig> {
    result(
        WidgetService::new(&database)
            .mark_visible()
            .and_then(|config| {
                apply_widget_config(&app, &config, true)?;
                Ok(config)
            }),
    )
}

#[cfg(feature = "desktop-app")]
#[tauri::command]
pub fn widget_unlock(
    database: tauri::State<'_, Database>,
    app: tauri::AppHandle,
) -> CommandResult<WidgetConfig> {
    result(unlock_widget(&database, &app))
}

#[cfg(feature = "desktop-app")]
fn unlock_widget(database: &Database, app: &tauri::AppHandle) -> Result<WidgetConfig, DomainError> {
    let service = WidgetService::new(database);
    let config = service.unlock()?;
    apply_widget_config(app, &config, true)?;
    crate::commands::settings::emit_current_settings(app, database)?;
    Ok(config)
}

#[cfg(feature = "desktop-app")]
pub(crate) fn apply_widget_config(
    app: &tauri::AppHandle,
    config: &WidgetConfig,
    show: bool,
) -> Result<(), DomainError> {
    use tauri::{Emitter, Manager};

    let window = app
        .get_webview_window("widget")
        .ok_or_else(|| DomainError {
            code: "WIDGET_WINDOW_MISSING".into(),
            message: "widget window is unavailable".into(),
            field: None,
        })?;
    let applied_mode = apply_widget_mode(app, &window, config.input.mode)?;
    let actual_mode = match applied_mode {
        AppliedWidgetMode::Desktop => WidgetMode::Desktop,
        AppliedWidgetMode::Floating => WidgetMode::Floating,
    };
    apply_widget_window_behavior(&window, actual_mode, config.input.locked)?;
    window
        .set_size(tauri::LogicalSize::new(
            config.input.width,
            config.input.height,
        ))
        .map_err(window_error)?;
    window
        .set_position(restored_widget_position(&window, &config.input)?)
        .map_err(window_error)?;
    window
        .emit("widget://config-changed", config)
        .map_err(window_error)?;
    if show {
        window.show().map_err(window_error)?;
        if applied_mode == AppliedWidgetMode::Floating {
            window.set_focus().map_err(window_error)?;
        }
    }
    Ok(())
}

#[cfg(feature = "desktop-app")]
fn window_error(error: impl std::fmt::Display) -> DomainError {
    DomainError {
        code: "WIDGET_WINDOW_OPERATION_FAILED".into(),
        message: error.to_string(),
        field: None,
    }
}

#[cfg(feature = "desktop-app")]
fn result<T>(value: Result<T, DomainError>) -> CommandResult<T> {
    CommandResult::from_result(module_path!(), value, 1)
}
