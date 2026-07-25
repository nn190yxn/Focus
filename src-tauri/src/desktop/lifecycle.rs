#[cfg(any(feature = "desktop-app", test))]
use crate::{repositories::database::Database, services::focus_service::FocusService, DomainError};

#[cfg(feature = "desktop-app")]
pub fn persist_before_exit(app: &tauri::AppHandle) -> Result<(), DomainError> {
    use tauri::Manager;

    let database = app.state::<Database>();
    persist_focus_state(&database)?;
    super::main_window::persist_main_window_state(app)?;
    allow_missing_widget_window(super::widget_window::persist_widget_geometry(app))
}

#[cfg(any(feature = "desktop-app", test))]
fn allow_missing_widget_window(result: Result<(), DomainError>) -> Result<(), DomainError> {
    match result {
        Err(error) if error.code == "WIDGET_WINDOW_MISSING" => Ok(()),
        other => other,
    }
}

#[cfg(any(feature = "desktop-app", test))]
fn persist_focus_state(database: &Database) -> Result<(), DomainError> {
    FocusService::new(database).reconcile()?;
    Ok(())
}

#[cfg(feature = "desktop-app")]
pub fn request_exit(app: &tauri::AppHandle, code: i32) -> Result<(), DomainError> {
    persist_before_exit(app)?;
    app.exit(code);
    Ok(())
}

#[cfg(any(feature = "desktop-app", test))]
pub(crate) fn install_after_persist(
    persist: impl FnOnce() -> Result<(), DomainError>,
    install: impl FnOnce() -> Result<(), DomainError>,
) -> Result<(), DomainError> {
    persist()?;
    install()
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use chrono::{Duration, Utc};

    use super::*;
    use crate::{
        domain::focus::{ActiveFocus, ActiveFocusStatus, FocusTarget},
        repositories::focus_repository::FocusRepository,
    };

    #[test]
    fn exit_persistence_preserves_paused_focus_state() {
        let database = Database::open_in_memory().unwrap();
        let now = Utc::now();
        database
            .write(|tx| {
                tx.execute(
                    "INSERT INTO tasks(id, title, category, priority, status, created_at, updated_at) VALUES (?1, ?2, ?3, 0, 'pending', ?4, ?4)",
                    rusqlite::params!["task-1", "Exit persistence", "work", now.to_rfc3339()],
                )?;
                Ok(())
            })
            .unwrap();
        let mut focus = ActiveFocus::start(
            FocusTarget {
                task_id: Some("task-1".into()),
                task_instance_id: None,
            },
            25,
            now,
        )
        .unwrap();
        focus.pause(now + Duration::seconds(30)).unwrap();
        FocusRepository::new(&database)
            .insert_active(&focus)
            .unwrap();

        persist_focus_state(&database).unwrap();

        let restored = FocusRepository::new(&database)
            .get_active()
            .unwrap()
            .unwrap();
        assert_eq!(restored.status, ActiveFocusStatus::Paused);
        assert_eq!(restored.remaining_seconds, 1470);
    }

    #[test]
    fn update_install_persists_before_starting_the_installer() {
        let calls = Rc::new(RefCell::new(Vec::new()));
        let persist_calls = Rc::clone(&calls);
        let install_calls = Rc::clone(&calls);

        install_after_persist(
            || {
                persist_calls.borrow_mut().push("persist");
                Ok(())
            },
            || {
                install_calls.borrow_mut().push("install");
                Ok(())
            },
        )
        .expect("update installation should succeed");

        assert_eq!(*calls.borrow(), ["persist", "install"]);
    }

    #[test]
    fn update_install_stops_when_exit_persistence_fails() {
        let installer_called = Rc::new(RefCell::new(false));
        let called = Rc::clone(&installer_called);
        let persistence_error = DomainError {
            code: "MAIN_WINDOW_STATE_SAVE_FAILED".into(),
            message: "window state save failed".into(),
            field: None,
        };

        let error = install_after_persist(
            || Err(persistence_error),
            || {
                *called.borrow_mut() = true;
                Ok(())
            },
        )
        .expect_err("persistence failure should stop installation");

        assert_eq!(error.code, "MAIN_WINDOW_STATE_SAVE_FAILED");
        assert!(!*installer_called.borrow());
    }

    #[test]
    fn missing_widget_window_does_not_block_exit_persistence() {
        assert_eq!(
            allow_missing_widget_window(Err(DomainError {
                code: "WIDGET_WINDOW_MISSING".into(),
                message: "widget window is unavailable".into(),
                field: None,
            })),
            Ok(())
        );

        let operation_error = DomainError {
            code: "WIDGET_WINDOW_OPERATION_FAILED".into(),
            message: "window state save failed".into(),
            field: None,
        };
        assert_eq!(
            allow_missing_widget_window(Err(operation_error.clone())),
            Err(operation_error)
        );
    }
}
