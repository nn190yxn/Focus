use std::{sync::Mutex, time::Duration};

use serde::Serialize;
use tauri::Emitter;
use tauri_plugin_updater::{Update, UpdaterExt};

use crate::{desktop, CommandResult, DomainError};

const UPDATE_ENDPOINT: Option<&str> = option_env!("ARRIVE_FOCUS_UPDATE_ENDPOINT");
const UPDATE_PUBLIC_KEY: Option<&str> = option_env!("ARRIVE_FOCUS_UPDATE_PUBLIC_KEY");
const UPDATE_EVENT: &str = "update://download-progress";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateMetadata {
    pub current_version: String,
    pub version: String,
    pub notes: Option<String>,
    pub published_at: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateDownloadProgress {
    pub downloaded: u64,
    pub content_length: Option<u64>,
}

#[derive(Default)]
enum PendingUpdate {
    #[default]
    Empty,
    Checking,
    Available(Update),
    Downloading,
    Downloaded {
        update: Update,
        bytes: Vec<u8>,
    },
    Installing,
}

#[derive(Default)]
pub struct PendingUpdateState(Mutex<PendingUpdate>);

pub fn plugin<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R, tauri_plugin_updater::Config> {
    tauri_plugin_updater::Builder::new()
        .pubkey(UPDATE_PUBLIC_KEY.unwrap_or_default())
        .build()
}

#[tauri::command]
pub async fn update_check(
    app: tauri::AppHandle,
    state: tauri::State<'_, PendingUpdateState>,
) -> Result<CommandResult<Option<UpdateMetadata>>, ()> {
    Ok(result(check(&app, &state).await))
}

#[tauri::command]
pub async fn update_download(
    app: tauri::AppHandle,
    state: tauri::State<'_, PendingUpdateState>,
) -> Result<CommandResult<UpdateDownloadProgress>, ()> {
    Ok(result(download(&app, &state).await))
}

#[tauri::command]
pub fn update_install(
    app: tauri::AppHandle,
    state: tauri::State<'_, PendingUpdateState>,
) -> CommandResult<()> {
    let (update, bytes) = match take_downloaded(&state) {
        Ok(downloaded) => downloaded,
        Err(error) => return result(Err(error)),
    };

    let install_result = desktop::lifecycle::install_after_persist(
        || desktop::lifecycle::persist_before_exit(&app),
        || {
            update
                .install(&bytes)
                .map_err(|_| update_error("UPDATE_INSTALL_FAILED"))
        },
    );

    if let Err(error) = install_result {
        restore_downloaded(&state, update, bytes);
        return result(Err(error));
    }

    app.restart()
}

async fn check(
    app: &tauri::AppHandle,
    state: &PendingUpdateState,
) -> Result<Option<UpdateMetadata>, DomainError> {
    let endpoint = release_endpoint()?;
    transition_to_busy(state, PendingUpdate::Checking)?;

    let checked = async {
        let updater = app
            .updater_builder()
            .timeout(Duration::from_secs(30))
            .endpoints(vec![endpoint])
            .map_err(|_| update_error("UPDATE_CONFIGURATION_INVALID"))?
            .build()
            .map_err(|_| update_error("UPDATE_CONFIGURATION_INVALID"))?;
        updater
            .check()
            .await
            .map_err(|_| update_error("UPDATE_CHECK_FAILED"))
    }
    .await;

    match checked {
        Ok(Some(update)) => {
            let metadata = metadata(&update);
            replace_state(state, PendingUpdate::Available(update));
            Ok(Some(metadata))
        }
        Ok(None) => {
            replace_state(state, PendingUpdate::Empty);
            Ok(None)
        }
        Err(error) => {
            replace_state(state, PendingUpdate::Empty);
            Err(error)
        }
    }
}

async fn download(
    app: &tauri::AppHandle,
    state: &PendingUpdateState,
) -> Result<UpdateDownloadProgress, DomainError> {
    let update = take_available(state)?;
    let mut downloaded = 0_u64;
    let bytes = update
        .download(
            |chunk_length, content_length| {
                downloaded = downloaded.saturating_add(chunk_length as u64);
                let _ = app.emit(
                    UPDATE_EVENT,
                    UpdateDownloadProgress {
                        downloaded,
                        content_length,
                    },
                );
            },
            || {},
        )
        .await;

    match bytes {
        Ok(bytes) => {
            let progress = UpdateDownloadProgress {
                downloaded: bytes.len() as u64,
                content_length: Some(bytes.len() as u64),
            };
            replace_state(state, PendingUpdate::Downloaded { update, bytes });
            Ok(progress)
        }
        Err(_) => {
            replace_state(state, PendingUpdate::Available(update));
            Err(update_error("UPDATE_DOWNLOAD_FAILED"))
        }
    }
}

fn release_endpoint() -> Result<tauri::Url, DomainError> {
    let endpoint = UPDATE_ENDPOINT
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| update_error("UPDATE_NOT_CONFIGURED"))?;
    if !matches!(UPDATE_PUBLIC_KEY, Some(value) if !value.trim().is_empty()) {
        return Err(update_error("UPDATE_NOT_CONFIGURED"));
    }
    let endpoint = endpoint
        .parse::<tauri::Url>()
        .map_err(|_| update_error("UPDATE_CONFIGURATION_INVALID"))?;
    if endpoint.scheme() != "https" {
        return Err(update_error("UPDATE_CONFIGURATION_INVALID"));
    }
    Ok(endpoint)
}

fn transition_to_busy(state: &PendingUpdateState, next: PendingUpdate) -> Result<(), DomainError> {
    let mut current = state
        .0
        .lock()
        .map_err(|_| update_error("UPDATE_STATE_UNAVAILABLE"))?;
    if matches!(
        *current,
        PendingUpdate::Checking
            | PendingUpdate::Downloading
            | PendingUpdate::Downloaded { .. }
            | PendingUpdate::Installing
    ) {
        return Err(update_error("UPDATE_BUSY"));
    }
    *current = next;
    Ok(())
}

fn take_available(state: &PendingUpdateState) -> Result<Update, DomainError> {
    let mut current = state
        .0
        .lock()
        .map_err(|_| update_error("UPDATE_STATE_UNAVAILABLE"))?;
    match std::mem::replace(&mut *current, PendingUpdate::Downloading) {
        PendingUpdate::Available(update) => Ok(update),
        previous => {
            *current = previous;
            Err(update_error("UPDATE_NOT_AVAILABLE"))
        }
    }
}

fn take_downloaded(state: &PendingUpdateState) -> Result<(Update, Vec<u8>), DomainError> {
    let mut current = state
        .0
        .lock()
        .map_err(|_| update_error("UPDATE_STATE_UNAVAILABLE"))?;
    match std::mem::replace(&mut *current, PendingUpdate::Installing) {
        PendingUpdate::Downloaded { update, bytes } => Ok((update, bytes)),
        previous => {
            *current = previous;
            Err(update_error("UPDATE_NOT_DOWNLOADED"))
        }
    }
}

fn restore_downloaded(state: &PendingUpdateState, update: Update, bytes: Vec<u8>) {
    replace_state(state, PendingUpdate::Downloaded { update, bytes });
}

fn replace_state(state: &PendingUpdateState, next: PendingUpdate) {
    if let Ok(mut current) = state.0.lock() {
        *current = next;
    }
}

fn metadata(update: &Update) -> UpdateMetadata {
    UpdateMetadata {
        current_version: update.current_version.clone(),
        version: update.version.clone(),
        notes: update.body.clone(),
        published_at: update.date.map(|date| date.unix_timestamp()),
    }
}

fn update_error(code: &str) -> DomainError {
    DomainError {
        code: code.into(),
        message: "update operation failed".into(),
        field: None,
    }
}

fn result<T>(value: Result<T, DomainError>) -> CommandResult<T> {
    CommandResult::from_result(module_path!(), value, 1)
}
