use std::{path::PathBuf, sync::Mutex};

use tauri::{Emitter, Manager};
use tauri_plugin_dialog::DialogExt;
use uuid::Uuid;

use crate::{
    domain::backup::{BackupExportResult, BackupInspection, BackupRestoreResult, ValidatedBackup},
    repositories::database::Database,
    services::backup_service::BackupService,
    CommandResult, DomainError,
};

pub const BACKUP_RESTORED_EVENT: &str = "backup://restored";

struct PendingRestore {
    token: String,
    source_path: PathBuf,
    backup: ValidatedBackup,
}

#[derive(Default)]
pub struct BackupRestoreState {
    pending: Mutex<Option<PendingRestore>>,
}

#[tauri::command]
pub fn backup_export(
    database: tauri::State<'_, Database>,
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
) -> CommandResult<Option<BackupExportResult>> {
    result((|| {
        ensure_main_window(&window)?;
        let Some(file_path) = app
            .dialog()
            .file()
            .set_file_name("arrive-focus-backup.json")
            .add_filter("JSON", &["json"])
            .blocking_save_file()
        else {
            return Ok(None);
        };
        let path = file_path.into_path().map_err(invalid_dialog_path)?;
        BackupService::new(&database)
            .export_to_path(&path, chrono::Utc::now())
            .map(Some)
    })())
}

#[tauri::command]
pub fn backup_inspect(
    restore_state: tauri::State<'_, BackupRestoreState>,
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
) -> CommandResult<Option<BackupInspection>> {
    result((|| {
        ensure_main_window(&window)?;
        let Some(file_path) = app
            .dialog()
            .file()
            .add_filter("JSON", &["json"])
            .blocking_pick_file()
        else {
            return Ok(None);
        };
        let path = file_path.into_path().map_err(invalid_dialog_path)?;
        let backup = BackupService::inspect_path(&path)?;
        let token = Uuid::new_v4().to_string();
        let inspection = BackupInspection {
            token: token.clone(),
            path: path.to_string_lossy().into_owned(),
            format_version: backup.envelope.format_version,
            exported_at: backup.envelope.exported_at.clone(),
            summary: backup.summary.clone(),
        };
        let mut pending = restore_state
            .pending
            .lock()
            .map_err(|_| state_lock_error())?;
        *pending = Some(PendingRestore {
            token,
            source_path: path,
            backup,
        });
        Ok(Some(inspection))
    })())
}

#[tauri::command]
pub fn backup_restore(
    database: tauri::State<'_, Database>,
    restore_state: tauri::State<'_, BackupRestoreState>,
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    token: String,
) -> CommandResult<BackupRestoreResult> {
    result((|| {
        ensure_main_window(&window)?;
        let pending = {
            let mut state = restore_state
                .pending
                .lock()
                .map_err(|_| state_lock_error())?;
            if state.as_ref().map(|item| item.token.as_str()) != Some(token.as_str()) {
                return Err(DomainError {
                    code: "BACKUP_CONFIRMATION_INVALID".into(),
                    message: "backup confirmation is missing or expired".into(),
                    field: Some("token".into()),
                });
            }
            state.take().expect("validated pending restore")
        };
        let snapshot_directory = app.path().app_data_dir().map_err(|error| DomainError {
            code: "BACKUP_SNAPSHOT_FAILED".into(),
            message: error.to_string(),
            field: Some("snapshotDirectory".into()),
        })?;
        let restore_result = BackupService::new(&database).restore(
            pending.backup.clone(),
            &pending.source_path,
            &snapshot_directory.join("backups"),
            chrono::Utc::now(),
        );
        let restored = match restore_result {
            Ok(restored) => restored,
            Err(error) => {
                let mut state = restore_state
                    .pending
                    .lock()
                    .map_err(|_| state_lock_error())?;
                if state.is_none() {
                    *state = Some(pending);
                }
                return Err(error);
            }
        };
        let _ = app.emit(BACKUP_RESTORED_EVENT, &restored);
        let _ = super::settings::emit_current_settings(&app, &database);
        Ok(restored)
    })())
}

fn ensure_main_window(window: &tauri::WebviewWindow) -> Result<(), DomainError> {
    if window.label() == "main" {
        return Ok(());
    }
    Err(DomainError {
        code: "BACKUP_WINDOW_FORBIDDEN".into(),
        message: "backup operations are only available from the main window".into(),
        field: None,
    })
}

fn invalid_dialog_path(error: impl std::fmt::Display) -> DomainError {
    DomainError {
        code: "BACKUP_PATH_INVALID".into(),
        message: error.to_string(),
        field: Some("path".into()),
    }
}

fn state_lock_error() -> DomainError {
    DomainError {
        code: "BACKUP_STATE_UNAVAILABLE".into(),
        message: "backup restore state is unavailable".into(),
        field: None,
    }
}

fn result<T>(value: Result<T, DomainError>) -> CommandResult<T> {
    CommandResult::from_result(module_path!(), value, 1)
}
