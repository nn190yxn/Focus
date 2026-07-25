#[cfg(feature = "desktop-app")]
use crate::{
    domain::recurrence::{RecurrenceChangeScope, RecurrenceRule, RecurrenceStatus},
    repositories::{database::Database, recurrence_repository::TaskInstanceRecord},
    services::recurrence_service::{GenerationSummary, RecurrenceService},
    CommandResult, DomainError,
};

#[cfg(feature = "desktop-app")]
#[tauri::command]
pub fn recurrence_get(
    database: tauri::State<'_, Database>,
    rule_id: String,
) -> CommandResult<RecurrenceRule> {
    result(RecurrenceService::new(&database).get_rule(&rule_id))
}

#[cfg(feature = "desktop-app")]
#[tauri::command]
pub fn recurrence_create(
    database: tauri::State<'_, Database>,
    app: tauri::AppHandle,
    rule: RecurrenceRule,
    range_start: String,
    range_end: String,
) -> CommandResult<GenerationSummary> {
    result_after_today_change(
        &app,
        parse_date(&range_start, "rangeStart").and_then(|start| {
            parse_date(&range_end, "rangeEnd")
                .and_then(|end| RecurrenceService::new(&database).create_rule(rule, start, end))
        }),
    )
}

#[cfg(feature = "desktop-app")]
#[tauri::command]
pub fn recurrence_update(
    database: tauri::State<'_, Database>,
    app: tauri::AppHandle,
    proposed: RecurrenceRule,
    scope: RecurrenceChangeScope,
    range_end: String,
) -> CommandResult<GenerationSummary> {
    result_after_today_change(
        &app,
        parse_date(&range_end, "rangeEnd").and_then(|end| {
            RecurrenceService::new(&database).apply_schedule_change(proposed, scope, end)
        }),
    )
}

#[cfg(feature = "desktop-app")]
#[tauri::command]
pub fn recurrence_set_status(
    database: tauri::State<'_, Database>,
    app: tauri::AppHandle,
    rule_id: String,
    status: RecurrenceStatus,
) -> CommandResult<RecurrenceRule> {
    result_after_today_change(
        &app,
        RecurrenceService::new(&database).set_rule_status(&rule_id, status),
    )
}

#[cfg(feature = "desktop-app")]
#[tauri::command]
pub fn instance_complete(
    database: tauri::State<'_, Database>,
    app: tauri::AppHandle,
    instance_id: String,
) -> CommandResult<TaskInstanceRecord> {
    result_after_today_change(
        &app,
        RecurrenceService::new(&database).complete_instance(&instance_id),
    )
}

#[cfg(feature = "desktop-app")]
#[tauri::command]
pub fn instance_skip(
    database: tauri::State<'_, Database>,
    app: tauri::AppHandle,
    instance_id: String,
) -> CommandResult<TaskInstanceRecord> {
    result_after_today_change(
        &app,
        RecurrenceService::new(&database).skip_instance(&instance_id),
    )
}

#[cfg(feature = "desktop-app")]
#[tauri::command]
pub fn instance_delay_today(
    database: tauri::State<'_, Database>,
    app: tauri::AppHandle,
    instance_id: String,
    local_time: String,
) -> CommandResult<TaskInstanceRecord> {
    result_after_today_change(
        &app,
        RecurrenceService::new(&database).delay_instance_today(&instance_id, &local_time),
    )
}

#[cfg(feature = "desktop-app")]
#[tauri::command]
pub fn instance_reschedule_tomorrow(
    database: tauri::State<'_, Database>,
    app: tauri::AppHandle,
    instance_id: String,
) -> CommandResult<TaskInstanceRecord> {
    result_after_today_change(
        &app,
        RecurrenceService::new(&database).reschedule_instance_tomorrow(&instance_id),
    )
}

#[cfg(feature = "desktop-app")]
fn parse_date(value: &str, field: &str) -> Result<chrono::NaiveDate, DomainError> {
    chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d").map_err(|_| DomainError {
        code: "RECURRENCE_DATE_INVALID".into(),
        message: "date must use YYYY-MM-DD".into(),
        field: Some(field.into()),
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
