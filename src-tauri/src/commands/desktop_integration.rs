use tauri::Manager;
use tauri_plugin_autostart::ManagerExt as AutostartManagerExt;

use crate::desktop::shortcuts::{replace_shortcuts, ShortcutRuntime};
use crate::domain::settings::{DesktopIntegrationSettings, ShortcutPreferences};
use crate::repositories::{database::Database, preferences_repository::PreferencesRepository};
use crate::{CommandResult, DomainError};

#[tauri::command]
pub fn desktop_integration_get_settings(
    database: tauri::State<'_, Database>,
    app: tauri::AppHandle,
) -> CommandResult<DesktopIntegrationSettings> {
    result(get_settings(&database, &app))
}

#[tauri::command]
pub fn desktop_integration_update_shortcuts(
    database: tauri::State<'_, Database>,
    app: tauri::AppHandle,
    shortcuts: ShortcutPreferences,
) -> CommandResult<ShortcutPreferences> {
    result(
        update_shortcuts(&database, &app, shortcuts).and_then(|shortcuts| {
            crate::commands::settings::emit_current_settings(&app, &database)?;
            Ok(shortcuts)
        }),
    )
}

#[tauri::command]
pub fn desktop_integration_set_autostart(
    database: tauri::State<'_, Database>,
    app: tauri::AppHandle,
    enabled: bool,
) -> CommandResult<bool> {
    result(set_autostart(&database, &app, enabled).and_then(|enabled| {
        crate::commands::settings::emit_current_settings(&app, &database)?;
        Ok(enabled)
    }))
}

fn get_settings(
    database: &Database,
    app: &tauri::AppHandle,
) -> Result<DesktopIntegrationSettings, DomainError> {
    let preferences = PreferencesRepository::new(database).get_desktop_integration()?;
    let autostart_enabled = app.autolaunch().is_enabled().map_err(autostart_error)?;
    Ok(DesktopIntegrationSettings {
        shortcuts: preferences.shortcuts,
        autostart_enabled,
        shortcut_error: app.state::<ShortcutRuntime>().last_error(),
    })
}

fn update_shortcuts(
    database: &Database,
    app: &tauri::AppHandle,
    candidate: ShortcutPreferences,
) -> Result<ShortcutPreferences, DomainError> {
    let runtime = app.state::<ShortcutRuntime>();
    let active = runtime.active_preferences();
    replace_shortcuts(app, &active, &candidate)?;

    let repository = PreferencesRepository::new(database);
    let mut preferences = repository.get_desktop_integration()?;
    preferences.shortcuts = candidate.clone();
    if let Err(error) = repository.set_desktop_integration(preferences) {
        let _ = replace_shortcuts(app, &candidate, &active);
        return Err(error);
    }
    Ok(candidate)
}

fn set_autostart(
    database: &Database,
    app: &tauri::AppHandle,
    enabled: bool,
) -> Result<bool, DomainError> {
    let registry = TauriAutostartRegistry { app };
    synchronize_autostart(&registry, enabled, || {
        let repository = PreferencesRepository::new(database);
        let mut preferences = repository.get_desktop_integration()?;
        preferences.autostart_enabled = enabled;
        repository.set_desktop_integration(preferences)?;
        Ok(())
    })?;
    Ok(enabled)
}

trait AutostartRegistry {
    fn is_enabled(&self) -> Result<bool, DomainError>;
    fn set_enabled(&self, enabled: bool) -> Result<(), DomainError>;
}

struct TauriAutostartRegistry<'a> {
    app: &'a tauri::AppHandle,
}

impl AutostartRegistry for TauriAutostartRegistry<'_> {
    fn is_enabled(&self) -> Result<bool, DomainError> {
        self.app.autolaunch().is_enabled().map_err(autostart_error)
    }

    fn set_enabled(&self, enabled: bool) -> Result<(), DomainError> {
        if enabled {
            self.app.autolaunch().enable()
        } else {
            self.app.autolaunch().disable()
        }
        .map_err(autostart_error)
    }
}

fn synchronize_autostart(
    registry: &impl AutostartRegistry,
    enabled: bool,
    persist: impl FnOnce() -> Result<(), DomainError>,
) -> Result<(), DomainError> {
    let previous = registry.is_enabled()?;
    registry.set_enabled(enabled)?;
    if let Err(error) = persist() {
        let _ = registry.set_enabled(previous);
        return Err(error);
    }
    Ok(())
}

fn autostart_error(error: impl std::fmt::Display) -> DomainError {
    DomainError {
        code: "AUTOSTART_SYNC_FAILED".into(),
        message: format!("开机启动状态同步失败：{error}"),
        field: Some("autostartEnabled".into()),
    }
}

fn result<T>(value: Result<T, DomainError>) -> CommandResult<T> {
    CommandResult::from_result(module_path!(), value, 1)
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    struct FakeAutostart {
        enabled: Mutex<bool>,
        changes: Mutex<Vec<bool>>,
    }

    impl FakeAutostart {
        fn new(enabled: bool) -> Self {
            Self {
                enabled: Mutex::new(enabled),
                changes: Mutex::new(Vec::new()),
            }
        }
    }

    impl AutostartRegistry for FakeAutostart {
        fn is_enabled(&self) -> Result<bool, DomainError> {
            Ok(*self.enabled.lock().unwrap())
        }

        fn set_enabled(&self, enabled: bool) -> Result<(), DomainError> {
            *self.enabled.lock().unwrap() = enabled;
            self.changes.lock().unwrap().push(enabled);
            Ok(())
        }
    }

    #[test]
    fn autostart_changes_system_state_before_persisting() {
        let registry = FakeAutostart::new(false);
        synchronize_autostart(&registry, true, || {
            assert!(*registry.enabled.lock().unwrap());
            Ok(())
        })
        .unwrap();
        assert_eq!(*registry.changes.lock().unwrap(), vec![true]);
    }

    #[test]
    fn autostart_rolls_back_system_state_when_persistence_fails() {
        let registry = FakeAutostart::new(false);
        let error = synchronize_autostart(&registry, true, || {
            Err(DomainError {
                code: "DATABASE_ERROR".into(),
                message: "save failed".into(),
                field: None,
            })
        })
        .unwrap_err();
        assert_eq!(error.code, "DATABASE_ERROR");
        assert!(!*registry.enabled.lock().unwrap());
        assert_eq!(*registry.changes.lock().unwrap(), vec![true, false]);
    }
}
