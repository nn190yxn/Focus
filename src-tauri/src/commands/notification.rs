use crate::{
    desktop::notifications::{open_notification_settings, TauriNotificationPublisher},
    domain::settings::{NotificationPreferences, NotificationSettings},
    repositories::database::Database,
    services::notification_service::NotificationService,
    CommandResult, DomainError,
};

#[tauri::command]
pub fn notification_get_settings(
    database: tauri::State<'_, Database>,
    app: tauri::AppHandle,
) -> CommandResult<NotificationSettings> {
    result(NotificationService::new(&database).settings(&TauriNotificationPublisher::new(&app)))
}

#[tauri::command]
pub fn notification_update_preferences(
    database: tauri::State<'_, Database>,
    app: tauri::AppHandle,
    preferences: NotificationPreferences,
) -> CommandResult<NotificationPreferences> {
    result(
        NotificationService::new(&database)
            .update_preferences(preferences)
            .and_then(|preferences| {
                crate::commands::settings::emit_current_settings(&app, &database)?;
                Ok(preferences)
            }),
    )
}

#[tauri::command]
pub fn notification_open_settings() -> CommandResult<()> {
    result(open_notification_settings())
}

fn result<T>(value: Result<T, DomainError>) -> CommandResult<T> {
    CommandResult::from_result(module_path!(), value, 1)
}
