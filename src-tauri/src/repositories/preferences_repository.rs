use chrono::Utc;
use rusqlite::{params, OptionalExtension};
use serde::Serialize;

use super::database::Database;
use crate::{
    domain::settings::{
        DesktopIntegrationPreferences, GeneralPreferences, NotificationPreferences,
    },
    domain::window::MainWindowState,
    DomainError,
};

const GENERAL_PREFERENCES_KEY: &str = "generalPreferences";
const NOTIFICATION_PREFERENCES_KEY: &str = "notificationPreferences";
const DESKTOP_INTEGRATION_PREFERENCES_KEY: &str = "desktopIntegrationPreferences";
const MAIN_WINDOW_STATE_KEY: &str = "mainWindowState";

pub struct PreferencesRepository<'a> {
    database: &'a Database,
}

impl<'a> PreferencesRepository<'a> {
    pub fn new(database: &'a Database) -> Self {
        Self { database }
    }

    pub fn get_general(&self) -> Result<GeneralPreferences, DomainError> {
        self.get_json(GENERAL_PREFERENCES_KEY, GeneralPreferences::default())
    }

    pub fn set_general(
        &self,
        preferences: GeneralPreferences,
    ) -> Result<GeneralPreferences, DomainError> {
        self.set_json(GENERAL_PREFERENCES_KEY, &preferences)?;
        Ok(preferences)
    }

    pub fn get_notifications(&self) -> Result<NotificationPreferences, DomainError> {
        let value = self.database.read(|connection| {
            connection
                .query_row(
                    "SELECT value_json FROM preferences WHERE key = ?1",
                    [NOTIFICATION_PREFERENCES_KEY],
                    |row| row.get::<_, String>(0),
                )
                .optional()
        })?;
        match value {
            Some(value) => serde_json::from_str(&value).map_err(|error| DomainError {
                code: "PREFERENCES_INVALID".into(),
                message: error.to_string(),
                field: None,
            }),
            None => Ok(NotificationPreferences::default()),
        }
    }

    pub fn set_notifications(
        &self,
        preferences: NotificationPreferences,
    ) -> Result<NotificationPreferences, DomainError> {
        let value = serde_json::to_string(&preferences).map_err(|error| DomainError {
            code: "PREFERENCES_INVALID".into(),
            message: error.to_string(),
            field: None,
        })?;
        self.database.write(|tx| {
            tx.execute(
                "INSERT INTO preferences(key, value_json, updated_at) VALUES (?1, ?2, ?3)
                 ON CONFLICT(key) DO UPDATE SET value_json = excluded.value_json, updated_at = excluded.updated_at",
                params![NOTIFICATION_PREFERENCES_KEY, value, Utc::now().to_rfc3339()],
            )?;
            Ok(())
        })?;
        Ok(preferences)
    }

    pub fn get_desktop_integration(&self) -> Result<DesktopIntegrationPreferences, DomainError> {
        self.get_json(
            DESKTOP_INTEGRATION_PREFERENCES_KEY,
            DesktopIntegrationPreferences::default(),
        )
    }

    pub fn set_desktop_integration(
        &self,
        preferences: DesktopIntegrationPreferences,
    ) -> Result<DesktopIntegrationPreferences, DomainError> {
        self.set_json(DESKTOP_INTEGRATION_PREFERENCES_KEY, &preferences)?;
        Ok(preferences)
    }

    pub fn get_main_window_state(&self) -> Result<Option<MainWindowState>, DomainError> {
        let value = self.database.read(|connection| {
            connection
                .query_row(
                    "SELECT value_json FROM preferences WHERE key = ?1",
                    [MAIN_WINDOW_STATE_KEY],
                    |row| row.get::<_, String>(0),
                )
                .optional()
        })?;
        Ok(value
            .and_then(|value| serde_json::from_str::<MainWindowState>(&value).ok())
            .filter(MainWindowState::is_valid))
    }

    pub fn set_main_window_state(&self, state: &MainWindowState) -> Result<(), DomainError> {
        if !state.is_valid() {
            return Err(DomainError {
                code: "MAIN_WINDOW_STATE_INVALID".into(),
                message: "main window state is invalid".into(),
                field: None,
            });
        }
        self.set_json(MAIN_WINDOW_STATE_KEY, state)
    }

    fn get_json<T: serde::de::DeserializeOwned>(
        &self,
        key: &str,
        default: T,
    ) -> Result<T, DomainError> {
        let value = self.database.read(|connection| {
            connection
                .query_row(
                    "SELECT value_json FROM preferences WHERE key = ?1",
                    [key],
                    |row| row.get::<_, String>(0),
                )
                .optional()
        })?;
        value.map_or(Ok(default), |value| {
            serde_json::from_str(&value).map_err(preferences_error)
        })
    }

    fn set_json<T: Serialize>(&self, key: &str, value: &T) -> Result<(), DomainError> {
        let value = serde_json::to_string(value).map_err(preferences_error)?;
        self.database.write(|tx| {
            tx.execute(
                "INSERT INTO preferences(key, value_json, updated_at) VALUES (?1, ?2, ?3)
                 ON CONFLICT(key) DO UPDATE SET value_json = excluded.value_json, updated_at = excluded.updated_at",
                params![key, value, Utc::now().to_rfc3339()],
            )?;
            Ok(())
        })
    }
}

fn preferences_error(error: serde_json::Error) -> DomainError {
    DomainError {
        code: "PREFERENCES_INVALID".into(),
        message: error.to_string(),
        field: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_preferences_use_enabled_defaults_and_updates_round_trip() {
        let database = Database::open_in_memory().unwrap();
        let repository = PreferencesRepository::new(&database);

        assert_eq!(
            repository.get_notifications().unwrap(),
            NotificationPreferences::default()
        );

        let updated = NotificationPreferences {
            notifications_enabled: false,
            sound_enabled: false,
        };
        repository.set_notifications(updated).unwrap();
        assert_eq!(repository.get_notifications().unwrap(), updated);
    }

    #[test]
    fn malformed_preferences_return_a_stable_error() {
        let database = Database::open_in_memory().unwrap();
        database
            .write(|tx| {
                tx.execute(
                    "INSERT INTO preferences(key, value_json, updated_at) VALUES (?1, ?2, ?3)",
                    params![NOTIFICATION_PREFERENCES_KEY, "{}", Utc::now().to_rfc3339()],
                )?;
                Ok(())
            })
            .unwrap();

        let error = PreferencesRepository::new(&database)
            .get_notifications()
            .unwrap_err();
        assert_eq!(error.code, "PREFERENCES_INVALID");
    }

    #[test]
    fn desktop_integration_preferences_round_trip() {
        let database = Database::open_in_memory().unwrap();
        let repository = PreferencesRepository::new(&database);
        assert_eq!(
            repository.get_desktop_integration().unwrap(),
            DesktopIntegrationPreferences::default()
        );

        let updated = DesktopIntegrationPreferences {
            shortcuts: crate::domain::settings::ShortcutPreferences {
                enabled: true,
                bindings: crate::domain::settings::ShortcutBindings {
                    toggle_focus: "Ctrl+Shift+Space".into(),
                    ..crate::domain::settings::ShortcutBindings::default()
                },
            },
            autostart_enabled: true,
        };
        repository.set_desktop_integration(updated.clone()).unwrap();
        assert_eq!(repository.get_desktop_integration().unwrap(), updated);
    }

    #[test]
    fn general_preferences_use_defaults_and_round_trip_independently() {
        let database = Database::open_in_memory().unwrap();
        let repository = PreferencesRepository::new(&database);
        assert_eq!(
            repository.get_general().unwrap(),
            GeneralPreferences::default()
        );

        let updated = GeneralPreferences {
            language: crate::domain::settings::LanguagePreference::En,
            appearance: crate::domain::settings::AppearancePreference::Dark,
            theme: crate::domain::settings::ThemePreference::Noir,
            background_running: false,
        };
        repository.set_general(updated).unwrap();
        repository
            .set_notifications(NotificationPreferences {
                notifications_enabled: false,
                sound_enabled: false,
            })
            .unwrap();

        assert_eq!(repository.get_general().unwrap(), updated);
        assert_eq!(
            repository.get_notifications().unwrap(),
            NotificationPreferences {
                notifications_enabled: false,
                sound_enabled: false,
            }
        );
    }

    #[test]
    fn main_window_state_round_trips_and_invalid_state_is_ignored() {
        let database = Database::open_in_memory().unwrap();
        let repository = PreferencesRepository::new(&database);
        assert_eq!(repository.get_main_window_state().unwrap(), None);

        let state = MainWindowState {
            x: -900.0,
            y: 120.0,
            width: 1280.0,
            height: 760.0,
            maximized: true,
            monitor_id: Some("DISPLAY2".into()),
            scale_factor: 1.25,
        };
        repository.set_main_window_state(&state).unwrap();
        assert_eq!(repository.get_main_window_state().unwrap(), Some(state));

        database
            .write(|tx| {
                tx.execute(
                    "UPDATE preferences SET value_json = ?1 WHERE key = ?2",
                    params![r#"{"x":0.0,"y":0.0,"width":0.0,"height":760.0,"maximized":false,"monitorId":null,"scaleFactor":1.0}"#, MAIN_WINDOW_STATE_KEY],
                )?;
                Ok(())
            })
            .unwrap();
        assert_eq!(repository.get_main_window_state().unwrap(), None);

        database
            .write(|tx| {
                tx.execute(
                    "UPDATE preferences SET value_json = ?1 WHERE key = ?2",
                    params!["{}", MAIN_WINDOW_STATE_KEY],
                )?;
                Ok(())
            })
            .unwrap();
        assert_eq!(repository.get_main_window_state().unwrap(), None);
    }
}
