use tauri::Emitter;

use crate::{
    domain::settings::{GeneralPreferences, GeneralPreferencesPatch},
    repositories::database::Database,
    services::settings_service::SettingsService,
    CommandResult, DomainError,
};

pub const SETTINGS_CHANGED_EVENT: &str = "settings://changed";

#[tauri::command]
pub fn settings_get(database: tauri::State<'_, Database>) -> CommandResult<GeneralPreferences> {
    result(SettingsService::new(&database).get())
}

#[tauri::command]
pub fn settings_update(
    database: tauri::State<'_, Database>,
    app: tauri::AppHandle,
    patch: GeneralPreferencesPatch,
) -> CommandResult<GeneralPreferences> {
    result(
        SettingsService::new(&database)
            .update(patch)
            .and_then(|preferences| {
                emit_settings_changed(&app, preferences)?;
                Ok(preferences)
            }),
    )
}

pub(crate) fn emit_current_settings(
    app: &tauri::AppHandle,
    database: &Database,
) -> Result<(), DomainError> {
    emit_settings_changed(app, SettingsService::new(database).get()?)
}

fn emit_settings_changed(
    app: &tauri::AppHandle,
    preferences: GeneralPreferences,
) -> Result<(), DomainError> {
    app.emit(SETTINGS_CHANGED_EVENT, preferences)
        .map_err(|error| DomainError {
            code: "SETTINGS_BROADCAST_FAILED".into(),
            message: error.to_string(),
            field: None,
        })
}

fn result<T>(value: Result<T, DomainError>) -> CommandResult<T> {
    CommandResult::from_result(module_path!(), value, 1)
}
