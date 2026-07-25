use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

use tauri::Manager;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutEvent, ShortcutState};

use crate::domain::settings::{ShortcutBindings, ShortcutPreferences};
use crate::repositories::{database::Database, preferences_repository::PreferencesRepository};
use crate::DomainError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ShortcutAction {
    ShowMainWindow,
    ToggleFocus,
    CreateQuickTask,
    UnlockWidget,
}

impl ShortcutAction {
    fn field(self) -> &'static str {
        match self {
            Self::ShowMainWindow => "showMainWindow",
            Self::ToggleFocus => "toggleFocus",
            Self::CreateQuickTask => "createQuickTask",
            Self::UnlockWidget => "unlockWidget",
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct ParsedShortcut {
    action: ShortcutAction,
    shortcut: Shortcut,
}

#[derive(Default)]
pub struct ShortcutRuntime {
    actions: Mutex<HashMap<u32, ShortcutAction>>,
    active: Mutex<ShortcutPreferences>,
    last_error: Mutex<Option<String>>,
}

impl ShortcutRuntime {
    pub fn active_preferences(&self) -> ShortcutPreferences {
        self.active.lock().unwrap().clone()
    }

    pub fn last_error(&self) -> Option<String> {
        self.last_error.lock().unwrap().clone()
    }

    fn action_for(&self, shortcut: &Shortcut) -> Option<ShortcutAction> {
        self.actions.lock().unwrap().get(&shortcut.id()).copied()
    }

    fn apply(&self, preferences: ShortcutPreferences, parsed: &[ParsedShortcut]) {
        let actions = parsed
            .iter()
            .map(|item| (item.shortcut.id(), item.action))
            .collect();
        *self.actions.lock().unwrap() = actions;
        *self.active.lock().unwrap() = preferences;
        *self.last_error.lock().unwrap() = None;
    }

    fn set_error(&self, error: &DomainError) {
        *self.last_error.lock().unwrap() = Some(error.message.clone());
    }
}

pub fn plugin() -> tauri::plugin::TauriPlugin<tauri::Wry> {
    tauri_plugin_global_shortcut::Builder::new()
        .with_handler(handle_shortcut_event)
        .build()
}

pub fn initialize(app: &tauri::AppHandle) -> Result<(), DomainError> {
    let database = app.state::<Database>();
    let configured = PreferencesRepository::new(&database)
        .get_desktop_integration()?
        .shortcuts;
    match replace_shortcuts(app, &ShortcutPreferences::default(), &configured) {
        Ok(()) => Ok(()),
        Err(error) => {
            app.state::<ShortcutRuntime>().set_error(&error);
            Ok(())
        }
    }
}

pub fn replace_shortcuts(
    app: &tauri::AppHandle,
    current: &ShortcutPreferences,
    candidate: &ShortcutPreferences,
) -> Result<(), DomainError> {
    let current_parsed = parse_preferences(current)?;
    let candidate_parsed = parse_preferences(candidate)?;
    let registry = TauriShortcutRegistry { app };
    replace_registrations(&registry, &current_parsed, &candidate_parsed)?;
    app.state::<ShortcutRuntime>()
        .apply(candidate.clone(), &candidate_parsed);
    Ok(())
}

fn parse_preferences(
    preferences: &ShortcutPreferences,
) -> Result<Vec<ParsedShortcut>, DomainError> {
    preferences.bindings.validate()?;
    let parsed = parse_bindings(&preferences.bindings)?;
    if preferences.enabled {
        Ok(parsed)
    } else {
        Ok(Vec::new())
    }
}

fn parse_bindings(bindings: &ShortcutBindings) -> Result<Vec<ParsedShortcut>, DomainError> {
    let values = [
        (ShortcutAction::ShowMainWindow, &bindings.show_main_window),
        (ShortcutAction::ToggleFocus, &bindings.toggle_focus),
        (ShortcutAction::CreateQuickTask, &bindings.create_quick_task),
        (ShortcutAction::UnlockWidget, &bindings.unlock_widget),
    ];
    let mut ids = HashSet::new();
    values
        .into_iter()
        .map(|(action, value)| {
            let shortcut = value.parse::<Shortcut>().map_err(|error| DomainError {
                code: "SHORTCUT_INVALID".into(),
                message: format!("快捷键 {value} 无法识别：{error}"),
                field: Some(action.field().into()),
            })?;
            if !ids.insert(shortcut.id()) {
                return Err(DomainError {
                    code: "SHORTCUT_DUPLICATE".into(),
                    message: "每项操作需要使用不同的快捷键".into(),
                    field: Some(action.field().into()),
                });
            }
            Ok(ParsedShortcut { action, shortcut })
        })
        .collect()
}

trait ShortcutRegistry {
    fn register(&self, shortcut: Shortcut) -> Result<(), String>;
    fn unregister(&self, shortcut: Shortcut) -> Result<(), String>;
}

struct TauriShortcutRegistry<'a> {
    app: &'a tauri::AppHandle,
}

impl ShortcutRegistry for TauriShortcutRegistry<'_> {
    fn register(&self, shortcut: Shortcut) -> Result<(), String> {
        self.app
            .global_shortcut()
            .register(shortcut)
            .map_err(|error| error.to_string())
    }

    fn unregister(&self, shortcut: Shortcut) -> Result<(), String> {
        self.app
            .global_shortcut()
            .unregister(shortcut)
            .map_err(|error| error.to_string())
    }
}

fn replace_registrations(
    registry: &impl ShortcutRegistry,
    current: &[ParsedShortcut],
    candidate: &[ParsedShortcut],
) -> Result<(), DomainError> {
    let current_ids: HashSet<_> = current.iter().map(|item| item.shortcut.id()).collect();
    let candidate_ids: HashSet<_> = candidate.iter().map(|item| item.shortcut.id()).collect();
    let mut registered = Vec::new();

    for item in candidate {
        if current_ids.contains(&item.shortcut.id()) {
            continue;
        }
        if let Err(reason) = registry.register(item.shortcut) {
            for shortcut in registered.iter().rev() {
                let _ = registry.unregister(*shortcut);
            }
            return Err(DomainError {
                code: "SHORTCUT_CONFLICT".into(),
                message: format!("快捷键 {} 已被系统或其他应用占用：{reason}", item.shortcut),
                field: Some(item.action.field().into()),
            });
        }
        registered.push(item.shortcut);
    }

    let mut unregistered = Vec::new();
    for item in current {
        if candidate_ids.contains(&item.shortcut.id()) {
            continue;
        }
        if let Err(reason) = registry.unregister(item.shortcut) {
            for shortcut in unregistered.iter().rev() {
                let _ = registry.register(*shortcut);
            }
            for shortcut in registered.iter().rev() {
                let _ = registry.unregister(*shortcut);
            }
            return Err(DomainError {
                code: "SHORTCUT_OPERATION_FAILED".into(),
                message: format!("快捷键切换失败：{reason}"),
                field: Some(item.action.field().into()),
            });
        }
        unregistered.push(item.shortcut);
    }
    Ok(())
}

fn handle_shortcut_event(app: &tauri::AppHandle, shortcut: &Shortcut, event: ShortcutEvent) {
    if event.state != ShortcutState::Pressed {
        return;
    }
    let action = app.state::<ShortcutRuntime>().action_for(shortcut);
    let result = match action {
        Some(ShortcutAction::ShowMainWindow) => crate::desktop::tray::show_main_window(app),
        Some(ShortcutAction::ToggleFocus) => crate::desktop::tray::toggle_focus(app),
        Some(ShortcutAction::CreateQuickTask) => crate::desktop::tray::open_quick_task(app),
        Some(ShortcutAction::UnlockWidget) => crate::desktop::tray::unlock_widget(app),
        None => Ok(()),
    };
    if let Err(error) = result {
        app.state::<ShortcutRuntime>().set_error(&error);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    #[derive(Default)]
    struct FakeRegistry {
        registered: Mutex<Vec<u32>>,
        conflict: Mutex<Option<u32>>,
    }

    impl ShortcutRegistry for FakeRegistry {
        fn register(&self, shortcut: Shortcut) -> Result<(), String> {
            if *self.conflict.lock().unwrap() == Some(shortcut.id()) {
                return Err("already registered".into());
            }
            self.registered.lock().unwrap().push(shortcut.id());
            Ok(())
        }

        fn unregister(&self, shortcut: Shortcut) -> Result<(), String> {
            let mut registered = self.registered.lock().unwrap();
            if let Some(index) = registered.iter().position(|id| *id == shortcut.id()) {
                registered.remove(index);
            }
            Ok(())
        }
    }

    fn enabled(bindings: ShortcutBindings) -> ShortcutPreferences {
        ShortcutPreferences {
            enabled: true,
            bindings,
        }
    }

    #[test]
    fn invalid_shortcut_reports_the_matching_field() {
        let bindings = ShortcutBindings {
            toggle_focus: "Ctrl+NoSuchKey".into(),
            ..ShortcutBindings::default()
        };
        let error = parse_preferences(&enabled(bindings)).unwrap_err();
        assert_eq!(error.code, "SHORTCUT_INVALID");
        assert_eq!(error.field.as_deref(), Some("toggleFocus"));
    }

    #[test]
    fn successful_replacement_registers_new_before_releasing_old() {
        let current = parse_preferences(&enabled(ShortcutBindings::default())).unwrap();
        let changed = ShortcutBindings {
            toggle_focus: "Ctrl+Shift+Space".into(),
            ..ShortcutBindings::default()
        };
        let candidate = parse_preferences(&enabled(changed)).unwrap();
        let registry = FakeRegistry::default();
        for item in &current {
            registry.register(item.shortcut).unwrap();
        }

        replace_registrations(&registry, &current, &candidate).unwrap();
        let registered = registry.registered.lock().unwrap();
        assert_eq!(registered.len(), 4);
        assert!(candidate
            .iter()
            .all(|item| registered.contains(&item.shortcut.id())));
    }

    #[test]
    fn conflict_rolls_back_candidates_and_keeps_all_old_shortcuts() {
        let current = parse_preferences(&enabled(ShortcutBindings::default())).unwrap();
        let changed = ShortcutBindings {
            show_main_window: "Ctrl+Shift+A".into(),
            toggle_focus: "Ctrl+Shift+Space".into(),
            ..ShortcutBindings::default()
        };
        let candidate = parse_preferences(&enabled(changed)).unwrap();
        let registry = FakeRegistry::default();
        for item in &current {
            registry.register(item.shortcut).unwrap();
        }
        *registry.conflict.lock().unwrap() = Some(candidate[1].shortcut.id());

        let error = replace_registrations(&registry, &current, &candidate).unwrap_err();
        assert_eq!(error.code, "SHORTCUT_CONFLICT");
        assert_eq!(error.field.as_deref(), Some("toggleFocus"));
        let registered = registry.registered.lock().unwrap();
        assert_eq!(registered.len(), 4);
        assert!(current
            .iter()
            .all(|item| registered.contains(&item.shortcut.id())));
    }
}
