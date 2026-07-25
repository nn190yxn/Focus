use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum LanguagePreference {
    #[default]
    System,
    ZhCn,
    En,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum AppearancePreference {
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum ThemePreference {
    #[default]
    Mint,
    Noir,
    Office,
    Blush,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneralPreferences {
    pub language: LanguagePreference,
    pub appearance: AppearancePreference,
    pub theme: ThemePreference,
    pub background_running: bool,
}

impl Default for GeneralPreferences {
    fn default() -> Self {
        Self {
            language: LanguagePreference::System,
            appearance: AppearancePreference::System,
            theme: ThemePreference::Mint,
            background_running: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GeneralPreferencesPatch {
    pub language: Option<LanguagePreference>,
    pub appearance: Option<AppearancePreference>,
    pub theme: Option<ThemePreference>,
    pub background_running: Option<bool>,
}

impl GeneralPreferences {
    pub fn apply(self, patch: GeneralPreferencesPatch) -> Self {
        Self {
            language: patch.language.unwrap_or(self.language),
            appearance: patch.appearance.unwrap_or(self.appearance),
            theme: patch.theme.unwrap_or(self.theme),
            background_running: patch.background_running.unwrap_or(self.background_running),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationPreferences {
    pub notifications_enabled: bool,
    pub sound_enabled: bool,
}

impl Default for NotificationPreferences {
    fn default() -> Self {
        Self {
            notifications_enabled: true,
            sound_enabled: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NotificationPermissionState {
    Granted,
    Denied,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationSettings {
    pub preferences: NotificationPreferences,
    pub permission_state: NotificationPermissionState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShortcutBindings {
    pub show_main_window: String,
    pub toggle_focus: String,
    pub create_quick_task: String,
    pub unlock_widget: String,
}

impl Default for ShortcutBindings {
    fn default() -> Self {
        Self {
            show_main_window: "Ctrl+Alt+A".into(),
            toggle_focus: "Ctrl+Alt+Space".into(),
            create_quick_task: "Ctrl+Alt+N".into(),
            unlock_widget: "Ctrl+Alt+U".into(),
        }
    }
}

impl ShortcutBindings {
    pub fn entries(&self) -> [(&'static str, &str); 4] {
        [
            ("showMainWindow", &self.show_main_window),
            ("toggleFocus", &self.toggle_focus),
            ("createQuickTask", &self.create_quick_task),
            ("unlockWidget", &self.unlock_widget),
        ]
    }

    pub fn validate(&self) -> Result<(), crate::DomainError> {
        let mut seen = std::collections::HashSet::new();
        for (field, value) in self.entries() {
            let normalized = value.trim().to_ascii_lowercase();
            if normalized.is_empty() {
                return Err(shortcut_error(
                    "SHORTCUT_INVALID",
                    "shortcut cannot be empty",
                    field,
                ));
            }
            if !seen.insert(normalized) {
                return Err(shortcut_error(
                    "SHORTCUT_DUPLICATE",
                    "each shortcut must use a different key combination",
                    field,
                ));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ShortcutPreferences {
    pub enabled: bool,
    pub bindings: ShortcutBindings,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DesktopIntegrationPreferences {
    pub shortcuts: ShortcutPreferences,
    pub autostart_enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopIntegrationSettings {
    pub shortcuts: ShortcutPreferences,
    pub autostart_enabled: bool,
    pub shortcut_error: Option<String>,
}

fn shortcut_error(code: &str, message: &str, field: &str) -> crate::DomainError {
    crate::DomainError {
        code: code.into(),
        message: message.into(),
        field: Some(field.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notification_preferences_default_to_enabled() {
        assert_eq!(
            NotificationPreferences::default(),
            NotificationPreferences {
                notifications_enabled: true,
                sound_enabled: true,
            }
        );
    }

    #[test]
    fn general_preferences_default_to_system_choices_and_background_running() {
        assert_eq!(
            GeneralPreferences::default(),
            GeneralPreferences {
                language: LanguagePreference::System,
                appearance: AppearancePreference::System,
                theme: ThemePreference::Mint,
                background_running: true,
            }
        );
        let value = serde_json::to_value(GeneralPreferences::default()).unwrap();
        assert_eq!(value["language"], "system");
        assert_eq!(value["appearance"], "system");
        assert_eq!(value["theme"], "mint");
        assert_eq!(value["backgroundRunning"], true);
    }

    #[test]
    fn general_preferences_patch_preserves_unspecified_fields() {
        let updated = GeneralPreferences::default().apply(GeneralPreferencesPatch {
            appearance: Some(AppearancePreference::Dark),
            theme: Some(ThemePreference::Office),
            ..GeneralPreferencesPatch::default()
        });
        assert_eq!(updated.language, LanguagePreference::System);
        assert_eq!(updated.appearance, AppearancePreference::Dark);
        assert_eq!(updated.theme, ThemePreference::Office);
        assert!(updated.background_running);
    }

    #[test]
    fn notification_preferences_use_camel_case_json() {
        let value = serde_json::to_value(NotificationPreferences::default()).unwrap();
        assert_eq!(value["notificationsEnabled"], true);
        assert_eq!(value["soundEnabled"], true);
    }

    #[test]
    fn shortcut_defaults_are_disabled_and_use_unique_bindings() {
        let preferences = ShortcutPreferences::default();
        assert!(!preferences.enabled);
        assert!(preferences.bindings.validate().is_ok());

        let value = serde_json::to_value(preferences).unwrap();
        assert_eq!(value["bindings"]["showMainWindow"], "Ctrl+Alt+A");
        assert_eq!(value["bindings"]["unlockWidget"], "Ctrl+Alt+U");
    }

    #[test]
    fn shortcut_bindings_reject_empty_and_duplicate_combinations() {
        let bindings = ShortcutBindings {
            toggle_focus: " ".into(),
            ..ShortcutBindings::default()
        };
        let empty = bindings.validate().unwrap_err();
        assert_eq!(empty.code, "SHORTCUT_INVALID");
        assert_eq!(empty.field.as_deref(), Some("toggleFocus"));

        let bindings = ShortcutBindings {
            create_quick_task: "ctrl+alt+a".into(),
            ..ShortcutBindings::default()
        };
        let duplicate = bindings.validate().unwrap_err();
        assert_eq!(duplicate.code, "SHORTCUT_DUPLICATE");
        assert_eq!(duplicate.field.as_deref(), Some("createQuickTask"));
    }
}
