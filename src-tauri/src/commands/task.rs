#[cfg(feature = "desktop-app")]
use crate::{
    domain::task::{TaskInput, TaskListFilter},
    repositories::{database::Database, task_repository::TaskListItem},
    services::task_service::{TaskDetail, TaskService},
    CommandResult, DomainError,
};

#[cfg(feature = "desktop-app")]
#[tauri::command]
pub fn task_list(
    database: tauri::State<'_, Database>,
    filter: TaskListFilter,
) -> CommandResult<Vec<TaskListItem>> {
    result(TaskService::new(&database).list(filter))
}

#[cfg(feature = "desktop-app")]
#[tauri::command]
pub fn task_get(database: tauri::State<'_, Database>, id: String) -> CommandResult<TaskDetail> {
    result(TaskService::new(&database).get(&id))
}

#[cfg(feature = "desktop-app")]
#[tauri::command]
pub fn task_create(
    database: tauri::State<'_, Database>,
    app: tauri::AppHandle,
    input: TaskInput,
    today: String,
) -> CommandResult<TaskDetail> {
    result_after_today_change(
        &app,
        parse_date(&today).and_then(|date| TaskService::new(&database).create(input, date)),
    )
}

#[cfg(feature = "desktop-app")]
#[tauri::command]
pub fn task_update(
    database: tauri::State<'_, Database>,
    app: tauri::AppHandle,
    id: String,
    input: TaskInput,
    today: String,
) -> CommandResult<TaskDetail> {
    result_after_today_change(
        &app,
        parse_date(&today).and_then(|date| TaskService::new(&database).update(&id, input, date)),
    )
}

#[cfg(feature = "desktop-app")]
#[tauri::command]
pub fn task_set_completed(
    database: tauri::State<'_, Database>,
    app: tauri::AppHandle,
    id: String,
    completed: bool,
) -> CommandResult<TaskDetail> {
    result_after_today_change(
        &app,
        TaskService::new(&database).set_completed(&id, completed),
    )
}

#[cfg(feature = "desktop-app")]
#[tauri::command]
pub fn task_remove(
    database: tauri::State<'_, Database>,
    app: tauri::AppHandle,
    id: String,
) -> CommandResult<()> {
    result_after_today_change(&app, TaskService::new(&database).remove(&id))
}

#[cfg(feature = "desktop-app")]
#[tauri::command]
pub fn task_set_check_item_completed(
    database: tauri::State<'_, Database>,
    app: tauri::AppHandle,
    task_id: String,
    item_id: String,
    completed: bool,
) -> CommandResult<TaskDetail> {
    result_after_today_change(
        &app,
        TaskService::new(&database).set_check_item_completed(&task_id, &item_id, completed),
    )
}

#[cfg(feature = "desktop-app")]
#[tauri::command]
pub fn task_reorder_check_items(
    database: tauri::State<'_, Database>,
    app: tauri::AppHandle,
    task_id: String,
    ordered_ids: Vec<String>,
) -> CommandResult<TaskDetail> {
    result_after_today_change(
        &app,
        TaskService::new(&database).reorder_check_items(&task_id, &ordered_ids),
    )
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
