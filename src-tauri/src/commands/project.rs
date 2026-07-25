#[cfg(feature = "desktop-app")]
use crate::{
    domain::project::{ProjectInput, ProjectRemovalResolution, ProjectStatus},
    repositories::{database::Database, project_repository::ProjectRecord},
    services::project_service::{ProjectDetail, ProjectService, ProjectSummary},
    CommandResult, DomainError,
};

#[cfg(feature = "desktop-app")]
#[tauri::command]
pub fn project_create(
    database: tauri::State<'_, Database>,
    app: tauri::AppHandle,
    input: ProjectInput,
) -> CommandResult<ProjectRecord> {
    result_after_today_change(&app, ProjectService::new(&database).create(input))
}

#[cfg(feature = "desktop-app")]
#[tauri::command]
pub fn project_update(
    database: tauri::State<'_, Database>,
    app: tauri::AppHandle,
    id: String,
    input: ProjectInput,
) -> CommandResult<ProjectRecord> {
    result_after_today_change(&app, ProjectService::new(&database).update(&id, input))
}

#[cfg(feature = "desktop-app")]
#[tauri::command]
pub fn project_set_status(
    database: tauri::State<'_, Database>,
    app: tauri::AppHandle,
    id: String,
    status: ProjectStatus,
) -> CommandResult<ProjectRecord> {
    result_after_today_change(&app, ProjectService::new(&database).set_status(&id, status))
}

#[cfg(feature = "desktop-app")]
#[tauri::command]
pub fn project_remove(
    database: tauri::State<'_, Database>,
    app: tauri::AppHandle,
    id: String,
    resolution: ProjectRemovalResolution,
) -> CommandResult<()> {
    result_after_today_change(&app, ProjectService::new(&database).remove(&id, resolution))
}

#[cfg(feature = "desktop-app")]
#[tauri::command]
pub fn project_list(
    database: tauri::State<'_, Database>,
    status: Option<ProjectStatus>,
    today: String,
) -> CommandResult<Vec<ProjectSummary>> {
    result(parse_date(&today).and_then(|date| ProjectService::new(&database).list(status, date)))
}

#[cfg(feature = "desktop-app")]
#[tauri::command]
pub fn project_get(
    database: tauri::State<'_, Database>,
    id: String,
    today: String,
) -> CommandResult<ProjectDetail> {
    result(parse_date(&today).and_then(|date| ProjectService::new(&database).get(&id, date)))
}

#[cfg(feature = "desktop-app")]
fn parse_date(value: &str) -> Result<chrono::NaiveDate, DomainError> {
    chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d").map_err(|_| DomainError {
        code: "DATE_INVALID".into(),
        message: "date must use YYYY-MM-DD".into(),
        field: Some("today".into()),
    })
}

#[cfg(feature = "desktop-app")]
fn result<T>(value: Result<T, DomainError>) -> CommandResult<T> {
    CommandResult::from_result(module_path!(), value, 1)
}

#[cfg(feature = "desktop-app")]
fn result_after_today_change<T>(
    app: &tauri::AppHandle,
    value: Result<T, DomainError>,
) -> CommandResult<T> {
    result(crate::desktop::today_events::after_today_change(
        value,
        || {
            crate::desktop::today_events::emit_today_changed(app);
        },
    ))
}
